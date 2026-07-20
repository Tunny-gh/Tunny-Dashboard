use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::state::messages::AppMessage;

use super::escalate_error;

/// The shared fingerprint-based polling loop body for SQLite / RDB.
///
/// The two differ only in "how to obtain a fingerprint from the connection
/// target once" (SQLite: reopening the local file path, RDB: reconnecting to
/// the connection URL); the change detection, error escalation, and
/// completion-hint dispatch logic is entirely shared. By confining that
/// difference to the `fingerprint_fn` closure, this function is shared by
/// both `SqliteLivePoller` and `RdbLivePoller`.
///
/// The message sent reuses the `SqliteLiveChanged` message already defined
/// for SQLite as-is (no new message is added for RDB; the caller dispatches
/// the reload target via `storage_kind`).
///
/// `fingerprint_fn` requires `FnMut`. Since the RDB side reuses a connection
/// session (`tunny_core::rdb::RdbFingerprintSession`) across ticks, it needs
/// to mutate state (`Option<RdbFingerprintSession>`) inside the closure
/// (the SQLite side can remain a stateless `Fn` closure, and since `Fn`
/// automatically satisfies `FnMut`, compatibility is preserved).
#[allow(clippy::too_many_arguments)]
pub(super) fn fingerprint_polling_loop<F>(
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
// Tests for the shared fingerprint_polling_loop logic (fake fingerprint function)
//
// SqliteLivePoller uses a real SQLite fixture DB, and RdbLivePoller requires
// a real DB connection, so neither can be tested directly in CI (an
// `#[ignore]` integration test is provided separately for the RDB side).
// Since the shared loop body (`fingerprint_polling_loop`) itself uses
// closure injection, the core logic of both pollers can be verified with a
// fake fingerprint function.
// =============================================================================

#[cfg(test)]
mod fingerprint_polling_loop_tests {
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread::JoinHandle;

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

    /// Verifies that `SqliteLiveChanged` (the reused message) is sent on
    /// change detection, regardless of whether the closure is injected for
    /// SqliteLivePoller or RdbLivePoller.
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
                // Change starting from the 2nd call (a repeated value counts as no change).
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
