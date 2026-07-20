use super::compute::normalize_weights;
use super::controls::McdmTopN;
use super::ranking::build_ranking_rows;
use super::*;
use crate::state::results::TopsisResult;
use std::collections::HashMap;
use std::sync::Arc;
use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

#[test]
fn adopt_compute_state_syncs_runtime_and_preserves_ui_settings() {
    let mut item = McdmRankChart {
        controls: McdmControls {
            computing: true,
            method: McdmMethod::Vikor,
            top_n: McdmTopN::Top20,
            v_param: 0.7,
            ..Default::default()
        },
    };
    let global = McdmRankChart {
        controls: McdmControls {
            computing: false,
            weights: vec![0.25, 0.75],
            ..Default::default()
        },
    };

    item.adopt_compute_state(&global);

    // Execution state and shared output are adopted.
    assert!(!item.controls.computing);
    assert_eq!(item.controls.weights, vec![0.25, 0.75]);
    // UI settings remain item-specific.
    assert_eq!(item.controls.method, McdmMethod::Vikor);
    assert_eq!(item.controls.top_n, McdmTopN::Top20);
    assert_eq!(item.controls.v_param, 0.7);
}

fn make_simple_view(n: usize) -> StudyView {
    if n == 0 {
        let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        return StudyView::new(Arc::new(df), vec![]);
    }
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &[], &[], &[], 0);
    StudyView::new(Arc::new(df), vec![0; n])
}

fn make_view_with_objectives(objective_rows: &[Vec<f64>]) -> (StudyView, Vec<String>) {
    let n = objective_rows.len();
    if n == 0 {
        return (make_simple_view(0), vec![]);
    }
    let n_obj = objective_rows[0].len();
    let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: objective_rows[i].clone(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
    (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
}

fn make_topsis_result(scores: Vec<f64>, ranked_indices: Vec<u32>) -> McdmResult {
    McdmResult::Topsis(TopsisResult {
        scores,
        ranked_indices,
        duration_ms: 10.0,
    })
}

#[test]
fn mcdm_top_n_values() {
    assert_eq!(McdmTopN::Top5.value(), 5);
    assert_eq!(McdmTopN::Top10.value(), 10);
    assert_eq!(McdmTopN::Top20.value(), 20);
}

#[test]
fn normalize_weights_equal() {
    let result = normalize_weights(&[0.5, 0.5]);
    assert!((result[0] - 0.5).abs() < 1e-9);
    assert!((result[1] - 0.5).abs() < 1e-9);
}

#[test]
fn normalize_weights_unequal() {
    let result = normalize_weights(&[1.0, 3.0]);
    assert!((result[0] - 0.25).abs() < 1e-9);
    assert!((result[1] - 0.75).abs() < 1e-9);
}

#[test]
fn normalize_weights_three_equal() {
    let result = normalize_weights(&[2.0, 2.0, 2.0]);
    for w in &result {
        assert!((w - 1.0 / 3.0).abs() < 1e-9);
    }
}

#[test]
fn normalize_weights_zero_fallback() {
    let result = normalize_weights(&[0.0, 0.0]);
    assert!((result[0] - 0.5).abs() < 1e-9);
    assert!((result[1] - 0.5).abs() < 1e-9);
}

#[test]
fn normalize_weights_empty() {
    let result = normalize_weights(&[]);
    assert!(result.is_empty());
}

#[test]
fn mcdm_rank_chart_default() {
    let chart = McdmRankChart::default();
    let c = &chart.controls;
    assert_eq!(c.method, McdmMethod::Topsis);
    assert_eq!(c.weight_mode, WeightMode::Manual);
    assert!(!c.computing);
    assert!(c.pending_compute.is_none());
    assert!(!c.pending_entropy);
    assert!(c.entropy_result.is_none());
    assert_eq!(c.top_n, McdmTopN::Top10);
    assert!(c.weights.is_empty());
    assert!((c.v_param - 0.5).abs() < f64::EPSILON);
}

#[test]
fn mcdm_table_default() {
    let table = McdmTable::default();
    assert_eq!(table.controls.top_n, McdmTopN::Top10);
}

#[test]
fn enumerate_ranked_top5_with_5_results() {
    let result = make_topsis_result(vec![0.9, 0.7, 0.5, 0.3, 0.1], vec![0, 1, 2, 3, 4]);
    let view = make_simple_view(5);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 5);
    assert!((ranking[0].score - 0.9).abs() < 1e-9);
    assert!((ranking[4].score - 0.1).abs() < 1e-9);
}

#[test]
fn enumerate_ranked_top10_with_20_results() {
    let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
    let ranked: Vec<u32> = (0..20).collect();
    let result = make_topsis_result(scores, ranked);
    let view = make_simple_view(20);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
    assert_eq!(ranking.len(), 10);
}

#[test]
fn enumerate_ranked_top5_with_3_results_min_applied() {
    let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
    let view = make_simple_view(3);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 3);
}

