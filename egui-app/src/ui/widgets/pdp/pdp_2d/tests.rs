use super::math::{check_params_different, normalize_value};
use super::mesh::{push_edge, push_tri, surface_quads};
use super::*;

#[test]
fn normalize_value_midpoint() {
    let t = normalize_value(0.5, 0.0, 1.0);
    assert!((t - 0.5).abs() < 1e-5);
}

#[test]
fn normalize_value_clamps_below_zero() {
    let t = normalize_value(-1.0, 0.0, 1.0);
    assert_eq!(t, 0.0);
}

#[test]
fn normalize_value_clamps_above_one() {
    let t = normalize_value(2.0, 0.0, 1.0);
    assert_eq!(t, 1.0);
}

#[test]
fn normalize_value_equal_range_returns_half() {
    let t = normalize_value(5.0, 5.0, 5.0);
    assert_eq!(t, 0.5);
}

#[test]
fn value_range_of_correct() {
    let grid = vec![vec![1.0, 3.0], vec![2.0, 0.5]];
    let (v_min, v_max) = value_range_of(&grid);
    assert!((v_min - 0.5).abs() < 1e-9);
    assert!((v_max - 3.0).abs() < 1e-9);
}

#[test]
fn value_range_of_empty_returns_default() {
    let grid: Vec<Vec<f64>> = vec![];
    let (v_min, v_max) = value_range_of(&grid);
    assert_eq!(v_min, 0.0);
    assert_eq!(v_max, 1.0);
}

#[test]
fn check_params_different_true_for_different() {
    assert!(check_params_different("x", "y"));
}

#[test]
fn check_params_different_false_for_same() {
    assert!(!check_params_different("x", "x"));
}

#[test]
fn check_params_different_false_for_empty() {
    assert!(!check_params_different("", "y"));
    assert!(!check_params_different("x", ""));
}

#[test]
fn surface_quads_count_matches_grid_cells() {
    // 3x4 grid -> (3-1)*(4-1) = 6 cells
    let grid = vec![
        vec![0.0, 1.0, 2.0, 3.0],
        vec![1.0, 2.0, 3.0, 4.0],
        vec![2.0, 3.0, 4.0, 5.0],
    ];
    let quads = surface_quads(&grid, 0.0, 5.0);
    assert_eq!(quads.len(), 6);
}

#[test]
fn surface_quads_corners_span_clip_space() {
    let grid = vec![vec![0.0, 1.0], vec![1.0, 2.0]];
    let quads = surface_quads(&grid, 0.0, 2.0);
    assert_eq!(quads.len(), 1);
    let (corners, mean) = &quads[0];
    // x (row) and z (column) sit at the [-1, 1] boundary
    assert!((corners[0][0] - (-1.0)).abs() < 1e-6);
    assert!((corners[0][2] - (-1.0)).abs() < 1e-6);
    assert!((corners[2][0] - 1.0).abs() < 1e-6);
    assert!((corners[2][2] - 1.0).abs() < 1e-6);
    // y is value normalization: 0.0 -> -1, 2.0 -> +1
    assert!((corners[0][1] - (-1.0)).abs() < 1e-6);
    assert!((corners[2][1] - 1.0).abs() < 1e-6);
    assert!((mean - 1.0).abs() < 1e-9);
}

#[test]
fn surface_quads_empty_for_single_row_or_col() {
    assert!(surface_quads(&[vec![1.0, 2.0]], 0.0, 1.0).is_empty());
    assert!(surface_quads(&[vec![1.0], vec![2.0]], 0.0, 1.0).is_empty());
    let empty: Vec<Vec<f64>> = vec![];
    assert!(surface_quads(&empty, 0.0, 1.0).is_empty());
}

#[test]
fn surface_quads_skips_ragged_rows() {
    // Row 2 is short -> the missing cell is skipped
    let grid = vec![vec![0.0, 1.0, 2.0], vec![1.0, 2.0], vec![2.0, 3.0, 4.0]];
    let quads = surface_quads(&grid, 0.0, 4.0);
    // Only (row0,col0) and (row1,col0) are valid (col1 is missing from row1)
    assert_eq!(quads.len(), 2);
}

#[test]
fn pdp2d_default_camera_is_tilted() {
    let s = PdpChart2DState::default();
    assert_ne!(s.camera.rotation, [0.0, 0.0, 0.0, 1.0]);
    assert!(s.show_uncertainty);
    assert!(!s.show_observed);
    assert!(!s.feasible_only);
}

#[test]
fn band_grids_computes_95_ci() {
    // variance 4 -> sigma = 2 -> ±1.96×2 = ±3.92
    let z = vec![vec![10.0, 20.0]];
    let var = vec![vec![4.0, 0.0]];
    let (lower, upper) = band_grids(&z, &var);
    assert!((lower[0][0] - (10.0 - 3.92)).abs() < 1e-9);
    assert!((upper[0][0] - (10.0 + 3.92)).abs() < 1e-9);
    // variance 0 -> band matches the Mean
    assert_eq!(lower[0][1], 20.0);
    assert_eq!(upper[0][1], 20.0);
}

