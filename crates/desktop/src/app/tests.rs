use super::files::has_discardable_state;
use super::*;
use crate::state::app_state::StudyMeta;

#[test]
fn importance_all_retains_sobol_objectives_and_reuses_first_total_across_canvas_frames() {
    use crate::state::layout_state::{ChartId, PanelItem};
    use crate::ui::widgets::importance_chart::{ImportanceMetric as M, ALL_OBJECTIVES};
    let mut app = artifact_test_app(None);
    app.app_state = crate::state::analysis_job::tests::importance_multi_state(false);
    let id = app.layout.canvas.add(
        PanelItem::Chart(ChartId::ImportanceChart),
        0.0,
        0.0,
        700.0,
        500.0,
    );
    let mut widgets = WidgetStates {
        analysis_consumer: id + 1,
        ..Default::default()
    };
    widgets.importance.metric = M::SobolFirst;
    widgets.importance.pending_compute = Some((M::SobolFirst, ALL_OBJECTIVES, false));
    app.canvas_widgets.insert(id, widgets);
    let ctx = egui::Context::default();
    let run = |app: &mut TunnyApp| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let tx = app.sender();
            crate::ui::poll_chart::poll_chart_work(
                &mut app.app_state,
                app.canvas_widgets.get_mut(&id).unwrap(),
                &ChartId::ImportanceChart,
                &tx,
            );
            app.poll_messages(&ctx);
            app.retain_analysis_interests();
            let chart = &app.canvas_widgets[&id].importance;
            if !chart.computing && chart.active.is_none() && chart.queue.is_empty() {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    run(&mut app);
    let key = app
        .app_state
        .sobol_cache
        .selected(&(0, false))
        .unwrap()
        .key()
        .clone();
    for i in 0..3 {
        assert_eq!(
            app.app_state
                .sobol_cache
                .selected(&(i, false))
                .unwrap()
                .key(),
            &key,
            "ALL must reuse the single all-objective Sobol calculation"
        );
    }
    let scores = app.canvas_widgets[&id].importance.results(&app.app_state);
    assert_eq!(scores.len(), 3);
    assert_eq!(
        scores[2].scores[0].1, 0.0,
        "Constant output remains a genuine numerical zero"
    );
    let chart = &mut app.canvas_widgets.get_mut(&id).unwrap().importance;
    chart.metric = M::SobolTotal;
    chart.pending_compute = Some((M::SobolTotal, ALL_OBJECTIVES, false));
    run(&mut app);
    assert_eq!(
        app.app_state
            .sobol_cache
            .selected(&(0, false))
            .unwrap()
            .key(),
        &key
    );
    assert!(app
        .app_state
        .journal_studies
        .as_ref()
        .unwrap()
        .analysis_jobs
        .borrow()
        .is_empty());
    assert!(app.rx.try_recv().is_err());
}

fn enable_recording_reuse_checkbox(mut render: impl FnMut(&egui::Context)) {
    let ctx = egui::Context::default();
    let mut frame = |events| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 1600.0),
                )),
                events,
                ..Default::default()
            },
            |ui| render(ui.ctx()),
        )
    };
    for _ in 0..2 {
        let _ = frame(vec![]);
    }
    let output = frame(vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if t.galley.text() == "Add results to existing Study (if present)" =>
            {
                Some(t.pos + t.galley.size() * 0.5)
            }
            _ => None,
        })
        .expect("The production dialog must expose the explicit reuse checkbox");
    let text = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("not a history-aware resume"));
    for pressed in [true, false] {
        let _ = frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
    }
}

#[test]
fn d7_live_filter_session_and_csv_export_keep_trial_identity() {
    use crate::io::{export, session};
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("entry.log");
    std::fs::write(&source, concat!(
        "{\"op_code\":0,\"study_name\":\"other\",\"directions\":[1]}\n",
        "{\"op_code\":0,\"study_name\":\"entry\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":4,\"study_id\":1}\n",
        "{\"op_code\":4,\"study_id\":1,\"state\":1,\"params\":{\"x\":2},\"distributions\":{},\"values\":[20]}\n",
        "{\"op_code\":4,\"study_id\":1,\"state\":1,\"params\":{\"x\":3},\"distributions\":{},\"values\":[30]}\n",
    )).unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(source);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    select_artifact_study(&mut app, 1);
    app.app_state.set_filter("x", 2.5, 3.5);
    assert_eq!(app.app_state.selected_indices, vec![3]);
    app.app_state.pinned_trials = vec![2];
    app.app_state.convergence_indicator = tdashboard_analysis::indicators::MoIndicator::IgdPlus;
    let session_path = directory.path().join("entry-session.json");
    session::write_session_to_path(
        &LayoutState::default(),
        &Default::default(),
        &session::ViewSettings::capture(&app.app_state),
        &session_path,
    )
    .unwrap();
    app.app_state.filter_ranges.clear();
    app.app_state.pinned_trials.clear();
    app.app_state.convergence_indicator = tdashboard_analysis::indicators::MoIndicator::Hypervolume;
    app.apply_toolbar_actions(vec![ToolbarAction::LoadSession(session_path)]);
    assert_eq!(app.app_state.filter_ranges.get("x"), Some(&(2.5, 3.5)));
    assert_eq!(app.app_state.selected_indices, vec![3]);
    assert_eq!(app.app_state.pinned_trials, vec![2]);
    assert_eq!(
        app.app_state.convergence_indicator,
        tdashboard_analysis::indicators::MoIndicator::IgdPlus
    );
    let study = app.app_state.current_study.as_ref().unwrap();
    let rows = export::select_row_indices_for_export(
        &study.view,
        &app.app_state.selected_indices,
        study.pareto_indices(),
        &export::ExportTarget::SelectedOnly,
    );
    assert_eq!(rows, vec![1]);
    let csv_path = directory.path().join("selected.csv");
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    export::spawn_view_csv_export(
        study.view.clone(),
        rows,
        study.meta.param_names.clone(),
        study.meta.objective_names.clone(),
        csv_path.clone(),
        tx,
    );
    assert!(matches!(
        rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap(),
        AppMessage::CsvExportDone
    ));
    let csv = std::fs::read_to_string(csv_path).unwrap();
    let lines: Vec<_> = csv.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("trial_id,trial_number,x,obj0,"));
    assert!(lines[1].starts_with("3,2,3,30,"), "{csv}");
}

#[test]
fn d6_process_launch_progress_cancel_completion_and_reload() {
    use tdashboard_optimization_runner::process::*;
    for cancel in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.log");
        let command = if cfg!(windows) {
            CommandSpec {
                program: "cmd.exe".into(),
                args: vec!["/C".into(), "echo f=3".into()],
                ..CommandSpec::new("cmd.exe")
            }
        } else {
            CommandSpec {
                program: "sh".into(),
                args: vec!["-c".into(), "echo f=3".into()],
                ..CommandSpec::new("sh")
            }
        };
        let def = ProcessDefinition {
            param_names: vec!["x".into()],
            input: InputSpec::Env,
            command,
            objectives: vec![OutputSpec {
                name: "f".into(),
                source: OutputSource::Stdout,
                extractor: Extractor::Regex {
                    pattern: "f=([0-9]+)".into(),
                },
            }],
            constraints: vec![],
            pre_command: None,
            post_command: None,
        };
        let mut dialog = crate::state::app_state::ProcessOptDialogState::new(
            def,
            &dir.path().join("definition.json"),
        );
        dialog.journal_path = path.to_string_lossy().into_owned();
        dialog.sampler_is_random = true;
        dialog.n_trials = if cancel { 1000 } else { 3 };
        assert!(!dialog.load_if_exists);
        let mut reuse_dialog = dialog.clone();
        let mut app = artifact_test_app(None);
        app.start_process_run(dialog);
        assert!(app.app_state.process_opt_dialog.is_none());
        let progress = app.app_state.gh_opt_run.as_ref().unwrap().progress.clone();
        if cancel {
            progress.request_cancel();
        }
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {
            app.show_ghx_opt_overlay(&ctx)
        });
        poll_until(&mut app, |a| {
            !a.is_loading
                && !a.reload_when_idle
                && a.app_state
                    .gh_opt_run
                    .as_ref()
                    .is_some_and(|r| r.finished.is_some())
        });
        let finished = app
            .app_state
            .gh_opt_run
            .as_ref()
            .unwrap()
            .finished
            .as_ref()
            .unwrap();
        assert!(finished.is_ok(), "{finished:?}");
        let study = app.app_state.current_study.as_ref().unwrap();
        assert_eq!(study.view.row_count(), progress.snapshot().done);
        if cancel {
            assert!(progress.is_cancelled());
            assert!(finished.as_ref().unwrap().contains("cancelled"));
            assert!(study.view.row_count() < 1000);
        } else {
            assert_eq!(study.view.row_count(), 3);
            let input = study.view.input().unwrap();
            assert!(
                input.stamp().input.generation >= 2,
                "Completion must reload the recorded journal"
            );
        }
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {
            app.show_ghx_opt_overlay(&ctx)
        });
        if !cancel {
            let prefix = std::fs::read(&path).unwrap();
            for mismatch in [false, true] {
                let mut rejected = reuse_dialog.clone();
                rejected.load_if_exists = mismatch;
                rejected.maximize[0] = mismatch;
                let mut failed_app = artifact_test_app(None);
                failed_app.start_process_run(rejected);
                assert!(failed_app.app_state.gh_opt_run.is_none());
                let state = failed_app.app_state.process_opt_dialog.as_mut().unwrap();
                assert!(state.error.as_ref().unwrap().contains(if mismatch {
                    "directions"
                } else {
                    "already exists"
                }));
                let _ = ctx.run_ui(egui::RawInput::default(), |_| {
                    crate::ui::widgets::process_opt_modal::show(&ctx, state);
                });
                assert_eq!(std::fs::read(&path).unwrap(), prefix);
            }
            enable_recording_reuse_checkbox(|ctx| {
                crate::ui::widgets::process_opt_modal::show(ctx, &mut reuse_dialog);
            });
            assert!(reuse_dialog.load_if_exists);
            reuse_dialog.def.objectives[0].name = "changed label".into();
            let _ = ctx.run_ui(egui::RawInput::default(), |_| {
                crate::ui::widgets::process_opt_modal::show(&ctx, &mut reuse_dialog);
            });
            app.start_process_run(reuse_dialog);
            assert!(app.app_state.process_opt_dialog.is_none());
            poll_until(&mut app, |a| {
                !a.is_loading
                    && !a.reload_when_idle
                    && a.app_state
                        .gh_opt_run
                        .as_ref()
                        .is_some_and(|r| r.finished.is_some())
            });
            let run = app.app_state.gh_opt_run.as_ref().unwrap();
            assert!(run
                .finished
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .contains("3 trials succeeded"));
            assert_eq!(run.progress.snapshot().done, 3);
            let study = app.app_state.current_study.as_ref().unwrap();
            assert_eq!(study.view.row_count(), 6);
            assert_eq!(study.view.numeric_column("f").unwrap(), &[3.0; 6]);
            assert!(std::fs::read(&path).unwrap().starts_with(&prefix));
        }
    }
}

