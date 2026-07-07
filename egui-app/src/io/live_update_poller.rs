use std::io::Read;
use std::io::Seek;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use tunny_core::io::journal::live_update::{
    append_journal_diff, set_next_trial_id, set_study_trial_number_seeds, LiveUpdateContext,
};

use crate::state::messages::AppMessage;

pub struct LiveUpdatePoller {
    stop_signal: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    thread_handle: Option<JoinHandle<()>>,
}

impl LiveUpdatePoller {
    pub fn start(context: LiveUpdateContext, tx: SyncSender<AppMessage>, interval_ms: u64) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let interval = Arc::new(AtomicU64::new(interval_ms));

        let stop_clone = stop_signal.clone();
        let interval_clone = interval.clone();
        let no_change_timeout = Duration::from_millis(context.no_change_timeout_ms);

        let handle = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                polling_loop(
                    context,
                    &tx,
                    &stop_clone,
                    &interval_clone,
                    no_change_timeout,
                );
            }));
            if result.is_err() {
                let _ = tx.send(AppMessage::LiveUpdateError(
                    "ポーリングスレッドが異常終了".to_string(),
                ));
            }
        });

        LiveUpdatePoller {
            stop_signal,
            interval_ms: interval,
            thread_handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn update_interval(&self, new_interval_ms: u64) {
        self.interval_ms.store(new_interval_ms, Ordering::Relaxed);
    }
}

impl Drop for LiveUpdatePoller {
    /// ポーラーが明示 `stop()` されずに破棄された場合でもスレッドをリークさせない。
    /// `stop()` は冪等（2 回目以降は thread_handle が None のため no-op）で二重呼び出し安全。
    fn drop(&mut self) {
        self.stop();
    }
}

/// Increments error_count and, on the third consecutive error, sends the auto-stop message and
/// signals the thread to stop. Returns `true` if the loop should `break`.
fn escalate_error(
    error_count: &mut u32,
    tx: &SyncSender<AppMessage>,
    stop_signal: &AtomicBool,
) -> bool {
    *error_count += 1;
    if *error_count >= 3 {
        let _ = tx.send(AppMessage::LiveUpdateError(
            "連続エラーにより自動停止".to_string(),
        ));
        stop_signal.store(true, Ordering::Relaxed);
        return true;
    }
    false
}

