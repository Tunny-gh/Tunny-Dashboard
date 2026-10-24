use super::*;
use crate::state::results::{PrometheeResult, TopsisResult, VikorResult};
use crate::theme::chart_colors::COLOR_MCDM_NONE;
use std::collections::HashMap;
use std::sync::Arc;
use tdashboard_analysis::dataframe::{DataFrame, TrialRow as CoreRow};

// ── Test helpers ──────────────────────────────────────────

fn make_view_with_objectives(objective_rows: &[Vec<f64>]) -> (StudyView, Vec<String>) {
    let n = objective_rows.len();
    if n == 0 {
        let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        return (StudyView::new(Arc::new(df), vec![]), vec![]);
    }
    let n_obj = objective_rows[0].len();
    let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: objective_rows[i].clone(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
    (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
}

fn make_empty_view() -> StudyView {
    let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
    StudyView::new(Arc::new(df), vec![])
}

fn make_vikor(n: usize) -> VikorResult {
    let values: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    VikorResult {
        s_values: values.clone(),
        r_values: values.clone(),
        q_values: values.clone(),
        display_scores: values.iter().map(|v| 1.0 - v).collect(),
        ranked_indices: (0..n as u32).collect(),
        compromise_indices: if n > 0 { vec![0] } else { vec![] },
        duration_ms: 1.0,
    }
}

fn make_vikor_result(n: usize) -> McdmResult {
    McdmResult::Vikor(make_vikor(n))
}

fn make_topsis(n: usize) -> TopsisResult {
    TopsisResult {
        scores: (0..n).map(|i| i as f64 / n as f64).collect(),
        ranked_indices: (0..n as u32).rev().collect(),
        duration_ms: 1.0,
    }
}

fn make_promethee(n: usize) -> PrometheeResult {
    let v: Vec<f64> = (0..n).map(|i| i as f64 * 0.05).collect();
    PrometheeResult {
        phi_plus: v.clone(),
        phi_minus: v.iter().map(|x| 1.0 - x).collect(),
        phi_net: v.clone(),
        ranked_indices_i: (0..n as u32).collect(),
        ranked_indices_ii: (0..n as u32).collect(),
        incomparable_counts: vec![0; n],
        duration_ms: 1.0,
    }
}

// ── Struct / initialization tests ─────────────────────────────────────

#[test]
fn test_scatter_chart_default_values() {
    let chart = McdmScatterChart::default();
    assert_eq!(chart.x_axis, "Objective0");
    assert_eq!(chart.y_axis, "Objective1");
    assert!(chart.display_batches.is_none());
    assert!(chart.cache_key.is_none());
    assert!(chart.error_message.is_none());
}

#[test]
fn test_cache_not_stale_same_key() {
    use crate::state::types::ColormapName;
    let cmap_name = ColormapName::Viridis;
    let mut chart = McdmScatterChart::default();
    let result = crate::state::analysis_job::fixture_result(McdmResult::Topsis(make_topsis(100)));
    assert!(chart.is_cache_stale(100, &result, &cmap_name, 10));
    chart.cache_key = Some(chart.make_cache_key(100, &result, &cmap_name, 10));
    assert!(!chart.is_cache_stale(100, &result, &cmap_name, 10));
    assert!(chart.is_cache_stale(150, &result, &cmap_name, 10));
}

pub(crate) fn assert_accepted_score_refresh(
    results: [&SharedResult<McdmResult>; 2],
    view: &StudyView,
    names: &[String],
    axis: &str,
    scores: [&[f64]; 2],
) {
    use crate::ui::widgets::trial_detail_modal::hit_test_nearest;
    let mut chart = McdmScatterChart {
        x_axis: axis.into(),
        y_axis: "Objective0".into(),
        ..Default::default()
    };
    chart.controls.method = results[0].method();
    chart.controls.weights = vec![0.7, 0.3];
    chart.controls.v_param = 0.37;
    chart.controls.top_n = serde_json::from_value(serde_json::json!("Top5")).unwrap();
    let controls = serde_json::to_value(&chart.controls).unwrap();
    let ctx = egui::Context::default();
    let cmap = ColorMap::viridis();
    let mut colors = Vec::new();
    for (step, result) in results.into_iter().enumerate() {
        let _ = ctx.run_ui(Default::default(), |ui| {
            chart.show(
                ui,
                Some(result),
                view,
                &[],
                names,
                &cmap,
                &crate::state::types::ColormapName::Viridis,
                &HashMap::new(),
                &[],
            );
        });
        let mut points = chart.display_batches.as_ref().unwrap().points();
        assert_eq!(points.len(), scores[step].len());
        points.sort_by_key(|p| p.0);
        for &(id, xy, _) in &points {
            let row = view.trial_ids.iter().position(|&v| v == id).unwrap();
            assert_eq!(
                xy,
                [
                    scores[step][row],
                    view.numeric_column(&names[0]).unwrap()[row]
                ]
            );
        }
        let current_colors: Vec<_> = points.iter().map(|p| (p.0, p.2)).collect();
        if step == 0 {
            colors = current_colors;
        } else {
            assert_eq!(colors, current_colors);
        }
        let candidates = chart.hit_candidates.as_ref().unwrap();
        assert_eq!(candidates.len(), scores[step].len());
        for &(id, row, xy) in candidates {
            assert_eq!(xy[0], scores[step][row]);
            assert_eq!(id, view.trial_ids[row]);
        }
        let _ = ctx.run_ui(Default::default(), |ui| {
            egui_plot::Plot::new("picking regression").show(ui, |plot_ui| {
                for &(id, row, xy) in candidates {
                    let pos = plot_ui
                        .transform()
                        .position_from_point(&egui_plot::PlotPoint::new(xy[0], xy[1]));
                    assert_eq!(
                        hit_test_nearest(plot_ui, candidates, pos, 1.0),
                        Some((id, row))
                    );
                }
            });
        });
        let pointer = chart.hit_candidates.as_ref().unwrap().as_ptr();
        let elapsed = chart.metadata.as_ref().unwrap().compute_time_ms;
        let _ = ctx.run_ui(Default::default(), |ui| {
            chart.show(
                ui,
                Some(result),
                view,
                &[],
                names,
                &cmap,
                &crate::state::types::ColormapName::Viridis,
                &HashMap::new(),
                &[],
            );
        });
        assert_eq!(chart.hit_candidates.as_ref().unwrap().as_ptr(), pointer);
        assert_eq!(chart.metadata.as_ref().unwrap().compute_time_ms, elapsed);
        assert_eq!(chart.x_axis, axis);
        assert_eq!(chart.y_axis, "Objective0");
        assert_eq!(serde_json::to_value(&chart.controls).unwrap(), controls);
    }
}

#[test]
fn accepted_non_primary_scores_refresh_both_scatter_caches() {
    let (view, names) = make_view_with_objectives(&[
        vec![1.0, 8.0],
        vec![2.0, 6.0],
        vec![4.0, 3.0],
        vec![8.0, 1.0],
    ]);
    for axis in ["VIKOR_R", "Phi-"] {
        let scores = [vec![0.1, 0.3, 0.5, 0.9], vec![0.2, 0.4, 0.8, 1.2]];
        let results: Vec<_> = scores
            .iter()
            .map(|values| {
                let result = if axis == "VIKOR_R" {
                    let mut r = make_vikor(4);
                    r.r_values = values.clone();
                    McdmResult::Vikor(r)
                } else {
                    let mut r = make_promethee(4);
                    r.phi_minus = values.clone();
                    McdmResult::PrometheeII(r)
                };
                crate::state::analysis_job::fixture_result(result)
            })
            .collect();
        assert_eq!(results[0].primary_scores(), results[1].primary_scores());
        assert_accepted_score_refresh(
            [&results[0], &results[1]],
            &view,
            &names,
            axis,
            [&scores[0], &scores[1]],
        );
        crate::ui::widgets::mcdm_scatter_chart_3d::tests::assert_accepted_score_refresh(
            [&results[0], &results[1]],
            &view,
            &names,
            axis,
            [&scores[0], &scores[1]],
        );
    }
}

// ── get_axis_options tests ──────────────────────────────────

#[test]
fn test_axis_options_vikor_has_scores() {
    let result = McdmResult::Vikor(make_vikor(3));
    let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
    let options = get_axis_options(&result, &obj_names);

    assert!(options.iter().any(|o| o.id == "Objective0"));
    assert!(options.iter().any(|o| o.id == "Objective1"));
    assert!(options.iter().any(|o| o.id == "VIKOR_Q"));
    assert!(options.iter().any(|o| o.id == "VIKOR_S"));
    assert!(options.iter().any(|o| o.id == "VIKOR_R"));
}

#[test]
fn test_axis_options_topsis() {
    let result = McdmResult::Topsis(make_topsis(3));
    let options = get_axis_options(&result, &["obj".to_string()]);
    assert!(options.iter().any(|o| o.id == "TOPSIS_Score"));
    assert!(!options.iter().any(|o| o.id == "VIKOR_Q"));
    let options = get_axis_options(&result, &[]);
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].id, "TOPSIS_Score");
}