#[test]
fn d6_gh_launch_mock_compute_completion_and_reload() {
    use std::io::{BufRead, Read, Write};
    use tdashboard_optimization_runner::gh::*;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut reader = std::io::BufReader::new(stream);
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert!(!line.is_empty());
                if line.trim().is_empty() {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(request["values"][0]["ParamName"], "RH_IN:x");
            let reply = r#"{"values":[{"ParamName":"RH_OUT:f","InnerTree":{"{0}":[{"type":"System.Double","data":"7.5"}]}}]}"#;
            write!(
                reader.get_mut(),
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            )
            .unwrap();
        }
    });
    let problem = GhProblem {
        variables: vec![GhVariable {
            instance_guid: "slider".into(),
            name: "x".into(),
            low: 0.0,
            high: 10.0,
            value: 5.0,
            digits: 2,
            is_integer: false,
            gene_pool: None,
        }],
        objectives: vec![GhObjective {
            source_guid: "objective".into(),
            name: "f".into(),
        }],
        constraints: vec![],
        attributes: vec![],
        tunny_instance_guid: "tunny".into(),
        tunny_component: "Tunny".into(),
        warnings: vec![],
    };
    // Minimal archive: this mock verifies Dashboard orchestration, not GH solving.
    let xml = r#"<Archive><chunks count="1"><chunk name="Definition"><chunks count="1"><chunk name="DefinitionObjects"><items count="1"><item name="ObjectCount" type_name="gh_int32" type_code="3">0</item></items><chunks count="0"></chunks></chunk></chunks></chunk></chunks></Archive>"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gh.log");
    let mut dialog = crate::state::app_state::GhOptDialogState::new(
        dir.path().join("test.ghx"),
        xml.into(),
        problem,
    );
    dialog.compute_use_exe = false;
    dialog.compute_url = format!("http://{address}");
    dialog.journal_path = path.to_string_lossy().into_owned();
    dialog.sampler = crate::state::app_state::GhSamplerChoice::Random;
    dialog.n_trials = 1;
    assert!(!dialog.load_if_exists);
    let mut reuse_dialog = dialog.clone();
    let mut app = artifact_test_app(None);
    app.start_ghx_run(dialog);
    assert!(app.app_state.gh_opt_dialog.is_none());
    assert!(std::fs::read_to_string(path.with_extension("compute.ghx"))
        .unwrap()
        .contains("RH_IN:x"));
    poll_until(&mut app, |a| {
        !a.is_loading
            && !a.reload_when_idle
            && a.app_state
                .gh_opt_run
                .as_ref()
                .is_some_and(|r| r.finished.is_some())
    });
    let run = app.app_state.gh_opt_run.as_ref().unwrap();
    assert!(run.finished.as_ref().unwrap().is_ok());
    assert_eq!(run.progress.snapshot().done, 1);
    let study = app.app_state.current_study.as_ref().unwrap();
    assert_eq!(study.view.row_count(), 1);
    assert_eq!(study.view.numeric_column("f").unwrap(), &[7.5]);
    assert!(study.view.input().unwrap().stamp().input.generation >= 2);
    let prefix = std::fs::read(&path).unwrap();
    let ctx = egui::Context::default();
    for mismatch in [false, true] {
        let mut rejected = reuse_dialog.clone();
        rejected.load_if_exists = mismatch;
        rejected.maximize[0] = mismatch;
        let mut failed_app = artifact_test_app(None);
        failed_app.start_ghx_run(rejected);
        assert!(failed_app.app_state.gh_opt_run.is_none());
        let state = failed_app.app_state.gh_opt_dialog.as_mut().unwrap();
        assert!(state.error.as_ref().unwrap().contains(if mismatch {
            "directions"
        } else {
            "already exists"
        }));
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {
            crate::ui::widgets::ghx_opt_modal::show(&ctx, state);
        });
        assert_eq!(std::fs::read(&path).unwrap(), prefix);
    }
    enable_recording_reuse_checkbox(|ctx| {
        crate::ui::widgets::ghx_opt_modal::show(ctx, &mut reuse_dialog);
    });
    assert!(reuse_dialog.load_if_exists);
    // Keep the Compute output contract unchanged; the writer already tests ignored requested labels.
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {
        crate::ui::widgets::ghx_opt_modal::show(&ctx, &mut reuse_dialog);
    });
    app.start_ghx_run(reuse_dialog);
    assert!(app.app_state.gh_opt_dialog.is_none());
    poll_until(&mut app, |a| {
        !a.is_loading
            && !a.reload_when_idle
            && a.app_state
                .gh_opt_run
                .as_ref()
                .is_some_and(|r| r.finished.is_some())
    });
    server.join().unwrap();
    let run = app.app_state.gh_opt_run.as_ref().unwrap();
    assert_eq!(run.progress.snapshot().done, 1);
    assert!(run
        .finished
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .contains("1 trials succeeded"));
    let study = app.app_state.current_study.as_ref().unwrap();
    assert_eq!(study.view.numeric_column("f").unwrap(), &[7.5, 7.5]);
    assert!(std::fs::read(&path).unwrap().starts_with(&prefix));
}

#[test]
fn d5_pdp_1d_rerun_delivers_success_to_both_rebound_canvas_consumers() {
    check_pdp_rebound_consumers(false);
    check_pdp_rebound_consumers(true);
}

#[test]
fn d5_generic_failure_finishes_only_the_rebound_consumers_selected_slot() {
    use crate::state::analysis_job::{reject_settings, DeliverySlot};
    let mut app = artifact_test_app(None);
    app.app_state = crate::state::analysis_job::tests::state(true).0;
    for owner in [10, 20] {
        let mut widgets = WidgetStates {
            analysis_consumer: owner + 1,
            ..Default::default()
        };
        widgets.pdp_2d.computing = true;
        widgets.importance.computing = true;
        app.canvas_widgets.insert(owner, widgets);
        app.app_state.analysis_consumer.set(owner + 1);
        reject_settings(
            &app.app_state,
            app.sender(),
            DeliverySlot::Pdp2d,
            "same unavailable input".into(),
        );
    }
    let mut queued = (0..2)
        .map(|_| {
            app.rx
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap()
        })
        .collect::<Vec<_>>();
    queued.sort_by_key(|message| {
        let AppMessage::AnalysisCompleted { completion, .. } = message else {
            panic!()
        };
        completion.key().run
    });
    for (index, message) in queued.into_iter().enumerate() {
        app.sender().send(message).unwrap();
        app.poll_messages(&egui::Context::default());
        for widgets in app.canvas_widgets.values() {
            assert_eq!(widgets.pdp_2d.computing, index == 0);
            assert!(
                widgets.importance.computing,
                "Only the completed slot is cleared"
            );
        }
    }
    assert_eq!(app.load_error.as_deref(), Some("PDP 2D computation failed"));
}

fn check_pdp_rebound_consumers(two_dimensional: bool) {
    use crate::state::analysis_job::DeliverySlot;
    use crate::state::layout_state::ChartId;
    use crate::ui::widgets::{pdp_2d::Pdp2dComputeRequest, pdp_chart::PdpComputeRequest};
    use std::sync::Arc;
    use tdashboard_study::*;
    for latest_first in [false, true] {
        let mut app = artifact_test_app(None);
        app.app_state = crate::state::analysis_job::tests::state(true).0;
        let journal = app.app_state.journal_studies.as_mut().unwrap();
        let manager = journal.managers.get_mut(&0).unwrap();
        let input = manager.acquire().unwrap();
        let upserts = input
            .data()
            .trials()
            .iter()
            .map(|trial| {
                let mut trial = (**trial).clone();
                let parameter = trial.parameters["x"].clone();
                Arc::make_mut(&mut trial.parameters).insert("z".into(), parameter);
                Arc::new(trial)
            })
            .collect();
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Delta {
                    upserts,
                    removals: vec![],
                    metadata: MetadataEdit::Unchanged,
                },
            )
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        MessageHandler::handle(
            AppMessage::JournalActivate {
                key: input.stamp().input.study,
            },
            &mut app.app_state,
            &mut app.widget_states,
            &mut app.is_loading,
            &mut app.load_error,
        );
        let slot = if two_dimensional {
            DeliverySlot::Pdp2d
        } else {
            DeliverySlot::Pdp1d
        };
        let mut requests = vec![];
        for owner in [10, 20] {
            let mut widgets = WidgetStates {
                analysis_consumer: owner + 1,
                ..Default::default()
            };
            let chart = if two_dimensional {
                widgets.pdp_2d.pending_compute = Some(Pdp2dComputeRequest {
                    param1: "x".into(),
                    param2: "z".into(),
                    objective: "y".into(),
                    model_type: "ridge".into(),
                    n_grid: 5,
                    feasible_only: false,
                });
                ChartId::PdpChart2D
            } else {
                widgets.pdp_chart.pending_compute = Some(PdpComputeRequest {
                    param: "x".into(),
                    objective: "y".into(),
                    model_type: "ridge".into(),
                    n_grid: 5,
                    feasible_only: false,
                });
                ChartId::PdpChart
            };
            let tx = app.sender();
            crate::ui::poll_chart::poll_chart_work(&mut app.app_state, &mut widgets, &chart, &tx);
            requests.push(
                app.app_state
                    .journal_studies
                    .as_ref()
                    .unwrap()
                    .analysis_jobs
                    .borrow()[&(owner + 1, slot)]
                    .clone(),
            );
            app.canvas_widgets.insert(owner, widgets);
        }
        assert_ne!(
            requests[0], requests[1],
            "The existing manual request starts a new run"
        );
        let jobs = app
            .app_state
            .journal_studies
            .as_ref()
            .unwrap()
            .analysis_jobs
            .borrow();
        assert!(jobs.values().all(|key| key == &requests[1]));
        drop(jobs);
        let mut queued = (0..2)
            .map(|_| {
                app.rx
                    .recv_timeout(std::time::Duration::from_secs(30))
                    .unwrap()
            })
            .collect::<Vec<_>>();
        queued.sort_by_key(|message| {
            let AppMessage::AnalysisCompleted { completion, .. } = message else {
                panic!()
            };
            (completion.key() == &requests[1]) != latest_first
        });
        for (index, message) in queued.into_iter().enumerate() {
            app.sender().send(message).unwrap();
            app.poll_messages(&egui::Context::default());
            let finished = latest_first || index == 1;
            for owner in [10, 20] {
                let widgets = &app.canvas_widgets[&owner];
                if two_dimensional {
                    assert_eq!(
                        !widgets.pdp_2d.computing, finished,
                        "consumer {owner}, latest_first={latest_first}"
                    );
                    assert_eq!(widgets.pdp_2d.result.is_some(), finished);
                    if let Some(result) = &widgets.pdp_2d.result {
                        assert_eq!(result.x_values.len(), 5);
                        assert_eq!(result.z_values.len(), 5);
                    }
                } else {
                    assert_eq!(
                        !widgets.pdp_chart.computing, finished,
                        "consumer {owner}, latest_first={latest_first}"
                    );
                    assert_eq!(widgets.pdp_chart.result.is_some(), finished);
                    if let Some(result) = &widgets.pdp_chart.result {
                        assert_eq!(result.x_values.len(), 5);
                        assert_eq!(result.param_name, "x");
                    }
                    assert_eq!(widgets.pdp_chart.cache.is_empty(), !finished);
                }
            }
            assert!(app.load_error.is_none(), "{:?}", app.load_error);
        }
    }
}

