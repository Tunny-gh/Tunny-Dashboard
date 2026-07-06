use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// ネイティブファイルダイアログで最適化結果ファイルを選択する。
/// `.log` は Optuna の Journal ストレージ、`.csv` は DesignExplorer 向け形式、
/// `.db`/`.sqlite`/`.sqlite3` は Optuna の RDB（SQLite）ストレージであることが
/// フィルタ名から分かるようにする。
pub fn open_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(
            "Optimization Result (Optuna .log / DesignExplorer .csv / Optuna SQLite .db/.sqlite/.sqlite3)",
            &["log", "csv", "db", "sqlite", "sqlite3"],
        )
        .add_filter("Optuna Result (*.log)", &["log"])
        .add_filter("DesignExplorer (*.csv)", &["csv"])
        .add_filter(
            "Optuna SQLite (.db/.sqlite/.sqlite3)",
            &["db", "sqlite", "sqlite3"],
        )
        .pick_file()
}

/// ファイルをバイト列として読み込む
pub fn read_journal_file(path: &PathBuf) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}

/// 同一プロセス内での一時ファイル名衝突を防ぐ連番。
static ATOMIC_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 一時ファイルへ書き込んでから `rename` するアトミック書き込み。
///
/// `std::fs::write` は truncate → 書き込みの 2 段階のため、上書き保存中の
/// ディスク満杯・クラッシュで既存ファイルが破損・消失する（品質レビュー M-2）。
/// 本関数は書き込みが完全に成功してから rename で置き換えるので、途中失敗しても
/// 既存ファイルは無傷のまま残る。
///
/// 一時ファイルは対象と同じディレクトリに作る（別ファイルシステムを跨ぐ
/// rename は失敗するため、`std::env::temp_dir()` などは使わない）。
pub fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    // 親ディレクトリ（相対パスの単独ファイル名なら現在ディレクトリ扱い）。
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    // 一意な一時ファイル名（pid + 連番で同時書き込み・再入の衝突を防ぐ）。
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let tmp_path = dir.join(format!(
        ".{}.{}-{}.tmp",
        file_name,
        std::process::id(),
        ATOMIC_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    std::fs::write(&tmp_path, contents)?;
    match std::fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // rename に失敗したら一時ファイルを残さない（掃除失敗は無視）。
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_existing_file_succeeds() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "test content").unwrap();
        let path = tmp.path().to_path_buf();
        let result = read_journal_file(&path);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn read_nonexistent_file_returns_err() {
        let path = PathBuf::from("/nonexistent/path/file.log");
        let result = read_journal_file(&path);
        assert!(result.is_err());
    }

    // ── write_atomic（M-2: アトミック書き込み） ──────────────────

    #[test]
    fn write_atomic_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        write_atomic(&path, b"hello").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn write_atomic_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        std::fs::write(&path, b"old content").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn write_atomic_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        write_atomic(&path, b"a").unwrap();
        write_atomic(&path, b"b").unwrap();
        // 成功後、ディレクトリには対象ファイルだけが残る（一時ファイルなし）。
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("out.json")]);
    }

    #[test]
    fn write_atomic_fails_on_missing_parent_dir() {
        let path = Path::new("/nonexistent_dir_xyz/out.json");
        assert!(write_atomic(path, b"x").is_err());
    }
}