#[test]
fn enumerate_ranked_scores_match_ranked_order() {
    let result = make_topsis_result(vec![0.1, 0.9, 0.5], vec![1, 2, 0]);
    let view = make_simple_view(3);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
    assert_eq!(ranking.len(), 3);
    assert!((ranking[0].score - 0.9).abs() < 1e-9);
    assert!((ranking[1].score - 0.5).abs() < 1e-9);
    assert!((ranking[2].score - 0.1).abs() < 1e-9);
}

#[test]
fn enumerate_ranked_empty_result() {
    let result = make_topsis_result(vec![], vec![]);
    let view = make_simple_view(0);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert!(ranking.is_empty());
}

#[test]
fn top_n_toggle_cycle() {
    let mut chart = McdmRankChart::default();
    assert_eq!(chart.controls.top_n, McdmTopN::Top10);
    chart.controls.top_n = McdmTopN::Top5;
    assert_eq!(chart.controls.top_n.value(), 5);
    chart.controls.top_n = McdmTopN::Top20;
    assert_eq!(chart.controls.top_n.value(), 20);
}

#[test]
fn build_ranking_rows_basic() {
    let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
    let view = make_simple_view(3);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 3);
    assert_eq!(ranking[0].rank, 1);
    assert_eq!(ranking[0].trial_number, 0);
    assert!((ranking[0].score - 0.9).abs() < 1e-9);
}

#[test]
fn build_ranking_rows_top_n_limit() {
    let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
    let ranked: Vec<u32> = (0..20).collect();
    let result = make_topsis_result(scores, ranked);
    let view = make_simple_view(20);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 5);
}

#[test]
fn build_ranking_rows_rank_starts_at_1() {
    let result = make_topsis_result(vec![0.8], vec![0]);
    let view = make_simple_view(1);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking[0].rank, 1);
}

#[test]
fn build_ranking_rows_distinguishes_trial_id_and_number() {
    // Verify both are resolved correctly for a Study where trial_id (global, used
    // for pinning) and trial.number (for display) diverge (e.g. when it includes
    // pruned/failed trials).
    let core_rows: Vec<CoreRow> = (0..3)
        .map(|i| CoreRow {
            trial_id: i as u32 + 10,
            trial_number: i as u32 + 100,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &[], &[], &[], 0);
    let view = StudyView::new(Arc::new(df), vec![0; 3]);

    let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![2, 0, 1]);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    // rank 1 is trial_idx 2 -> trial_id 12 / number 102
    assert_eq!(ranking[0].trial_id, 12);
    assert_eq!(ranking[0].trial_number, 102);
}

#[test]
fn build_ranking_rows_empty() {
    let result = make_topsis_result(vec![], vec![]);
    let view = make_simple_view(0);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert!(ranking.is_empty());
}

#[test]
fn build_ranking_rows_objectives_included() {
    let result = make_topsis_result(vec![0.9, 0.5], vec![0, 1]);
    let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
    let ranking = build_ranking_rows(&result, &view, &[], &obj_names, 10);
    assert_eq!(ranking[0].objectives, vec![1.0, 2.0]);
    assert_eq!(ranking[1].objectives, vec![3.0, 4.0]);
}

