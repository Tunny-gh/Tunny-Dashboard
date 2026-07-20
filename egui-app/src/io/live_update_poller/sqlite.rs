// =============================================================================
// SQLite live update poller
//
// The journal can be parsed incrementally via a byte-offset diff, but SQLite
// is not: Optuna updates trial state in place (RUNNING→COMPLETE, etc.), so an
// offset diff can't be used. Instead, `tunny_core::sqlite::study_fingerprint`
// is used to cheaply detect whether anything changed, and when a change is
// detected, a signal is sent to the main thread to fully reload the target
// study (the actual re-parsing is done by the study worker thread; this
// poller only retrieves the fingerprint).
// =============================================================================

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::state::messages::AppMessage;

use super::fingerprint::fingerprint_polling_loop;

/// Context passed to the SQLite live update polling thread.
#[derive(Debug, Clone)]
pub struct SqliteLiveUpdateContext {
    pub file_path: PathBuf,
    /// The study being polled (fingerprints can only be obtained per study).
    pub study_id: u32,
    /// The fingerprint at the start of polling (equivalent to the journal's `initial_byte_offset`).
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
                    "Polling thread terminated unexpectedly".to_string(),
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
    /// Prevents the polling thread from leaking even if dropped without an
    /// explicit `stop()` call (`stop()` is idempotent and safe to call twice).
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

// =============================================================================
// SqliteLivePoller tests
// =============================================================================

#[cfg(test)]
mod sqlite_poller_tests {
    use super::*;
    use std::sync::mpsc;

    /// Creates a fixture DB with only the minimal tables required by
    /// `study_fingerprint` / `ensure_optuna_schema` (similar in spirit to
    /// `create_schema` in `rust_core::io::sqlite::tests`, but provided
    /// independently here since it's private from the egui-app side).
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

        // Simulate the same situation as Optuna transitioning a RUNNING trial to COMPLETE.
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
