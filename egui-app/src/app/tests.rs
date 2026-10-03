use super::files::has_discardable_state;
use super::*;
use crate::state::app_state::StudyMeta;

fn make_channel() -> (mpsc::SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
    mpsc::sync_channel(32)
}

fn artifact_test_app() -> TunnyApp {
    let (tx, rx) = make_channel();
    TunnyApp {
        app_state: AppState::new(),
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

fn pump_until(app: &mut TunnyApp, ready: impl Fn(&TunnyApp) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        app.poll_messages(&egui::Context::default());
        assert!(app.load_error.is_none(), "{:?}", app.load_error);
        if ready(app) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "message pump timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn select_artifact_study(app: &mut TunnyApp, id: u32) {
    let meta = app
        .app_state
        .all_studies
        .iter()
        .find(|m| m.study_id == id)
        .unwrap()
        .clone();
    app.apply_toolbar_actions(vec![ToolbarAction::SelectStudy(meta)]);
    pump_until(app, |a| {
        !a.is_loading
            && a.app_state
                .current_study
                .as_ref()
                .is_some_and(|s| s.meta.study_id == id)
    });
}

fn verify_gallery_render_and_image(app: &mut TunnyApp, expected_trials: usize) {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let study = app.app_state.current_study.as_ref().unwrap();
    let entry = study
        .view
        .trial_ids
        .iter()
        .find_map(|id| app.app_state.artifact_map.get(id).and_then(|v| v.first()))
        .unwrap();
    assert!(entry.path.extension().is_none());
    let uri = crate::io::artifacts::file_image_uri(&entry.path).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match ctx.try_load_image(&uri, egui::load::SizeHint::default()) {
            Ok(egui::load::ImagePoll::Ready { image }) => {
                assert!(image.size[0] > 0 && image.size[1] > 0);
                println!("Loaded extensionless image: {uri}, size {:?}", image.size);
                break;
            }
            Ok(egui::load::ImagePoll::Pending { .. }) => {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(e) => panic!("image loader failed: {e}"),
        }
    }
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            ..Default::default()
        },
        |ui| {
            app.widget_states
                .artifact_gallery
                .show(ui, &mut app.app_state)
        },
    );
    let text = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!text.contains("No artifacts loaded"), "{text}");
    assert!(
        text.contains(&format!("{expected_trials} trials with artifacts")),
        "{text}"
    );
    assert!(text.contains("Trial"), "{text}");
    assert!(
        ctx.tessellate(output.shapes.clone(), output.pixels_per_point)
            .iter()
            .any(|s| matches!(&s.primitive,
            egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id != egui::TextureId::default()
            )),
        "gallery did not draw an image texture"
    );
    println!(
        "Gallery rendered {} shapes; {expected_trials} trials with artifacts",
        output.shapes.len()
    );
}