fn polling_loop(
    context: LiveUpdateContext,
    tx: &SyncSender<AppMessage>,
    stop_signal: &AtomicBool,
    interval_ms: &AtomicU64,
    no_change_timeout: Duration,
) {
    set_next_trial_id(context.next_trial_id);
    set_study_trial_number_seeds(context.study_trial_number_seeds.clone());
    let mut byte_offset = context.initial_byte_offset;
    let mut error_count: u32 = 0;
    let mut last_changed = SystemTime::now();
    let mut completion_hint_sent = false;

    loop {
        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        let sleep_ms = interval_ms.load(Ordering::Relaxed);
        thread::sleep(Duration::from_millis(sleep_ms));

        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        let path = &context.file_path;

        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                let _ = tx.send(AppMessage::LiveUpdateError(format!(
                    "Live update: file metadata error ({})",
                    e
                )));
                if escalate_error(&mut error_count, tx, stop_signal) {
                    break;
                }
                continue;
            }
        };

        let file_size = metadata.len();

        // ジャーナルのローテーション/切り詰め検出。ファイルサイズが既読オフセットより
        // 小さくなった場合、ログが置き換え（ローテーション）または切り詰められたと判断し、
        // オフセットを 0 に戻して先頭から読み直す。放置すると差分が永久に検出されず、
        // 無変化タイムアウトで「最適化完了」と誤認してしまうため。
        if file_size < byte_offset {
            byte_offset = 0;
        }

        if file_size <= byte_offset {
            // No new data — check for completion hint
            if !completion_hint_sent {
                if let Ok(elapsed) = SystemTime::now().duration_since(last_changed) {
                    if elapsed >= no_change_timeout {
                        let _ = tx.send(AppMessage::LiveUpdateMaybeComplete);
                        completion_hint_sent = true;
                    }
                }
            }
            error_count = 0;
            continue;
        }

        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => {
                if escalate_error(&mut error_count, tx, stop_signal) {
                    break;
                }
                continue;
            }
        };

        if file.seek(std::io::SeekFrom::Start(byte_offset)).is_err() {
            if escalate_error(&mut error_count, tx, stop_signal) {
                break;
            }
            continue;
        }

        // 差分全体を一括確保せず、1 tick あたり最大 MAX_READ_CHUNK バイトまでに制限する。
        // 残りは次 tick で読む（append_journal_diff は行単位で consumed_bytes を返すため、
        // チャンク末尾が行途中でも byte_offset は完全な行までしか進まず、取りこぼさない）。
        const MAX_READ_CHUNK: u64 = 8 * 1024 * 1024;
        let read_size = (file_size - byte_offset).min(MAX_READ_CHUNK) as usize;
        let mut buf = vec![0u8; read_size];
        if file.read_exact(&mut buf).is_err() {
            if escalate_error(&mut error_count, tx, stop_signal) {
                break;
            }
            continue;
        }

        error_count = 0;
        let result = append_journal_diff(&buf);
        byte_offset += result.consumed_bytes as u64;

        let extras = &result.extras_events;
        let has_extras = !extras.new_trials.is_empty()
            || !extras.intermediate_values.is_empty()
            || !extras.state_changes.is_empty();

        if !result.new_trial_rows.is_empty() || has_extras {
            last_changed = SystemTime::now();
            completion_hint_sent = false;
            let _ = tx.send(AppMessage::LiveUpdateDone {
                new_trial_rows: result.new_trial_rows,
                updated_study_counts: result.updated_study_counts,
                extras_events: result.extras_events,
            });
        }
    }
}

// =============================================================================
// SQLite ライブ更新ポーラー
//
// journal はバイトオフセット差分で追記分だけを解析できるが、SQLite は
// Optuna が trial の状態をインプレースで更新する（RUNNING→COMPLETE 等）ため
// オフセット差分が使えない。代わりに `tunny_core::sqlite::study_fingerprint`
// で変化の有無だけを安価に検出し、変化を検出したら対象 study を丸ごと
// 再ロードするようメインスレッドへシグナルを送る（実際の再パースは
// study worker スレッドが行う。本ポーラーはフィンガープリント取得のみ）。
// =============================================================================

/// SQLite ライブ更新ポーリングスレッドへ渡すコンテキスト。
#[derive(Debug, Clone)]
pub struct SqliteLiveUpdateContext {
    pub file_path: PathBuf,
    /// ポーリング対象 study（フィンガープリントは study 単位でしか取れない）。
    pub study_id: u32,
    /// ポーリング開始時点のフィンガープリント（journal の `initial_byte_offset` に相当）。
    pub initial_fingerprint: tunny_core::sqlite::StudyFingerprint,
    /// Milliseconds of no change before sending completion hint (default: 60_000)
    pub no_change_timeout_ms: u64,
}

pub struct SqliteLivePoller {
    stop_signal: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    thread_handle: Option<JoinHandle<()>>,
}

impl SqliteLivePoller {
    pub fn start(
        context: SqliteLiveUpdateContext,
        tx: SyncSender<AppMessage>,
        interval_ms: u64,
    ) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let interval = Arc::new(AtomicU64::new(interval_ms));

        let stop_clone = stop_signal.clone();
        let interval_clone = interval.clone();
        let no_change_timeout = Duration::from_millis(context.no_change_timeout_ms);

        let handle = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                sqlite_polling_loop(
                    context,
                    &tx,
                    &stop_clone,
                    &interval_clone,
                    no_change_timeout,
                );
            }));
            if result.is_err() {
                let _ = tx.send(AppMessage::LiveUpdateError(
                    "ポーリングスレッドが異常終了".to_string(),
                ));
            }
        });

        SqliteLivePoller {
            stop_signal,
            interval_ms: interval,
            thread_handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn update_interval(&self, new_interval_ms: u64) {
        self.interval_ms.store(new_interval_ms, Ordering::Relaxed);
    }
}

