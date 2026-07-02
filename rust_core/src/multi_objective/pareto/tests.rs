use super::*;
use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use std::collections::HashMap;

fn make_row_obj(trial_id: u32, obj: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        trial_number: trial_id,
        param_display: HashMap::new(),
        param_category_label: HashMap::new(),
        objective_values: obj,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![],
    }
}

fn make_row_constrained(trial_id: u32, obj: Vec<f64>, constraints: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        trial_number: trial_id,
        param_display: HashMap::new(),
        param_category_label: HashMap::new(),
        objective_values: obj,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: constraints,
    }
}

fn setup_study(rows: Vec<TrialRow>, obj_names: &[&str]) {
    let names: Vec<String> = obj_names.iter().map(|s| s.to_string()).collect();
    let df = DataFrame::from_trials(&rows, &[], &names, &[], &[], 0);
    store_dataframes(vec![df]);
    select_study(0).unwrap();
}

fn setup_study_constrained(rows: Vec<TrialRow>, obj_names: &[&str], max_constraints: usize) {
    let names: Vec<String> = obj_names.iter().map(|s| s.to_string()).collect();
    let df = DataFrame::from_trials(&rows, &[], &names, &[], &[], max_constraints);
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
fn tc_hv_201_11_compute_pareto_ranks_3obj_two_points_overlap() {
    // 2点 (0,1,1), (1,0,0) は互いに非支配で、3次元目の値も両点で異なる
    // （3次元目を無視する旧実装のバグでは値が変わらない座標を避けるため）。
    // nadir=(1,1,1), ideal=(0,0,0) -> ref = nadir + 0.1*range = (1.1,1.1,1.1)。
    // 包除原理で手計算: Vol(p1)=1.1*0.1*0.1=0.011, Vol(p2)=0.1*1.1*1.1=0.121,
    // 重複=(1.1-1)^3=0.001 -> HV=0.011+0.121-0.001=0.131
    let rows = vec![
        make_row_obj(0, vec![0.0, 1.0, 1.0]),
        make_row_obj(1, vec![1.0, 0.0, 0.0]),
    ];
    setup_study(rows, &["obj0", "obj1", "obj2"]);
    let result = compute_pareto_ranks(&[true, true, true]);
    let hv = result
        .hypervolume
        .expect("hypervolume should be Some for m=3");
    assert!((hv - 0.131).abs() < 1e-9, "HV = {}, expected 0.131", hv);
}

#[test]
fn hv_history_with_ref_returns_used_ref_point() {
    let objs = vec![vec![1.0, 1.0], vec![0.5, 2.0]];
    let ids: Vec<u32> = vec![0, 1];
    let is_min = [true, true];
    // 指定なし: 自動算出された参照点が返る（全要素が観測の nadir 超）。
    let auto = compute_hv_history_with_ref(&ids, &objs, &is_min, None);
    assert_eq!(auto.ref_point.len(), 2);
    assert!(auto.ref_point[0] > 1.0 && auto.ref_point[1] > 2.0);
}

#[test]
fn hv_history_with_ref_honors_override() {
    let objs = vec![vec![1.0, 1.0], vec![2.0, 0.5]];
    let ids: Vec<u32> = vec![0, 1];
    let is_min = [true, true];
    let custom = vec![10.0, 10.0];
    let r = compute_hv_history_with_ref(&ids, &objs, &is_min, Some(&custom));
    assert_eq!(r.ref_point, custom);
    // 参照点を広げると HV は自動算出時より大きくなる。
    let auto = compute_hv_history_with_ref(&ids, &objs, &is_min, None);
    assert!(r.hv_values.last().unwrap() > auto.hv_values.last().unwrap());
}

#[test]
fn hv_history_with_ref_ignores_wrong_dim_override() {
    let objs = vec![vec![1.0, 1.0], vec![2.0, 0.5]];
    let ids: Vec<u32> = vec![0, 1];
    let is_min = [true, true];
    // 次元不一致の指定は無視して自動算出にフォールバックする。
    let bad = vec![10.0, 10.0, 10.0];
    let r = compute_hv_history_with_ref(&ids, &objs, &is_min, Some(&bad));
    assert_eq!(r.ref_point.len(), 2);
    assert_ne!(r.ref_point, bad);
}

#[test]
fn tc_201_p01_ndsort_1000_points_under_100ms() {
    let n = 1_000usize;

    let objs: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let x = ((i.wrapping_mul(7_919).wrapping_add(1_234_567)) % n) as f64 / n as f64;
            let y = ((i.wrapping_mul(6_271).wrapping_add(9_876_543)) % n) as f64 / n as f64;
            vec![x, y]
        })
        .collect();
    let is_min = [true, true];

    let ranks = nd_sort(&objs, &is_min);

    assert_eq!(ranks.len(), n);
}

