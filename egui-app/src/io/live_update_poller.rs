use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;

use crate::state::messages::AppMessage;

mod fingerprint;
mod journal;
mod rdb;
mod sqlite;

pub use journal::LiveUpdatePoller;
pub use rdb::{RdbLivePoller, RdbLiveUpdateContext};
pub use sqlite::{SqliteLivePoller, SqliteLiveUpdateContext};

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
            "Stopped automatically after repeated errors".to_string(),
        ));
        stop_signal.store(true, Ordering::Relaxed);
        return true;
    }
    false
}