impl Drop for SqliteLivePoller {
    /// 明示 `stop()` されずに破棄されてもポーリングスレッドをリークさせない
    /// （`stop()` は冪等で二重呼び出し安全）。
    fn drop(&mut self) {
        self.stop();
    }
}

fn sqlite_polling_loop(
    context: SqliteLiveUpdateContext,
    tx: &SyncSender<AppMessage>,
    stop_signal: &AtomicBool,
    interval_ms: &AtomicU64,
    no_change_timeout: Duration,
) {
    let file_path = context.file_path;
    fingerprint_polling_loop(
        context.study_id,
        context.initial_fingerprint,
        move |study_id| tunny_core::sqlite::study_fingerprint(&file_path, study_id),
        "sqlite",
        tx,
        stop_signal,
        interval_ms,
        no_change_timeout,
    );
}

/// SQLite / RDB 共通のフィンガープリント方式ポーリングループ本体。
///
/// 両者は「接続先からフィンガープリントを 1 回取得する手段」だけが異なり
/// （SQLite: ローカルファイルパスの再オープン、RDB: 接続 URL への再接続）、
/// 変化検出・エラーエスカレーション・完了ヒント送出のロジックは完全に共通。
/// その差分を `fingerprint_fn` クロージャへ閉じ込めることで
/// `SqliteLivePoller` / `RdbLivePoller` の両方から本関数を共有する。
///
/// 送信メッセージは SQLite 用に定義済みの `SqliteLiveChanged` をそのまま流用する
/// （RDB 用の新規メッセージは増やさない。呼び出し側は `storage_kind` で
/// 再ロード先を振り分ける）。
///
/// `fingerprint_fn` は `FnMut` を要求する。RDB 側は接続セッション
/// （`tunny_core::rdb::RdbFingerprintSession`）を tick を跨いで使い回すため、
/// クロージャ内部で状態（`Option<RdbFingerprintSession>`）を書き換える必要がある
/// （SQLite 側は状態を持たない `Fn` クロージャのままでよく、`Fn` は `FnMut` を
/// 自動的に満たすため互換性は保たれる）。
#[allow(clippy::too_many_arguments)]
fn fingerprint_polling_loop<F>(
    study_id: u32,
    initial_fingerprint: tunny_core::rdb::StudyFingerprint,
    mut fingerprint_fn: F,
    error_source: &str,
    tx: &SyncSender<AppMessage>,
    stop_signal: &AtomicBool,
    interval_ms: &AtomicU64,
    no_change_timeout: Duration,
) where
    F: FnMut(u32) -> Result<tunny_core::rdb::StudyFingerprint, String>,
{
    let mut last_fingerprint = initial_fingerprint;
    let mut error_count: u32 = 0;
    let mut last_changed = SystemTime::now();
    let mut completion_hint_sent = false;

    loop {
        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        let sleep_ms = interval_ms.load(Ordering::Relaxed);
        thread::sleep(Duration::from_millis(sleep_ms));

        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        match fingerprint_fn(study_id) {
            Ok(fingerprint) => {
                error_count = 0;
                if fingerprint != last_fingerprint {
                    last_fingerprint = fingerprint;
                    last_changed = SystemTime::now();
                    completion_hint_sent = false;
                    let _ = tx.send(AppMessage::SqliteLiveChanged { study_id });
                } else if !completion_hint_sent {
                    if let Ok(elapsed) = SystemTime::now().duration_since(last_changed) {
                        if elapsed >= no_change_timeout {
                            let _ = tx.send(AppMessage::LiveUpdateMaybeComplete);
                            completion_hint_sent = true;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(AppMessage::LiveUpdateError(format!(
                    "Live update: {error_source} fingerprint error ({e})"
                )));
                if escalate_error(&mut error_count, tx, stop_signal) {
                    break;
                }
            }
        }
    }
}

// =============================================================================
// RDB（PostgreSQL/MySQL）ライブ更新ポーラー
//
// SQLite と同じくフィンガープリント方式（`fingerprint_polling_loop` を共有）だが、
// 接続先がローカルファイルパスではなく `RdbUrl`（接続 URL）である点が異なる。
// フィンガープリント取得は `tunny_core::rdb::RdbFingerprintSession` で接続を
// 保持し、tick を跨いで使い回す（毎 tick 再接続すると TCP ハンドシェイクの
// コストがポーリング間隔ごとに掛かるため）。セッションは初回 tick で遅延接続し、
// フィンガープリント取得がエラーになったら破棄して次 tick で再接続を試みる。
// =============================================================================

/// RDB ライブ更新ポーリングスレッドへ渡すコンテキスト。
#[derive(Debug, Clone)]
pub struct RdbLiveUpdateContext {
    pub url: tunny_core::rdb::RdbUrl,
    /// ポーリング対象 study（フィンガープリントは study 単位でしか取れない）。
    pub study_id: u32,
    /// ポーリング開始時点のフィンガープリント。
    pub initial_fingerprint: tunny_core::rdb::StudyFingerprint,
    /// Milliseconds of no change before sending completion hint (default: 60_000)
    pub no_change_timeout_ms: u64,
}

pub struct RdbLivePoller {
    stop_signal: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    thread_handle: Option<JoinHandle<()>>,
}

impl RdbLivePoller {
    pub fn start(
        context: RdbLiveUpdateContext,
        tx: SyncSender<AppMessage>,
        interval_ms: u64,
    ) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let interval = Arc::new(AtomicU64::new(interval_ms));

        let stop_clone = stop_signal.clone();
        let interval_clone = interval.clone();
        let no_change_timeout = Duration::from_millis(context.no_change_timeout_ms);

        let handle = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                rdb_polling_loop(
                    context,
                    &tx,
                    &stop_clone,
                    &interval_clone,
                    no_change_timeout,
                );
            }));
            if result.is_err() {
                let _ = tx.send(AppMessage::LiveUpdateError(
                    "ポーリングスレッドが異常終了".to_string(),
                ));
            }
        });

        RdbLivePoller {
            stop_signal,
            interval_ms: interval,
            thread_handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn update_interval(&self, new_interval_ms: u64) {
        self.interval_ms.store(new_interval_ms, Ordering::Relaxed);
    }
}