#[test]
fn d5_trained_model_reuse_does_not_finish_another_instances_fit() {
    use crate::state::analysis_job::{spawn, DeliverySlot};
    use tdashboard_analysis::preparation::{FitObjectives, Preparation};
    use tdashboard_analysis::surrogate_opt::*;
    let mut app = artifact_test_app(None);
    app.app_state = crate::state::analysis_job::tests::state(true).0;
    for owner in [10, 20] {
        let mut widgets = WidgetStates {
            analysis_consumer: owner + 1,
            ..Default::default()
        };
        widgets.surrogate_opt.fitting = true;
        widgets.surrogate_opt.error_message = Some("independent diagnostic".into());
        app.canvas_widgets.insert(owner, widgets);
    }
    app.app_state.analysis_consumer.set(11);
    spawn(
        &app.app_state,
        app.sender(),
        DeliverySlot::Fit,
        "Ridge",
        Preparation::Fit {
            objectives: FitObjectives::Name("y".into()),
            constraints: false,
        },
        |prepared| {
            let data = prepared.data().fit();
            let request = SurrogateFitRequest {
                x_matrix: data.x.clone(),
                y: data.ys[0].clone(),
                param_names: data.names.clone(),
                objective_name: "y".into(),
                model: SurrogateModelKind::Ridge,
                auto_select: false,
                constraints: vec![],
                priority_rows: vec![],
                param_bounds: Some(data.bounds.clone()),
            };
            AppMessage::SurrogateFitDone(std::sync::Arc::new(
                fit_surrogate_with_validation(&request).unwrap(),
            ))
        },
    );
    poll_until(&mut app, |app| {
        !app.canvas_widgets[&10].surrogate_opt.fitting
    });
    let target = &app.canvas_widgets[&10].surrogate_opt;
    let other = &app.canvas_widgets[&20].surrogate_opt;
    assert!(other.fitting);
    assert_eq!(
        other.error_message.as_deref(),
        Some("independent diagnostic")
    );
    let trained = target.trained.as_ref().unwrap();
    assert!(std::sync::Arc::ptr_eq(
        trained,
        other.trained.as_ref().unwrap()
    ));
    assert!(std::sync::Arc::ptr_eq(
        trained,
        app.widget_states.surrogate_opt.trained.as_ref().unwrap()
    ));
    assert_eq!(
        trained.fit_key().unwrap().analysis.inputs,
        [app.app_state
            .current_study
            .as_ref()
            .unwrap()
            .view
            .input()
            .unwrap()
            .stamp()]
    );
}

#[test]
fn d5_mcdm_rank_table_and_gallery_share_the_retained_result() {
    use crate::state::layout_state::ChartId;
    use crate::state::results::McdmMethod;
    use crate::ui::widgets::artifact_gallery::ArtifactViewMode;
    use crate::ui::widgets::mcdm_chart::McdmComputeRequest;
    use crate::ui::widgets::trial_table::TrialTableMode;
    let mut app = artifact_test_app(None);
    app.app_state = crate::state::analysis_job::tests::d5_front_state();
    let key = crate::ui::widgets::mcdm_chart::McdmCacheKey::from_settings(
        McdmMethod::Topsis,
        crate::state::results::WeightMode::Manual,
        &[1.0],
        0.5,
    )
    .unwrap();
    for owner in 1..=3 {
        let mut widgets = WidgetStates {
            analysis_consumer: owner + 1,
            ..Default::default()
        };
        widgets.trial_table.mode = TrialTableMode::Mcdm;
        widgets.artifact_gallery.mode = ArtifactViewMode::Mcdm;
        let controls = match owner {
            1 => &mut widgets.mcdm_chart.controls,
            2 => &mut widgets.trial_table.mcdm.controls,
            _ => &mut widgets.artifact_gallery.mcdm,
        };
        controls.weights = vec![1.0];
        controls.pending_compute = Some(McdmComputeRequest {
            method: McdmMethod::Topsis,
            weights: vec![1.0],
            v: 0.5,
        });
        controls.computing = true;
        let tx = app.sender();
        if owner == 2 {
            crate::ui::poll_chart::poll_trial_table_work(&mut app.app_state, &mut widgets, &tx);
        } else {
            crate::ui::poll_chart::poll_chart_work(
                &mut app.app_state,
                &mut widgets,
                if owner == 1 {
                    &ChartId::McdmRankChart
                } else {
                    &ChartId::ArtifactGallery
                },
                &tx,
            );
        }
        app.canvas_widgets.insert(owner, widgets);
    }
    let jobs: Vec<_> = app
        .app_state
        .journal_studies
        .as_ref()
        .unwrap()
        .analysis_jobs
        .borrow()
        .values()
        .cloned()
        .collect();
    assert_eq!(jobs.len(), 3);
    assert!(jobs.iter().all(|key| key == &jobs[0]));
    poll_until(&mut app, |app| app.app_state.mcdm_cache.contains_key(&key));
    assert!(!app.canvas_widgets[&1].mcdm_chart.controls.computing);
    assert!(!app.canvas_widgets[&2].trial_table.mcdm.controls.computing);
    assert!(!app.canvas_widgets[&3].artifact_gallery.mcdm.computing);
    assert!(app
        .app_state
        .mcdm_cache
        .selected(&key)
        .unwrap()
        .handle()
        .same_result(app.app_state.mcdm_result.as_ref().unwrap().handle()));
    let ctx = egui::Context::default();
    for owner in 1..=3 {
        let widgets = app.canvas_widgets.get_mut(&owner).unwrap();
        let _ = ctx.run_ui(Default::default(), |ui| {
            if owner == 2 {
                widgets.trial_table.show(ui, &mut app.app_state);
            } else {
                crate::ui::render_chart::render_chart(
                    ui,
                    &mut app.app_state,
                    widgets,
                    if owner == 1 {
                        &ChartId::McdmRankChart
                    } else {
                        &ChartId::ArtifactGallery
                    },
                );
            }
        });
    }
}

#[test]
fn mcdm_changed_weights_refresh_accepted_scatter_coordinates_without_reordering() {
    use crate::state::layout_state::ChartId;
    use crate::state::results::{McdmMethod, WeightMode};
    use crate::ui::widgets::mcdm_chart::{McdmCacheKey, McdmComputeRequest};
    use std::{collections::BTreeMap, sync::Arc};
    use tdashboard_study::{LoadKind, Number, ObjectiveDescriptor, ObjectiveIndex, StudyBatch};

    let mut app = artifact_test_app(None);
    app.app_state = crate::state::analysis_job::tests::state(true).0;
    let journal = app.app_state.journal_studies.as_mut().unwrap();
    let manager = journal.managers.get_mut(&0).unwrap();
    let input = manager.acquire().unwrap();
    let mut metadata = (**input.data().metadata()).clone();
    metadata.objectives.push(ObjectiveDescriptor {
        index: ObjectiveIndex(1),
        name: Some("z".into()),
        direction: tdashboard_study::Direction::Minimize,
    });
    let objectives = [[1.0, 9.0], [2.0, 6.0], [5.0, 3.0], [9.0, 1.0]];
    let trials = input
        .data()
        .trials()
        .iter()
        .zip(objectives)
        .map(|(trial, values)| {
            let mut trial = (**trial).clone();
            trial.objectives = Arc::new(BTreeMap::from([
                (ObjectiveIndex(0), Number(values[0])),
                (ObjectiveIndex(1), Number(values[1])),
            ]));
            Arc::new(trial)
        })
        .collect();
    let ticket = manager.begin(LoadKind::Refresh).unwrap();
    let receipt = manager
        .apply(
            manager.header(ticket).unwrap(),
            StudyBatch::Replace { trials, metadata },
        )
        .unwrap();
    manager.finish(ticket, receipt.next_sequence).unwrap();
    let input = manager.acquire().unwrap();
    let projection = journal.analysis.borrow_mut().project(input.clone());
    app.app_state.journal_display = Some(input.stamp());
    app.app_state.current_study = Some(crate::state::app_state::StudyContext {
        meta: crate::io::journal::convert_projection_meta(projection.meta.clone()),
        view: crate::state::types::StudyView::from_projection(projection, input, None),
        pareto_indices: vec![],
    });

    let mut results = Vec::new();
    for weights in [vec![0.9, 0.1], vec![0.85, 0.15]] {
        let key =
            McdmCacheKey::from_settings(McdmMethod::Topsis, WeightMode::Manual, &weights, 0.5)
                .unwrap();
        let controls = &mut app.widget_states.scatter_chart.controls;
        controls.weights = weights.clone();
        controls.pending_compute = Some(McdmComputeRequest {
            method: McdmMethod::Topsis,
            weights,
            v: 0.5,
        });
        let tx = app.sender();
        crate::ui::poll_chart::poll_chart_work(
            &mut app.app_state,
            &mut app.widget_states,
            &ChartId::McdmScatterChart,
            &tx,
        );
        poll_until(&mut app, |app| app.app_state.mcdm_cache.contains_key(&key));
        assert!(!app.widget_states.scatter_chart.controls.computing);
        results.push(app.app_state.mcdm_cache.selected(&key).unwrap().clone());
    }
    assert_ne!(results[0].key(), results[1].key());
    assert_eq!(results[0].ranked_indices(), results[1].ranked_indices());
    assert_eq!(results[0].ranked_indices().len(), 4);
    assert_ne!(results[0].primary_scores(), results[1].primary_scores());
    let view = &app.app_state.current_study.as_ref().unwrap().view;
    let names = vec!["y".into(), "z".into()];
    let scores = [results[0].primary_scores(), results[1].primary_scores()];
    crate::ui::widgets::mcdm_scatter_chart::tests::assert_accepted_score_refresh(
        [&results[0], &results[1]],
        view,
        &names,
        "TOPSIS_Score",
        scores,
    );
    crate::ui::widgets::mcdm_scatter_chart_3d::tests::assert_accepted_score_refresh(
        [&results[0], &results[1]],
        view,
        &names,
        "TOPSIS_Score",
        scores,
    );
}

