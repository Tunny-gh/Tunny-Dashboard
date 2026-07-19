use super::*;
use crate::state::app_state::AppState;
use crate::state::results::ConvergenceHistory;
use crate::state::types::{Direction, StudyContext, StudyMeta, TrialRow};
use crate::ui::widget_states::WidgetStates;
use std::collections::HashMap;

fn make_study(
    param_names: Vec<String>,
    obj_names: Vec<String>,
    directions: Vec<Direction>,
) -> StudyContext {
    let meta = StudyMeta {
        study_id: 0,
        name: "test".to_string(),
        directions,
        completed_trials: 0,
        param_names,
        objective_names: obj_names,
        param_bounds: Default::default(),
    };
    StudyContext::from_rows_for_test(meta, vec![])
}

fn make_trial(id: u32, params: HashMap<String, f64>, objectives: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id: id,
        trial_number: id,
        params,
        objectives,
        ..Default::default()
    }
}

fn make_trial_ranked(
    id: u32,
    params: HashMap<String, f64>,
    objectives: Vec<f64>,
    pareto_rank: u32,
) -> TrialRow {
    TrialRow {
        trial_id: id,
        trial_number: id,
        params,
        objectives,
        pareto_rank,
        ..Default::default()
    }
}

#[test]
fn csv_export_filename_optimization_history() {
    assert_eq!(
        csv_export_filename(&ChartId::OptimizationHistory),
        "optimization_history.csv"
    );
}

#[test]
fn csv_export_filename_all_end_with_csv() {
    let ids = vec![
        ChartId::OptimizationHistory,
        ChartId::ConvergenceIndicators,
        ChartId::ImportanceChart,
        ChartId::PdpChart,
        ChartId::PdpChart2D,
        ChartId::ParallelCoordinates,
        ChartId::ScatterMatrix,
        ChartId::ClusterScatter,
        ChartId::SensitivityHeatmap,
        ChartId::ParetoScatter2D,
        ChartId::ParetoScatter3D,
        ChartId::McdmRankChart,
        ChartId::McdmScatterChart,
        ChartId::SliceChart,
        ChartId::SurrogateOpt,
    ];
    for id in &ids {
        assert!(
            csv_export_filename(id).ends_with(".csv"),
            "{:?} filename does not end with .csv",
            id
        );
    }
}

#[test]
fn opt_history_csv_minimize_tracks_cumulative_min() {
    let mut state = AppState::default();
    let mut study = make_study(
        vec!["x".into()],
        vec!["f".into()],
        vec![Direction::Minimize],
    );
    study.set_rows_for_test(vec![
        make_trial(0, HashMap::new(), vec![3.0]),
        make_trial(1, HashMap::new(), vec![1.0]),
        make_trial(2, HashMap::new(), vec![2.0]),
    ]);
    state.current_study = Some(study);
    let widgets = WidgetStates::default();

    let csv = build_optimization_history_csv(&state, &widgets).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "trial_index,objective_value,best_value");
    assert_eq!(lines[1], "0,3,3");
    assert_eq!(lines[2], "1,1,1");
    assert_eq!(lines[3], "2,2,1");
}

#[test]
fn opt_history_csv_nan_objective_becomes_empty_field() {
    let mut state = AppState::default();
    let mut study = make_study(
        vec!["x".into()],
        vec!["f".into()],
        vec![Direction::Minimize],
    );
    study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![f64::NAN])]);
    state.current_study = Some(study);
    let widgets = WidgetStates::default();

    let csv = build_optimization_history_csv(&state, &widgets).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    // NaN objective and the still-infinite running best both serialize as empty fields.
    assert_eq!(lines[1], "0,,");
}

#[test]
fn opt_history_csv_returns_none_when_no_study() {
    let state = AppState::default();
    let widgets = WidgetStates::default();
    assert!(build_optimization_history_csv(&state, &widgets).is_none());
}

#[test]
fn convergence_csv_uses_index_times_step() {
    let state = AppState {
        convergence_history: Some(ConvergenceHistory {
            trial_ids: vec![10, 20, 30],
            values: vec![0.1, 0.5, 0.8],
            sample_step: 5,
            ref_point: vec![],
        }),
        // convergence_indicator is initialized to Hypervolume by AppState::default().
        ..AppState::default()
    };
    let csv = build_convergence_csv(&state).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "trial_index,Hypervolume");
    assert_eq!(lines[1], "0,0.1");
    assert_eq!(lines[2], "5,0.5");
    assert_eq!(lines[3], "10,0.8");
}

