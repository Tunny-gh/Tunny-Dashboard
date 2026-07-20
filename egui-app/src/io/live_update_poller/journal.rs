use std::io::Read;
use std::io::Seek;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use tunny_core::io::journal::live_update::{
    append_journal_diff, set_next_trial_id, set_study_trial_number_seeds, LiveUpdateContext,
};

use crate::state::messages::AppMessage;

use super::escalate_error;

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
                    "Polling thread terminated unexpectedly".to_string(),
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
    /// Prevents the thread from leaking even if the poller is dropped without
    /// an explicit `stop()` call. `stop()` is idempotent (a no-op on the
    /// second and later calls since thread_handle is None) and safe to call
    /// twice.
    fn drop(&mut self) {
        self.stop();
    }
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

        // The size must come from an opened handle, not from path-based
        // std::fs::metadata: on Windows (NTFS) the directory entry that a
        // path query reads is not refreshed while a writer keeps the file
        // open, so it reports a stale size for the entire duration of a run
        // (the .ghx runner holds the journal open in append mode). A
        // handle-based query always sees the current size.
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                let _ = tx.send(AppMessage::LiveUpdateError(format!(
                    "Live update: file open error ({})",
                    e
                )));
                if escalate_error(&mut error_count, tx, stop_signal) {
                    break;
                }
                continue;
            }
        };

        let file_size = match file.metadata() {
            Ok(m) => m.len(),
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

        // Detect journal rotation/truncation. If the file size becomes
        // smaller than the already-read offset, assume the log was replaced
        // (rotated) or truncated, and reset the offset to 0 to re-read from
        // the beginning. Otherwise the diff would never be detected, and the
        // no-change timeout would misidentify this as "optimization complete".
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

        if file.seek(std::io::SeekFrom::Start(byte_offset)).is_err() {
            if escalate_error(&mut error_count, tx, stop_signal) {
                break;
            }
            continue;
        }

        // Rather than allocating the entire diff at once, cap each tick at up
        // to MAX_READ_CHUNK bytes. The remainder is read on the next tick
        // (since append_journal_diff returns consumed_bytes on a per-line
        // basis, byte_offset only advances through complete lines even if
        // the chunk ends mid-line, so nothing is lost).
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

    /// End-to-end repro of the .ghx D&D optimization flow: prepare_gh_run creates
    /// the study, the poller starts exactly as restart_poller does (offset/counters
    /// read from the file), then run_prepared appends trials from worker threads.
    /// Every completed trial must be delivered through LiveUpdateDone.
    #[test]
    fn gh_run_trials_stream_through_live_update() {
        use tunny_core::gh::{prepare_gh_run, run_prepared, GhRunConfig, GhSampler};
        use tunny_core::io::journal::parser::OptimizationDirection;

        struct SumEvaluator;
        impl tunny_core::gh::GhEvaluator for SumEvaluator {
            fn evaluate(&self, values: &[f64]) -> Result<tunny_core::gh::GhEvaluation, String> {
                // Simulate a slow solve so trials arrive across several poll ticks.
                thread::sleep(Duration::from_millis(20));
                Ok(tunny_core::gh::GhEvaluation {
                    objectives: vec![values.iter().sum()],
                    constraints: vec![values[0] - 5.0],
                    attributes: vec![Some(tunny_core::gh::GhAttrValue::Number(values[0] * 2.0))],
                })
            }
        }

        let problem = tunny_core::gh::GhProblem {
            variables: vec![tunny_core::gh::GhVariable {
                instance_guid: "g1".to_string(),
                name: "x".to_string(),
                low: 0.0,
                high: 10.0,
                value: 5.0,
                digits: 2,
                is_integer: false,
                gene_pool: None,
            }],
            objectives: vec![tunny_core::gh::GhObjective {
                source_guid: "o1".to_string(),
                name: "f".to_string(),
            }],
            constraints: vec![tunny_core::gh::GhConstraint {
                source_guid: "c1".to_string(),
                name: "g".to_string(),
            }],
            attributes: vec![tunny_core::gh::GhAttribute {
                source_guid: "a1".to_string(),
                name: "double_x".to_string(),
            }],
            tunny_component: "Tunny".to_string(),
            warnings: vec![],
        };
        let cfg = GhRunConfig {
            study_name: "live-test".to_string(),
            directions: vec![OptimizationDirection::Minimize],
            sampler: GhSampler::Random,
            n_trials: 8,
            ..GhRunConfig::default()
        };

        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run_optuna.log");
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();

        // Mirror restart_poller's prep: offset and counters from the current file.
        let bytes = std::fs::read(&journal).unwrap();
        let ctx = LiveUpdateContext {
            file_path: journal.clone(),
            initial_byte_offset: bytes.len() as u64,
            next_trial_id: tunny_core::io::journal::live_update::count_created_trials(&bytes),
            study_trial_number_seeds:
                tunny_core::io::journal::live_update::count_created_trials_per_study(&bytes),
            study_distributions: vec![],
            no_change_timeout_ms: 60_000,
        };
        let (tx, rx) = make_channel();
        let mut poller = LiveUpdatePoller::start(ctx, tx, 30);

        let progress = tunny_core::surrogate_opt::FitProgress::new();
        let summary = run_prepared(&prep, &problem, &SumEvaluator, &cfg, &progress).unwrap();
        assert_eq!(summary.completed, 8);

        // Collect rows until all 8 completed trials have streamed through.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut rows = 0usize;
        while std::time::Instant::now() < deadline && rows < 8 {
            if let Ok(AppMessage::LiveUpdateDone { new_trial_rows, .. }) = rx.try_recv() {
                for row in &new_trial_rows {
                    assert_eq!(row.study_id, 0);
                    assert_eq!(row.objectives.len(), 1);
                    assert!(row.params.contains_key("x"));
                    // op9 constraints stream through into the live rows
                    assert_eq!(row.constraint_values.len(), 1);
                    let x = row.params["x"];
                    assert!((row.constraint_values[0] - (x - 5.0)).abs() < 1e-9);
                    // op8 user attributes stream through as well
                    let attr = row.user_attrs_numeric.get("double_x").copied().unwrap();
                    assert!((attr - x * 2.0).abs() < 1e-9);
                }
                rows += new_trial_rows.len();
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
        poller.stop();
        assert_eq!(rows, 8, "all completed trials must arrive via live update");
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
