use std::io::Read;
use std::io::Seek;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use tunny_core::io::journal::live_update::{
    append_journal_diff, set_next_trial_id, LiveUpdateContext,
};

use crate::state::messages::AppMessage;

pub struct LiveUpdatePoller {
    stop_signal: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    thread_handle: Option<JoinHandle<()>>,
}

impl LiveUpdatePoller {
    pub fn start(
        context: LiveUpdateContext,
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
                polling_loop(context, &tx, &stop_clone, &interval_clone, no_change_timeout);
            }));
            if let Err(_) = result {
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

    pub fn is_running(&self) -> bool {
        self.thread_handle.is_some()
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

        let read_size = (file_size - byte_offset) as usize;
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

        if !result.new_trial_rows.is_empty() {
            last_changed = SystemTime::now();
            completion_hint_sent = false;
            let _ = tx.send(AppMessage::LiveUpdateDone {
                new_trial_rows: result.new_trial_rows,
                updated_study_counts: result.updated_study_counts,
            });
        }
    }
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
            study_distributions: vec![],
            no_change_timeout_ms: 200, // short timeout for tests
        }
    }

    fn make_channel() -> (SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
        mpsc::sync_channel(32)
    }

    // ── TASK-2219: lifecycle tests ─────────────────────────────────────

    #[test]
    fn tc_2219_01_start_poller_returns_running() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, b"").unwrap();

        let (tx, _rx) = make_channel();
        let mut poller = LiveUpdatePoller::start(make_context(path, 0), tx, 50);
        assert!(poller.is_running());
        poller.stop();
        assert!(!poller.is_running());
    }

    #[test]
    fn tc_2219_02_stop_poller_joins_thread() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, b"").unwrap();

        let (tx, _rx) = make_channel();
        let mut poller = LiveUpdatePoller::start(make_context(path, 0), tx, 50);
        poller.stop();
        assert!(!poller.is_running());
    }

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
        assert!(!has_done, "No LiveUpdateDone expected when file is unchanged");
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
        let mut poller =
            LiveUpdatePoller::start(make_context(nonexistent, 0), tx, 50);

        // Wait for auto-stop (3 errors × 50ms interval + buffer)
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut got_error = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AppMessage::LiveUpdateError(_)) = rx.try_recv() {
                got_error = true;
            }
            if !poller.is_running()
                || poller
                    .stop_signal
                    .load(Ordering::Relaxed)
            {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        poller.stop();
        assert!(got_error, "Expected LiveUpdateError after consecutive errors");
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
        assert!(found, "Expected LiveUpdateMaybeComplete after no-change timeout");
    }
}