#[test]
fn d5_canvas_consumers_share_equal_conditions_and_keep_unequal_jobs_independent() {
    use crate::state::analysis_job::DeliverySlot;
    use crate::state::layout_state::ChartId;
    use crate::state::messages::ClusterChartSource;
    use crate::ui::widgets::cluster_scatter::{
        ClusterComputeRequest, ClusterSpace, KMeansInitStrategy, KSelectionMode,
    };
    for equal in [true, false] {
        let mut app = artifact_test_app(None);
        app.app_state = crate::state::analysis_job::tests::d5_front_state();
        let mut cache_keys = vec![];
        for (owner, k) in [(10, 2), (20, if equal { 2 } else { 3 })] {
            let mut widgets = WidgetStates {
                analysis_consumer: owner + 1,
                ..Default::default()
            };
            let cluster = &mut widgets.cluster_scatter;
            cluster.k = k;
            cluster.target_space = ClusterSpace::Variable;
            cluster.k_mode = KSelectionMode::Manual;
            cluster.init_strategy = KMeansInitStrategy::Deterministic;
            cluster.computing = true;
            cluster.pending_compute = Some(ClusterComputeRequest {
                k,
                target_space: cluster.target_space,
                k_mode: cluster.k_mode,
                init_strategy: cluster.init_strategy,
                elbow_max_k: cluster.elbow_max_k,
            });
            cache_keys.push(cluster.cache_key());
            let tx = app.sender();
            crate::ui::poll_chart::poll_chart_work(
                &mut app.app_state,
                &mut widgets,
                &ChartId::ClusterScatter,
                &tx,
            );
            app.canvas_widgets.insert(owner, widgets);
            app.layout
                .canvas
                .items
                .push(crate::state::layout_state::CanvasItem {
                    id: owner,
                    content: crate::state::layout_state::PanelItem::Chart(ChartId::ClusterScatter),
                    x: 0.0,
                    y: 0.0,
                    w: 300.0,
                    h: 300.0,
                });
        }
        let journal = app.app_state.journal_studies.as_ref().unwrap();
        let jobs = journal.analysis_jobs.borrow();
        let a = jobs[&(11, DeliverySlot::Cluster(ClusterChartSource::Scatter2D))].clone();
        let b = jobs[&(21, DeliverySlot::Cluster(ClusterChartSource::Scatter2D))].clone();
        assert_eq!(a == b, equal);
        drop(jobs);
        if !equal {
            let first = app
                .rx
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap();
            let second = app
                .rx
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap();
            let AppMessage::AnalysisCompleted { completion, .. } = &first else {
                panic!()
            };
            let first_owner = if completion.key() == &a { 10 } else { 20 };
            app.sender().send(first).unwrap();
            app.poll_messages(&egui::Context::default());
            assert!(!app.canvas_widgets[&first_owner].cluster_scatter.computing);
            assert!(
                app.canvas_widgets[&(30 - first_owner)]
                    .cluster_scatter
                    .computing
            );
            app.sender().send(second).unwrap();
        }
        poll_until(&mut app, |app| {
            app.app_state.cluster_cache.contains_key(&cache_keys[0])
                && app.app_state.cluster_cache.contains_key(&cache_keys[1])
        });
        for owner in [10, 20] {
            assert!(!app.canvas_widgets[&owner].cluster_scatter.computing);
        }
        let first = app
            .app_state
            .cluster_cache
            .selected(&cache_keys[0])
            .unwrap();
        let second = app
            .app_state
            .cluster_cache
            .selected(&cache_keys[1])
            .unwrap();
        assert_eq!(first.handle().same_result(second.handle()), equal);
        assert_eq!(first.labels[2], -1);
        assert_eq!(first.rows()[2].stored_id, 92);
        // Exercise the actual plot and table renderers against the shared payloads.
        let ctx = egui::Context::default();
        for owner in [10, 20] {
            let widgets = app.canvas_widgets.get_mut(&owner).unwrap();
            let _ = ctx.run_ui(Default::default(), |ui| {
                crate::ui::render_chart::render_chart(
                    ui,
                    &mut app.app_state,
                    widgets,
                    &ChartId::ClusterScatter,
                );
            });
        }
        app.retain_analysis_interests();
        assert!(app.app_state.cluster_cache.contains_key(&cache_keys[0]));
        app.canvas_widgets.remove(&10);
        app.canvas_widgets.remove(&20);
        app.retain_analysis_interests();
        assert!(!app.app_state.cluster_cache.contains_key(&cache_keys[0]));
        assert!(app
            .app_state
            .journal_studies
            .as_ref()
            .unwrap()
            .analysis
            .borrow()
            .lookup(&a.analysis)
            .is_none());
    }
}

#[test]
fn d5_global_deliveries_do_not_finish_unrelated_canvas_requests() {
    use crate::state::analysis_job::{spawn, DeliverySlot};
    use crate::ui::widgets::importance_chart::ImportanceMetric;
    use tdashboard_analysis::{preparation::Preparation, sensitivity::heatmap::HeatmapMethod};
    for panic in [false, true] {
        let mut app = artifact_test_app(None);
        app.app_state = crate::state::analysis_job::tests::state(true).0;
        let mut item = WidgetStates::default();
        item.importance.computing = true;
        app.canvas_widgets.insert(42, item);
        app.widget_states.importance.computing = true;
        spawn(
            &app.app_state,
            app.sender(),
            DeliverySlot::Importance,
            "current request",
            Preparation::Sensitivity {
                method: HeatmapMethod::Spearman,
                objective: Some(0),
                feasible_only: false,
                samples: 32,
            },
            move |prepared| {
                if panic {
                    panic!("current keyed worker panic");
                }
                AppMessage::SensitivityDone {
                    key: (ImportanceMetric::Spearman.cache_id(), 0, false),
                    result: crate::state::results::SensitivityResult {
                        param_names: prepared.data().sensitivity().df.param_col_names().to_vec(),
                        unsupported_categorical: vec![],
                        spearman: vec![vec![1.0]],
                        ridge: vec![],
                        rf_anova: None,
                        mdi: None,
                        shap: None,
                        permutation: None,
                        ard: None,
                    },
                }
            },
        );
        let ctx = egui::Context::default();
        let start = std::time::Instant::now();
        loop {
            app.poll_messages(&ctx);
            let done = if panic {
                app.load_error.is_some()
            } else {
                app.app_state.importance_cache.contains_key(&(
                    ImportanceMetric::Spearman.cache_id(),
                    0,
                    false,
                ))
            };
            if done {
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(30));
            std::thread::yield_now();
        }
        assert!(!app.widget_states.importance.computing);
        assert!(
            app.canvas_widgets[&42].importance.computing,
            "A global request must not finish an unrelated canvas request"
        );
        if panic {
            assert!(app
                .load_error
                .as_ref()
                .unwrap()
                .contains("current keyed worker panic"));
        }
    }
}

#[test]
fn journal_late_attributes_correct_the_live_prefix_before_completion_and_reload() {
    use serde_json::json;
    use tdashboard_study::FeasibilityState;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("late-attrs.log");
    let mut bytes = concat!(
        "{\"op_code\":0,\"study_name\":\"target\",\"directions\":[1]}\n",
        "{\"op_code\":0,\"study_name\":\"other\",\"directions\":[1]}\n",
    )
    .to_string();
    for i in 0..1000 {
        bytes.push_str(
            &(json!({"op_code":4,"study_id":0,"distributions":{},"state":1,
            "params":{"z":i},"values":[i],"user_attrs":{"payload":{"old":true}},
            "system_attrs":{"constraints":[-1]}})
            .to_string()
                + "\n"),
        );
    }
    bytes.push_str(concat!(
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":2}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":3}\n",
        "{\"op_code\":8,\"trial_id\":0,\"user_attr\":{\"payload\":null,\"empty\":[]}}\n",
        "{\"op_code\":9,\"trial_id\":0,\"system_attr\":{\"constraints\":[1]}}\n",
        "{\"op_code\":9,\"trial_id\":1000,\"system_attr\":{\"constraints\":[-1,null,0]}}\n",
        "{\"op_code\":9,\"trial_id\":1001,\"system_attr\":{\"constraints\":[-1,0]}}\n",
    ));
    std::fs::write(&path, bytes).unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(path);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    let meta = app
        .app_state
        .all_studies
        .iter()
        .find(|m| m.study_id == 0)
        .unwrap()
        .clone();
    app.apply_toolbar_actions(vec![ToolbarAction::SelectStudy(meta)]);
    let first = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    assert!(matches!(&first, AppMessage::JournalBatch { .. }));
    // Deliver one real message directly, so the next worker delivery cannot
    // conceal the early visible prefix behind the message pump's drain loop.
    crate::state::message_handler::MessageHandler::handle(
        first,
        &mut app.app_state,
        &mut app.widget_states,
        &mut app.is_loading,
        &mut app.load_error,
    );
    let old = app.app_state.journal_studies.as_ref().unwrap().managers[&0]
        .acquire()
        .unwrap();
    let old_df = app
        .app_state
        .current_study
        .as_ref()
        .unwrap()
        .view
        .df
        .clone();
    assert!(app.is_loading);
    assert_eq!(old_df.row_count(), 1000);
    assert_eq!(old_df.feasibility().state(0), FeasibilityState::Feasible);
    assert_eq!(
        old_df.user_attr_value("payload", 0),
        Some(&json!({"old":true}))
    );
    let correction = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let AppMessage::JournalBatch { batch, .. } = &correction else {
        panic!("expected final dirty batch")
    };
    assert!(
        matches!(batch, tdashboard_study::StudyBatch::Delta { upserts, .. } if upserts.len() == 3)
    );
    crate::state::message_handler::MessageHandler::handle(
        correction,
        &mut app.app_state,
        &mut app.widget_states,
        &mut app.is_loading,
        &mut app.load_error,
    );
    let corrected = &app.app_state.current_study.as_ref().unwrap().view.df;
    assert!(app.is_loading, "dirty changes must publish before Complete");
    assert_eq!(corrected.row_count(), 1000);
    assert_eq!(corrected.get_trial_id(0), Some(0));
    assert_eq!(corrected.get_trial_id(999), Some(999));
    assert_eq!(corrected.param_col_names(), ["z"]);
    assert_eq!(
        corrected.user_attr_value("payload", 0),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(corrected.user_attr_value("empty", 0), Some(&json!([])));
    assert_eq!(corrected.constraint_col_names().len(), 3);
    assert_eq!(
        corrected.feasibility().state(0),
        FeasibilityState::Infeasible
    );
    assert_eq!(
        corrected.feasibility().state(1),
        FeasibilityState::Unverified
    );
    assert_eq!(old.data().constraint_schema().width, 1);
    assert_eq!(old_df.feasibility().state(0), FeasibilityState::Feasible);
    poll_until(&mut app, |a| !a.is_loading);
    assert!(app.load_error.is_none());
    app.reload_current();
    poll_until(&mut app, |a| !a.is_loading && a.pending_reload.is_none());
    let loaded = &app.app_state.current_study.as_ref().unwrap().view.df;
    assert_eq!(loaded.row_count(), 1000);
    assert_eq!(
        loaded.user_attr_value("payload", 0),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(loaded.feasibility().state(0), FeasibilityState::Infeasible);
}

#[test]
fn d2_journal_live_open_select_reload_replace_and_headless_projection() {
    use tdashboard_study::*;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("slice.log");
    let bytes = concat!(
        "{\"op_code\":0,\"study_name\":\"a\",\"directions\":[1]}\n",
        "{\"op_code\":0,\"study_name\":\"b\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":1,\"state\":1,\"distributions\":{},\"params\":{},\"values\":[9]}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":8,\"trial_id\":1,\"user_attr\":{\"json\":[1,null]}}\n",
        "{\"op_code\":6,\"trial_id\":2,\"state\":1,\"values\":[4]}\n",
        "{\"op_code\":6,\"trial_id\":1,\"state\":1,\"values\":[2]}\n",
        "{\"op_code\":4,\"study_id\":0,\"state\":2,\"distributions\":{},\"params\":{\"hidden\":7},\"values\":[8],\"system_attrs\":{\"constraints\":[-1,null]}}\n",
    );
    std::fs::write(&path, bytes).unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(path.clone());
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    select_artifact_study(&mut app, 0);
    let journal = app.app_state.journal_studies.as_ref().unwrap();
    let old = journal.managers[&0].acquire().unwrap();
    assert_eq!(old.data().trials().len(), 3);
    assert_eq!(old.data().trials()[2].state, TrialState::Pruned);
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().view.trial_ids,
        vec![2, 1]
    );
    assert_eq!(
        app.app_state.best_trial_history,
        tdashboard_analysis::projection::project(&old).history
    );
    select_artifact_study(&mut app, 1);
    select_artifact_study(&mut app, 0);
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().trial_count(),
        2
    );
    assert_eq!(app.app_state.journal_display, Some(old.stamp()));
    let ctx = egui::Context::default();
    let mut table = crate::ui::widgets::trial_table::TrialTable::default();
    let mut history = crate::ui::widgets::optimization_history::OptimizationHistoryChart::default();
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            ..Default::default()
        },
        |ui| {
            ui.scope(|ui| {
                ui.set_max_height(300.0);
                table.show(ui, &mut app.app_state);
            });
            let study = app.app_state.current_study.as_ref().unwrap();
            history.show_with_comparisons(
                ui,
                &study.view,
                &study.meta.objective_names,
                &study.meta.directions,
                &study.meta.param_names,
                &study.meta.name,
                &[],
                &app.app_state.artifact_map,
            );
        },
    );
    assert!(!ctx
        .tessellate(output.shapes, output.pixels_per_point)
        .is_empty());
    let independent = app.app_state.journal_studies.as_ref().unwrap().managers[&1]
        .acquire()
        .unwrap();
    app.reload_current();
    poll_until(&mut app, |a| a.pending_reload.is_none() && !a.is_loading);
    let journal = app.app_state.journal_studies.as_ref().unwrap();
    let reloaded = journal.managers[&0].acquire().unwrap();
    assert_eq!(old.stamp().input.study, reloaded.stamp().input.study);
    assert_ne!(
        old.stamp().input.generation,
        reloaded.stamp().input.generation
    );
    assert!(!reloaded.matches(old.stamp()));
    assert_eq!(
        journal.managers[&1].acquire().unwrap().stamp(),
        independent.stamp()
    );
    select_artifact_study(&mut app, 1);
    let reread = app.app_state.journal_studies.as_ref().unwrap().managers[&1]
        .acquire()
        .unwrap();
    assert_ne!(
        reread.stamp().input.generation,
        independent.stamp().input.generation
    );
    app.open_path(path);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    select_artifact_study(&mut app, 0);
    let replaced = app.app_state.journal_studies.as_ref().unwrap().managers[&0]
        .acquire()
        .unwrap();
    assert_ne!(
        replaced.stamp().input.study.source,
        old.stamp().input.study.source
    );
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().trial_count(),
        2
    );
}

