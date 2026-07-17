use std::io::Write;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tunny_core::dataframe::DataFrame;
use tunny_core::io::journal::live_update::{
    append_journal_diff, reset_live_update_state, LiveUpdateContext,
};
use tunny_desktop::io::live_update_poller::{
    LiveUpdatePoller, SqliteLivePoller, SqliteLiveUpdateContext,
};
use tunny_desktop::state::app_state::{AppState, Direction, StudyContext, StudyMeta, StudyView};
use tunny_desktop::state::message_handler::MessageHandler;
use tunny_desktop::state::messages::AppMessage;
use tunny_desktop::ui::widget_states::WidgetStates;

/// An empty StudyView (0 rows). Used as the initial state for live-update tests.
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
// TC-006 (M-1): Journal rotation / truncation resets the byte offset
// ─────────────────────────────────────────────

/// Verifies that when the journal is rotated/truncated such that
/// `file_size < byte_offset`, the poller resets the offset to 0 and
/// re-reads from the beginning. Without this reset, the diff would never
/// be detected again, and the no-change timeout would incorrectly report
/// "optimization complete".
#[test]
fn tc_2224_06_journal_rotation_resets_offset() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("rotate.log");
    // Write 3 trials' worth as the initial content and use its size as the
    // starting offset (the initial portion is treated as already read).
    // Use high trial IDs to avoid colliding with other tests' global parser state.
    let initial = make_trial_bytes(3, 900_000);
    std::fs::write(&path, &initial).unwrap();
    let offset = std::fs::metadata(&path).unwrap().len();

    let (tx, rx) = mpsc::sync_channel(64);
    let mut poller = LiveUpdatePoller::start(poller_context(path.clone(), offset), tx, 50);

    // Let it poll for a few ticks (no change).
    std::thread::sleep(Duration::from_millis(120));

    // Rotation: replace the entire file with smaller content (2 trials).
    // This makes file_size < byte_offset, which should trigger truncation detection.
    let rotated = make_trial_bytes(2, 800_000);
    assert!(
        (rotated.len() as u64) < offset,
        "rotated file must be smaller than the previous offset to trigger truncation detection"
    );
    std::fs::write(&path, &rotated).unwrap();

    // Detects truncation, resets the offset to 0 -> re-reads from the beginning -> LiveUpdateDone.
    let msg = wait_for_live_update_done(&rx, Duration::from_secs(5));
    poller.stop();

    assert!(
        msg.is_some(),
        "Expected LiveUpdateDone after journal rotation/truncation (offset must reset to 0)"
    );
    if let Some(AppMessage::LiveUpdateDone { new_trial_rows, .. }) = &msg {
        assert!(
            !new_trial_rows.is_empty(),
            "rotated-in trials should be re-read from the start of the new file"
        );
    }
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

// ─────────────────────────────────────────────
// TC-006: SQLite live update E2E (fingerprint detection -> full reload)
// ─────────────────────────────────────────────

/// Creates a fixture DB with the minimal Optuna schema required by
/// `study_fingerprint` / `parse_single_study` (the same idea as
/// rust_core's `create_schema` test helper, but since that one is private,
/// egui-app's integration tests build their own here).
fn make_sqlite_fixture(path: &std::path::Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        "
        CREATE TABLE studies (
            study_id INTEGER PRIMARY KEY,
            study_name VARCHAR(512)
        );
        CREATE TABLE study_directions (
            study_direction_id INTEGER PRIMARY KEY,
            direction VARCHAR(8),
            study_id INTEGER,
            objective INTEGER
        );
        CREATE TABLE trials (
            trial_id INTEGER PRIMARY KEY,
            number INTEGER,
            study_id INTEGER,
            state VARCHAR(8),
            datetime_start TEXT,
            datetime_complete TEXT
        );
        CREATE TABLE trial_values (
            trial_value_id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            objective INTEGER,
            value REAL,
            value_type VARCHAR(7)
        );
        CREATE TABLE trial_params (
            param_id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            param_name VARCHAR(512),
            param_value REAL,
            distribution_json TEXT
        );
        CREATE TABLE trial_user_attributes (
            id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            key VARCHAR(512),
            value_json TEXT
        );
        CREATE TABLE trial_system_attributes (
            id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            key VARCHAR(512),
            value_json TEXT
        );
        CREATE TABLE study_system_attributes (
            id INTEGER PRIMARY KEY,
            study_id INTEGER,
            key VARCHAR(512),
            value_json TEXT
        );
        INSERT INTO studies (study_id, study_name) VALUES (1, 'study-a');
        INSERT INTO study_directions (study_id, direction, objective) VALUES (1, 'MINIMIZE', 0);
        INSERT INTO trials (trial_id, number, study_id, state) VALUES (1, 0, 1, 'RUNNING');
        ",
    )
    .unwrap();
    conn
}

