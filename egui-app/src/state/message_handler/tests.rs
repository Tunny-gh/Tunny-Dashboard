use super::*;
use crate::state::app_state::{Direction, StudyMeta};

/// For tests: stores a DataFrame into the shared store (thread_local in
/// test builds) and returns a new StudySelected payload (study_id + pareto_rank).
fn make_study_message(trial_count: usize) -> AppMessage {
    let core_rows: Vec<CoreTrialRow> = (0..trial_count)
        .map(|i| CoreTrialRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
            param_category_label: std::collections::HashMap::new(),
            objective_values: vec![i as f64],
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(
        &core_rows,
        &["x".to_string()],
        &["y".to_string()],
        &[],
        &[],
        0,
    );
    tunny_core::dataframe::store_dataframes(vec![df]);

    AppMessage::StudySelected {
        meta: StudyMeta {
            study_id: 0,
            name: "s".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: trial_count,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        },
        study_id: 0,
        pareto_rank: vec![0; trial_count],
        pareto_indices: vec![],
    }
}

/// Guard for serializing tests that use the shared store (a process-global
/// in production builds). Since tunny-desktop's tests link tunny-core
/// normally, the store is shared across all tests. Tests using
/// store_dataframes + snapshot are serialized with this guard to prevent races.
fn test_store_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// For tests: builds a StudySelected for a single-objective Study, with
/// arbitrary objective values / direction (for verifying best_trial_history wiring).
fn make_study_message_single_objective(values: &[f64], direction: Direction) -> AppMessage {
    let trial_count = values.len();
    let core_rows: Vec<CoreTrialRow> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| CoreTrialRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
            param_category_label: std::collections::HashMap::new(),
            objective_values: vec![v],
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(
        &core_rows,
        &["x".to_string()],
        &["y".to_string()],
        &[],
        &[],
        0,
    );
    tunny_core::dataframe::store_dataframes(vec![df]);

    AppMessage::StudySelected {
        meta: StudyMeta {
            study_id: 0,
            name: "s".to_string(),
            directions: vec![direction],
            completed_trials: trial_count,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        },
        study_id: 0,
        pareto_rank: vec![0; trial_count],
        pareto_indices: vec![],
    }
}

/// For tests: builds a StudySelected for a 2-objective Study (for
/// verifying best_trial_history stays None for multi-objective).
fn make_study_message_multi_objective(trial_count: usize) -> AppMessage {
    let core_rows: Vec<CoreTrialRow> = (0..trial_count)
        .map(|i| CoreTrialRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
            param_category_label: std::collections::HashMap::new(),
            objective_values: vec![i as f64, (trial_count - i) as f64],
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(
        &core_rows,
        &["x".to_string()],
        &["y1".to_string(), "y2".to_string()],
        &[],
        &[],
        0,
    );
    tunny_core::dataframe::store_dataframes(vec![df]);

    AppMessage::StudySelected {
        meta: StudyMeta {
            study_id: 0,
            name: "s".to_string(),
            directions: vec![Direction::Minimize, Direction::Minimize],
            completed_trials: trial_count,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y1".to_string(), "y2".to_string()],
            param_bounds: Default::default(),
        },
        study_id: 0,
        pareto_rank: vec![0; trial_count],
        pareto_indices: vec![],
    }
}

#[test]
fn best_trial_history_set_for_single_objective_minimize() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;

    MessageHandler::handle(
        make_study_message_single_objective(&[3.0, 1.0, 2.0], Direction::Minimize),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert_eq!(
        app_state.best_trial_history,
        Some(vec![(0, 3.0), (1, 1.0), (2, 1.0)])
    );
}

#[test]
fn best_trial_history_set_for_single_objective_maximize() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;

    MessageHandler::handle(
        make_study_message_single_objective(&[1.0, 3.0, 2.0], Direction::Maximize),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert_eq!(
        app_state.best_trial_history,
        Some(vec![(0, 1.0), (1, 3.0), (2, 3.0)])
    );
}

#[test]
fn best_trial_history_none_for_multi_objective() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;

    MessageHandler::handle(
        make_study_message_multi_objective(3),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert!(app_state.best_trial_history.is_none());
}

#[test]
fn clustering_done_updates_state_when_lengths_match() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;

    MessageHandler::handle(
        make_study_message(3),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    widgets.cluster_scatter.computing = true;
    let key = widgets.cluster_scatter.cache_key();
    MessageHandler::handle(
        AppMessage::ClusteringDone {
            source: crate::state::messages::ClusterChartSource::Scatter2D,
            key: key.clone(),
            result: crate::state::results::ClusterResult {
                labels: vec![0, 1, 0],
                n_clusters: 2,
            },
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert!(app_state.cluster_cache.contains_key(&key));
    assert!(!widgets.cluster_scatter.computing);
    assert!(widgets.cluster_scatter.last_error.is_none());
}

#[test]
fn clustering_done_rejects_mismatched_label_length() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;

    MessageHandler::handle(
        make_study_message(3),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    let key = widgets.cluster_scatter.cache_key();
    MessageHandler::handle(
        AppMessage::ClusteringDone {
            source: crate::state::messages::ClusterChartSource::Scatter2D,
            key: key.clone(),
            result: crate::state::results::ClusterResult {
                labels: vec![0, 1],
                n_clusters: 2,
            },
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert!(app_state.cluster_cache.is_empty());
    assert!(widgets.cluster_scatter.last_error.is_some());
}

fn make_chunk_row(trial_id: u32, x: f64, obj: f64) -> CoreTrialRow {
    CoreTrialRow {
        trial_id,
        trial_number: trial_id,
        param_display: std::collections::HashMap::from([("x".to_string(), x)]),
        param_category_label: std::collections::HashMap::new(),
        objective_values: vec![obj],
        user_attrs_numeric: std::collections::HashMap::new(),
        user_attrs_string: std::collections::HashMap::new(),
        constraint_values: vec![],
    }
}

fn chunk_message(rows: Vec<CoreTrialRow>, is_first: bool, is_final: bool) -> AppMessage {
    AppMessage::StudyChunkLoaded {
        study_id: 0,
        meta: StudyMeta {
            study_id: 0,
            name: "s".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 0,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        },
        new_rows: rows,
        param_names: vec!["x".to_string()],
        objective_names: vec!["y".to_string()],
        user_attr_numeric_names: vec![],
        user_attr_string_names: vec![],
        max_constraints: 0,
        is_first,
        is_final,
    }
}

#[test]
fn study_chunks_accumulate_rows_across_batches() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = true;
    let mut load_error = None;

    // 1st batch: establishes study, still loading.
    MessageHandler::handle(
        chunk_message(
            vec![make_chunk_row(0, 0.1, 1.0), make_chunk_row(1, 0.2, 2.0)],
            true,
            false,
        ),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );
    assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 2);
    assert!(is_loading, "still loading mid-stream");

    // 2nd (final) batch: appends and finalizes.
    MessageHandler::handle(
        chunk_message(vec![make_chunk_row(2, 0.3, 3.0)], false, true),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );
    assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 3);
    assert!(!is_loading, "loading cleared on final batch");

    // The column data has been merged
    let xs = app_state
        .current_study
        .as_ref()
        .unwrap()
        .view
        .numeric_column("x")
        .unwrap()
        .to_vec();
    assert_eq!(xs, vec![0.1, 0.2, 0.3]);
}

/// Regression: a batch containing a row with fewer objectives than the study
/// declares must not panic the multi-objective Pareto computation on an
/// out-of-range slice. A reload re-reads every trial through this path, so a
/// journal holding one such row would otherwise crash on every reload.
#[test]
fn study_chunk_handles_ragged_objectives_without_panic() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = true;
    let mut load_error = None;

    let two_objective_meta = StudyMeta {
        study_id: 0,
        name: "s".to_string(),
        directions: vec![Direction::Minimize, Direction::Minimize],
        completed_trials: 3,
        param_names: vec!["x".to_string()],
        objective_names: vec!["o1".to_string(), "o2".to_string()],
        param_bounds: Default::default(),
    };
    let ragged_row = |trial_id: u32, objective_values: Vec<f64>| CoreTrialRow {
        trial_id,
        trial_number: trial_id,
        param_display: std::collections::HashMap::from([("x".to_string(), trial_id as f64)]),
        param_category_label: std::collections::HashMap::new(),
        objective_values,
        user_attrs_numeric: std::collections::HashMap::new(),
        user_attrs_string: std::collections::HashMap::new(),
        constraint_values: vec![],
    };

    MessageHandler::handle(
        AppMessage::StudyChunkLoaded {
            study_id: 0,
            meta: two_objective_meta,
            new_rows: vec![
                ragged_row(0, vec![1.0, 2.0]),
                // A trial that straddled the create/complete boundary when the
                // journal was written, so it carries no objectives at all.
                ragged_row(1, vec![]),
                // A trial with only part of the objective vector.
                ragged_row(2, vec![3.0]),
            ],
            param_names: vec!["x".to_string()],
            objective_names: vec!["o1".to_string(), "o2".to_string()],
            user_attr_numeric_names: vec![],
            user_attr_string_names: vec![],
            max_constraints: 0,
            is_first: true,
            is_final: true,
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    let study = app_state.current_study.as_ref().unwrap();
    assert_eq!(study.trial_count(), 3);
    // The complete row is the only Pareto candidate; the ragged ones are
    // treated as NaN rows and simply excluded.
    assert_eq!(study.pareto_indices, vec![0]);
}

#[test]
fn study_selected_resets_cluster_widget_runtime_state() {
    let _g = test_store_guard();
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;

    widgets.cluster_scatter.computing = true;
    widgets.cluster_scatter.pending_compute =
        Some(crate::ui::widgets::cluster_scatter::ClusterComputeRequest {
            k: 3,
            target_space: crate::ui::widgets::cluster_scatter::ClusterSpace::Objective,
            k_mode: crate::ui::widgets::cluster_scatter::KSelectionMode::Manual,
            init_strategy: crate::ui::widgets::cluster_scatter::KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
        });

    MessageHandler::handle(
        make_study_message(4),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert!(!widgets.cluster_scatter.computing);
    assert!(widgets.cluster_scatter.pending_compute.is_none());
    assert!(widgets.cluster_scatter.last_error.is_none());
}

// ── TASK-2230: comparison load message tests ─────────────────

#[test]
fn comparison_load_message_updates_state_entrypoint() {
    use crate::state::app_state::StudyContext;
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error = None;

    let context = StudyContext::from_rows_for_test(
        StudyMeta {
            study_id: 99,
            name: "compare".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 0,
            param_names: vec![],
            objective_names: vec![],
            param_bounds: Default::default(),
        },
        vec![],
    );

    MessageHandler::handle(
        AppMessage::ComparisonStudyLoaded {
            context: Box::new(context),
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert_eq!(app_state.comparison_studies.len(), 1);
    assert_eq!(app_state.comparison_studies[0].meta.study_id, 99);
    // Verifies the parallel Vecs stay the same length
    assert_eq!(app_state.comparison_colors.len(), 1);
    assert_eq!(app_state.comparison_convergence_histories.len(), 1);
}

#[test]
fn comparison_load_failed_message_sets_load_error() {
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error: Option<String> = None;

    MessageHandler::handle(
        AppMessage::ComparisonStudyLoadFailed("file not found".to_string()),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert_eq!(load_error.as_deref(), Some("file not found"));
}

// ── R4: report export done/failed messages ────────────────────

#[test]
fn report_export_done_stores_paths_and_clears_generating() {
    use crate::ui::widgets::report_modal::ReportDialogState;

    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error: Option<String> = None;
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

    let dialog = app_state.report_dialog.as_ref().expect("dialog remains");
    assert!(!dialog.generating);
    assert!(dialog.error.is_none());
    assert_eq!(dialog.success_paths.as_deref(), Some(paths.as_slice()));
    assert_eq!(
        dialog.overwrote_paths,
        vec![std::path::PathBuf::from("/tmp/report_s.json")]
    );
    assert!(load_error.is_none());
}

#[test]
fn report_export_done_without_dialog_is_noop() {
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error: Option<String> = None;

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
    let mut load_error: Option<String> = None;
    app_state.report_dialog = Some(ReportDialogState {
        generating: true,
        ..Default::default()
    });

    MessageHandler::handle(
        AppMessage::Error("disk full".to_string()),
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    let dialog = app_state.report_dialog.as_ref().expect("dialog remains");
    assert!(!dialog.generating);
    assert_eq!(dialog.error.as_deref(), Some("disk full"));
    assert_eq!(load_error.as_deref(), Some("disk full"));
}

// ── .ghx D&D -> optimization run: GhOptFinished ────────────────

fn make_gh_opt_run_state() -> crate::state::app_state::GhOptRunState {
    crate::state::app_state::GhOptRunState {
        progress: tunny_core::surrogate_opt::FitProgress::new(),
        journal_path: std::path::PathBuf::from("/tmp/model_optuna.log"),
        study_name: "model-000001".to_string(),
        finished: None,
    }
}

#[test]
fn gh_opt_finished_ok_formats_success_message() {
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error: Option<String> = None;
    app_state.gh_opt_run = Some(make_gh_opt_run_state());

    MessageHandler::handle(
        AppMessage::GhOptFinished {
            result: Ok(tunny_core::gh::GhRunSummary {
                study_id: 0,
                completed: 48,
                failed: 2,
                cancelled: false,
                adaptive_diagnostics: vec![],
                stop_reason: tunny_core::gh::GhStopReason::Completed,
            }),
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    let run = app_state.gh_opt_run.as_ref().expect("run state remains");
    assert_eq!(
        run.finished.as_ref(),
        Some(&Ok("Done: 48 trials succeeded / 2 failed".to_string()))
    );
}

#[test]
fn gh_opt_finished_ok_cancelled_appends_hint() {
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error: Option<String> = None;
    app_state.gh_opt_run = Some(make_gh_opt_run_state());

    MessageHandler::handle(
        AppMessage::GhOptFinished {
            result: Ok(tunny_core::gh::GhRunSummary {
                study_id: 0,
                completed: 10,
                failed: 0,
                cancelled: true,
                adaptive_diagnostics: vec![],
                stop_reason: tunny_core::gh::GhStopReason::Cancelled,
            }),
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    let run = app_state.gh_opt_run.as_ref().expect("run state remains");
    assert_eq!(
        run.finished.as_ref(),
        Some(&Ok(
            "Done: 10 trials succeeded / 0 failed (cancelled)".to_string()
        ))
    );
}

#[test]
fn gh_opt_finished_err_sets_error_string() {
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error: Option<String> = None;
    app_state.gh_opt_run = Some(make_gh_opt_run_state());

    MessageHandler::handle(
        AppMessage::GhOptFinished {
            result: Err("journal write failed".to_string()),
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    let run = app_state.gh_opt_run.as_ref().expect("run state remains");
    assert_eq!(
        run.finished.as_ref(),
        Some(&Err("journal write failed".to_string()))
    );
}

#[test]
fn gh_opt_finished_without_run_state_is_noop() {
    let mut app_state = AppState::new();
    let mut widgets = WidgetStates::default();
    let mut is_loading = false;
    let mut load_error: Option<String> = None;

    MessageHandler::handle(
        AppMessage::GhOptFinished {
            result: Err("no run".to_string()),
        },
        &mut app_state,
        &mut widgets,
        &mut is_loading,
        &mut load_error,
    );

    assert!(app_state.gh_opt_run.is_none());
    assert!(load_error.is_none());
}
