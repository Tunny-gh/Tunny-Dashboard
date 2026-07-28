use super::*;
use crate::state::app_state::StudyMeta;

fn make_channel() -> (mpsc::SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
    mpsc::sync_channel(32)
}

#[test]
fn unsupported_drop_message_guides_gh_binary_to_ghx() {
    let msg = unsupported_drop_message(&[std::path::PathBuf::from("/a/model.gh")]);
    assert!(msg.contains("model.gh"), "{msg}");
    assert!(msg.contains(".ghx"), "{msg}");
    assert!(msg.contains("Save As"), "{msg}");
}

#[test]
fn unsupported_drop_message_lists_supported_types() {
    let msg = unsupported_drop_message(&[std::path::PathBuf::from("notes.txt")]);
    assert!(msg.contains("notes.txt"), "{msg}");
    assert!(msg.contains("unsupported file type"), "{msg}");
    assert!(msg.contains(".ghx"), "{msg}");
}

#[test]
fn unsupported_drop_message_handles_missing_paths() {
    let msg = unsupported_drop_message(&[]);
    assert!(msg.contains("(unknown)"), "{msg}");
}

#[test]
fn channel_send_receive_journal_parsed() {
    let (tx, rx) = make_channel();
    let studies = vec![StudyMeta {
        study_id: 0,
        name: "test".to_string(),
        directions: vec![],
        completed_trials: 5,
        param_names: vec!["x".to_string()],
        objective_names: vec!["y".to_string()],
        param_bounds: Default::default(),
    }];
    tx.send(AppMessage::JournalParsed {
        studies,
        path: std::path::PathBuf::from("test.log"),
    })
    .unwrap();
    match rx.recv().unwrap() {
        AppMessage::JournalParsed { studies: s, .. } => assert_eq!(s.len(), 1),
        _ => panic!("Expected JournalParsed"),
    }
}

#[test]
fn channel_try_recv_empty_returns_error() {
    let (_tx, rx) = make_channel();
    assert!(rx.try_recv().is_err());
}

#[test]
fn convergence_done_maps_to_compute_sync() {
    // Regression guard: if IndicatorHistoryDone falls out of the sync targets, the
    // canvas item's computing flag never drops after compute finishes and the
    // spinner keeps spinning.
    use crate::state::app_state::ConvergenceHistory;
    let msg = AppMessage::IndicatorHistoryDone {
        indicator: tunny_core::indicators::MoIndicator::Hypervolume,
        base: ConvergenceHistory {
            trial_ids: vec![],
            values: vec![],
            sample_step: 1,
            ref_point: vec![],
        },
        comparisons: vec![],
    };
    assert!(matches!(
        ComputeSyncKind::from_message(&msg),
        Some(ComputeSyncKind::Convergence)
    ));
}

#[test]
fn surrogate_multi_messages_map_to_compute_sync() {
    // Regression guard: if multi-objective surrogate completion/failure falls out of
    // the sync targets, the canvas item's fitting/optimizing flag never drops and the
    // spinner keeps spinning.
    assert!(matches!(
        ComputeSyncKind::from_message(&AppMessage::SurrogateMultiFitFailed("e".into())),
        Some(ComputeSyncKind::SurrogateFit)
    ));
    assert!(matches!(
        ComputeSyncKind::from_message(&AppMessage::SurrogateMultiOptFailed("e".into())),
        Some(ComputeSyncKind::SurrogateOpt)
    ));
    let done =
        AppMessage::SurrogateMultiOptDone(crate::state::messages::SurrogateMultiOptUiResult {
            param_names: vec![],
            objective_names: vec![],
            front: vec![],
            r_squared: vec![],
        });
    assert!(matches!(
        ComputeSyncKind::from_message(&done),
        Some(ComputeSyncKind::SurrogateOpt)
    ));
    let fit_done = AppMessage::SurrogateMultiFitDone(std::sync::Arc::new(vec![]));
    assert!(matches!(
        ComputeSyncKind::from_message(&fit_done),
        Some(ComputeSyncKind::SurrogateFit)
    ));
}

#[test]
fn spawn_task_sends_message() {
    let (tx, rx) = make_channel();
    spawn_task(tx, || AppMessage::Error("from thread".to_string()));
    let msg = rx.recv().unwrap();
    match msg {
        AppMessage::Error(e) => assert_eq!(e, "from thread"),
        _ => panic!("Expected Error"),
    }
}

#[test]
fn spawn_task_captures_panic() {
    // M-4: a panic inside a worker is reported as TaskPanicked, preventing an
    // infinite spinner.
    let (tx, rx) = make_channel();
    spawn_task(tx, || panic!("boom in worker"));
    match rx.recv().unwrap() {
        AppMessage::TaskPanicked(detail) => assert!(detail.contains("boom in worker")),
        _ => panic!("Expected TaskPanicked"),
    }
}

#[test]
fn spawn_task_multiple_messages() {
    let (tx, rx) = make_channel();
    let tx2 = tx.clone();
    spawn_task(tx, || AppMessage::Error("msg1".to_string()));
    spawn_task(tx2, || AppMessage::Error("msg2".to_string()));
    let mut received = vec![];
    for _ in 0..2 {
        match rx.recv().unwrap() {
            AppMessage::Error(e) => received.push(e),
            _ => panic!("Expected Error"),
        }
    }
    assert_eq!(received.len(), 2);
}

#[test]
fn toggle_live_update_updates_state() {
    let mut app_state = AppState::new();
    assert!(!app_state.live_update.enabled);
    app_state.live_update.enabled = true;
    assert!(app_state.live_update.enabled);
    app_state.live_update.enabled = false;
    assert!(!app_state.live_update.enabled);
}

#[test]
fn set_poll_interval_updates_state() {
    let mut app_state = AppState::new();
    assert_eq!(app_state.live_update.interval_ms, 5000);
    app_state.live_update.interval_ms = 10000;
    assert_eq!(app_state.live_update.interval_ms, 10000);
}

// ── Phase C: window title password masking ─────────────────

#[test]
fn compute_window_title_no_path_returns_base_title() {
    assert_eq!(
        TunnyApp::compute_window_title(None),
        "Tunny Dashboard (Beta)"
    );
}

#[test]
fn compute_window_title_local_path_shows_full_path() {
    let path = std::path::PathBuf::from("/home/user/study.log");
    assert_eq!(
        TunnyApp::compute_window_title(Some(&path)),
        "Tunny Dashboard (Beta) - /home/user/study.log"
    );
}

#[test]
fn compute_window_title_rdb_url_masks_password() {
    let path = std::path::PathBuf::from("postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test");
    assert_eq!(
        TunnyApp::compute_window_title(Some(&path)),
        "Tunny Dashboard (Beta) - postgresql://tunny:***@127.0.0.1:5432/tunny_test"
    );
}

#[test]
fn compute_window_title_rdb_url_without_password_unchanged() {
    let path = std::path::PathBuf::from("mysql://tunny@127.0.0.1:3306/tunny_test");
    assert_eq!(
        TunnyApp::compute_window_title(Some(&path)),
        "Tunny Dashboard (Beta) - mysql://tunny@127.0.0.1:3306/tunny_test"
    );
}