// ── E2E / integration tests ──

fn multi_obj_data() -> Vec<Vec<f64>> {
    vec![
        vec![0.1, 0.9],
        vec![0.5, 0.5],
        vec![0.9, 0.1],
        vec![0.3, 0.7],
        vec![0.7, 0.3],
    ]
}

#[test]
fn topsis_full_pipeline_equal_weights() {
    let data = multi_obj_data();
    let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
    let weights = normalize_weights(&[1.0, 1.0]);
    let is_minimize = vec![true, true];

    let core_result =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();

    let mcdm_result = McdmResult::Topsis(TopsisResult {
        scores: core_result.scores.clone(),
        ranked_indices: core_result.ranked_indices.clone(),
        duration_ms: core_result.duration_ms,
    });

    assert_eq!(mcdm_result.primary_scores().len(), 5);
    assert!(!mcdm_result.primary_scores().iter().any(|s| s.is_nan()));

    let (view, obj_names) = make_view_with_objectives(&data);
    let ranking = build_ranking_rows(&mcdm_result, &view, &[], &obj_names, 5);
    assert_eq!(ranking.len(), 5);
    assert_eq!(ranking[0].rank, 1);
    for i in 1..ranking.len() {
        assert!(ranking[i - 1].score >= ranking[i].score);
    }
}

#[test]
fn topsis_weight_bias_changes_ranking() {
    let data = multi_obj_data();
    let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
    let is_minimize = vec![true, true];

    let weights_obj0 = normalize_weights(&[1.0, 0.0]);
    let r0 =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj0, &is_minimize).unwrap();

    let weights_obj1 = normalize_weights(&[0.0, 1.0]);
    let r1 =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj1, &is_minimize).unwrap();

    assert_ne!(
        r0.ranked_indices, r1.ranked_indices,
        "different weights should produce different rankings"
    );
}

#[test]
fn topsis_single_objective_works() {
    let objectives: Vec<f64> = (0..5).map(|i| i as f64 * 0.2).collect();
    let weights = normalize_weights(&[1.0]);
    let is_minimize = vec![true];

    let result = tunny_core::topsis::compute_topsis(&objectives, 5, 1, &weights, &is_minimize);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.scores.len(), 5);
}

#[test]
fn mcdm_chart_run_button_sets_pending_compute() {
    let mut chart = McdmRankChart::default();
    assert!(chart.controls.pending_compute.is_none());
    assert!(!chart.controls.computing);

    let normalized = normalize_weights(&[1.0, 1.0]);
    chart.controls.pending_compute = Some(McdmComputeRequest {
        method: McdmMethod::Topsis,
        weights: normalized,
        v: 0.5,
    });
    chart.controls.computing = true;

    assert!(chart.controls.pending_compute.is_some());
    assert!(chart.controls.computing);

    let payload = chart.controls.pending_compute.take();
    assert!(payload.is_some());
    assert!(chart.controls.pending_compute.is_none());
    assert!(chart.controls.computing);
}

#[test]
fn mcdm_compute_request_vikor_includes_v() {
    let req = McdmComputeRequest {
        method: McdmMethod::Vikor,
        weights: vec![0.5, 0.5],
        v: 0.3,
    };
    assert_eq!(req.method, McdmMethod::Vikor);
    assert!((req.v - 0.3).abs() < f64::EPSILON);
}

#[test]
fn top_n_toggle_updates_display() {
    let data = multi_obj_data();
    let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
    let weights = normalize_weights(&[1.0, 1.0]);
    let is_minimize = vec![true, true];

    let core_result =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();
    let mcdm = McdmResult::Topsis(TopsisResult {
        scores: core_result.scores,
        ranked_indices: core_result.ranked_indices,
        duration_ms: core_result.duration_ms,
    });

    let (view, obj_names) = make_view_with_objectives(&data);

    let rows5 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 5);
    assert_eq!(rows5.len(), 5);

    let rows3 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 3);
    assert_eq!(rows3.len(), 3);

    let rows10 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 10);
    assert_eq!(rows10.len(), 5);
}
