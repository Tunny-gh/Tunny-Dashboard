use super::*;

#[test]
fn normalize_min_maps_to_zero() {
    let n = normalize_value(0.0, 0.0, 10.0);
    assert!((n - 0.0).abs() < 1e-6);
}

#[test]
fn normalize_max_maps_to_one() {
    let n = normalize_value(10.0, 0.0, 10.0);
    assert!((n - 1.0).abs() < 1e-6);
}

#[test]
fn normalize_equal_min_max_returns_half() {
    let n = normalize_value(5.0, 5.0, 5.0);
    assert!((n - 0.5).abs() < 1e-6);
}

#[test]
fn normalize_clamps_out_of_range() {
    let below = normalize_value(-1.0, 0.0, 1.0);
    let above = normalize_value(2.0, 0.0, 1.0);
    assert!((below - 0.0).abs() < 1e-6);
    assert!((above - 1.0).abs() < 1e-6);
}

#[test]
fn normalized_to_screen_y_zero_maps_to_bottom() {
    let y = normalized_to_screen_y(0.0, 100.0, 400.0);
    assert!((y - 400.0).abs() < 1e-3);
}

#[test]
fn normalized_to_screen_y_one_maps_to_top() {
    let y = normalized_to_screen_y(1.0, 100.0, 400.0);
    assert!((y - 100.0).abs() < 1e-3);
}

#[test]
fn build_axis_order_concatenates_params_then_objectives() {
    let params = vec!["x".to_string(), "y".to_string()];
    let objs = vec!["obj0".to_string()];
    let axes = build_axis_order(&params, &objs);
    assert_eq!(axes, vec!["x", "y", "obj0"]);
}

#[test]
fn parallel_coords_chart_default() {
    let chart = ParallelCoordsChart::default();
    assert!(chart.axis_order.is_empty());
    assert!(chart.show_params);
    assert!(chart.show_objectives);
    assert!(chart.brush_ranges.is_empty());
    assert!(chart.drag_start.is_none());
    assert!(chart.color_axis.is_none());
}

// TASK-2022 tests

#[test]
fn ordered_brush_range_forward_drag() {
    let (min, max) = ordered_brush_range(0.2, 0.8);
    assert!((min - 0.2).abs() < 1e-6);
    assert!((max - 0.8).abs() < 1e-6);
}

#[test]
fn ordered_brush_range_reverse_drag() {
    // Dragging upward: start > end
    let (min, max) = ordered_brush_range(0.8, 0.2);
    assert!((min - 0.2).abs() < 1e-6);
    assert!((max - 0.8).abs() < 1e-6);
}

// TASK-2125 tests
#[test]
fn axis_visibility_filter() {
    use std::collections::HashMap;
    let mut visibility: HashMap<String, bool> = HashMap::new();
    visibility.insert("x1".to_string(), true);
    visibility.insert("x2".to_string(), false);
    visibility.insert("x3".to_string(), true);
    let axis_order = ["x1".to_string(), "x2".to_string(), "x3".to_string()];
    let visible: Vec<_> = axis_order
        .iter()
        .filter(|name| *visibility.get(*name).unwrap_or(&true))
        .collect();
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0], "x1");
    assert_eq!(visible[1], "x3");
}

#[test]
fn axis_reorder_logic() {
    let mut axis_order = vec!["x1".to_string(), "x2".to_string(), "x3".to_string()];
    let dragged = "x1";
    let target_idx = 2;
    if let Some(from_idx) = axis_order.iter().position(|a| a == dragged) {
        let name = axis_order.remove(from_idx);
        let insert_at = target_idx.min(axis_order.len());
        axis_order.insert(insert_at, name);
    }
    assert_eq!(axis_order, vec!["x2", "x3", "x1"]);
}