impl Drop for RdbLivePoller {
    /// 明示 `stop()` されずに破棄されてもポーリングスレッドをリークさせない
    /// （`stop()` は冪等で二重呼び出し安全）。
    fn drop(&mut self) {
        self.stop();
    }
}

fn rdb_polling_loop(
    context: RdbLiveUpdateContext,
    tx: &SyncSender<AppMessage>,
    stop_signal: &AtomicBool,
    interval_ms: &AtomicU64,
    no_change_timeout: Duration,
) {
    let url = context.url;
    // tick を跨いで再利用する接続セッション。初回 tick で遅延接続し、
    // フィンガープリント取得（あるいは接続そのもの）が失敗したら破棄して
    // 次 tick で再接続を試みる（接続状態が壊れている可能性があるため）。
    let mut session: Option<tunny_core::rdb::RdbFingerprintSession> = None;
    fingerprint_polling_loop(
        context.study_id,
        context.initial_fingerprint,
        move |study_id| {
            if session.is_none() {
                session = Some(tunny_core::rdb::RdbFingerprintSession::connect(&url)?);
            }
            // 直前で Some を保証しているため `expect` は panic しない。
            let result = session
                .as_mut()
                .expect("session was just connected above")
                .fingerprint(study_id);
            if result.is_err() {
                session = None;
            }
            result
        },
        "rdb",
        tx,
        stop_signal,
        interval_ms,
        no_change_timeout,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::mpsc;

    fn make_context(path: PathBuf, offset: u64) -> LiveUpdateContext {
        LiveUpdateContext {
            file_path: path,
            initial_byte_offset: offset,
            next_trial_id: 0,
            study_trial_number_seeds: std::collections::HashMap::new(),
            study_distributions: vec![],
            no_change_timeout_ms: 200, // short timeout for tests
        }
    }

    fn make_channel() -> (SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
        mpsc::sync_channel(32)
    }

    // ── TASK-2219: lifecycle tests ─────────────────────────────────────

    #[test]
    fn tc_2219_03_file_append_sends_live_update_done() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, b"").unwrap();

        let (tx, rx) = make_channel();
        let mut poller = LiveUpdatePoller::start(make_context(path.clone(), 0), tx, 50);

        // Write a complete trial
        let line =
            b"{\"op_code\":4,\"study_id\":0}\n{\"op_code\":6,\"trial_id\":0,\"state\":1,\"values\":[1.0]}\n";
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(line).unwrap();
        drop(f);

        // Wait for message
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            if let Ok(msg) = rx.try_recv() {
                if matches!(msg, AppMessage::LiveUpdateDone { .. }) {
                    found = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        poller.stop();
        assert!(found, "Expected LiveUpdateDone message");
    }

    #[test]
    fn tc_2219_04_no_change_no_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, b"some initial content\n").unwrap();
        let size = std::fs::metadata(&path).unwrap().len();

        let (tx, rx) = make_channel();
        // Start with byte_offset at end so no diff is available
        let mut poller = LiveUpdatePoller::start(make_context(path, size), tx, 50);

        thread::sleep(Duration::from_millis(200));
        poller.stop();

        // No LiveUpdateDone should arrive
        let messages: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        let has_done = messages
            .iter()
            .any(|m| matches!(m, AppMessage::LiveUpdateDone { .. }));
        assert!(
            !has_done,
            "No LiveUpdateDone expected when file is unchanged"
        );
    }

    #[test]
    fn tc_2219_06_update_interval_changes_timing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, b"").unwrap();

        let (tx, _rx) = make_channel();
        let poller = LiveUpdatePoller::start(make_context(path, 0), tx, 1000);
        poller.update_interval(100);
        assert_eq!(poller.interval_ms.load(Ordering::Relaxed), 100);

        let mut p = poller;
        p.stop();
    }

    // ── TASK-2220: error counting & completion tests ─────────────────

    #[test]
    fn tc_2220_01_three_consecutive_errors_auto_stop() {
        let dir = tempfile::tempdir().unwrap();
        let nonexistent = dir.path().join("does_not_exist.log");

        let (tx, rx) = make_channel();
        let mut poller = LiveUpdatePoller::start(make_context(nonexistent, 0), tx, 50);

        // Wait for auto-stop (3 errors × 50ms interval + buffer)
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut got_error = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::LiveUpdateError(_)) = rx.try_recv() {
                got_error = true;
            }
            if poller.stop_signal.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        poller.stop();
        assert!(
            got_error,
            "Expected LiveUpdateError after consecutive errors"
        );
    }

    #[test]
    fn tc_2220_03_no_change_60s_sends_maybe_complete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, b"content\n").unwrap();
        let size = std::fs::metadata(&path).unwrap().len();

        // Use 200ms timeout for test speed
        let ctx = LiveUpdateContext {
            file_path: path,
            initial_byte_offset: size,
            next_trial_id: 0,
            study_trial_number_seeds: std::collections::HashMap::new(),
            study_distributions: vec![],
            no_change_timeout_ms: 200,
        };

        let (tx, rx) = make_channel();
        let mut poller = LiveUpdatePoller::start(ctx, tx, 50);

        // Wait for MaybeComplete
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::LiveUpdateMaybeComplete) = rx.try_recv() {
                found = true;
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        poller.stop();
        assert!(
            found,
            "Expected LiveUpdateMaybeComplete after no-change timeout"
        );
    }
}

