use super::*;

#[test]
fn report_export_done_stores_paths_and_clears_generating() {
    use crate::ui::widgets::report_modal::ReportDialogState;

    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;
    app_state.report_dialog = Some(ReportDialogState {
        generating: true,
        ..Default::default()
    });
    let paths = vec![
        std::path::PathBuf::from("/tmp/report_s.html"),
        std::path::PathBuf::from("/tmp/report_s.json"),
    ];
    MessageHandler::handle(
        AppMessage::ReportExportDone {
            paths: paths.clone(),
            overwrote: vec![std::path::PathBuf::from("/tmp/report_s.json")],
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );
    let dialog = app_state.report_dialog.as_ref().unwrap();
    assert!(!dialog.generating);
    assert!(dialog.error.is_none());
    assert_eq!(dialog.success_paths.as_deref(), Some(paths.as_slice()));
    assert_eq!(
        dialog.overwrote_paths,
        vec![std::path::PathBuf::from("/tmp/report_s.json")]
    );
    assert!(load_error.is_none());

    app_state.report_dialog = None;
    MessageHandler::handle(
        AppMessage::ReportExportDone {
            paths: vec![],
            overwrote: vec![],
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );
    assert!(app_state.report_dialog.is_none());
    assert!(load_error.is_none());
}

#[test]
fn error_during_report_generation_surfaces_in_dialog() {
    use crate::ui::widgets::report_modal::ReportDialogState;
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;
    app_state.report_dialog = Some(ReportDialogState {
        generating: true,
        ..Default::default()
    });
    MessageHandler::handle(
        AppMessage::Error("disk full".into()),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );
    let dialog = app_state.report_dialog.as_ref().unwrap();
    assert!(!dialog.generating);
    assert_eq!(dialog.error.as_deref(), Some("disk full"));
    assert_eq!(load_error.as_deref(), Some("disk full"));
}

fn make_gh_opt_run_state() -> crate::state::app_state::GhOptRunState {
    crate::state::app_state::GhOptRunState {
        progress: tdashboard_analysis::surrogate_opt::FitProgress::new(),
        journal_path: std::path::PathBuf::from("/tmp/model_optuna.log"),
        study_name: "model-000001".into(),
        finished: None,
    }
}

#[test]
fn gh_opt_finished_ok_formats_success_message() {
    use tdashboard_optimization_runner::gh::{GhRunSummary, GhStopReason};
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;
    for (completed, failed, cancelled, stop_reason, expected) in [
        (
            48,
            2,
            false,
            GhStopReason::Completed,
            "Done: 48 trials succeeded / 2 failed",
        ),
        (
            10,
            0,
            true,
            GhStopReason::Cancelled,
            "Done: 10 trials succeeded / 0 failed (cancelled)",
        ),
    ] {
        app_state.gh_opt_run = Some(make_gh_opt_run_state());
        MessageHandler::handle(
            AppMessage::GhOptFinished {
                result: Ok(GhRunSummary {
                    study_id: 0,
                    completed,
                    failed,
                    cancelled,
                    adaptive_diagnostics: vec![],
                    stop_reason,
                }),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(
            app_state.gh_opt_run.as_ref().unwrap().finished.as_ref(),
            Some(&Ok(expected.to_string()))
        );
    }
}

#[test]
fn gh_opt_finished_err_sets_error_string() {
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;
    app_state.gh_opt_run = Some(make_gh_opt_run_state());
    MessageHandler::handle(
        AppMessage::GhOptFinished {
            result: Err("journal write failed".into()),
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );
    assert_eq!(
        app_state.gh_opt_run.as_ref().unwrap().finished.as_ref(),
        Some(&Err("journal write failed".into()))
    );
    app_state.gh_opt_run = None;
    MessageHandler::handle(
        AppMessage::GhOptFinished {
            result: Err("no run".into()),
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );
    assert!(app_state.gh_opt_run.is_none());
    assert!(load_error.is_none());
}