#[test]
fn axis_visibility_all_hidden() {
    use std::collections::HashMap;
    let mut visibility: HashMap<String, bool> = HashMap::new();
    visibility.insert("x1".to_string(), false);
    visibility.insert("x2".to_string(), false);
    let axis_order = ["x1".to_string(), "x2".to_string()];
    let visible: Vec<_> = axis_order
        .iter()
        .filter(|name| *visibility.get(*name).unwrap_or(&true))
        .collect();
    assert!(visible.is_empty());
}

#[test]
fn axis_visibility_default_true_for_unknown() {
    use std::collections::HashMap;
    let visibility: HashMap<String, bool> = HashMap::new();
    let axis_order = ["unknown_axis".to_string()];
    let visible: Vec<_> = axis_order
        .iter()
        .filter(|name| *visibility.get(*name).unwrap_or(&true))
        .collect();
    assert_eq!(visible.len(), 1);
}

// ── constraint-aware visualization (TASK-2349) ──────────────────

#[test]
fn tc_cav_parallel_coords_show_infeasible_default_true() {
    let chart = ParallelCoordsChart::default();
    assert!(chart.show_infeasible);
}

// --- TASK-2242: PCP brush tests ---

#[test]
fn multi_axis_brush_applies_and_filter() {
    use std::collections::HashMap;
    let trial_ids = vec![0u32, 1, 2];
    // col_data: axis 0 = x, axis 1 = obj
    let col_data = [
        vec![2.0, 8.0, 2.0], // x values
        vec![5.0, 5.0, 9.0], // obj values
    ];
    let cols: Vec<Option<&[f64]>> =
        vec![Some(col_data[0].as_slice()), Some(col_data[1].as_slice())];
    let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
    let all_names = vec!["x".to_string(), "obj".to_string()];

    let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    // x in [0.0, 0.5] = values 0..5 → trial 0 and 2 pass
    brush_ranges.insert("x".to_string(), Some((0.0, 0.5)));
    // obj in [0.0, 0.6] = values 0..6 → trial 0 passes; trial 2 (obj=9) fails
    brush_ranges.insert("obj".to_string(), Some((0.0, 0.6)));

    let sel = filter_trials_by_brushes(&trial_ids, &brush_ranges, &cols, &col_ranges, &all_names);
    assert_eq!(sel.len(), 1);
    assert_eq!(sel[0], 0);
}

#[test]
fn shifted_brush_range_moves_within_bounds() {
    let (lo, hi) = shifted_brush_range((0.2, 0.5), 0.1);
    assert!((lo - 0.3).abs() < 1e-6);
    assert!((hi - 0.6).abs() < 1e-6);
}

#[test]
fn shifted_brush_range_clamps_at_top() {
    // width 0.3, shift up by 0.4 → would exceed 1.0, clamp so hi == 1.0
    let (lo, hi) = shifted_brush_range((0.5, 0.8), 0.4);
    assert!((hi - 1.0).abs() < 1e-6);
    assert!((lo - 0.7).abs() < 1e-6); // width preserved
}

#[test]
fn shifted_brush_range_clamps_at_bottom() {
    // shift down past 0 → clamp so lo == 0.0, width preserved
    let (lo, hi) = shifted_brush_range((0.2, 0.5), -0.4);
    assert!((lo - 0.0).abs() < 1e-6);
    assert!((hi - 0.3).abs() < 1e-6);
}

#[test]
fn shifted_brush_range_preserves_width() {
    let orig = (0.1_f32, 0.6_f32);
    let (lo, hi) = shifted_brush_range(orig, 0.25);
    assert!(((hi - lo) - (orig.1 - orig.0)).abs() < 1e-6);
}

#[test]
fn trial_passes_brushes_no_active_brush_passes() {
    use std::collections::HashMap;
    let col_data = [vec![2.0, 8.0], vec![5.0, 9.0]];
    let cols: Vec<Option<&[f64]>> =
        vec![Some(col_data[0].as_slice()), Some(col_data[1].as_slice())];
    let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
    let all_names = vec!["x".to_string(), "obj".to_string()];
    // No brush set (None only) -> everything passes.
    let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    brush_ranges.insert("x".to_string(), None);
    assert!(trial_passes_brushes(
        0,
        &brush_ranges,
        &cols,
        &col_ranges,
        &all_names
    ));
}