// =============================================================================
// SqliteLivePoller tests
// =============================================================================

#[cfg(test)]
mod sqlite_poller_tests {
    use super::*;
    use std::sync::mpsc;

    /// `study_fingerprint` / `ensure_optuna_schema` が要求する最小限のテーブルだけを
    /// 持つフィクスチャ DB を作る（`rust_core::io::sqlite::tests` の `create_schema` と
    /// 同趣旨だが、egui-app 側からは private のため独自に用意する）。
    fn make_fixture_db(path: &std::path::Path) -> rusqlite::Connection {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE studies (
                study_id INTEGER PRIMARY KEY,
                study_name VARCHAR(512)
            );
            CREATE TABLE trials (
                trial_id INTEGER PRIMARY KEY,
                number INTEGER,
                study_id INTEGER,
                state VARCHAR(8),
                datetime_start TEXT,
                datetime_complete TEXT
            );
            INSERT INTO studies (study_id, study_name) VALUES (1, 'study-a');
            INSERT INTO trials (trial_id, number, study_id, state) VALUES (1, 0, 1, 'RUNNING');
            ",
        )
        .unwrap();
        conn
    }

    fn make_sqlite_context(
        path: PathBuf,
        study_id: u32,
        fingerprint: tunny_core::sqlite::StudyFingerprint,
    ) -> SqliteLiveUpdateContext {
        SqliteLiveUpdateContext {
            file_path: path,
            study_id,
            initial_fingerprint: fingerprint,
            no_change_timeout_ms: 200, // short timeout for tests
        }
    }

    fn make_channel() -> (SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
        mpsc::sync_channel(32)
    }

    #[test]
    fn sqlite_poller_detects_trial_state_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("study.db");
        let conn = make_fixture_db(&path);

        let initial = tunny_core::sqlite::study_fingerprint(&path, 1).unwrap();
        let (tx, rx) = make_channel();
        let mut poller =
            SqliteLivePoller::start(make_sqlite_context(path.clone(), 1, initial), tx, 50);

        // Optuna が RUNNING trial を COMPLETE へ遷移させたのと同じ状況を作る。
        conn.execute(
            "UPDATE trials SET state = 'COMPLETE' WHERE trial_id = 1",
            [],
        )
        .unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::SqliteLiveChanged { study_id }) = rx.try_recv() {
                assert_eq!(study_id, 1);
                found = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        poller.stop();
        assert!(found, "Expected SqliteLiveChanged after state transition");
    }

    #[test]
    fn sqlite_poller_no_change_sends_no_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("study.db");
        let _conn = make_fixture_db(&path);

        let initial = tunny_core::sqlite::study_fingerprint(&path, 1).unwrap();
        let (tx, rx) = make_channel();
        let mut poller = SqliteLivePoller::start(make_sqlite_context(path, 1, initial), tx, 50);

        thread::sleep(Duration::from_millis(150));
        poller.stop();

        let messages: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(
            !messages
                .iter()
                .any(|m| matches!(m, AppMessage::SqliteLiveChanged { .. })),
            "No SqliteLiveChanged expected when the DB is unchanged"
        );
    }

    #[test]
    fn sqlite_poller_no_change_timeout_sends_maybe_complete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("study.db");
        let _conn = make_fixture_db(&path);

        let initial = tunny_core::sqlite::study_fingerprint(&path, 1).unwrap();
        let (tx, rx) = make_channel();
        let mut poller = SqliteLivePoller::start(make_sqlite_context(path, 1, initial), tx, 50);

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::LiveUpdateMaybeComplete) = rx.try_recv() {
                found = true;
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        poller.stop();
        assert!(
            found,
            "Expected LiveUpdateMaybeComplete after no-change timeout"
        );
    }

    #[test]
    fn sqlite_poller_missing_file_auto_stops_after_errors() {
        let dir = tempfile::tempdir().unwrap();
        let nonexistent = dir.path().join("does_not_exist.db");

        let (tx, rx) = make_channel();
        let mut poller = SqliteLivePoller::start(
            make_sqlite_context(nonexistent, 1, Default::default()),
            tx,
            50,
        );

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut got_error = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::LiveUpdateError(_)) = rx.try_recv() {
                got_error = true;
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        poller.stop();
        assert!(
            got_error,
            "Expected LiveUpdateError after consecutive fingerprint errors"
        );
    }
}