#[test]
fn band_grids_negative_variance_does_not_produce_nan() {
    // A Gaussian process posterior variance can become slightly negative due to numerical error
    let z = vec![vec![5.0]];
    let var = vec![vec![-1e-12]];
    let (lower, upper) = band_grids(&z, &var);
    assert!(lower[0][0].is_finite());
    assert!(upper[0][0].is_finite());
    assert_eq!(lower[0][0], 5.0);
    assert_eq!(upper[0][0], 5.0);
}

#[test]
fn band_grids_truncates_to_shorter_rows() {
    let z = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let var = vec![vec![0.0]]; // both row and column counts are short
    let (lower, upper) = band_grids(&z, &var);
    assert_eq!(lower.len(), 1);
    assert_eq!(lower[0].len(), 1);
    assert_eq!(upper[0].len(), 1);
}

#[test]
fn push_tri_appends_three_vertices_and_indices() {
    let mut mesh = egui::Mesh::default();
    push_tri(
        &mut mesh,
        [
            egui::pos2(0.0, 0.0),
            egui::pos2(1.0, 0.0),
            egui::pos2(0.0, 1.0),
        ],
        egui::Color32::RED,
    );
    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn push_edge_zero_length_adds_nothing() {
    let mut mesh = egui::Mesh::default();
    let p = egui::pos2(5.0, 5.0);
    push_edge(&mut mesh, p, p, egui::Color32::RED, 0.35);
    assert!(mesh.is_empty());
}

#[test]
fn push_edge_builds_finite_quad() {
    // Vertex coordinates stay bounded for any segment (no infinities like miter divergence)
    let mut mesh = egui::Mesh::default();
    push_edge(
        &mut mesh,
        egui::pos2(0.0, 0.0),
        egui::pos2(100.0, 0.001),
        egui::Color32::RED,
        0.35,
    );
    assert_eq!(mesh.vertices.len(), 6);
    for v in &mesh.vertices {
        assert!(v.pos.x.is_finite() && v.pos.y.is_finite());
        assert!(v.pos.x.abs() <= 101.0 && v.pos.y.abs() <= 1.0);
    }
}

// ── extract_observed_3d ──────────────────────────────────────────

fn make_view_2p_ranked(p1: &[f64], p2: &[f64], obj: &[f64], ranks: Vec<u32>) -> StudyView {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
    let n = obj.len();
    let param_names = vec!["p1".to_string(), "p2".to_string()];
    let obj_names = vec!["obj0".to_string()];
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: [("p1".to_string(), p1[i]), ("p2".to_string(), p2[i])].into(),
            param_category_label: HashMap::new(),
            objective_values: vec![obj[i]],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &param_names, &obj_names, &[], &[], 0);
    StudyView::new(Arc::new(df), ranks)
}

fn make_view_2p(p1: &[f64], p2: &[f64], obj: &[f64]) -> StudyView {
    let n = obj.len();
    make_view_2p_ranked(p1, p2, obj, vec![0; n])
}

#[test]
fn extract_observed_3d_returns_all_rows_without_selection() {
    let view = make_view_2p(&[1.0, 2.0], &[10.0, 20.0], &[0.5, 1.5]);
    let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[], &[]);
    assert_eq!(pts.len(), 2);
    assert_eq!(pts[0].0, 0);
    assert_eq!(pts[0].1, [1.0, 10.0, 0.5]);
    assert_eq!(pts[1].0, 1);
    assert_eq!(pts[1].1, [2.0, 20.0, 1.5]);
}

#[test]
fn extract_observed_3d_filters_by_selection_and_pinned() {
    let view = make_view_2p(&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0], &[0.1, 0.2, 0.3]);
    let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[0], &[2]);
    let p1s: Vec<f64> = pts.iter().map(|(_, p, _)| p[0]).collect();
    assert!(p1s.contains(&1.0), "selected row must be visible");
    assert!(p1s.contains(&3.0), "pinned row must remain visible");
    assert!(
        !p1s.contains(&2.0),
        "unselected unpinned row must be hidden"
    );
}

#[test]
fn extract_observed_3d_missing_column_returns_empty() {
    let view = make_view_2p(&[1.0], &[10.0], &[0.5]);
    assert!(extract_observed_3d(&view, "nope", "p2", "obj0", &[], &[]).is_empty());
    assert!(extract_observed_3d(&view, "p1", "p2", "nope", &[], &[]).is_empty());
}

#[test]
fn extract_observed_3d_skips_non_finite_rows() {
    let view = make_view_2p(&[1.0, f64::NAN], &[10.0, 20.0], &[0.5, 1.5]);
    let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[], &[]);
    assert_eq!(pts.len(), 1);
    assert_eq!(pts[0].0, 0);
    assert_eq!(pts[0].1, [1.0, 10.0, 0.5]);
}

#[test]
fn extract_observed_3d_classifies_by_pareto_rank() {
    // rank 0 -> Pareto (red), rank > 0 -> NonPareto (blue)
    let view = make_view_2p_ranked(&[1.0, 2.0], &[10.0, 20.0], &[0.5, 1.5], vec![0, 1]);
    let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[], &[]);
    assert_eq!(pts[0].2, ObservedKind::Pareto);
    assert_eq!(pts[1].2, ObservedKind::NonPareto);
}
