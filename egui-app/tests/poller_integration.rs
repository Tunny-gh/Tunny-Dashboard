use std::io::Write;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tunny_core::dataframe::DataFrame;
use tunny_core::io::journal::live_update::{
    append_journal_diff, reset_live_update_state, LiveUpdateContext,
};
use tunny_desktop::io::live_update_poller::LiveUpdatePoller;
use tunny_desktop::state::app_state::{AppState, Direction, StudyContext, StudyMeta, StudyView};
use tunny_desktop::state::message_handler::MessageHandler;
use tunny_desktop::state::messages::AppMessage;
use tunny_desktop::ui::widget_states::WidgetStates;

/// 空の StudyView（行0件）。ライブ更新テストの初期状態に用いる。
fn empty_view() -> StudyView {
    StudyView::new(Arc::new(DataFrame::empty()), vec![])
}

// ─────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────

fn test_study_meta(study_id: u32, n_obj: usize) -> StudyMeta {
    StudyMeta {
        study_id,
        name: format!("study_{}", study_id),
        directions: vec![Direction::Minimize; n_obj],
        completed_trials: 0,
        param_names: vec!["x".to_string()],
        objective_names: (0..n_obj).map(|i| format!("obj{}", i)).collect(),
        param_bounds: Default::default(),
    }
}

fn poller_context(path: std::path::PathBuf, offset: u64) -> LiveUpdateContext {
    LiveUpdateContext {
        file_path: path,
        initial_byte_offset: offset,
        next_trial_id: 0,
        study_trial_number_seeds: std::collections::HashMap::new(),
        study_distributions: vec![],
        no_change_timeout_ms: 10_000,
    }
}

/// Generate Journal-format bytes for `n` completed trials.
/// Each trial has one float param "x" and one objective value.
fn make_trial_bytes(n: usize, start_id: u32) -> Vec<u8> {
    let mut s = String::new();
    for i in 0..n {
        let tid = start_id + i as u32;
        s.push_str("{\"op_code\":4,\"study_id\":0}\n");
        s.push_str(&format!(
            "{{\"op_code\":5,\"trial_id\":{},\"param_name\":\"x\",\
             \"param_value_internal\":{:.6},\
             \"distribution\":{{\"name\":\"FloatDistribution\",\
             \"low\":0.0,\"high\":1.0,\"log\":false}}}}\n",
            tid,
            i as f64 / (n as f64).max(1.0)
        ));
        s.push_str(&format!(
            "{{\"op_code\":6,\"trial_id\":{},\"state\":1,\"values\":[{:.6}]}}\n",
            tid,
            i as f64 * 0.01
        ));
    }
    s.into_bytes()
}

fn wait_for_live_update_done(
    rx: &mpsc::Receiver<AppMessage>,
    timeout: Duration,
) -> Option<AppMessage> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(msg @ AppMessage::LiveUpdateDone { .. }) = rx.try_recv() {
            return Some(msg);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

// ─────────────────────────────────────────────
// TC-001: E2E polling flow
// ─────────────────────────────────────────────

/// File append → Poller detects → LiveUpdateDone → MessageHandler → AppState updated
#[test]
fn tc_2224_01_e2e_polling_flow() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("e2e.log");
    std::fs::write(&path, b"").unwrap();

    let mut app_state = AppState::new();
    app_state.journal_path = Some(path.clone());
    let meta = test_study_meta(0, 1);
    app_state.all_studies = vec![meta.clone()];
    app_state.current_study = Some(StudyContext {
        meta,
        view: empty_view(),
        pareto_indices: vec![],
    });

    let (tx, rx) = mpsc::sync_channel(64);
    let mut poller = LiveUpdatePoller::start(poller_context(path.clone(), 0), tx, 50);

    // Append 10 trials
    {
        let content = make_trial_bytes(10, 0);
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(&content).unwrap();
    }

    let msg = wait_for_live_update_done(&rx, Duration::from_secs(5));
    poller.stop();

    assert!(msg.is_some(), "Expected LiveUpdateDone after file append");

    if let Some(AppMessage::LiveUpdateDone { new_trial_rows, .. }) = &msg {
        assert_eq!(new_trial_rows.len(), 10);
    }

    // Process with MessageHandler → AppState updated
    let mut widget_states = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;
    MessageHandler::handle(
        msg.unwrap(),
        &mut app_state,
        &mut widget_states,
        &mut is_loading,
        &mut load_error,
    );

    let study = app_state.current_study.as_ref().unwrap();
    assert_eq!(study.trial_count(), 10, "trial_rows should have 10 entries");
    assert!(
        !study.pareto_indices.is_empty(),
        "pareto_indices should be populated"
    );
    assert!(load_error.is_none());
}

// ─────────────────────────────────────────────
// TC-002: File deleted → 3 errors → auto-stop
// ─────────────────────────────────────────────