#[test]
fn artifacts_follow_storage_lifecycle_through_app_actions() {
    let _guard = test_store_guard();
    let tmp = tempfile::tempdir().unwrap();
    let journal = tmp.path().join("study.log");
    let folder = tmp.path().join("artifacts");
    std::fs::create_dir(&folder).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save_with_format(folder.join("target"), image::ImageFormat::Png)
        .unwrap();
    let records = [
        r#"{"op_code":0,"study_name":"a","directions":[1]}"#,
        r#"{"op_code":0,"study_name":"b","directions":[1]}"#,
        r#"{"op_code":4,"study_id":0,"state":1,"values":[1],"distributions":{},"system_attrs":{"artifacts:target":"{\"filename\":\"result.png\",\"mimetype\":\"image/png\"}"}}"#,
        r#"{"op_code":4,"study_id":1,"state":1,"values":[2],"distributions":{}}"#,
    ].join("\n");
    std::fs::write(&journal, &records).unwrap();
    let mut app = artifact_test_app();
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal.clone())]);
    pump_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    // Manual scan before the first streamed study activation.
    app.apply_toolbar_actions(vec![ToolbarAction::ScanArtifacts(folder.clone())]);
    pump_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
    select_artifact_study(&mut app, 0);
    assert_eq!(app.app_state.artifact_map[&0].len(), 1);
    assert_eq!(app.app_state.artifacts_dir.as_ref(), Some(&folder));
    verify_gallery_render_and_image(&mut app, 1);
    select_artifact_study(&mut app, 1); // first streamed chunk
    select_artifact_study(&mut app, 0); // cached StudySelected
    assert_eq!(app.app_state.artifact_map[&0].len(), 1);

    // Explicit folder operation after study activation remains authoritative.
    let manual = tmp.path().join("manual");
    std::fs::create_dir(&manual).unwrap();
    std::fs::write(manual.join("target"), b"image").unwrap();
    app.apply_toolbar_actions(vec![ToolbarAction::ScanArtifacts(manual.clone())]);
    pump_until(&mut app, |a| {
        a.app_state.artifacts_dir.as_ref() == Some(&manual)
            && a.app_state
                .artifact_map
                .get(&0)
                .is_some_and(|v| v[0].path == manual.join("target"))
    });
    select_artifact_study(&mut app, 1);
    select_artifact_study(&mut app, 0);
    assert_eq!(app.app_state.artifacts_dir.as_ref(), Some(&manual));
    std::fs::remove_file(manual.join("target")).unwrap();
    app.apply_toolbar_actions(vec![ToolbarAction::Reload]);
    pump_until(&mut app, |a| {
        !a.is_loading && a.pending_reload.is_none() && a.app_state.artifact_map.is_empty()
    });
    assert_eq!(app.app_state.artifacts_dir.as_ref(), Some(&manual));

    // A different storage reuses trial ID zero but has no adjacent folder.
    let other = tempfile::tempdir().unwrap();
    let other_journal = other.path().join("other.log");
    std::fs::write(&other_journal, &records).unwrap();
    let stale = app.latest_artifact_scan_id;
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(other_journal)]);
    app.tx
        .send(AppMessage::ArtifactsDirScanned {
            trial_artifacts: HashMap::from([(0, vec![])]),
            artifacts_dir: manual.clone(),
            scan_id: Some(stale),
        })
        .unwrap();
    pump_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    select_artifact_study(&mut app, 0);
    assert!(app.app_state.artifact_map.is_empty());
    assert!(app.app_state.artifacts_dir.is_none());

    // Fresh open automatically discovers the adjacent folder before selection.
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal)]);
    pump_until(&mut app, |a| {
        a.app_state.all_studies.len() == 2 && a.app_state.artifact_map.contains_key(&0)
    });
    select_artifact_study(&mut app, 0);
    assert_eq!(app.app_state.artifacts_dir.as_ref(), Some(&folder));
    let stale = app.latest_artifact_scan_id;
    app.apply_toolbar_actions(vec![ToolbarAction::New]);
    assert!(app.app_state.new_confirm_open);
    app.reset_to_empty(); // confirmation's production action
    app.tx
        .send(AppMessage::ArtifactsDirScanned {
            trial_artifacts: HashMap::from([(0, vec![])]),
            artifacts_dir: folder,
            scan_id: Some(stale),
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert!(app.app_state.artifact_map.is_empty());
    assert!(app.app_state.artifacts_dir.is_none());
}

fn write_artifact_race_fixture(dir: &std::path::Path, image_name: &str) -> std::path::PathBuf {
    let folder = dir.join("artifacts");
    std::fs::create_dir(&folder).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .save_with_format(folder.join(image_name), image::ImageFormat::Png)
        .unwrap();
    let path = dir.join("study.log");
    let records = [
        r#"{"op_code":0,"study_name":"a","directions":[1]}"#.to_string(),
        r#"{"op_code":0,"study_name":"b","directions":[1]}"#.to_string(),
        format!(
            r#"{{"op_code":4,"study_id":0,"state":1,"values":[1],"distributions":{{}},"system_attrs":{{"artifacts:{image_name}":"{{\"filename\":\"result.png\",\"mimetype\":\"image/png\"}}"}}}}"#
        ),
    ];
    std::fs::write(&path, records.join("\n")).unwrap();
    path
}

#[test]
fn artifact_race_late_journal_scan_cannot_replace_requested_storage() {
    let _guard = test_store_guard();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let a_path = write_artifact_race_fixture(a.path(), "image-a");
    // Workers echo the requested local path, including lexical components.
    let b_path = write_artifact_race_fixture(b.path(), "image-b");
    let b_path = b_path.parent().unwrap().join(".").join("study.log");
    let mut app = artifact_test_app();
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(a_path)]);
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(
        egui::RawInput {
            dropped_files: vec![egui::DroppedFile {
                path: Some(b_path.clone()),
                ..Default::default()
            }],
            ..Default::default()
        },
        |ui| app.handle_dropped_files(ui.ctx()),
    );
    assert!(app.is_loading);
    assert_eq!(app.app_state.journal_path.as_ref(), Some(&b_path));

    // Hold B's real worker completion so A is processed on its own first.
    let late_a = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let current_b = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    assert!(matches!(&late_a, AppMessage::JournalParsed { path, .. } if path != &b_path));
    assert!(matches!(&current_b, AppMessage::JournalParsed { path, .. } if path == &b_path));
    let scan_id = app.latest_artifact_scan_id;
    app.tx.send(late_a).unwrap();
    app.poll_messages(&ctx);
    assert_eq!(
        app.latest_artifact_scan_id, scan_id,
        "late A started an artifact scan"
    );
    assert_eq!(app.app_state.journal_path.as_ref(), Some(&b_path));
    assert!(app.is_loading);
    assert!(app.app_state.all_studies.is_empty());
    assert!(app.app_state.artifacts_dir.is_none());

    app.tx.send(current_b).unwrap();
    pump_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
    select_artifact_study(&mut app, 0);
    assert_eq!(
        app.app_state.artifacts_dir,
        Some(b_path.parent().unwrap().join("artifacts"))
    );
    assert_eq!(app.app_state.artifact_map.len(), 1);
    assert_eq!(
        app.app_state.artifact_map[&0][0].path.file_name().unwrap(),
        "image-b"
    );
    verify_gallery_render_and_image(&mut app, 1);
    let studies = app.app_state.all_studies.clone();
    app.reset_to_empty();
    let scan_id = app.latest_artifact_scan_id;
    app.tx
        .send(AppMessage::JournalParsed {
            studies,
            path: b_path,
        })
        .unwrap();
    app.poll_messages(&ctx);
    assert_eq!(app.latest_artifact_scan_id, scan_id);
    assert!(app.app_state.journal_path.is_none());
    assert!(app.app_state.all_studies.is_empty());
    assert!(app.app_state.artifacts_dir.is_none());
}

