use super::files::has_discardable_state;
use super::*;
use crate::state::app_state::StudyMeta;

fn make_channel() -> (mpsc::SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
    mpsc::sync_channel(32)
}

fn artifact_test_app(root: Option<std::path::PathBuf>) -> TunnyApp {
    let (tx, rx) = make_channel();
    let mut app_state = AppState::new();
    app_state.artifacts_dir = root.clone();
    app_state.explicit_artifact_root = root;
    TunnyApp {
        app_state,
        layout: LayoutState::default(),
        widget_states: WidgetStates::default(),
        canvas_widgets: HashMap::new(),
        is_loading: false,
        load_error: None,
        tx,
        rx,
        pending_reload: None,
        latest_artifact_scan_id: 0,
        reload_when_idle: false,
        current_window_title: None,
        beta_notice: BetaNoticeState::default(),
        beta_notice_ack: None,
    }
}

fn poll_until(app: &mut TunnyApp, ready: impl Fn(&TunnyApp) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready(app) {
        app.poll_messages(&egui::Context::default());
        assert!(app.load_error.is_none(), "{:?}", app.load_error);
        assert!(std::time::Instant::now() < deadline, "worker timed out");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn designexplorer_worker_rebases_images_and_gui_selection_survives_late_messages() {
    let source = tempfile::tempdir().unwrap();
    let cli_root = tempfile::tempdir().unwrap();
    let gui_root = tempfile::tempdir().unwrap();
    let path = source.path().join("results.csv");
    std::fs::write(
        &path,
        "in:x,out:f,img\n1,2,picture.png\n2,3,second.png\n3,4,../outside.png\n",
    )
    .unwrap();
    std::fs::write(cli_root.path().join("picture.png"), b"cli").unwrap();
    std::fs::write(gui_root.path().join("second.png"), b"gui").unwrap();
    // A legacy-looking file must not be inferred into the img association map.
    std::fs::write(gui_root.path().join("0_inferred.png"), b"legacy").unwrap();
    let mut app = artifact_test_app(Some(cli_root.path().to_path_buf()));
    app.open_path(path.clone());
    poll_until(&mut app, |app| app.app_state.csv_import_settings.is_some());
    let meta = app.app_state.all_studies[0].clone();
    app.apply_toolbar_actions(vec![ToolbarAction::SelectStudy(meta)]);
    poll_until(&mut app, |app| app.app_state.csv_images.is_some());
    assert_eq!(app.app_state.artifact_map.len(), 1);
    assert_eq!(
        app.app_state.artifact_map[&0][0].path,
        cli_root.path().join("picture.png")
    );
    let images = app.app_state.csv_images.clone().unwrap();
    app.apply_toolbar_actions(vec![ToolbarAction::ScanArtifacts(
        gui_root.path().to_path_buf(),
    )]);
    assert_eq!(app.app_state.artifact_map.len(), 1);
    assert_eq!(
        app.app_state.artifact_map[&1][0].path,
        gui_root.path().join("second.png")
    );
    app.tx
        .send(AppMessage::CsvArtifacts {
            source_path: path.clone(),
            source_generation: app.app_state.source_generation,
            images: images.clone(),
        })
        .unwrap();
    app.tx
        .send(AppMessage::ArtifactsDirScanned {
            trial_artifacts: HashMap::new(),
            artifacts_dir: cli_root.path().to_path_buf(),
            scan_id: app.latest_artifact_scan_id,
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert_eq!(
        app.app_state.artifacts_dir.as_deref(),
        Some(gui_root.path())
    );
    assert!(app.app_state.artifact_map.contains_key(&1));

    // Reopening the same source invalidates even same-path late CSV results.
    let old_generation = app.app_state.source_generation;
    app.open_path(path.clone());
    app.tx
        .send(AppMessage::CsvArtifacts {
            source_path: path,
            source_generation: old_generation,
            images,
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert!(app.app_state.csv_images.is_none());
    assert!(app.app_state.artifact_map.is_empty());
    assert_eq!(
        app.app_state.explicit_artifact_root.as_deref(),
        Some(gui_root.path())
    );
    poll_until(&mut app, |app| {
        !app.is_loading && !app.app_state.all_studies.is_empty()
    });

    let other_path = source.path().join("other.csv");
    std::fs::write(&other_path, "in:x,out:f,img\n5,6,second.png\n").unwrap();
    app.open_path(other_path);
    poll_until(&mut app, |app| {
        !app.is_loading && !app.app_state.all_studies.is_empty()
    });
    let meta = app.app_state.all_studies[0].clone();
    app.apply_toolbar_actions(vec![ToolbarAction::SelectStudy(meta)]);
    poll_until(&mut app, |app| app.app_state.csv_images.is_some());
    assert_eq!(
        app.app_state.artifacts_dir.as_deref(),
        Some(gui_root.path())
    );
    assert_eq!(
        app.app_state.artifact_map[&0][0].path,
        gui_root.path().join("second.png")
    );

    // Switching source formats and the real reload dispatch both retain GUI priority.
    let journal = source.path().join("study.log");
    std::fs::write(
        &journal,
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"test\",\"directions\":[1]}\n",
    )
    .unwrap();
    std::fs::write(gui_root.path().join("10_result.png"), b"journal artifact").unwrap();
    app.app_state.csv_import_settings = None;
    app.open_path(journal);
    poll_until(&mut app, |app| {
        !app.is_loading && app.app_state.artifact_map.contains_key(&10)
    });
    assert_eq!(
        app.app_state.artifacts_dir.as_deref(),
        Some(gui_root.path())
    );
    let old_scan = app.latest_artifact_scan_id;
    app.reload_current();
    assert!(app.is_loading);
    app.tx
        .send(AppMessage::ArtifactsDirScanned {
            trial_artifacts: HashMap::new(),
            artifacts_dir: cli_root.path().to_path_buf(),
            scan_id: old_scan,
        })
        .unwrap();
    poll_until(&mut app, |app| {
        !app.is_loading
            && app.pending_reload.is_none()
            && app.app_state.artifact_map.contains_key(&10)
    });
    assert_eq!(
        app.app_state.explicit_artifact_root.as_deref(),
        Some(gui_root.path())
    );
    assert_eq!(
        app.app_state.artifact_map[&10][0].path,
        gui_root.path().join("10_result.png")
    );
}

#[test]
fn designexplorer_omission_uses_parent_and_rejects_other_source_messages() {
    let source = tempfile::tempdir().unwrap();
    let path = source.path().join("results.csv");
    std::fs::write(source.path().join("image.png"), b"image").unwrap();
    let mut app = artifact_test_app(None);
    app.app_state.journal_path = Some(path.clone());
    app.tx
        .send(AppMessage::CsvArtifacts {
            source_path: path,
            source_generation: 0,
            images: vec![(7, "image.png".into())],
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert_eq!(app.app_state.artifacts_dir.as_deref(), Some(source.path()));
    assert_eq!(
        app.app_state.artifact_map[&7][0].path,
        source.path().join("image.png")
    );
    app.tx
        .send(AppMessage::CsvArtifacts {
            source_path: source.path().join("other.csv"),
            source_generation: 0,
            images: vec![],
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert!(app.app_state.artifact_map.contains_key(&7));
}

#[test]
fn session_root_survives_study_activation_and_new_but_omission_still_clears() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("9_result.png"), b"artifact").unwrap();
    let mut app = artifact_test_app(Some(root.path().to_path_buf()));
    app.app_state.journal_path = Some("study.log".into());
    for study_id in [98210, 98211] {
        app.tx
            .send(AppMessage::StudyChunkLoaded {
                study_id,
                meta: StudyMeta {
                    study_id,
                    name: "study".into(),
                    directions: vec![],
                    completed_trials: 0,
                    param_names: vec![],
                    objective_names: vec![],
                    param_bounds: Default::default(),
                },
                new_rows: vec![],
                param_names: vec![],
                objective_names: vec![],
                user_attr_numeric_names: vec![],
                user_attr_string_names: vec![],
                max_constraints: 0,
                has_constraints: false,
                is_first: true,
                is_final: true,
            })
            .unwrap();
        app.poll_messages(&egui::Context::default());
        poll_until(&mut app, |app| !app.app_state.artifact_map.is_empty());
        assert_eq!(app.app_state.artifacts_dir.as_deref(), Some(root.path()));
    }
    app.reset_to_empty();
    assert_eq!(
        app.app_state.explicit_artifact_root.as_deref(),
        Some(root.path())
    );
    assert!(app.app_state.artifact_map.is_empty());
    let mut omitted = AppState::new();
    omitted.artifacts_dir = Some(root.path().to_path_buf());
    omitted.clear();
    assert!(omitted.artifacts_dir.is_none());
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
        source_df: std::sync::Arc::new(tunny_core::dataframe::DataFrame::empty()),
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

// ── File > New: when to confirm before resetting ───────────

#[test]
fn has_discardable_state_is_false_on_the_startup_state() {
    // Nothing open and nothing placed: a New here produces the state the app is
    // already in, so asking for confirmation would be pure noise.
    assert!(!has_discardable_state(
        &AppState::new(),
        &LayoutState::default()
    ));
}

#[test]
fn has_discardable_state_detects_a_scanned_storage() {
    let mut app_state = AppState::new();
    app_state.all_studies.push(StudyMeta {
        study_id: 0,
        name: "test".to_string(),
        directions: vec![],
        completed_trials: 0,
        param_names: vec![],
        objective_names: vec![],
        param_bounds: Default::default(),
    });
    assert!(has_discardable_state(&app_state, &LayoutState::default()));
}

#[test]
fn has_discardable_state_detects_a_storage_holding_no_studies() {
    // The title bar names the file even when the scan found nothing in it, so a
    // silent reset would look like the app dropped an open file on its own.
    let mut app_state = AppState::new();
    app_state.journal_path = Some(std::path::PathBuf::from("/tmp/empty.log"));
    assert!(has_discardable_state(&app_state, &LayoutState::default()));
}

#[test]
fn has_discardable_state_detects_canvas_items_without_data() {
    // A layout can be built up from a restored session before any file is opened,
    // and it is only recoverable from a session file.
    let mut layout = LayoutState::default();
    layout.canvas.add(
        crate::state::layout_state::PanelItem::TrialTable,
        0.0,
        0.0,
        360.0,
        280.0,
    );
    assert!(has_discardable_state(&AppState::new(), &layout));
}