#[test]
fn test_axis_options_promethee() {
    let result = McdmResult::PrometheeI(make_promethee(3));
    let options = get_axis_options(&result, &[]);
    assert!(options.iter().any(|o| o.id == "Phi+"));
    assert!(options.iter().any(|o| o.id == "Phi-"));
    assert!(options.iter().any(|o| o.id == "Phi_Net"));
}

// ── extract_axis_values tests ────────────────────────────────

#[test]
fn test_extract_objective0() {
    let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
    let result = McdmResult::Vikor(make_vikor(2));
    let vals = extract_axis_values("Objective0", &result, &view, &obj_names).unwrap();
    assert_eq!(vals, vec![1.0, 3.0]);
    let vals = extract_axis_values("Objective1", &result, &view, &obj_names).unwrap();
    assert_eq!(vals, vec![2.0, 4.0]);
    let (view, obj_names) = make_view_with_objectives(&[vec![1.0]]);
    let result = McdmResult::Vikor(make_vikor(1));
    assert!(extract_axis_values("Objective5", &result, &view, &obj_names).is_err());
}

#[test]
fn test_extract_vikor_q() {
    let vikor = make_vikor(3);
    let q = vikor.q_values.clone();
    let result = McdmResult::Vikor(vikor);
    let view = make_empty_view();
    let vals = extract_axis_values("VIKOR_Q", &result, &view, &[]).unwrap();
    assert_eq!(vals, q);
    assert!(extract_axis_values("NonExistent", &result, &view, &[]).is_err());
}

