// =============================================================================
// RDB (PostgreSQL/MySQL) live update poller
//
// Like SQLite, this uses the fingerprint approach (sharing
// `fingerprint_polling_loop`), but differs in that the connection target is
// an `RdbUrl` (connection URL) rather than a local file path. Fingerprint
// retrieval holds the connection via `tunny_core::rdb::RdbFingerprintSession`
// and reuses it across ticks (reconnecting every tick would incur the cost
// of a TCP handshake each polling interval). The session connects lazily on
// the first tick, and if fingerprint retrieval errors out, it's discarded
// and reconnection is attempted on the next tick.
// =============================================================================

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::state::messages::AppMessage;

use super::fingerprint::fingerprint_polling_loop;

/// Context passed to the RDB live update polling thread.
#[derive(Debug, Clone)]
pub struct RdbLiveUpdateContext {
    pub url: tunny_core::rdb::RdbUrl,
    /// The study being polled (fingerprints can only be obtained per study).
    pub study_id: u32,
    /// The fingerprint at the start of polling.
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
                    "Polling thread terminated unexpectedly".to_string(),
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
    /// Prevents the polling thread from leaking even if dropped without an
    /// explicit `stop()` call (`stop()` is idempotent and safe to call twice).
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
    // A connection session reused across ticks. It connects lazily on the
    // first tick, and if fingerprint retrieval (or the connection itself)
    // fails, it's discarded and reconnection is attempted on the next tick
    // (since the connection state may be broken).
    let mut session: Option<tunny_core::rdb::RdbFingerprintSession> = None;
    fingerprint_polling_loop(
        context.study_id,
        context.initial_fingerprint,
        move |study_id| {
            if session.is_none() {
                session = Some(tunny_core::rdb::RdbFingerprintSession::connect(&url)?);
            }
            // `expect` cannot panic here since `Some` was just guaranteed above.
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