#[test]
fn convergence_csv_returns_none_when_missing() {
    let state = AppState::default();
    assert!(build_convergence_csv(&state).is_none());
}

#[test]
fn importance_csv_returns_none_when_computing() {
    let state = AppState::default();
    let mut widgets = WidgetStates::default();
    widgets.importance.computing = true;
    assert!(build_importance_csv(&state, &widgets).is_none());
}

#[test]
fn importance_csv_returns_none_when_no_cache() {
    let state = AppState::default();
    let widgets = WidgetStates::default();
    // importance_cache is empty, should return None
    assert!(build_importance_csv(&state, &widgets).is_none());
}

#[test]
fn importance_csv_has_expected_columns() {
    use crate::state::app_state::SensitivityResult;
    use crate::state::results::RidgeResult;
    let mut state = AppState::default();
    let result = SensitivityResult {
        param_names: vec!["x".into(), "y".into()],
        spearman: vec![vec![0.9, 0.3]],
        ridge: vec![RidgeResult {
            beta: vec![0.8, 0.2],
            r_squared: 0.95,
        }],
        rf_anova: None,
        mdi: None,
        shap: None,
        permutation: None,
        ard: None,
    };
    // Spearman is cache_id=0
    state.importance_cache.insert((0u8, 0, false), result);
    let widgets = WidgetStates::default(); // metric=Spearman, obj_idx=0
    let csv = build_importance_csv(&state, &widgets).unwrap();
    let header = csv.lines().next().unwrap();
    assert_eq!(header, "variable,importance_score,method");
    // 2 params → 2 data rows + header
    assert_eq!(csv.lines().count(), 3);
}

#[test]
fn importance_csv_quotes_param_name_with_comma() {
    use crate::state::app_state::SensitivityResult;
    use crate::state::results::RidgeResult;
    let mut state = AppState::default();
    let result = SensitivityResult {
        param_names: vec!["x,y".into()],
        spearman: vec![vec![0.9]],
        ridge: vec![RidgeResult {
            beta: vec![0.8],
            r_squared: 0.95,
        }],
        rf_anova: None,
        mdi: None,
        shap: None,
        permutation: None,
        ard: None,
    };
    state.importance_cache.insert((0u8, 0, false), result);
    let widgets = WidgetStates::default();
    let csv = build_importance_csv(&state, &widgets).unwrap();
    assert_eq!(csv.lines().nth(1).unwrap(), "\"x,y\",0.9,Spearman");
}

#[test]
fn importance_csv_guards_param_name_starting_with_equals() {
    use crate::state::app_state::SensitivityResult;
    use crate::state::results::RidgeResult;
    let mut state = AppState::default();
    let result = SensitivityResult {
        param_names: vec!["=SUM(A1)".into()],
        spearman: vec![vec![0.9]],
        ridge: vec![RidgeResult {
            beta: vec![0.8],
            r_squared: 0.95,
        }],
        rf_anova: None,
        mdi: None,
        shap: None,
        permutation: None,
        ard: None,
    };
    state.importance_cache.insert((0u8, 0, false), result);
    let widgets = WidgetStates::default();
    let csv = build_importance_csv(&state, &widgets).unwrap();
    assert_eq!(csv.lines().nth(1).unwrap(), "'=SUM(A1),0.9,Spearman");
}

#[test]
fn sensitivity_csv_has_objective_columns_in_header() {
    use crate::state::app_state::HeatmapMatrix;
    let widgets = WidgetStates::default(); // default metric = Spearman (id 0)
    let mut state = AppState::default();
    state.sensitivity_heatmap_cache.insert(
        (widgets.sensitivity_heatmap.metric.cache_id(), false),
        HeatmapMatrix {
            param_names: vec!["x".into(), "y".into()],
            objective_names: vec!["f1".into(), "f2".into()],
            values: vec![vec![0.9, 0.3], vec![0.5, 0.7]],
            signed: true,
        },
    );
    let csv = build_sensitivity_csv(&state, &widgets).unwrap();
    let header = csv.lines().next().unwrap();
    assert_eq!(header, "variable,f1,f2");
    assert_eq!(csv.lines().count(), 3); // header + 2 params
}

#[test]
fn sensitivity_csv_returns_none_when_no_result() {
    let state = AppState::default(); // sensitivity_heatmap_cache is empty
    let widgets = WidgetStates::default();
    assert!(build_sensitivity_csv(&state, &widgets).is_none());
}