#[test]
fn test_extract_topsis_score() {
    let topsis = make_topsis(3);
    let scores = topsis.scores.clone();
    let result = McdmResult::Topsis(topsis);
    let view = make_empty_view();
    let vals = extract_axis_values("TOPSIS_Score", &result, &view, &[]).unwrap();
    assert_eq!(vals, scores);
}

#[test]
fn test_extract_phi_plus() {
    let promethee = make_promethee(3);
    let phi_plus = promethee.phi_plus.clone();
    let result = McdmResult::PrometheeI(promethee);
    let view = make_empty_view();
    let vals = extract_axis_values("Phi+", &result, &view, &[]).unwrap();
    assert_eq!(vals, phi_plus);
}

// ── build_rank_map tests ────────────────────────────────────

#[test]
fn test_build_rank_map_basic() {
    let ranked: Vec<u32> = vec![5, 2, 8];
    let map = build_rank_map(&ranked, 10);
    assert_eq!(map[5], 0);
    assert_eq!(map[2], 1);
    assert_eq!(map[8], 2);
    assert_eq!(map[0], usize::MAX); // outside the ranking
    assert_eq!(map[3], usize::MAX);
    let n = 5usize;
    let ranked: Vec<u32> = vec![4, 3, 2, 1, 0];
    let map = build_rank_map(&ranked, n);
    assert_eq!(map[4], 0); // trial 4 is rank 0 (best)
    assert_eq!(map[0], 4); // trial 0 is rank 4 (worst)
}

// ── compute_scatter_points integration tests ─────────────────────────

#[test]
fn test_compute_scatter_points_basic() {
    use crate::state::types::ColormapName;
    use crate::theme::colormap_name::colormap_from_name;
    let n = 10;
    let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (n - i) as f64]).collect();
    let (view, obj_names) = make_view_with_objectives(&data);
    let result = make_vikor_result(n);
    let cmap = colormap_from_name(&ColormapName::Viridis);

    let (points, _, meta) = compute_scatter_points(
        &result,
        &view,
        &obj_names,
        "Objective0",
        "Objective1",
        &cmap,
        n,
    )
    .unwrap();

    assert_eq!(points.len(), n);
    assert_eq!(meta.total_trials, n);
    assert!((points[0].0 - 0.0).abs() < 1e-10);
    assert!((points[0].1 - 10.0).abs() < 1e-10);
}

#[test]
fn test_compute_scatter_points_rank0_gets_best_color() {
    use crate::state::types::ColormapName;
    use crate::theme::colormap_name::colormap_from_name;
    let n = 20;
    let top_n = 10_usize;
    let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, i as f64]).collect();
    let (view, obj_names) = make_view_with_objectives(&data);
    let result = make_vikor_result(n);
    let cmap = colormap_from_name(&ColormapName::Viridis);

    let (points, _, _) = compute_scatter_points(
        &result,
        &view,
        &obj_names,
        "Objective0",
        "Objective1",
        &cmap,
        top_n,
    )
    .unwrap();

    // rank 0 (best) -> t=1.0 -> top end of the colormap
    let expected = cmap.interpolate(1.0);
    assert_eq!(points[0].2, expected);
    // Outside top_n (rank >= top_n) is gray
    assert_eq!(points[n - 1].2, COLOR_MCDM_NONE());
}

#[test]
fn test_compute_scatter_points_empty_trials() {
    use crate::state::types::ColormapName;
    use crate::theme::colormap_name::colormap_from_name;
    let vikor = make_vikor(0);
    let result = McdmResult::Vikor(vikor);
    let view = make_empty_view();
    let cmap = colormap_from_name(&ColormapName::Viridis);

    let (points, _, meta) =
        compute_scatter_points(&result, &view, &[], "Objective0", "Objective1", &cmap, 10).unwrap();
    assert!(points.is_empty());
    assert_eq!(meta.total_trials, 0);
}

#[test]
fn test_compute_scatter_points_vikor_axis() {
    use crate::state::types::ColormapName;
    use crate::theme::colormap_name::colormap_from_name;
    let n = 5;
    let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
    let (view, obj_names) = make_view_with_objectives(&data);
    let result = make_vikor_result(n);
    let cmap = colormap_from_name(&ColormapName::Viridis);

    let (points, _, _) =
        compute_scatter_points(&result, &view, &obj_names, "VIKOR_Q", "VIKOR_S", &cmap, n).unwrap();

    assert_eq!(points.len(), n);
}