// =============================================================================
// fingerprint_polling_loop 共有ロジックのテスト（fake フィンガープリント関数）
//
// SqliteLivePoller は実 SQLite フィクスチャ DB で、RdbLivePoller は実 DB 接続が要る
// ため CI では直接テストできない（RDB 側は #[ignore] 統合テストを別途用意する）。
// 共有ループ本体（`fingerprint_polling_loop`）自体はクロージャ注入のため、
// fake なフィンガープリント関数で両ポーラーの中核ロジックを検証できる。
// =============================================================================

#[cfg(test)]
mod fingerprint_polling_loop_tests {
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::sync::mpsc;

    fn make_channel() -> (SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
        mpsc::sync_channel(32)
    }

    fn spawn_loop<F>(
        study_id: u32,
        initial: tunny_core::rdb::StudyFingerprint,
        fingerprint_fn: F,
        error_source: &'static str,
        tx: SyncSender<AppMessage>,
        no_change_timeout_ms: u64,
        interval_ms: u64,
    ) -> (Arc<AtomicBool>, JoinHandle<()>)
    where
        F: Fn(u32) -> Result<tunny_core::rdb::StudyFingerprint, String> + Send + 'static,
    {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let interval = Arc::new(AtomicU64::new(interval_ms));
        let stop_clone = stop_signal.clone();
        let interval_clone = interval.clone();
        let no_change_timeout = Duration::from_millis(no_change_timeout_ms);
        let handle = thread::spawn(move || {
            fingerprint_polling_loop(
                study_id,
                initial,
                fingerprint_fn,
                error_source,
                &tx,
                &stop_clone,
                &interval_clone,
                no_change_timeout,
            );
        });
        (stop_signal, handle)
    }

