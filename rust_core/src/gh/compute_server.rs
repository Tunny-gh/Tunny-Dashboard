//! rhino.compute のローカルプロセス起動・停止の管理。
//!
//! Compute の接続先は「稼働中サーバーの URL」に加えて「rhino.compute の
//! 実行ファイルパス」を受け付ける。EXE 指定の場合は Dashboard が
//! `--port` 付きでプロセスを起動し、HTTP が応答するまで待ってから
//! 評価に使う。返される `ComputeServerHandle` の Drop でプロセスを停止する
//! （最適化ループの終了・エラー・キャンセルで確実に片付く）。
//!
//! 制限: 停止するのは起動した親プロセスのみ。rhino.compute が生成する
//! compute.geometry 子プロセスは rhino.compute 自身のシャットダウン処理に
//! 委ねる。

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::surrogate_opt::FitProgress;

/// ユーザー入力（URL または EXE パス）の解釈結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeTarget {
    /// 稼働中の rhino.compute サーバーの URL（例 `http://localhost:6500`）
    Url(String),
    /// rhino.compute 実行ファイルのパス。Dashboard が起動・停止を管理する
    Exe(PathBuf),
}

/// Compute 接続先入力を分類する。`http://` / `https://`（大文字小文字無視)で
/// 始まれば URL、それ以外はローカル実行ファイルのパスとみなす。
pub fn classify_compute_input(input: &str) -> ComputeTarget {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        ComputeTarget::Url(trimmed.trim_end_matches('/').to_string())
    } else {
        ComputeTarget::Exe(PathBuf::from(trimmed))
    }
}

/// 起動した rhino.compute プロセスのハンドル。Drop で kill + wait する。
#[derive(Debug)]
pub struct ComputeServerHandle {
    child: Child,
    url: String,
}