#[test]
fn d2_stale_same_path_scan_and_old_loader_cannot_populate_replaced_source() {
    use tdashboard_study::*;
    let mut app = artifact_test_app(None);
    let mut old = crate::state::journal_studies::JournalStudies::new(1).unwrap();
    let key = StudyKey {
        source: old.source,
        stored_id: 0,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let header = manager.header(ticket).unwrap();
    old.managers.insert(0, manager);
    app.app_state.journal_studies =
        Some(crate::state::journal_studies::JournalStudies::new(2).unwrap());
    app.app_state.journal_path = Some("same.log".into());
    app.is_loading = true;
    app.tx
        .send(AppMessage::JournalScanned {
            source: old.source,
            scan: 1,
            path: "same.log".into(),
            result: Ok((vec![], vec![])),
        })
        .unwrap();
    let (reply, ack) = mpsc::sync_channel(1);
    app.tx
        .send(AppMessage::JournalBatch {
            header,
            batch: StudyBatch::Replace {
                trials: vec![],
                metadata: Default::default(),
            },
            reply,
        })
        .unwrap();
    app.tx
        .send(AppMessage::JournalFinished {
            header,
            result: Err("obsolete error".into()),
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert!(ack.recv().unwrap().is_err());
    app.poll_messages(&egui::Context::default());
    assert!(app.app_state.current_study.is_none());
    assert!(app
        .app_state
        .journal_studies
        .as_ref()
        .unwrap()
        .bytes
        .is_none());
    assert!(app.is_loading);
    assert!(app.load_error.is_none());
}

#[test]
fn d2_gui_reload_failure_preserves_the_correct_generation_before_and_after_acceptance() {
    use tdashboard_study::*;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("reload.log");
    std::fs::write(&path, "{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}\n{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[3]}\n").unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(path);
    poll_until(&mut app, |a| {
        !a.is_loading && a.app_state.current_study.is_some()
    });
    let old = app.app_state.journal_studies.as_ref().unwrap().managers[&0]
        .acquire()
        .unwrap();
    let manager = app
        .app_state
        .journal_studies
        .as_mut()
        .unwrap()
        .managers
        .get_mut(&0)
        .unwrap();
    let ticket = manager.begin(LoadKind::Reload).unwrap();
    let header = manager.header(ticket).unwrap();
    app.tx
        .send(AppMessage::JournalFinished {
            header,
            result: Err("before acceptance".into()),
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert_eq!(app.app_state.journal_display, Some(old.stamp()));
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().trial_count(),
        1
    );
    let manager = app
        .app_state
        .journal_studies
        .as_mut()
        .unwrap()
        .managers
        .get_mut(&0)
        .unwrap();
    let ticket = manager.begin(LoadKind::Reload).unwrap();
    let header = manager.header(ticket).unwrap();
    let (reply, ack) = mpsc::sync_channel(1);
    app.tx
        .send(AppMessage::JournalBatch {
            header,
            batch: StudyBatch::Replace {
                trials: vec![],
                metadata: Default::default(),
            },
            reply,
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    let receipt = ack.recv().unwrap().unwrap();
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().trial_count(),
        0
    );
    assert_ne!(
        app.app_state.journal_display.unwrap().input.generation,
        old.stamp().input.generation
    );
    app.tx
        .send(AppMessage::JournalFinished {
            header: receipt.next_header(ticket),
            result: Err("after acceptance".into()),
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().trial_count(),
        0
    );
    let failed = app.app_state.journal_studies.as_ref().unwrap().managers[&0]
        .acquire()
        .unwrap();
    assert_eq!(failed.load_state().phase, LoadPhase::Failed);
    assert_eq!(
        app.app_state.journal_display.unwrap().input,
        receipt.stamp.input
    );
    assert_eq!(old.data().trials().len(), 1);
    assert_eq!(app.load_error.as_deref(), Some("after acceptance"));
    // Failed comparison acquisition retains its domain prefix, but must not
    // append a partial comparison view (the existing caller error policy).
    let manager = app
        .app_state
        .journal_studies
        .as_mut()
        .unwrap()
        .managers
        .get_mut(&0)
        .unwrap();
    let ticket = manager.begin(LoadKind::Reload).unwrap();
    let receipt = manager
        .apply(
            manager.header(ticket).unwrap(),
            StudyBatch::Replace {
                trials: vec![],
                metadata: Default::default(),
            },
        )
        .unwrap();
    app.tx
        .send(AppMessage::JournalFinished {
            header: receipt.next_header(ticket),
            result: Err("comparison failed".into()),
        })
        .unwrap();
    app.poll_messages(&egui::Context::default());
    assert!(app.app_state.comparison_studies.is_empty());
}

fn make_channel() -> (mpsc::SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
    mpsc::sync_channel(32)
}

fn d2_comparison_fixture() -> String {
    use serde_json::json;
    let mut lines = vec![
        json!({"op_code":0,"study_name":"base","directions":[1]}),
        json!({"op_code":0,"study_name":"large","directions":[1]}),
        json!({"op_code":0,"study_name":"small","directions":[1]}),
        json!({"op_code":4,"study_id":0,"distributions":{},"state":1,"values":[0]}),
    ];
    for (study, ids) in [(1, 1..1002), (2, 1002..1004)] {
        for _ in ids.clone() {
            lines.push(json!({"op_code":4,"study_id":study}));
        }
        for id in ids.rev() {
            lines.push(json!({"op_code":6,"trial_id":id,"state":1,"values":[id]}));
        }
    }
    lines
        .into_iter()
        .map(|line| line.to_string() + "\n")
        .collect()
}

#[test]
fn d2_replaced_sqlite_callbacks_cannot_overwrite_a_completed_journal_load() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("old.db");
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection.execute_batch("
        CREATE TABLE studies (study_id INTEGER PRIMARY KEY, study_name TEXT);
        CREATE TABLE study_directions (study_direction_id INTEGER PRIMARY KEY, study_id INTEGER, objective INTEGER, direction TEXT);
        CREATE TABLE trials (trial_id INTEGER PRIMARY KEY, number INTEGER, study_id INTEGER, state TEXT, datetime_start TEXT, datetime_complete TEXT);
        CREATE TABLE trial_values (trial_value_id INTEGER PRIMARY KEY, trial_id INTEGER, objective INTEGER, value REAL, value_type TEXT);
        CREATE TABLE trial_params (param_id INTEGER PRIMARY KEY, trial_id INTEGER, param_name TEXT, param_value REAL, distribution_json TEXT);
        CREATE TABLE trial_user_attributes (id INTEGER PRIMARY KEY, trial_id INTEGER, key TEXT, value_json TEXT);
        CREATE TABLE trial_system_attributes (id INTEGER PRIMARY KEY, trial_id INTEGER, key TEXT, value_json TEXT);
        CREATE TABLE study_system_attributes (id INTEGER PRIMARY KEY, study_id INTEGER, key TEXT, value_json TEXT);
        INSERT INTO studies VALUES (0, 'old'), (1, 'other');
        INSERT INTO study_directions (study_id, objective, direction) VALUES (0, 0, 'MINIMIZE'), (1, 0, 'MINIMIZE');
        INSERT INTO trials (trial_id, number, study_id, state) VALUES (0, 0, 0, 'COMPLETE'), (9, 1, 0, 'FAIL');
        INSERT INTO trial_values (trial_id, objective, value, value_type) VALUES (0, 0, 99, 'FINITE');
    ").unwrap();
    drop(connection);
    let journal_path = dir.path().join("new.log");
    std::fs::write(&journal_path, "{\"op_code\":0,\"study_name\":\"new\",\"directions\":[1]}\n{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[3]}\n").unwrap();
    for callback in ["batch", "finish", "comparison", "error"] {
        let mut app = artifact_test_app(None);
        app.open_path(db.clone());
        poll_until(&mut app, |a| a.app_state.all_studies.len() == 2);
        let mut meta = app
            .app_state
            .all_studies
            .iter()
            .find(|s| s.study_id == 0)
            .unwrap()
            .clone();
        if callback == "error" {
            meta.study_id = 999;
        }
        let old_source = app.app_state.journal_studies.as_ref().unwrap().source;
        let delivery = app.tx.clone();
        let (intercept, results) = make_channel();
        app.tx = intercept;
        app.select_study(meta, callback == "comparison", callback == "comparison");
        let mut held = results
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        if callback == "finish" {
            delivery.send(held).unwrap();
            app.poll_messages(&egui::Context::default());
            held = results
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
        }
        let header = match &held {
            AppMessage::JournalBatch { header, .. }
                if callback != "error" && callback != "finish" =>
            {
                header
            }
            AppMessage::JournalFinished { header, result } => {
                assert_eq!(result.is_err(), callback == "error");
                header
            }
            _ => panic!("storage callback lacks an owned update ticket"),
        };
        assert_eq!(header.ticket.study().source, old_source);
        app.tx = delivery.clone();
        app.open_path(journal_path.clone());
        poll_until(&mut app, |a| {
            !a.is_loading && a.app_state.journal_display.is_some()
        });
        let stamp = app.app_state.journal_display;
        let df = app
            .app_state
            .current_study
            .as_ref()
            .unwrap()
            .view
            .df
            .clone();
        assert_eq!(df.get_numeric_column("obj0"), Some([3.0].as_slice()));
        delivery.send(held).unwrap();
        app.poll_messages(&egui::Context::default());
        assert_eq!(app.app_state.journal_display, stamp);
        assert_eq!(
            app.app_state.current_study.as_ref().unwrap().meta.name,
            "new"
        );
        assert!(std::sync::Arc::ptr_eq(
            &app.app_state.current_study.as_ref().unwrap().view.df,
            &df
        ));
        assert!(std::sync::Arc::ptr_eq(
            &app.app_state
                .journal_studies
                .as_ref()
                .unwrap()
                .analysis
                .borrow_mut()
                .project(
                    app.app_state
                        .current_study
                        .as_ref()
                        .unwrap()
                        .view
                        .input()
                        .unwrap()
                )
                .dataframe,
            &df
        ));
        assert_eq!(
            app.app_state
                .current_study
                .as_ref()
                .unwrap()
                .view
                .extras()
                .unwrap()
                .trials
                .len(),
            1
        );
        assert!(app.app_state.comparison_studies.is_empty());
        assert!(!app.is_loading);
        assert!(app.load_error.is_none());
    }
}

#[test]
fn d3_csv_and_sqlite_use_owned_managers_for_selection_and_reload() {
    let directory = tempfile::tempdir().unwrap();
    let csv = directory.path().join("input.csv");
    std::fs::write(&csv, "in:x,out:f,score\n1,10,inf\n2,20,-inf\n").unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(csv);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 1);
    let mut meta = app.app_state.all_studies[0].clone();
    meta.directions = vec![crate::state::app_state::Direction::Maximize];
    app.app_state.csv_import_settings = None;
    app.select_study(meta, false, false);
    poll_until(&mut app, |a| {
        !a.is_loading && a.app_state.journal_display.is_some()
    });
    let old_stamp = app.app_state.journal_display.unwrap();
    let csv_input = app.app_state.journal_studies.as_ref().unwrap().managers[&0]
        .acquire()
        .unwrap();
    assert_eq!(
        csv_input.data().metadata().objectives[0].direction,
        tdashboard_study::Direction::Maximize
    );
    assert!(
        matches!(csv_input.data().trials()[0].user_attributes["score"].as_ref(),tdashboard_study::AttributeValue::Number(tdashboard_study::Number(n)) if *n == f64::INFINITY)
    );
    let meta = app.app_state.all_studies[0].clone();
    app.select_study(meta, false, false);
    assert_eq!(app.app_state.journal_display.unwrap(), old_stamp);
    assert_eq!(
        app.app_state
            .current_study
            .as_ref()
            .unwrap()
            .view
            .df
            .get_numeric_column("score")
            .unwrap(),
        &[f64::INFINITY, f64::NEG_INFINITY]
    );

    let path = directory.path().join("study.db");
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch(r#"
        CREATE TABLE studies(study_id INTEGER,study_name TEXT);
        CREATE TABLE study_directions(study_id INTEGER,objective INTEGER,direction TEXT);
        CREATE TABLE study_system_attributes(study_id INTEGER,key TEXT,value_json TEXT);
        CREATE TABLE trials(trial_id INTEGER,number INTEGER,study_id INTEGER,state TEXT,datetime_start TEXT,datetime_complete TEXT);
        CREATE TABLE trial_values(trial_id INTEGER,objective INTEGER,value REAL,value_type TEXT);
        CREATE TABLE trial_params(trial_id INTEGER,param_name TEXT,param_value REAL,distribution_json TEXT);
        CREATE TABLE trial_user_attributes(trial_id INTEGER,key TEXT,value_json TEXT);
        CREATE TABLE trial_system_attributes(trial_id INTEGER,key TEXT,value_json TEXT);
        INSERT INTO studies VALUES(0,'sqlite'); INSERT INTO study_directions VALUES(0,0,'MINIMIZE');
        INSERT INTO trials VALUES(10,0,0,'COMPLETE',NULL,NULL),(20,4,0,'RUNNING',NULL,NULL);
        INSERT INTO trial_values VALUES(10,0,10,'FINITE'),(20,0,20,'FINITE');
        INSERT INTO trial_user_attributes VALUES(10,'tag','1.5'),(20,'running','{"v":true}');
    "#).unwrap();
    app.open_path(path);
    poll_until(&mut app, |a| {
        !a.is_loading
            && a.app_state
                .current_study
                .as_ref()
                .is_some_and(|s| s.meta.name == "sqlite")
    });
    let before = app.app_state.journal_display.unwrap();
    assert_ne!(before.input.study.source, old_stamp.input.study.source);
    let input = app.app_state.journal_studies.as_ref().unwrap().managers[&0]
        .acquire()
        .unwrap();
    assert_eq!(input.data().trials().len(), 2);
    assert_eq!(
        input.data().trials()[1].objectives[&tdashboard_study::ObjectiveIndex(0)].0,
        20.0
    );
    assert!(input.data().trials()[1]
        .user_attributes
        .contains_key("running"));
    writer
        .execute(
            "UPDATE trial_user_attributes SET value_json='Infinity' WHERE trial_id=10",
            [],
        )
        .unwrap();
    app.reload_current();
    poll_until(&mut app, |a| !a.is_loading && a.pending_reload.is_none());
    let after = app.app_state.journal_display.unwrap();
    assert_eq!(before.input.study, after.input.study);
    assert!(after.input.generation > before.input.generation);
    assert_eq!(
        app.app_state
            .current_study
            .as_ref()
            .unwrap()
            .view
            .df
            .get_numeric_column("tag")
            .unwrap(),
        &[f64::INFINITY]
    );
    assert_eq!(
        input.data().trials()[0].user_attributes["tag"].as_ref(),
        &serde_json::json!(1.5)
    );
    assert_eq!(
        csv_input.data().trials()[1].user_attributes["score"].as_ref(),
        &tdashboard_study::AttributeValue::from(f64::NEG_INFINITY)
    );
}

#[test]
#[ignore = "requires the disposable genuine Optuna server fixtures"]
fn d3_live_servers_use_the_owned_gui_handoff_and_reload() {
    for family in ["POSTGRES", "MYSQL"] {
        let url = std::env::var(format!("TUNNY_SNAPSHOT_{family}_URL"))
            .expect("disposable server required");
        let mut app = artifact_test_app(None);
        app.open_path(url.into());
        poll_until(&mut app, |a| a.app_state.all_studies.len() == 32);
        let meta = app
            .app_state
            .all_studies
            .iter()
            .find(|s| s.name == "tunny-server-snapshot-20")
            .unwrap()
            .clone();
        let id = meta.study_id;
        app.select_study(meta, false, false);
        poll_until(&mut app, |a| {
            !a.is_loading && a.app_state.current_study.is_some()
        });
        let before = app.app_state.journal_display.unwrap();
        let input = app.app_state.journal_studies.as_ref().unwrap().managers[&id]
            .acquire()
            .unwrap();
        assert_eq!(input.data().trials().len(), 2);
        assert!(input.data().trials()[1].parameters.contains_key("x"));
        assert!(input.data().trials()[1].user_attributes.contains_key("tag"));
        let meta = app.app_state.current_study.as_ref().unwrap().meta.clone();
        app.select_study(meta, false, false);
        assert_eq!(
            app.app_state
                .current_study
                .as_ref()
                .unwrap()
                .view
                .row_count(),
            1
        );
        assert_eq!(app.app_state.journal_display.unwrap(), before);
        app.reload_current();
        poll_until(&mut app, |a| !a.is_loading && a.pending_reload.is_none());
        assert!(app.app_state.journal_display.unwrap().input.generation > before.input.generation);
        assert_eq!(
            app.app_state
                .current_study
                .as_ref()
                .unwrap()
                .view
                .row_count(),
            1
        );
    }
}

#[test]
fn d2_streaming_gui_preserves_column_discovery_order_across_the_batch_boundary() {
    use serde_json::json;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("columns.log");
    let mut bytes = json!({"op_code":0,"study_name":"columns","directions":[1]}).to_string() + "\n";
    for i in 0..1001 {
        let name = if i < 1000 { "z" } else { "a" };
        bytes.push_str(&(json!({"op_code":4,"study_id":0,"distributions":{},"state":1,"params":{name:i},"values":[i],"user_attrs":{format!("{name}_num"):i,format!("{name}_text"):name}}).to_string() + "\n"));
    }
    std::fs::write(&path, &bytes).unwrap();
    let expected = crate::test_support::streamed(bytes.as_bytes(), 0, 1000);
    let legacy = expected.dataframe;
    let mut app = artifact_test_app(None);
    app.open_path(path);
    poll_until(&mut app, |a| {
        !a.is_loading && a.app_state.current_study.is_some()
    });
    let view = &app.app_state.current_study.as_ref().unwrap().view;
    assert_eq!(view.param_names(), ["z", "a"]);
    assert_eq!(view.df.column_names(), legacy.column_names());
    assert_eq!(
        view.df
            .user_attr_numeric_columns()
            .map(|(n, _)| n)
            .collect::<Vec<_>>(),
        vec!["z_num", "a_num"]
    );
    assert_eq!(
        view.df
            .user_attr_string_columns()
            .map(|(n, _)| n)
            .collect::<Vec<_>>(),
        vec!["z_text", "a_text"]
    );
    for name in legacy.column_names() {
        assert_eq!(
            view.df.get_string_column(&name),
            legacy.get_string_column(&name)
        );
        let equal_numeric = view
            .df
            .get_numeric_column(&name)
            .zip(legacy.get_numeric_column(&name))
            .is_none_or(|(a, b)| {
                a.iter()
                    .zip(b)
                    .all(|(a, b)| a == b || a.is_nan() && b.is_nan())
            });
        assert!(equal_numeric, "column {name}");
    }
}

#[test]
fn d2_comparison_reload_preserves_request_order_and_full_read_rows_on_reverse_completion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("comparison.log");
    let bytes = d2_comparison_fixture();
    std::fs::write(&path, &bytes).unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(path);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 3);
    select_artifact_study(&mut app, 0);
    let requested: Vec<_> = [1, 2]
        .into_iter()
        .map(|id| {
            app.app_state
                .all_studies
                .iter()
                .find(|s| s.study_id == id)
                .unwrap()
                .clone()
        })
        .collect();
    app.apply_toolbar_actions(
        requested
            .into_iter()
            .map(ToolbarAction::AddComparisonStudy)
            .collect(),
    );
    poll_until(&mut app, |a| a.app_state.comparison_studies.len() == 2);
    let assert_comparisons = |app: &TunnyApp| {
        assert_eq!(
            app.app_state
                .comparison_studies
                .iter()
                .map(|s| s.meta.study_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            app.app_state.comparison_colors,
            (0..2)
                .map(crate::theme::color_compute::comparison_color_at)
                .collect::<Vec<_>>()
        );
        assert_eq!(app.app_state.comparison_convergence_histories.len(), 2);
        for context in &app.app_state.comparison_studies {
            let expected = crate::test_support::journal(bytes.as_bytes(), context.meta.study_id);
            let legacy = expected.dataframe;
            assert_eq!(
                context.view.trial_ids,
                (0..legacy.row_count())
                    .map(|row| legacy.get_trial_id(row).unwrap())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                context.view.numeric_column("obj0"),
                legacy.get_numeric_column("obj0")
            );
            assert_eq!(
                (0..context.trial_count())
                    .map(|row| context.view.df.get_trial_number(row))
                    .collect::<Vec<_>>(),
                (0..legacy.row_count())
                    .map(|row| legacy.get_trial_number(row))
                    .collect::<Vec<_>>()
            );
        }
    };
    // Both comparisons were initially unloaded: their order must match the old
    // full-read caller, not the primary Study's streaming completion order.
    assert_comparisons(&app);

    // Intercept production worker delivery, not sleeps or a guessed timing gap.
    // Forward the real reload scan/base/batches and small comparison completion,
    // but withhold the large comparison's finish until the small one is complete.
    let delivery = app.tx.clone();
    let (intercept, completions) = make_channel();
    app.tx = intercept;
    app.reload_current();
    let mut held_large = None;
    let mut small_finished = false;
    while held_large.is_none() || !small_finished {
        let message = completions
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        if matches!(&message, AppMessage::JournalFinished { header, .. } if header.ticket.study().stored_id == 1)
        {
            held_large = Some(message);
        } else {
            let is_small = matches!(&message, AppMessage::JournalFinished { header, result: Ok(()) } if header.ticket.study().stored_id == 2);
            delivery.send(message).unwrap();
            app.poll_messages(&egui::Context::default());
            assert!(app.load_error.is_none(), "{:?}", app.load_error);
            if is_small {
                small_finished = true;
                assert!(
                    app.app_state.comparison_studies.is_empty(),
                    "later requests must not take the earlier display/color slot"
                );
                assert_eq!(
                    app.app_state.journal_studies.as_ref().unwrap().managers[&2]
                        .load_state()
                        .unwrap()
                        .phase,
                    tdashboard_study::LoadPhase::Complete
                );
            }
        }
    }
    delivery.send(held_large.unwrap()).unwrap();
    app.poll_messages(&egui::Context::default());
    assert_comparisons(&app);
}

#[test]
fn d2_comparison_cached_streaming_input_is_reused_but_forced_reload_uses_full_read_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cached-comparison.log");
    std::fs::write(&path, d2_comparison_fixture()).unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(path);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 3);
    select_artifact_study(&mut app, 1);
    let streaming = app
        .app_state
        .current_study
        .as_ref()
        .unwrap()
        .view
        .trial_ids
        .clone();
    assert_eq!(streaming, (1..1002).rev().collect::<Vec<_>>());
    let stamp = app.app_state.journal_studies.as_ref().unwrap().managers[&1]
        .acquire()
        .unwrap()
        .stamp();
    select_artifact_study(&mut app, 0);
    let meta = app
        .app_state
        .all_studies
        .iter()
        .find(|s| s.study_id == 1)
        .unwrap()
        .clone();
    app.apply_toolbar_actions(vec![ToolbarAction::AddComparisonStudy(meta)]);
    poll_until(&mut app, |a| a.app_state.comparison_studies.len() == 1);
    assert_eq!(
        app.app_state.comparison_studies[0].view.trial_ids,
        streaming
    );
    assert_eq!(
        app.app_state.journal_studies.as_ref().unwrap().managers[&1]
            .acquire()
            .unwrap()
            .stamp(),
        stamp
    );
    app.reload_current();
    poll_until(&mut app, |a| {
        a.pending_reload.is_none() && a.app_state.comparison_studies.len() == 1
    });
    assert_eq!(
        app.app_state.comparison_studies[0].view.trial_ids,
        (1..1002).collect::<Vec<_>>()
    );
    let reloaded = app.app_state.journal_studies.as_ref().unwrap().managers[&1]
        .acquire()
        .unwrap();
    assert_ne!(reloaded.stamp().input.generation, stamp.input.generation);
}

fn assert_d2_overlapping_journal_load_is_shared(comparison_first: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overlap.log");
    std::fs::write(&path, d2_comparison_fixture()).unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(path);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 3);
    select_artifact_study(&mut app, 0);
    let b = app
        .app_state
        .all_studies
        .iter()
        .find(|s| s.study_id == 1)
        .unwrap()
        .clone();
    let c = app
        .app_state
        .all_studies
        .iter()
        .find(|s| s.study_id == 2)
        .unwrap()
        .clone();
    app.apply_toolbar_actions(vec![if comparison_first {
        ToolbarAction::AddComparisonStudy(b.clone())
    } else {
        ToolbarAction::SelectStudy(b.clone())
    }]);
    // Hold the real first batch while its worker waits for acknowledgement.
    // The opposite interest is added before any B input is published.
    let first_batch = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let AppMessage::JournalBatch { header, .. } = &first_batch else {
        panic!("expected B's first batch");
    };
    let ticket = header.ticket;
    app.apply_toolbar_actions(vec![if comparison_first {
        ToolbarAction::SelectStudy(b.clone())
    } else {
        ToolbarAction::AddComparisonStudy(b.clone())
    }]);
    app.apply_toolbar_actions(vec![ToolbarAction::AddComparisonStudy(b.clone())]);
    let journal = app.app_state.journal_studies.as_ref().unwrap();
    assert_eq!(journal.managers[&1].load_state().unwrap().ticket, ticket);
    assert!(journal.has_comparison(ticket.study()));
    assert!(
        app.rx.try_recv().is_err(),
        "sharing must not launch another worker"
    );
    app.tx.send(first_batch).unwrap();
    poll_until(&mut app, |a| {
        !a.is_loading && a.app_state.comparison_studies.len() == 1
    });
    let journal = app.app_state.journal_studies.as_ref().unwrap();
    let input = journal.managers[&1].acquire().unwrap();
    assert_eq!(
        input.load_state().phase,
        tdashboard_study::LoadPhase::Complete
    );
    assert_eq!(input.load_state().ticket, ticket);
    assert_eq!(app.app_state.journal_display, Some(input.stamp()));
    assert!(!journal.has_comparison(ticket.study()));
    let primary = app.app_state.current_study.as_ref().unwrap();
    let comparison = &app.app_state.comparison_studies[0];
    let expected: Vec<u32> = if comparison_first {
        (1..1002).collect()
    } else {
        (1..1002).rev().collect()
    };
    assert_eq!(primary.meta.study_id, 1);
    assert_eq!(primary.view.trial_ids, expected);
    assert_eq!(comparison.view.trial_ids, expected);
    assert!(std::sync::Arc::ptr_eq(
        &primary.view.df,
        &comparison.view.df
    ));
    let front = primary.view.front.as_ref().unwrap();
    assert!(front
        .handle()
        .same_result(comparison.view.front.as_ref().unwrap().handle()));
    assert_eq!(primary.view.pareto_rank.as_ptr(), front.ranks.as_ptr());
    assert!(std::sync::Arc::ptr_eq(
        &primary.view.df,
        &app.app_state
            .journal_studies
            .as_ref()
            .unwrap()
            .analysis
            .borrow_mut()
            .project(primary.view.input().unwrap())
            .dataframe
    ));
    assert_eq!(app.app_state.comparison_colors.len(), 1);
    assert_eq!(app.app_state.comparison_convergence_histories.len(), 1);

    // A later comparison must not wait behind an orphaned B reservation.
    app.apply_toolbar_actions(vec![ToolbarAction::AddComparisonStudy(c)]);
    poll_until(&mut app, |a| a.app_state.comparison_studies.len() == 2);
    assert_eq!(
        app.app_state
            .comparison_studies
            .iter()
            .map(|s| s.meta.study_id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    select_artifact_study(&mut app, 0);
    app.apply_toolbar_actions(vec![
        ToolbarAction::RemoveComparisonStudy(0),
        ToolbarAction::AddComparisonStudy(b),
    ]);
    poll_until(&mut app, |a| {
        a.app_state
            .comparison_studies
            .iter()
            .map(|s| s.meta.study_id)
            .collect::<Vec<_>>()
            == [2, 1]
    });
    assert!(!app
        .app_state
        .journal_studies
        .as_ref()
        .unwrap()
        .has_comparison(ticket.study()));
}

#[test]
fn d2_comparison_in_flight_then_primary_selection_shares_the_study_load() {
    assert_d2_overlapping_journal_load_is_shared(true);
    assert_d2_overlapping_journal_load_is_shared(false);
}

#[test]
fn d2_shared_journal_failure_retry_and_forced_supersede_resolve_comparison_interests() {
    for comparison_first in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("overlap-failure.log");
        std::fs::write(&path, d2_comparison_fixture()).unwrap();
        let mut app = artifact_test_app(None);
        app.open_path(path);
        poll_until(&mut app, |a| a.app_state.all_studies.len() == 3);
        select_artifact_study(&mut app, 0);
        let b = app
            .app_state
            .all_studies
            .iter()
            .find(|s| s.study_id == 1)
            .unwrap()
            .clone();
        let c = app
            .app_state
            .all_studies
            .iter()
            .find(|s| s.study_id == 2)
            .unwrap()
            .clone();
        let delivery = app.tx.clone();
        let (intercept, results) = make_channel();
        app.tx = intercept;
        app.apply_toolbar_actions(vec![if comparison_first {
            ToolbarAction::AddComparisonStudy(b.clone())
        } else {
            ToolbarAction::SelectStudy(b.clone())
        }]);
        let mut bad_batch = results
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let AppMessage::JournalBatch { header, .. } = &mut bad_batch else {
            panic!("expected B batch");
        };
        let failed_ticket = header.ticket;
        header.base_revision += 1;
        app.apply_toolbar_actions(vec![
            if comparison_first {
                ToolbarAction::SelectStudy(b.clone())
            } else {
                ToolbarAction::AddComparisonStudy(b.clone())
            },
            ToolbarAction::AddComparisonStudy(c),
        ]);
        while app
            .app_state
            .journal_studies
            .as_ref()
            .unwrap()
            .managers
            .get(&2)
            .and_then(|m| m.load_state())
            .is_none_or(|s| s.phase != tdashboard_study::LoadPhase::Complete)
        {
            let msg = results
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            delivery.send(msg).unwrap();
            app.poll_messages(&egui::Context::default());
        }
        assert!(app.app_state.comparison_studies.is_empty());
        delivery.send(bad_batch).unwrap();
        app.poll_messages(&egui::Context::default());
        // Rejection travels back through the real worker's fallible sink/finish.
        let failure = results
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        assert!(
            matches!(&failure, AppMessage::JournalFinished { header, result: Err(_), .. } if header.ticket == failed_ticket)
        );
        delivery.send(failure).unwrap();
        app.poll_messages(&egui::Context::default());
        assert!(!app.is_loading);
        assert!(app.load_error.is_some());
        assert_eq!(
            app.app_state
                .comparison_studies
                .iter()
                .map(|s| s.meta.study_id)
                .collect::<Vec<_>>(),
            [2]
        );
        assert!(!app
            .app_state
            .journal_studies
            .as_ref()
            .unwrap()
            .has_comparison(failed_ticket.study()));
        app.apply_toolbar_actions(vec![
            ToolbarAction::ClearLoadError,
            ToolbarAction::AddComparisonStudy(b.clone()),
        ]);
        let old_batch = results
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let AppMessage::JournalBatch { header, .. } = &old_batch else {
            panic!("expected retry batch");
        };
        let old_ticket = header.ticket;
        app.apply_toolbar_actions(vec![ToolbarAction::SelectStudy(b.clone())]);
        // Force uses the production reload entry adapter, retaining the pending
        // comparison interest while replacing the acquisition ticket.
        app.select_study(b, false, true);
        let current_ticket = app.app_state.journal_studies.as_ref().unwrap().managers[&1]
            .load_state()
            .unwrap()
            .ticket;
        assert_ne!(current_ticket, old_ticket);
        delivery.send(old_batch).unwrap();
        app.poll_messages(&egui::Context::default());
        let mut stale_finished = false;
        loop {
            let msg = results
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            stale_finished |= matches!(&msg, AppMessage::JournalFinished { header, .. } if header.ticket == old_ticket);
            delivery.send(msg).unwrap();
            app.poll_messages(&egui::Context::default());
            assert!(app.load_error.is_none(), "{:?}", app.load_error);
            let journal = app.app_state.journal_studies.as_ref().unwrap();
            if stale_finished
                && journal.managers[&1].load_state().unwrap().phase
                    == tdashboard_study::LoadPhase::Complete
                && !journal.has_comparison(current_ticket.study())
            {
                break;
            }
        }
        assert!(!app.is_loading);
        assert_eq!(
            app.app_state
                .comparison_studies
                .iter()
                .map(|s| s.meta.study_id)
                .collect::<Vec<_>>(),
            [2, 1]
        );
        let input = app.app_state.journal_studies.as_ref().unwrap().managers[&1]
            .acquire()
            .unwrap();
        assert_eq!(input.load_state().ticket, current_ticket);
        assert_eq!(app.app_state.journal_display, Some(input.stamp()));
    }
}

#[test]
fn d2_cached_journal_selection_and_comparison_complete_with_a_full_gui_channel() {
    fn apply_without_draining(app: &mut TunnyApp, action: ToolbarAction) {
        assert!(matches!(
            app.tx.try_send(AppMessage::CsvExportDone),
            Err(mpsc::TrySendError::Full(_))
        ));
        // Keep the GUI receiver undrained. A watchdog releases one slot only on
        // regression, so a blocking self-send fails instead of hanging the suite.
        // This timeout bounds the harness; it is not a product latency promise.
        let (_, placeholder) = mpsc::sync_channel(0);
        let receiver = std::mem::replace(&mut app.rx, placeholder);
        let (finished, done) = mpsc::channel();
        let watchdog = std::thread::spawn(move || {
            let timed_out = done
                .recv_timeout(std::time::Duration::from_secs(5))
                .is_err();
            if timed_out {
                let _ = receiver.recv();
            }
            (receiver, timed_out)
        });
        app.apply_toolbar_actions(vec![action]);
        let _ = finished.send(());
        let (receiver, timed_out) = watchdog.join().unwrap();
        app.rx = receiver;
        assert!(
            !timed_out,
            "cached activation blocked on the GUI's full channel"
        );
        assert!(
            matches!(
                app.tx.try_send(AppMessage::CsvExportDone),
                Err(mpsc::TrySendError::Full(_))
            ),
            "activation must not consume backlog to make space"
        );
    }

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cached-full-channel.log");
    std::fs::write(&path, d2_comparison_fixture()).unwrap();
    let mut app = artifact_test_app(None);
    app.open_path(path);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 3);
    select_artifact_study(&mut app, 1);
    select_artifact_study(&mut app, 2);
    select_artifact_study(&mut app, 0);
    let b = app
        .app_state
        .all_studies
        .iter()
        .find(|s| s.study_id == 1)
        .unwrap()
        .clone();
    let c = app
        .app_state
        .all_studies
        .iter()
        .find(|s| s.study_id == 2)
        .unwrap()
        .clone();
    let b_input = app.app_state.journal_studies.as_ref().unwrap().managers[&1]
        .acquire()
        .unwrap();
    let c_input = app.app_state.journal_studies.as_ref().unwrap().managers[&2]
        .acquire()
        .unwrap();
    for _ in 0..32 {
        assert!(app.tx.try_send(AppMessage::CsvExportDone).is_ok());
    }
    apply_without_draining(&mut app, ToolbarAction::SelectStudy(b));
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().meta.study_id,
        1
    );
    assert_eq!(app.app_state.journal_display, Some(b_input.stamp()));
    assert!(!app.is_loading);
    assert!(app.load_error.is_none());
    apply_without_draining(&mut app, ToolbarAction::AddComparisonStudy(c));
    assert_eq!(app.app_state.comparison_studies.len(), 1);
    assert_eq!(app.app_state.comparison_studies[0].meta.study_id, 2);
    assert_eq!(app.app_state.comparison_colors.len(), 1);
    assert_eq!(app.app_state.comparison_convergence_histories.len(), 1);
    let journal = app.app_state.journal_studies.as_ref().unwrap();
    assert_eq!(
        journal.managers[&1].acquire().unwrap().stamp(),
        b_input.stamp()
    );
    assert_eq!(
        journal.managers[&2].acquire().unwrap().stamp(),
        c_input.stamp()
    );
    assert!(!journal.has_comparison(c_input.stamp().input.study));
    assert!(!app.is_loading);
    app.poll_messages(&egui::Context::default());
    assert_eq!(
        app.app_state.current_study.as_ref().unwrap().meta.study_id,
        1
    );
    assert_eq!(app.app_state.comparison_studies.len(), 1);
    assert!(app.load_error.is_none());
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
    poll_until(app, |a| {
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
    let mut app = artifact_test_app(None);
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal.clone())]);
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    // Manual scan before the first streamed study activation.
    app.apply_toolbar_actions(vec![ToolbarAction::ScanArtifacts(folder.clone())]);
    poll_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
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
    poll_until(&mut app, |a| {
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
    poll_until(&mut app, |a| {
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
            scan_id: stale,
        })
        .unwrap();
    poll_until(&mut app, |a| a.app_state.all_studies.len() == 2);
    select_artifact_study(&mut app, 0);
    assert!(app.app_state.artifact_map.is_empty());
    assert_eq!(app.app_state.artifacts_dir.as_ref(), Some(&manual));
    assert_eq!(app.app_state.explicit_artifact_root.as_ref(), Some(&manual));

    // A fresh session without an explicit root discovers the adjacent folder.
    let mut app = artifact_test_app(None);
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal)]);
    poll_until(&mut app, |a| {
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
            scan_id: stale,
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
fn cli_root_overrides_adjacent_discovery_and_preserves_scans_across_studies() {
    let source = tempfile::tempdir().unwrap();
    let cli = tempfile::tempdir().unwrap();
    let journal = write_artifact_race_fixture(source.path(), "target");
    std::fs::write(cli.path().join("target"), b"cli image").unwrap();
    let mut app = artifact_test_app(Some(cli.path().to_path_buf()));
    app.open_path(journal.clone());
    poll_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
    assert_eq!(
        app.app_state.artifact_map[&0][0].path,
        cli.path().join("target")
    );
    let scan_id = app.latest_artifact_scan_id;
    select_artifact_study(&mut app, 0);
    select_artifact_study(&mut app, 1);
    select_artifact_study(&mut app, 0);
    assert_eq!(app.latest_artifact_scan_id, scan_id);
    assert_eq!(
        app.app_state.artifact_map[&0][0].path,
        cli.path().join("target")
    );

    std::fs::write(cli.path().join("new"), b"new image").unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new().append(true).open(&journal).unwrap()
        .write_all(concat!("\n", r#"{"op_code":9,"trial_id":0,"system_attr":{"artifacts:new":"{\"filename\":\"new.png\",\"mimetype\":\"image/png\"}"}}"#, "\n").as_bytes()).unwrap();
    app.reload_current();
    poll_until(&mut app, |a| {
        !a.is_loading && a.pending_reload.is_none() && a.app_state.artifact_map[&0].len() == 2
    });
    assert_eq!(
        app.latest_artifact_scan_id,
        scan_id + 2,
        "one invalidation and one reload scan"
    );
    assert_eq!(app.app_state.artifacts_dir.as_deref(), Some(cli.path()));
    app.reset_to_empty();
    assert!(app.app_state.artifact_map.is_empty());
    assert_eq!(
        app.app_state.explicit_artifact_root.as_deref(),
        Some(cli.path())
    );
    app.open_path(journal);
    poll_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
    assert_eq!(app.app_state.artifacts_dir.as_deref(), Some(cli.path()));
}

#[test]
fn artifact_race_late_journal_scan_cannot_replace_requested_storage() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let a_path = write_artifact_race_fixture(a.path(), "image-a");
    // Workers echo the requested local path, including lexical components.
    let b_path = write_artifact_race_fixture(b.path(), "image-b");
    let b_path = b_path.parent().unwrap().join(".").join("study.log");
    let mut app = artifact_test_app(None);
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
    let first = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let second = app
        .rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let (late_a, current_b) = if matches!(&first, AppMessage::JournalScanned { path, .. } if path == &b_path)
    {
        (second, first)
    } else {
        (first, second)
    };
    assert!(matches!(&late_a, AppMessage::JournalScanned { path, .. } if path != &b_path));
    assert!(matches!(&current_b, AppMessage::JournalScanned { path, .. } if path == &b_path));
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
    poll_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
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
    let tmp = tempfile::tempdir().unwrap();
    let journal = write_artifact_race_fixture(tmp.path(), "target");
    let manual = tmp.path().join("manual");
    std::fs::create_dir(&manual).unwrap();
    image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 255, 0, 255]))
        .save_with_format(manual.join("target"), image::ImageFormat::Png)
        .unwrap();
    let mut app = artifact_test_app(None);
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal.clone())]);
    poll_until(&mut app, |a| a.app_state.artifact_map.contains_key(&0));
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
            assert!(matches!(&msg, AppMessage::JournalScanned { .. }));
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
    poll_until(&mut app, |a| !a.is_loading && a.pending_reload.is_none());
    assert_eq!(app.app_state.artifacts_dir.as_ref(), Some(&manual));
    poll_until(&mut app, |a| a.app_state.artifact_map[&0].len() == 2);
    assert!(app.app_state.artifact_map[&0]
        .iter()
        .all(|e| e.path.parent() == Some(manual.as_path())));
    verify_gallery_render_and_image(&mut app, 1);
}