#[test]
fn trial_based_csv_has_trial_id_header() {
    let mut state = AppState::default();
    let mut study = make_study(
        vec!["x".into()],
        vec!["f".into()],
        vec![Direction::Minimize],
    );
    let mut p = HashMap::new();
    p.insert("x".to_string(), 1.0_f64);
    study.set_rows_for_test(vec![make_trial(0, p, vec![0.5])]);
    state.current_study = Some(study);
    let csv = build_trial_based_csv(&state).unwrap();
    assert!(csv.lines().next().unwrap().contains("trial_id"));
}

#[test]
fn trial_based_csv_returns_none_when_no_study() {
    let state = AppState::default();
    assert!(build_trial_based_csv(&state).is_none());
}

#[test]
fn cluster_csv_returns_none_when_no_cluster_result() {
    let mut state = AppState::default();
    let widgets = WidgetStates::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
    state.current_study = Some(study);
    // no cluster result cached
    assert!(build_cluster_csv(&ChartId::ClusterScatter, &state, &widgets).is_none());
}

#[test]
fn cluster_csv_includes_cluster_id_column() {
    use crate::state::results::ClusterResult;
    let mut state = AppState::default();
    let widgets = WidgetStates::default();
    let mut study = make_study(
        vec!["x".into()],
        vec!["f".into()],
        vec![Direction::Minimize],
    );
    let mut p = HashMap::new();
    p.insert("x".to_string(), 1.0_f64);
    study.set_rows_for_test(vec![
        make_trial(0, p.clone(), vec![0.5]),
        make_trial(1, p.clone(), vec![1.0]),
    ]);
    state.current_study = Some(study);
    // Register the result in the cache under the 2D chart's settings key.
    let key = widgets.cluster_scatter.cache_key();
    state.cluster_cache.insert(
        key,
        ClusterResult {
            labels: vec![0, 1],
            n_clusters: 2,
        },
    );
    let csv = build_cluster_csv(&ChartId::ClusterScatter, &state, &widgets).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines[0].ends_with(",cluster_id"), "header: {}", lines[0]);
    assert!(lines[1].ends_with(",0"), "row0: {}", lines[1]);
    assert!(lines[2].ends_with(",1"), "row1: {}", lines[2]);
}

#[test]
fn pareto_csv_includes_all_trials_with_rank() {
    let mut state = AppState::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![
        make_trial_ranked(0, HashMap::new(), vec![1.0], 0),
        make_trial_ranked(1, HashMap::new(), vec![2.0], 1),
        make_trial_ranked(2, HashMap::new(), vec![3.0], 2),
    ]);
    study.pareto_indices = vec![0];
    state.current_study = Some(study);
    let csv = build_pareto_csv(&state).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    // Header + every trial, each tagged with its Pareto rank.
    assert_eq!(lines.len(), 4, "header + 3 rows: {:?}", lines);
    assert!(lines[0].contains("pareto_rank"));
    assert!(lines[1].ends_with(",0"), "row0 rank: {}", lines[1]);
    assert!(lines[2].ends_with(",1"), "row1 rank: {}", lines[2]);
    assert!(lines[3].ends_with(",2"), "row2 rank: {}", lines[3]);
}

#[test]
fn pareto_csv_uses_row_rank_not_trial_id() {
    // Regression: rank must be read per row, not by matching trial ids
    // against pareto_indices. With non-contiguous trial ids (100/200/300)
    // the per-row rank must still be emitted correctly for every row.
    let mut state = AppState::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![
        make_trial_ranked(100, HashMap::new(), vec![1.0], 0),
        make_trial_ranked(200, HashMap::new(), vec![2.0], 1),
        make_trial_ranked(300, HashMap::new(), vec![3.0], 2),
    ]);
    study.pareto_indices = vec![0];
    state.current_study = Some(study);
    let csv = build_pareto_csv(&state).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 4, "header + 3 rows: {:?}", lines);
    // First data row: trial id 100, trial.number 100 (= test trial_number), rank 0.
    assert!(lines[1].starts_with("100,100,"), "row: {}", lines[1]);
    assert!(lines[1].ends_with(",0"), "row0 rank: {}", lines[1]);
    assert!(lines[3].starts_with("300,300,"), "row: {}", lines[3]);
    assert!(lines[3].ends_with(",2"), "row2 rank: {}", lines[3]);
}

#[test]
fn pareto_csv_returns_none_when_no_pareto() {
    let mut state = AppState::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
    // pareto_indices is empty
    state.current_study = Some(study);
    assert!(build_pareto_csv(&state).is_none());
}

// ── TASK-2325: PDP tests ──────────────────────────────────────