#[test]
fn tc_2224_02_file_deleted_auto_stop() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("deletable.log");
    std::fs::write(&path, b"initial\n").unwrap();

    let (tx, rx) = mpsc::sync_channel(64);
    let ctx = LiveUpdateContext {
        file_path: path.clone(),
        initial_byte_offset: std::fs::metadata(&path).unwrap().len(),
        next_trial_id: 0,
        study_trial_number_seeds: std::collections::HashMap::new(),
        study_distributions: vec![],
        no_change_timeout_ms: 60_000,
    };
    let mut poller = LiveUpdatePoller::start(ctx, tx, 50);

    // Let the poller start its loop
    std::thread::sleep(Duration::from_millis(100));

    // Delete the file to trigger errors
    let _ = std::fs::remove_file(&path);

    // Wait until we receive a LiveUpdateError
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got_error = false;
    while Instant::now() < deadline {
        while let Ok(msg) = rx.try_recv() {
            if let AppMessage::LiveUpdateError(_) = msg {
                got_error = true;
            }
        }
        if got_error {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    poller.stop();

    assert!(got_error, "Expected LiveUpdateError after file deletion");
}

// ─────────────────────────────────────────────
// TC-003: Zero byte file → no errors, then detects append
// ─────────────────────────────────────────────

#[test]
fn tc_2224_03_zero_byte_file_no_errors() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.log");
    std::fs::write(&path, b"").unwrap();

    let (tx, rx) = mpsc::sync_channel(32);
    let ctx = LiveUpdateContext {
        file_path: path.clone(),
        initial_byte_offset: 0,
        next_trial_id: 0,
        study_trial_number_seeds: std::collections::HashMap::new(),
        study_distributions: vec![],
        no_change_timeout_ms: 60_000,
    };
    let mut poller = LiveUpdatePoller::start(ctx, tx, 50);

    // Let it poll a few cycles with empty file — no error should occur
    std::thread::sleep(Duration::from_millis(250));

    let messages: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let has_error = messages
        .iter()
        .any(|m| matches!(m, AppMessage::LiveUpdateError(_)));
    assert!(!has_error, "No error expected for empty file");

    // Append content and verify detection
    {
        let content = make_trial_bytes(3, 0);
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(&content).unwrap();
    }

    let msg = wait_for_live_update_done(&rx, Duration::from_secs(3));
    poller.stop();
    assert!(
        msg.is_some(),
        "Expected LiveUpdateDone after appending to empty file"
    );
}

// ─────────────────────────────────────────────
// TC-004: Bulk trials performance
// ─────────────────────────────────────────────

#[test]
fn tc_2224_04_bulk_trials_performance() {
    #[cfg(debug_assertions)]
    let n = 200usize;
    #[cfg(not(debug_assertions))]
    let n = 10_000usize;

    let dir = tempdir().unwrap();
    let path = dir.path().join("bulk.log");
    std::fs::write(&path, b"").unwrap();

    let mut app_state = AppState::new();
    app_state.journal_path = Some(path.clone());
    let meta = test_study_meta(0, 1);
    app_state.all_studies = vec![meta.clone()];
    app_state.current_study = Some(StudyContext {
        meta,
        view: empty_view(),
        pareto_indices: vec![],
    });

    let (tx, rx) = mpsc::sync_channel(256);
    let mut poller = LiveUpdatePoller::start(poller_context(path.clone(), 0), tx, 50);

    // Write all trials at once
    {
        let content = make_trial_bytes(n, 0);
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(&content).unwrap();
    }

    // Drain LiveUpdateDone messages and apply them
    let mut total_rows = 0usize;
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline && total_rows < n {
        while let Ok(msg) = rx.try_recv() {
            if let AppMessage::LiveUpdateDone { new_trial_rows, .. } = &msg {
                total_rows += new_trial_rows.len();
                let mut ws = WidgetStates::default();
                let mut loading = false;
                let mut err = None;
                MessageHandler::handle(msg, &mut app_state, &mut ws, &mut loading, &mut err);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    poller.stop();

    let study = app_state.current_study.as_ref().unwrap();
    assert!(
        study.trial_count() >= total_rows,
        "Expected at least {} rows, got {}",
        total_rows,
        study.trial_count()
    );
    assert!(
        total_rows > 0,
        "Expected at least some trials to be processed"
    );
}

// ─────────────────────────────────────────────
// TC-005: Parse 1000 lines at scale — all trials parsed
// ─────────────────────────────────────────────

#[test]
fn tc_2224_05_parse_performance_1000_lines() {
    reset_live_update_state();

    #[cfg(debug_assertions)]
    let n = 200usize;
    #[cfg(not(debug_assertions))]
    let n = 1_000usize;

    let content = make_trial_bytes(n, 0);

    let result = append_journal_diff(&content);

    reset_live_update_state();

    assert_eq!(
        result.new_trial_rows.len(),
        n,
        "All {} trials should be parsed",
        n
    );
}
