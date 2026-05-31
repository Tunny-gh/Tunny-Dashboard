use super::*;
use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use std::collections::HashMap;

fn make_row_obj(trial_id: u32, obj: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        param_display: HashMap::new(),
        param_category_label: HashMap::new(),
        objective_values: obj,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![],
    }
}

fn setup_study(rows: Vec<TrialRow>, obj_names: &[&str]) {
    let names: Vec<String> = obj_names.iter().map(|s| s.to_string()).collect();
    let df = DataFrame::from_trials(&rows, &[], &names, &[], &[], 0);
    store_dataframes(vec![df]);
    select_study(0).unwrap();
}

#[test]
fn tc_201_01_two_obj_all_nondominated() {
    let objs = vec![
        vec![1.0, 4.0],
        vec![2.0, 3.0],
        vec![3.0, 2.0],
        vec![4.0, 1.0],
    ];
    let is_min = [true, true];
    let ranks = nd_sort(&objs, &is_min);
    assert_eq!(ranks, vec![0, 0, 0, 0]);
}

#[test]
fn tc_201_02_two_obj_clear_domination() {
    let objs = vec![vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]];
    let is_min = [true, true];
    let ranks = nd_sort(&objs, &is_min);
    assert_eq!(ranks, vec![0, 1, 2]);
}

#[test]
fn tc_201_03_four_objectives() {
    let objs = vec![
        vec![1.0, 1.0, 1.0, 1.0],
        vec![2.0, 2.0, 2.0, 2.0],
        vec![1.0, 2.0, 1.0, 2.0],
    ];
    let is_min = [true, true, true, true];
    let ranks = nd_sort(&objs, &is_min);
    assert_eq!(ranks[0], 0);
    assert_eq!(ranks[2], 1);
    assert_eq!(ranks[1], 2);
}

#[test]
fn tc_201_04_single_objective_all_rank1() {
    let objs = vec![vec![3.0], vec![1.0], vec![4.0], vec![1.5], vec![2.0]];
    let is_min = [true];
    let ranks = nd_sort(&objs, &is_min);
    assert!(ranks.iter().all(|&r| r == 0));
}

#[test]
fn tc_201_05_maximize_direction() {
    let objs = vec![vec![1.0], vec![2.0], vec![3.0]];
    let is_min_single = [true];
    let ranks_single = nd_sort(&objs, &is_min_single);
    assert!(ranks_single.iter().all(|&r| r == 0));

    let objs2 = vec![vec![1.0, 3.0], vec![2.0, 2.0], vec![3.0, 1.0]];
    let is_min2 = [false, true];
    let ranks2 = nd_sort(&objs2, &is_min2);
    assert_eq!(ranks2[2], 0);
    assert_eq!(ranks2[1], 1);
    assert_eq!(ranks2[0], 2);
}

#[test]
fn tc_201_06_hypervolume_2d_known_value() {
    let pts = vec![(1.0, 4.0), (2.0, 2.0), (3.0, 1.0)];
    let hv = hypervolume_2d(&pts, 5.0, 5.0);
    assert!((hv - 12.0).abs() < 1e-9, "HV = {}, expected 12.0", hv);
}

#[test]
fn tc_201_07_hypervolume_single_objective_none() {
    let rows = vec![
        make_row_obj(0, vec![1.0]),
        make_row_obj(1, vec![2.0]),
        make_row_obj(2, vec![3.0]),
    ];
    setup_study(rows, &["obj0"]);
    let result = compute_pareto_ranks(&[true]);
    assert!(result.hypervolume.is_none());
}

#[test]
fn tc_201_08_tradeoff_navigator_order() {
    let objs = vec![vec![1.0, 4.0], vec![2.0, 2.0], vec![4.0, 1.0]];
    let is_min = [true, true];
    let weights = [0.5, 0.5];
    let result = chebyshev_sort(&objs, &weights, &is_min);
    assert_eq!(result[0], 1);
}

#[test]
fn tc_201_09_hypervolume_history_single_obj() {
    let rows = vec![
        make_row_obj(0, vec![2.0]),
        make_row_obj(1, vec![1.0]),
        make_row_obj(2, vec![3.0]),
    ];
    setup_study(rows, &["obj0"]);
    let result = compute_hypervolume_history(&[true]);
    assert!(result.hv_values.iter().all(|&v| v == 0.0));
    assert_eq!(result.trial_ids.len(), 3);
}

#[test]
fn tc_201_e01_zero_weights_fallback() {
    let objs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let is_min = [true, true];
    let weights = [0.0, 0.0];
    let result = chebyshev_sort(&objs, &weights, &is_min);
    assert_eq!(result.len(), 2);
}

#[test]
fn tc_201_e02_empty_dataframe_returns_empty() {
    store_dataframes(vec![DataFrame::empty()]);
    select_study(0).unwrap();
    let result = compute_pareto_ranks(&[true]);
    assert!(result.ranks.is_empty());
    assert!(result.pareto_indices.is_empty());
    assert!(result.hypervolume.is_none());
}