#[test]
fn trial_passes_brushes_missing_value_with_active_brush_excluded() {
    use std::collections::HashMap;
    // axis 1 has only one value, so t_idx=1 is missing.
    let col_data_x = vec![2.0, 8.0];
    let col_data_obj = vec![5.0];
    let cols: Vec<Option<&[f64]>> =
        vec![Some(col_data_x.as_slice()), Some(col_data_obj.as_slice())];
    let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
    let all_names = vec!["x".to_string(), "obj".to_string()];
    let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    brush_ranges.insert("obj".to_string(), Some((0.0, 1.0)));
    // t_idx=1 is missing the obj value -> fails since the brush is active.
    assert!(!trial_passes_brushes(
        1,
        &brush_ranges,
        &cols,
        &col_ranges,
        &all_names
    ));
}

#[test]
fn visible_axis_indices_default_all_visible() {
    use std::collections::HashMap;
    let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let vis = HashMap::new(); // unregistered = all visible
    assert_eq!(visible_axis_indices(&names, &vis), vec![0, 1, 2]);
}

#[test]
fn visible_axis_indices_filters_hidden_and_preserves_order() {
    use std::collections::HashMap;
    let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut vis = HashMap::new();
    vis.insert("b".to_string(), false);
    assert_eq!(visible_axis_indices(&names, &vis), vec![0, 2]);
}

#[test]
fn visible_axis_indices_all_hidden_is_empty() {
    use std::collections::HashMap;
    let names = vec!["a".to_string(), "b".to_string()];
    let mut vis = HashMap::new();
    vis.insert("a".to_string(), false);
    vis.insert("b".to_string(), false);
    assert!(visible_axis_indices(&names, &vis).is_empty());
}

#[test]
fn feasible_color_range_excludes_infeasible_outliers() {
    use tunny_core::dataframe::Feasibility;
    // The infeasible solution (idx 3) has an outlier value of 1000.0, but
    // the range is computed from feasible solutions only.
    let col = [1.0, 2.0, 3.0, 1000.0];
    let feas_col = [1.0, 1.0, 1.0, 0.0];
    let feas = Feasibility::from_column(Some(&feas_col));
    let (mn, mx) = feasible_color_range(&col, feas, (0.0, 9999.0));
    assert_eq!(mn, 1.0);
    assert_eq!(mx, 3.0);
}

#[test]
fn feasible_color_range_no_constraints_uses_all() {
    use tunny_core::dataframe::Feasibility;
    let col = [1.0, 2.0, 3.0, 1000.0];
    let feas = Feasibility::from_column(None);
    let (mn, mx) = feasible_color_range(&col, feas, (0.0, 9999.0));
    assert_eq!(mn, 1.0);
    assert_eq!(mx, 1000.0);
}

#[test]
fn feasible_color_range_all_infeasible_falls_back() {
    use tunny_core::dataframe::Feasibility;
    let col = [1.0, 2.0, 3.0];
    let feas_col = [0.0, 0.0, 0.0];
    let feas = Feasibility::from_column(Some(&feas_col));
    let range = feasible_color_range(&col, feas, (-5.0, 5.0));
    assert_eq!(range, (-5.0, 5.0));
}

#[test]
fn feasible_color_range_skips_non_finite() {
    use tunny_core::dataframe::Feasibility;
    let col = [1.0, f64::NAN, f64::INFINITY, 4.0];
    let feas_col = [1.0, 1.0, 1.0, 1.0];
    let feas = Feasibility::from_column(Some(&feas_col));
    let (mn, mx) = feasible_color_range(&col, feas, (0.0, 0.0));
    assert_eq!(mn, 1.0);
    assert_eq!(mx, 4.0);
}