// ================================================================
// TC-CAV: constraint-aware Pareto ranking tests
// ================================================================

#[test]
fn tc_cav_01_infeasible_excluded_from_pareto_front() {
    // feasible [1,2] and [2,1] (both on Pareto front)
    // infeasible [0,0] (would dominate if feasible, constraint_sum=1.0)
    let rows = vec![
        make_row_constrained(0, vec![1.0, 2.0], vec![0.0]),
        make_row_constrained(1, vec![2.0, 1.0], vec![0.0]),
        make_row_constrained(2, vec![0.0, 0.0], vec![1.0]),
    ];
    setup_study_constrained(rows, &["obj0", "obj1"], 1);
    let result = compute_pareto_ranks(&[true, true]);
    assert!(
        !result.pareto_indices.contains(&2u32),
        "infeasible must not be in pareto_indices"
    );
    assert!(result.pareto_indices.contains(&0u32));
    assert!(result.pareto_indices.contains(&1u32));
    assert_eq!(result.ranks[0], 0);
    assert_eq!(result.ranks[1], 0);
    assert!(result.ranks[2] > 0, "infeasible must have rank > 0");
}

#[test]
fn tc_cav_02_infeasible_ranked_by_constraint_sum_ascending() {
    // All infeasible, ranked by constraint_sum ascending
    let rows = vec![
        make_row_constrained(0, vec![1.0, 2.0], vec![3.0]),
        make_row_constrained(1, vec![2.0, 1.0], vec![1.0]),
        make_row_constrained(2, vec![0.5, 0.5], vec![2.0]),
    ];
    setup_study_constrained(rows, &["obj0", "obj1"], 1);
    let result = compute_pareto_ranks(&[true, true]);
    assert!(
        result.pareto_indices.is_empty(),
        "no feasible → empty pareto"
    );
    // sum=1.0 (idx1) < sum=2.0 (idx2) < sum=3.0 (idx0)
    assert!(
        result.ranks[1] < result.ranks[2],
        "idx1 (sum=1.0) must rank lower than idx2 (sum=2.0)"
    );
    assert!(
        result.ranks[2] < result.ranks[0],
        "idx2 (sum=2.0) must rank lower than idx0 (sum=3.0)"
    );
}

#[test]
fn tc_cav_03_all_infeasible_gives_empty_pareto_indices() {
    let rows = vec![
        make_row_constrained(0, vec![1.0, 2.0], vec![1.0]),
        make_row_constrained(1, vec![2.0, 1.0], vec![2.0]),
    ];
    setup_study_constrained(rows, &["obj0", "obj1"], 1);
    let result = compute_pareto_ranks(&[true, true]);
    assert!(result.pareto_indices.is_empty());
    assert_eq!(result.ranks.len(), 2);
}

#[test]
fn tc_cav_04_no_constraints_unchanged_behavior() {
    // Constraints-free study must behave identically to the original
    let rows = vec![
        make_row_obj(0, vec![1.0, 2.0]),
        make_row_obj(1, vec![2.0, 1.0]),
        make_row_obj(2, vec![3.0, 3.0]),
    ];
    setup_study(rows, &["obj0", "obj1"]);
    let result = compute_pareto_ranks(&[true, true]);
    assert!(result.pareto_indices.contains(&0u32));
    assert!(result.pareto_indices.contains(&1u32));
    assert!(!result.pareto_indices.contains(&2u32));
    assert_eq!(result.ranks[0], 0);
    assert_eq!(result.ranks[1], 0);
    assert!(result.ranks[2] > 0);
}