#[test]
fn artifact_race_manual_folder_during_reload_stays_authoritative() {
    let _guard = test_store_guard();
    let tmp = tempfile::tempdir().unwrap();
    let journal = write_artifact_race_fixture(tmp.path(), "target");
    let manual = tmp.path().join("manual");
    std::fs::create_dir(&manual).unwrap();
    image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 255, 0, 255]))
        .save_with_format(manual.join("target"), image::ImageFormat::Png)
        .unwrap();
    let mut app = artifact_test_app();
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal.clone())]);
    pump_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
    select_artifact_study(&mut app, 0);
    assert_eq!(
        app.app_state.artifacts_dir,
        Some(tmp.path().join("artifacts"))
    );

    app.apply_toolbar_actions(vec![ToolbarAction::Reload]);
    assert!(app.pending_reload.is_some());
    assert!(app.is_loading);
    app.apply_toolbar_actions(vec![ToolbarAction::ScanArtifacts(manual.clone())]);
    // Process the explicit scan before allowing the reload's study activation.
    let mut held_reload = None;
    let mut manual_scan = None;
    while held_reload.is_none() || manual_scan.is_none() {
        let msg = app
            .rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        if matches!(&msg, AppMessage::ArtifactsDirScanned { .. }) {
            manual_scan = Some(msg);
        } else {
            assert!(matches!(&msg, AppMessage::JournalParsed { .. }));
            held_reload = Some(msg);
        }
    }
    app.tx.send(manual_scan.unwrap()).unwrap();
    app.poll_messages(&egui::Context::default());
    assert_eq!(
        app.app_state.artifact_map[&0][0].path,
        manual.join("target")
    );
    // Only the post-reload rescan can pick up this newly added metadata/file.
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&journal)
        .unwrap();
    file.write_all(concat!("\n", r#"{"op_code":9,"trial_id":0,"system_attr":{"artifacts:new":"{\"filename\":\"new.png\",\"mimetype\":\"image/png\"}"}}"#, "\n").as_bytes()).unwrap();
    std::fs::copy(manual.join("target"), manual.join("new")).unwrap();
    app.tx.send(held_reload.unwrap()).unwrap();
    pump_until(&mut app, |a| !a.is_loading && a.pending_reload.is_none());
    assert_eq!(app.app_state.artifacts_dir.as_ref(), Some(&manual));
    pump_until(&mut app, |a| a.app_state.artifact_map[&0].len() == 2);
    assert!(app.app_state.artifact_map[&0]
        .iter()
        .all(|e| e.path.parent() == Some(manual.as_path())));
    verify_gallery_render_and_image(&mut app, 1);
}

#[test]
fn journal_scan_guard_accepts_normalized_rdb_source() {
    let mut app = artifact_test_app();
    let requested = std::path::PathBuf::from("postgresql+psycopg2://u:p@localhost/db");
    let normalized =
        std::path::PathBuf::from(crate::io::rdb::path_as_rdb_url(&requested).unwrap().url);
    assert_ne!(requested, normalized);
    app.app_state.journal_path = Some(requested);
    app.is_loading = true;
    app.tx
        .send(AppMessage::JournalParsed {
            studies: vec![],
            path: normalized.clone(),
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert!(!app.is_loading);
    assert_eq!(app.app_state.journal_path, Some(normalized));
    assert_eq!(app.latest_artifact_scan_id, 0);
}

/// Read-only verification against a user-supplied Journal and adjacent artifacts.
#[test]
#[ignore = "requires TUNNY_ARTIFACT_JOURNAL and TUNNY_ARTIFACT_STUDY"]
fn real_journal_artifacts_through_app_actions() {
    let _guard = test_store_guard();
    let journal = std::path::PathBuf::from(std::env::var_os("TUNNY_ARTIFACT_JOURNAL").unwrap());
    let study_name = std::env::var("TUNNY_ARTIFACT_STUDY").unwrap();
    let mut app = artifact_test_app();
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal.clone())]);
    pump_until(&mut app, |a| {
        !a.app_state.all_studies.is_empty() && !a.app_state.artifact_map.is_empty()
    });
    let id = app
        .app_state
        .all_studies
        .iter()
        .find(|m| m.name == study_name)
        .unwrap()
        .study_id;
    select_artifact_study(&mut app, id);
    let count = app
        .app_state
        .current_study
        .as_ref()
        .unwrap()
        .view
        .trial_ids
        .iter()
        .filter(|id| {
            app.app_state
                .artifact_map
                .get(id)
                .is_some_and(|v| !v.is_empty())
        })
        .count();
    assert_eq!(count, 501);
    verify_gallery_render_and_image(&mut app, count);
    let folder = journal.parent().unwrap().join("artifacts");
    let prior_scan = app.latest_artifact_scan_id;
    app.apply_toolbar_actions(vec![ToolbarAction::ScanArtifacts(folder.clone())]);
    // Wait for the actual worker result, then pass it through poll_messages.
    let msg = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    assert!(
        matches!(&msg, AppMessage::ArtifactsDirScanned { scan_id: Some(id), .. } if *id > prior_scan)
    );
    app.tx.send(msg).unwrap();
    app.poll_messages(&egui::Context::default());
    assert_eq!(app.app_state.artifacts_dir, Some(folder));
    verify_gallery_render_and_image(&mut app, 501);
    println!("Verified study {study_name}: automatic and explicit manual scans, 501 entries");
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