#[test]
fn journal_scan_guard_accepts_normalized_rdb_source() {
    let mut app = artifact_test_app(None);
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
    let journal = std::path::PathBuf::from(std::env::var_os("TUNNY_ARTIFACT_JOURNAL").unwrap());
    let study_name = std::env::var("TUNNY_ARTIFACT_STUDY").unwrap();
    let mut app = artifact_test_app(None);
    app.apply_toolbar_actions(vec![ToolbarAction::OpenJournal(journal.clone())]);
    poll_until(&mut app, |a| {
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
        matches!(&msg, AppMessage::ArtifactsDirScanned { scan_id, .. } if *scan_id > prior_scan)
    );
    app.tx.send(msg).unwrap();
    app.poll_messages(&egui::Context::default());
    assert_eq!(app.app_state.artifacts_dir, Some(folder));
    verify_gallery_render_and_image(&mut app, 501);
    println!("Verified study {study_name}: automatic and explicit manual scans, 501 entries");
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
fn unsupported_drop_message_guides_gh_binary_to_ghx() {
    let msg = unsupported_drop_message(&[std::path::PathBuf::from("/a/model.gh")]);
    assert!(msg.contains("model.gh"), "{msg}");
    assert!(msg.contains(".ghx"), "{msg}");
    assert!(msg.contains("Save As"), "{msg}");
    let msg = unsupported_drop_message(&[std::path::PathBuf::from("notes.txt")]);
    assert!(msg.contains("notes.txt"), "{msg}");
    assert!(msg.contains("unsupported file type"), "{msg}");
    assert!(msg.contains(".ghx"), "{msg}");
    let msg = unsupported_drop_message(&[]);
    assert!(msg.contains("(unknown)"), "{msg}");
}

#[test]
fn convergence_done_maps_to_compute_sync() {
    // Regression guard: if IndicatorHistoryDone falls out of the sync targets, the
    // canvas item's computing flag never drops after compute finishes and the
    // spinner keeps spinning.
    use crate::state::app_state::ConvergenceHistory;
    let msg = AppMessage::IndicatorHistoryDone {
        source_df: std::sync::Arc::new(tdashboard_analysis::dataframe::DataFrame::empty()),
        indicator: tdashboard_analysis::indicators::MoIndicator::Hypervolume,
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

// ── Phase C: window title password masking ─────────────────

#[test]
fn compute_window_title_no_path_returns_base_title() {
    assert_eq!(
        TunnyApp::compute_window_title(None),
        "Tunny Dashboard (Beta)"
    );
    let path = std::path::PathBuf::from("/home/user/study.log");
    assert_eq!(
        TunnyApp::compute_window_title(Some(&path)),
        "Tunny Dashboard (Beta) - /home/user/study.log"
    );
    let path = std::path::PathBuf::from("postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test");
    assert_eq!(
        TunnyApp::compute_window_title(Some(&path)),
        "Tunny Dashboard (Beta) - postgresql://tunny:***@127.0.0.1:5432/tunny_test"
    );
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
    // The title bar names the file even when the scan found nothing in it, so a
    // silent reset would look like the app dropped an open file on its own.
    let mut app_state = AppState::new();
    app_state.journal_path = Some(std::path::PathBuf::from("/tmp/empty.log"));
    assert!(has_discardable_state(&app_state, &LayoutState::default()));
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