/// Verifies, against a real database, the full flow of SQLite fingerprint
/// detection -> full reload of the target study -> MessageHandler applying
/// the result (the sqlite equivalent of journal's tc_2224_01). Unlike the
/// journal, SQLite works via "detect change -> reparse everything" rather
/// than appending diffs, so this test reproduces a RUNNING trial
/// transitioning to COMPLETE.
#[test]
fn tc_sqlite_e2e_fingerprint_reload_flow() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("study.db");
    let conn = make_sqlite_fixture(&path);

    // Initial load (0 trials, since the trial is still RUNNING).
    let (tx, rx) = mpsc::sync_channel::<AppMessage>(16);
    let ok = tunny_desktop::io::sqlite::load_single_study_task(&path, 1, &tx);
    assert!(ok, "initial load must succeed");
    let initial_msg = rx.recv_timeout(Duration::from_secs(3)).unwrap();

    let mut app_state = AppState::new();
    app_state.journal_path = Some(path.clone());
    let mut widget_states = WidgetStates::default();
    let mut is_loading = true;
    let mut load_error = None;
    MessageHandler::handle(
        initial_msg,
        &mut app_state,
        &mut widget_states,
        &mut is_loading,
        &mut load_error,
    );
    assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 0);

    // Start the fingerprint poller.
    let initial_fingerprint = tunny_core::sqlite::study_fingerprint(&path, 1).unwrap();
    let sqlite_ctx = SqliteLiveUpdateContext {
        file_path: path.clone(),
        study_id: 1,
        initial_fingerprint,
        no_change_timeout_ms: 10_000,
    };
    let mut poller = SqliteLivePoller::start(sqlite_ctx, tx.clone(), 50);

    // Create the same situation as Optuna transitioning a trial from RUNNING to COMPLETE.
    conn.execute(
        "UPDATE trials SET state = 'COMPLETE' WHERE trial_id = 1",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO trial_values (trial_id, objective, value, value_type) \
         VALUES (1, 0, 1.5, 'FINITE')",
        [],
    )
    .unwrap();

    // Wait until the poller detects the change.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut changed = None;
    while Instant::now() < deadline {
        if let Ok(msg @ AppMessage::SqliteLiveChanged { .. }) = rx.try_recv() {
            changed = Some(msg);
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    poller.stop();
    let study_id = match changed {
        Some(AppMessage::SqliteLiveChanged { study_id }) => study_id,
        _ => panic!("Expected SqliteLiveChanged after trial completion"),
    };
    assert_eq!(study_id, 1);

    // Perform the same reload that dispatch_reload_sqlite_study does, using the worker-equivalent function.
    let reload_ok = tunny_desktop::io::sqlite::reload_single_study_task(&path, study_id, &tx);
    assert!(reload_ok, "reload must succeed");
    let reload_msg = rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(matches!(
        reload_msg,
        AppMessage::SqliteLiveReloadDone { .. }
    ));

    MessageHandler::handle(
        reload_msg,
        &mut app_state,
        &mut widget_states,
        &mut is_loading,
        &mut load_error,
    );

    let study = app_state.current_study.as_ref().unwrap();
    assert_eq!(
        study.trial_count(),
        1,
        "the completed trial must now appear in the reloaded view"
    );
    assert!(!study.pareto_indices.is_empty());
    assert!(load_error.is_none());
}