#[test]
fn pdp_csv_returns_none_when_no_result() {
    let state = AppState::default();
    let widgets = WidgetStates::default();
    assert!(build_pdp_csv(&state, &widgets).is_none());
}

#[test]
fn pdp_csv_has_correct_header() {
    use crate::state::messages::PdpResult1d;
    let mut widgets = WidgetStates::default();
    widgets.pdp_chart.result = Some(PdpResult1d {
        x_values: vec![0.0, 1.0],
        y_values: vec![0.5, 0.8],
        y_upper: Some(vec![0.6, 0.9]),
        y_lower: Some(vec![0.4, 0.7]),
        ice_lines: vec![],
        r2: None,
        param_name: "x".to_string(),
    });
    let state = AppState::default();
    let csv = build_pdp_csv(&state, &widgets).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[0],
        "variable,variable_value,predicted_objective,lower_ci,upper_ci"
    );
    assert_eq!(lines.len(), 3); // header + 2 points
    assert_eq!(lines[1], "x,0,0.5,0.4,0.6");
}

#[test]
fn pdp_csv_handles_missing_ci() {
    use crate::state::messages::PdpResult1d;
    let mut widgets = WidgetStates::default();
    widgets.pdp_chart.result = Some(PdpResult1d {
        x_values: vec![0.0],
        y_values: vec![0.5],
        y_upper: None,
        y_lower: None,
        ice_lines: vec![],
        r2: None,
        param_name: "x".to_string(),
    });
    let state = AppState::default();
    let csv = build_pdp_csv(&state, &widgets).unwrap();
    // lower_ci and upper_ci should be empty strings
    assert_eq!(csv.lines().nth(1).unwrap(), "x,0,0.5,,");
}

#[test]
fn pdp_2d_csv_returns_none_when_no_result() {
    let state = AppState::default();
    let widgets = WidgetStates::default();
    assert!(build_pdp_2d_csv(&state, &widgets).is_none());
}

#[test]
fn pdp_2d_csv_has_correct_header_and_grid() {
    use crate::state::messages::PdpResult2d;
    let mut widgets = WidgetStates::default();
    widgets.pdp_2d.result = Some(PdpResult2d {
        x_values: vec![0.0, 1.0],
        y_values: vec![2.0, 3.0],
        z_values: vec![vec![0.1, 0.2], vec![0.3, 0.4]],
        param1_name: "x".to_string(),
        param2_name: "y".to_string(),
        objective_name: "f".to_string(),
        uncertainties: None,
    });
    let state = AppState::default();
    let csv = build_pdp_2d_csv(&state, &widgets).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[0],
        "param1_name,param1_value,param2_name,param2_value,predicted_objective"
    );
    // 2x2 grid → 4 data rows
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[1], "x,0,y,2,0.1");
    assert_eq!(lines[2], "x,0,y,3,0.2");
}

// ── TASK-2324: MCDM/AHP tests ─────────────────────────────────

fn make_topsis_mcdm(trial_rows_len: usize) -> crate::state::app_state::McdmResult {
    use crate::state::results::TopsisResult;
    McdmResult::Topsis(TopsisResult {
        scores: (0..trial_rows_len).map(|i| i as f64 * 0.1 + 0.5).collect(),
        ranked_indices: (0..trial_rows_len as u32).rev().collect(),
        duration_ms: 1.0,
    })
}

#[test]
fn mcdm_rank_csv_has_correct_header_and_method_topsis() {
    let mut state = AppState::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![
        make_trial(10, HashMap::new(), vec![1.0]),
        make_trial(11, HashMap::new(), vec![2.0]),
    ]);
    state.current_study = Some(study);
    let result = make_topsis_mcdm(2);
    let csv = build_mcdm_rank_csv(&result, &state).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "trial_id,rank,score,method");
    assert!(lines[1].ends_with(",TOPSIS"), "method column: {}", lines[1]);
    assert_eq!(lines.len(), 3); // header + 2 rows
}

#[test]
fn mcdm_rank_csv_returns_none_when_no_study() {
    let state = AppState::default();
    let result = make_topsis_mcdm(1);
    // None when there's no current_study
    assert!(build_mcdm_rank_csv(&result, &state).is_none());
}

#[test]
fn mcdm_scatter_csv_has_correct_header() {
    let mut state = AppState::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
    state.current_study = Some(study);
    let result = make_topsis_mcdm(1);
    let csv = build_mcdm_scatter_csv(&result, &state).unwrap();
    assert_eq!(csv.lines().next().unwrap(), "trial_id,rank,primary_score");
}