impl ComputeServerHandle {
    /// 接続先 URL（`http://localhost:<port>`）。
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for ComputeServerHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// rhino.compute を起動し、HTTP が応答するまで待つ。
///
/// - `--port <port>` を引数に渡し、作業ディレクトリは EXE の親ディレクトリ
/// - rhino.compute の初回起動は Rhino のロードで数十秒かかることがあるため、
///   `startup_timeout_secs` は余裕を持たせる（UI 側の既定は 180 秒）
/// - `should_abort` が true を返したら待機を中断する（キャンセル対応）。
///   中断・失敗時は戻り値の Drop 経路と同様にプロセスを停止して返す
pub fn start_compute_server(
    exe_path: &Path,
    port: u16,
    startup_timeout_secs: u64,
    should_abort: &(dyn Fn() -> bool + Sync),
) -> Result<ComputeServerHandle, String> {
    if !exe_path.is_file() {
        return Err(format!(
            "rhino.compute の実行ファイルが見つかりません: {}",
            exe_path.display()
        ));
    }
    let url = format!("http://localhost:{port}");
    let mut cmd = Command::new(exe_path);
    cmd.arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = exe_path.parent() {
        if !dir.as_os_str().is_empty() {
            cmd.current_dir(dir);
        }
    }
    let child = cmd.spawn().map_err(|e| {
        format!(
            "rhino.compute の起動に失敗しました（{}）: {e}",
            exe_path.display()
        )
    })?;
    // 以降のエラーは handle の Drop がプロセスを停止する。
    let mut handle = ComputeServerHandle { child, url };
    wait_until_ready(&mut handle, startup_timeout_secs, should_abort)?;
    Ok(handle)
}

/// `FitProgress` にステージ表示とキャンセルを紐付けて rhino.compute を起動する。
///
/// UI から呼ぶための薄いラッパ（ステージ設定はクレート内限定 API のため
/// ここで行う）。起動待機中は「rhino.compute を起動中…」を表示し、
/// `progress.request_cancel()` で待機を中断できる。
pub fn start_compute_server_tracked(
    exe_path: &Path,
    port: u16,
    startup_timeout_secs: u64,
    progress: &FitProgress,
) -> Result<ComputeServerHandle, String> {
    progress.set_stage("rhino.compute を起動中…");
    start_compute_server(exe_path, port, startup_timeout_secs, &|| {
        progress.is_cancelled()
    })
}

/// HTTP が応答するまでポーリングする。
///
/// `/healthcheck` へ GET し、HTTP 応答が返れば（ステータスによらず）起動済みと
/// みなす。接続エラーの間は 500ms 間隔で再試行し、子プロセスの早期終了と
/// タイムアウトを検出する。
fn wait_until_ready(
    handle: &mut ComputeServerHandle,
    timeout_secs: u64,
    should_abort: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(2))
        .build();
    let health_url = format!("{}/healthcheck", handle.url);
    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
    loop {
        if should_abort() {
            return Err("rhino.compute の起動待機がキャンセルされました".to_string());
        }
        // 起動直後にクラッシュ・即終了した場合はタイムアウトを待たずに報告する。
        if let Ok(Some(status)) = handle.child.try_wait() {
            return Err(format!(
                "rhino.compute が起動直後に終了しました（{status}）。\
                 パスとポート {} の使用状況を確認してください",
                handle.url
            ));
        }
        match agent.get(&health_url).call() {
            // HTTP 応答が返ればサーバーは生きている（healthcheck 未実装の
            // ビルドでも 404 等が返れば起動済みとみなせる）。
            Ok(_) | Err(ureq::Error::Status(_, _)) => return Ok(()),
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "rhino.compute が {timeout_secs} 秒以内に応答しませんでした（{}）",
                handle.url
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_url_and_exe_inputs() {
        assert_eq!(
            classify_compute_input(" http://localhost:6500/ "),
            ComputeTarget::Url("http://localhost:6500".to_string())
        );
        assert_eq!(
            classify_compute_input("HTTPS://compute.example.com"),
            ComputeTarget::Url("HTTPS://compute.example.com".to_string())
        );
        assert_eq!(
            classify_compute_input(r"C:\Program Files\rhino.compute\rhino.compute.exe"),
            ComputeTarget::Exe(PathBuf::from(
                r"C:\Program Files\rhino.compute\rhino.compute.exe"
            ))
        );
        assert_eq!(
            classify_compute_input("/opt/compute/rhino.compute"),
            ComputeTarget::Exe(PathBuf::from("/opt/compute/rhino.compute"))
        );
    }

    #[test]
    fn missing_exe_is_reported() {
        let err = start_compute_server(
            Path::new("/nonexistent/rhino.compute.exe"),
            65001,
            1,
            &|| false,
        )
        .unwrap_err();
        assert!(err.contains("見つかりません"), "unexpected: {err}");
    }

    // 以下はプロセス起動を伴うテスト。シェルスクリプトを疑似 EXE として使うため
    // unix（Linux / macOS CI）のみで実行する。Windows CI では起動経路は
    // 実 rhino.compute での手動確認に委ねる。
    #[cfg(unix)]
    mod process_tests {
        use super::super::*;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        /// 実行可能な一時スクリプトを作る。
        fn write_script(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
            let path = dir.join("fake_compute.sh");
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "#!/bin/sh\n{body}").unwrap();
            let mut perm = f.metadata().unwrap().permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&path, perm).unwrap();
            path
        }

        #[test]
        fn early_exit_is_detected_before_timeout() {
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "exit 7");
            let start = std::time::Instant::now();
            let err = start_compute_server(&exe, 65002, 60, &|| false).unwrap_err();
            assert!(err.contains("終了しました"), "unexpected: {err}");
            // 60 秒のタイムアウトを待たずに返ること
            assert!(start.elapsed() < Duration::from_secs(30));
        }

        #[test]
        fn timeout_kills_child_and_reports() {
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "while true; do sleep 1; done");
            let err = start_compute_server(&exe, 65003, 1, &|| false).unwrap_err();
            assert!(err.contains("応答しませんでした"), "unexpected: {err}");
        }

        #[test]
        fn abort_stops_waiting() {
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "while true; do sleep 1; done");
            let err = start_compute_server(&exe, 65004, 60, &|| true).unwrap_err();
            assert!(err.contains("キャンセル"), "unexpected: {err}");
        }

        #[test]
        fn ready_when_http_responds() {
            // 空きポートで疑似 HTTP サーバーを立て、そのポートを指定して起動。
            // 子プロセス自体は待機するだけのスクリプトだが、HTTP 応答があれば
            // ready と判定される（healthcheck の応答内容は問わない）。
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            std::thread::spawn(move || {
                if let Ok((mut stream, _)) = listener.accept() {
                    use std::io::{Read, Write};
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf);
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nhealthy",
                    );
                }
            });
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "while true; do sleep 1; done");
            let handle = start_compute_server(&exe, port, 30, &|| false).unwrap();
            assert_eq!(handle.url(), format!("http://localhost:{port}"));
            // Drop で子プロセスが停止する（ハングしないことで確認）。
            drop(handle);
        }
    }
}
