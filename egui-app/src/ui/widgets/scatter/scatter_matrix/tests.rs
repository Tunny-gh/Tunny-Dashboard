use super::draw::data_to_screen;
use super::stats::{
    compute_correlation, compute_histogram, downsample_indices_to_cap, resolve_color_objective,
    split_feasibility_indices,
};
use super::*;

// ── resolve_color_objective ──────────────────────────────────────

#[test]
fn resolve_color_objective_none_returns_first() {
    let names = vec!["obj0".to_string(), "obj1".to_string()];
    assert_eq!(resolve_color_objective(&None, &names), Some("obj0"));
}

#[test]
fn resolve_color_objective_existing_name_returns_it() {
    let names = vec!["obj0".to_string(), "obj1".to_string()];
    assert_eq!(
        resolve_color_objective(&Some("obj1".to_string()), &names),
        Some("obj1")
    );
}

#[test]
fn resolve_color_objective_unknown_name_falls_back_to_first() {
    let names = vec!["obj0".to_string(), "obj1".to_string()];
    assert_eq!(
        resolve_color_objective(&Some("unknown".to_string()), &names),
        Some("obj0")
    );
}

#[test]
fn resolve_color_objective_empty_names_returns_none() {
    assert_eq!(resolve_color_objective(&None, &[]), None);
    assert_eq!(
        resolve_color_objective(&Some("obj0".to_string()), &[]),
        None
    );
}

// ── constraint-aware visualization (TASK-2350) ──────────────────

#[test]
fn tc_cav_scatter_matrix_show_infeasible_default_true() {
    let sm = ScatterMatrix::default();
    assert!(sm.show_infeasible);
}

#[test]
fn tc_cav_split_feasibility_no_constraints_all_feasible() {
    use tunny_core::dataframe::Feasibility;
    let feas = Feasibility::from_column(None);
    let (f, inf) = split_feasibility_indices(3, feas);
    assert_eq!(f, vec![0, 1, 2]);
    assert!(inf.is_empty());
}

#[test]
fn tc_cav_split_feasibility_mixed() {
    use tunny_core::dataframe::Feasibility;
    let col = vec![1.0_f64, 0.0, 1.0];
    let feas = Feasibility::from_column(Some(&col));
    let (f, inf) = split_feasibility_indices(3, feas);
    assert_eq!(f, vec![0, 2]);
    assert_eq!(inf, vec![1]);
}

#[test]
fn tc_cav_split_feasibility_all_infeasible() {
    use tunny_core::dataframe::Feasibility;
    let col = vec![0.0_f64, 0.0];
    let feas = Feasibility::from_column(Some(&col));
    let (f, inf) = split_feasibility_indices(2, feas);
    assert!(f.is_empty());
    assert_eq!(inf, vec![0, 1]);
}

// TASK-2019 tests

#[test]
fn scatter_matrix_default_mode() {
    let sm = ScatterMatrix::default();
    assert_eq!(sm.mode, MatrixMode::ParamsVsParams);
    assert_eq!(sm.sort, AxisSort::Alphabetical);
    assert!(sm.selected_cell.is_none());
}

#[test]
fn downsample_cap_keeps_all_when_under_cap() {
    let idx: Vec<u32> = (0..100).collect();
    let out = downsample_indices_to_cap(&idx, 4000);
    assert_eq!(out, idx);
}

#[test]
fn downsample_cap_limits_when_over_cap() {
    let idx: Vec<u32> = (0..100_000).collect();
    let out = downsample_indices_to_cap(&idx, 4000);
    assert!(out.len() <= 4000, "got {}", out.len());
    assert!(!out.is_empty());
    // The first element is preserved, and downsampling keeps ascending order.
    assert_eq!(out[0], 0);
    assert!(out.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn downsample_cap_zero_is_empty() {
    let idx: Vec<u32> = (0..10).collect();
    assert!(downsample_indices_to_cap(&idx, 0).is_empty());
}

#[test]
fn compute_histogram_bins_count() {
    let data = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let bins = compute_histogram(&data, 5);
    assert_eq!(bins.len(), 5);
    let total: usize = bins.iter().sum();
    assert_eq!(total, data.len());
}

#[test]
fn compute_histogram_all_in_same_bin() {
    let data = vec![5.0; 10];
    let bins = compute_histogram(&data, 4);
    let total: usize = bins.iter().sum();
    assert_eq!(total, 10);
}

#[test]
fn compute_histogram_empty_data() {
    let bins = compute_histogram(&[], 5);
    assert_eq!(bins.len(), 5);
    assert!(bins.iter().all(|&b| b == 0));
}

#[test]
fn compute_correlation_perfect_positive() {
    let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let y = x.clone();
    let corr = compute_correlation(&x, &y);
    assert!((corr - 1.0).abs() < 1e-9);
}

#[test]
fn compute_correlation_perfect_negative() {
    let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&v| -v).collect();
    let corr = compute_correlation(&x, &y);
    assert!((corr + 1.0).abs() < 1e-9);
}

#[test]
fn compute_correlation_range_bounded() {
    let x = vec![1.0, 3.0, 5.0, 7.0, 9.0];
    let y = vec![2.0, 1.0, 4.0, 3.0, 5.0];
    let corr = compute_correlation(&x, &y);
    assert!((-1.0..=1.0).contains(&corr));
}

#[test]
fn data_to_screen_min_maps_to_left_bottom() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
    let pos = data_to_screen(0.0, 0.0, (0.0, 1.0), (0.0, 1.0), rect);
    assert!((pos.x - 0.0).abs() < 1e-3);
    assert!((pos.y - 100.0).abs() < 1e-3); // y is inverted
}

#[test]
fn data_to_screen_max_maps_to_right_top() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
    let pos = data_to_screen(1.0, 1.0, (0.0, 1.0), (0.0, 1.0), rect);
    assert!((pos.x - 100.0).abs() < 1e-3);
    assert!((pos.y - 0.0).abs() < 1e-3); // y is inverted
}