    /// SqliteLivePoller / RdbLivePoller いずれの closure 注入でも変化検出で
    /// `SqliteLiveChanged`（流用メッセージ）が送られることを確認する。
    #[test]
    fn detects_fingerprint_change_via_injected_closure() {
        let (tx, rx) = make_channel();
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        let (stop_signal, handle) = spawn_loop(
            42,
            tunny_core::rdb::StudyFingerprint::default(),
            move |study_id| {
                assert_eq!(study_id, 42);
                let n = call_count_clone.fetch_add(1, Ordering::Relaxed);
                // 2 回目の呼び出しから変化させる（同じ値が続くのは変化なし扱い）。
                Ok(tunny_core::rdb::StudyFingerprint {
                    total_trials: if n >= 1 { 1 } else { 0 },
                    ..Default::default()
                })
            },
            "fake",
            tx,
            60_000,
            10,
        );

        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::SqliteLiveChanged { study_id }) = rx.try_recv() {
                assert_eq!(study_id, 42);
                found = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        stop_signal.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert!(found, "Expected SqliteLiveChanged from injected closure");
    }

    #[test]
    fn stable_fingerprint_sends_no_change_message() {
        let (tx, rx) = make_channel();
        let fixed = tunny_core::rdb::StudyFingerprint {
            total_trials: 5,
            ..Default::default()
        };
        let fixed_clone = fixed.clone();
        let (stop_signal, handle) = spawn_loop(
            1,
            fixed,
            move |_| Ok(fixed_clone.clone()),
            "fake",
            tx,
            60_000,
            10,
        );

        thread::sleep(Duration::from_millis(150));
        stop_signal.store(true, Ordering::Relaxed);
        let _ = handle.join();

        let messages: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(
            !messages
                .iter()
                .any(|m| matches!(m, AppMessage::SqliteLiveChanged { .. })),
            "No SqliteLiveChanged expected when the fingerprint is stable"
        );
    }

    #[test]
    fn error_closure_escalates_and_auto_stops() {
        let (tx, rx) = make_channel();
        let (stop_signal, handle) = spawn_loop(
            1,
            Default::default(),
            |_| Err("connection refused".to_string()),
            "fake",
            tx,
            60_000,
            10,
        );

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut got_error = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::LiveUpdateError(msg)) = rx.try_recv() {
                assert!(msg.contains("fake fingerprint error"));
                got_error = true;
            }
            if stop_signal.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        stop_signal.store(true, Ordering::Relaxed);
        let _ = handle.join();
        assert!(got_error, "Expected LiveUpdateError after closure errors");
    }
}