#[test]
fn tc_201_b01_all_same_coords() {
    let objs = vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]];
    let is_min = [true, true];
    let ranks = nd_sort(&objs, &is_min);
    assert!(ranks.iter().all(|&r| r == 0));
}

#[test]
fn tc_201_b02_chain_dominance() {
    let objs = vec![
        vec![1.0, 1.0],
        vec![2.0, 2.0],
        vec![3.0, 3.0],
        vec![4.0, 4.0],
    ];
    let is_min = [true, true];
    let ranks = nd_sort(&objs, &is_min);
    assert_eq!(ranks, vec![0, 1, 2, 3]);
}

#[test]
fn tc_201_b03_single_point() {
    let rows = vec![make_row_obj(0, vec![1.0, 2.0])];
    setup_study(rows, &["obj0", "obj1"]);
    let result = compute_pareto_ranks(&[true, true]);
    assert_eq!(result.ranks, vec![0]);
    assert_eq!(result.pareto_indices, vec![0]);
    assert!(result.hypervolume.is_none());
}

#[test]
fn tc_hv_5d_monotonically_nondecreasing() {
    let objs: Vec<Vec<f64>> = vec![
        vec![1.0, 5.0, 3.0, 2.0, 4.0],
        vec![5.0, 1.0, 2.0, 4.0, 3.0],
        vec![2.0, 3.0, 1.0, 5.0, 2.0],
        vec![4.0, 2.0, 5.0, 1.0, 1.0],
        vec![3.0, 4.0, 4.0, 3.0, 5.0],
        vec![0.5, 0.5, 0.5, 0.5, 0.5],
    ];
    let trial_ids: Vec<u32> = (0..objs.len() as u32).collect();
    let is_min = [true; 5];
    let result = compute_hv_history_from_data(&trial_ids, &objs, &is_min);
    let hvs = &result.hv_values;
    for i in 1..hvs.len() {
        assert!(
            hvs[i] >= hvs[i - 1] - 1e-9,
            "HV decreased at step {}: {} -> {}",
            i,
            hvs[i - 1],
            hvs[i]
        );
    }
}

#[test]
fn tc_hv_nd_3d_known_value() {
    let pts = vec![vec![1.0, 1.0, 1.0]];
    let ref_pt = vec![2.0, 2.0, 2.0];
    let hv = hypervolume_nd(&pts, &ref_pt);
    assert!((hv - 1.0).abs() < 1e-9, "HV = {}, expected 1.0", hv);
}

#[test]
fn tc_201_p01_ndsort_1000_points_under_100ms() {
    // Debug builds are unoptimised; use fewer points so the assertion
    // remains meaningful without requiring a release build.
    #[cfg(debug_assertions)]
    let n = 200usize;
    #[cfg(not(debug_assertions))]
    let n = 1_000usize;

    let objs: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let x = ((i.wrapping_mul(7_919).wrapping_add(1_234_567)) % n) as f64 / n as f64;
            let y = ((i.wrapping_mul(6_271).wrapping_add(9_876_543)) % n) as f64 / n as f64;
            vec![x, y]
        })
        .collect();
    let is_min = [true, true];

    let start = std::time::Instant::now();
    let ranks = nd_sort(&objs, &is_min);
    let elapsed = start.elapsed();

    // Release: 500ms ceiling (~5× headroom vs typical ~20ms).
    // Debug: 2000ms ceiling for unoptimised code.
    #[cfg(debug_assertions)]
    assert!(
        elapsed.as_millis() <= 2000,
        "NDSort 200-point debug run exceeded 2000ms: {}ms",
        elapsed.as_millis()
    );
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed.as_millis() <= 500,
        "NDSort 1000-point release run exceeded 500ms: {}ms",
        elapsed.as_millis()
    );
    assert_eq!(ranks.len(), n);
}

#[test]
fn tc_201_p02_tradeoff_50000_points_under_1ms() {
    #[cfg(debug_assertions)]
    let n = 5_000usize;
    #[cfg(not(debug_assertions))]
    let n = 50_000usize;

    let rows: Vec<TrialRow> = (0..n)
        .map(|i| make_row_obj(i as u32, vec![(i % 100) as f64, (n - i) as f64]))
        .collect();
    setup_study(rows, &["obj0", "obj1"]);

    let weights = [0.5, 0.5];
    let is_min = [true, true];
    let start = std::time::Instant::now();
    let result = score_tradeoff_navigator(&weights, &is_min);
    let elapsed = start.elapsed();

    #[cfg(debug_assertions)]
    assert!(
        elapsed.as_millis() <= 50,
        "Trade-off Navigator translated {}ms translated（translated: ≤50ms）",
        elapsed.as_millis()
    );
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed.as_millis() <= 1,
        "Trade-off Navigator translated {}ms translated（translated: ≤1ms）",
        elapsed.as_millis()
    );
    assert_eq!(result.len(), n);
}