#[test]
fn mcdm_table_csv_topsis_header() {
    let mut state = AppState::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
    state.current_study = Some(study);
    let result = make_topsis_mcdm(1);
    let csv = build_mcdm_table_csv(&result, &state).unwrap();
    assert_eq!(csv.lines().next().unwrap(), "trial_id,rank,topsis_score");
}

#[test]
fn mcdm_table_csv_vikor_header() {
    use crate::state::results::VikorResult;
    let mut state = AppState::default();
    let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
    study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
    state.current_study = Some(study);
    let result = McdmResult::Vikor(VikorResult {
        s_values: vec![0.3],
        r_values: vec![0.2],
        q_values: vec![0.1],
        display_scores: vec![0.4],
        ranked_indices: vec![0],
        compromise_indices: vec![0],
        duration_ms: 1.0,
    });
    let csv = build_mcdm_table_csv(&result, &state).unwrap();
    assert_eq!(
        csv.lines().next().unwrap(),
        "trial_id,rank,s_value,r_value,q_value"
    );
}

#[test]
fn mcdm_table_csv_returns_none_when_no_study() {
    let state = AppState::default();
    let result = make_topsis_mcdm(1);
    assert!(build_mcdm_table_csv(&result, &state).is_none());
}

#[test]
fn slice_csv_includes_param_obj_and_pareto() {
    let mut state = AppState::default();
    let mut study = make_study(
        vec!["x".into()],
        vec!["f".into()],
        vec![Direction::Minimize],
    );
    let mut p = HashMap::new();
    p.insert("x".to_string(), 1.5_f64);
    study.set_rows_for_test(vec![make_trial(0, p, vec![0.5])]);
    study.pareto_indices = vec![0];
    state.current_study = Some(study);
    let widgets = WidgetStates::default();

    let csv = build_slice_csv(&state, &widgets).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "trial_id,x,f,is_pareto");
    assert_eq!(lines[1], "0,1.5,0.5,true");
}

// ── Multi-objective surrogate optimization CSV tests ──────────────────────

fn make_multi_opt_result() -> crate::state::messages::SurrogateMultiOptUiResult {
    use tunny_core::surrogate_opt::ParetoFrontPoint;
    crate::state::messages::SurrogateMultiOptUiResult {
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_names: vec!["f0".to_string(), "f1".to_string()],
        front: vec![
            ParetoFrontPoint {
                params: vec![0.1, 0.2],
                values: vec![1.0, 4.0],
            },
            ParetoFrontPoint {
                params: vec![0.3, 0.4],
                values: vec![2.0, 3.0],
            },
        ],
        r_squared: vec![0.9, 0.85],
    }
}

#[test]
fn multi_opt_csv_header_is_objectives_then_params() {
    let result = make_multi_opt_result();
    let csv = build_surrogate_multi_opt_csv(&result);
    let header = csv.lines().next().unwrap();
    assert_eq!(header, "f0,f1,x,y");
}

#[test]
fn multi_opt_csv_row_count_equals_front_size() {
    let result = make_multi_opt_result();
    let csv = build_surrogate_multi_opt_csv(&result);
    // 1 header row + 2 front-point rows = 3 rows total
    assert_eq!(csv.lines().count(), 3);
}

#[test]
fn has_csv_data_true_when_multi_result_present() {
    let mut widgets = WidgetStates::default();
    let state = AppState::default();
    widgets.surrogate_opt.multi_result = Some(make_multi_opt_result());
    assert!(has_csv_data(&ChartId::SurrogateOpt, &state, &widgets));
}

#[test]
fn build_surrogate_opt_csv_prefers_multi_result() {
    let mut widgets = WidgetStates::default();
    widgets.surrogate_opt.multi_result = Some(make_multi_opt_result());
    // Also set a single-objective result (to verify multi-objective takes
    // priority).
    widgets.surrogate_opt.result = Some(crate::state::messages::SurrogateOptUiResult {
        best_params: vec![("x".to_string(), 0.5)],
        best_value: 1.0,
        predicted_std: None,
        r_squared: 0.9,
        objective_name: "f".to_string(),
        minimize: true,
        best_observed_value: 1.5,
        predicted_constraints: vec![],
        feasibility_probability: None,
    });
    let state = AppState::default();
    let csv = build_chart_csv(&ChartId::SurrogateOpt, &state, &widgets).unwrap();
    // The multi-objective CSV's header includes the objective names
    let header = csv.lines().next().unwrap();
    assert!(
        header.contains("f0") && header.contains("f1"),
        "header: {}",
        header
    );
}
