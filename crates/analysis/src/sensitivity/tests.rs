use super::sobol::{build_quad_features, compute_sobol_index_pair};
use super::*;
use crate::dataframe::{DataFrame, TrialRow};
use std::collections::HashMap;

fn make_row_multi(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        trial_number: trial_id,
        param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        distribution_metadata: Default::default(),
        param_category_label: HashMap::new(),
        objective_values: objectives,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        user_attrs_json: HashMap::new(),
        constraint_values: vec![],
    }
}

fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
    let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
    let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
    DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0)
}

#[test]
fn finite_selection_retains_method_specific_requirements() {
    let rows = vec![
        make_row_multi(0, &[("x", 0.0), ("other", f64::NAN)], vec![0.0, f64::NAN]),
        make_row_multi(1, &[("x", f64::NAN), ("other", 1.0)], vec![99.0, 1.0]),
        make_row_multi(
            2,
            &[("x", 2.0), ("other", f64::INFINITY)],
            vec![4.0, f64::NAN],
        ),
        make_row_multi(
            3,
            &[("x", 4.0), ("other", f64::NEG_INFINITY)],
            vec![8.0, f64::NAN],
        ),
    ];
    let df = setup_df(rows, &["x", "other"], &["obj", "unused_obj"]);
    let pairwise = SpearmanMetric.compute(&df, 0).unwrap();
    let x_index = pairwise
        .param_names
        .iter()
        .position(|name| name == "x")
        .unwrap();
    assert_eq!(
        pairwise.spearman[x_index][0],
        compute_spearman(&[0.0, 2.0, 4.0], &[0.0, 4.0, 8.0])
    );
    // Ridge requires all supported parameters, Sobol also requires all objectives.
    let ridge = RidgeMetric.compute(&df, 0).unwrap();
    assert_eq!(ridge.ridge[0].beta, vec![0.0; 2]);
    assert_eq!(ridge.ridge[0].r_squared, 0.0);
    assert!(compute_sobol_from_df(&df, 32).is_none());
    assert_eq!(compute_spearman(&[0.0], &[0.0, 1.0]), 0.0);
    assert!(compute_spearman(&[f64::NAN; 2], &[0.0; 3]).is_nan());
    assert_eq!(compute_spearman(&[0.0, 1.0, 999.0], &[0.0, 2.0]), 1.0);
}

#[test]
fn sobol_complete_case_selection_keeps_objectives_and_categories_aligned() {
    let complete: Vec<_> = (0..12)
        .map(|i| {
            let mut row =
                make_row_multi(i, &[("x", i as f64)], vec![2.0 * i as f64, (i * i) as f64]);
            row.param_category_label
                .insert("cat".to_string(), format!("c{}", i % 2));
            // Non-required constraints must not affect observation selection.
            row.constraint_values = vec![if i % 2 == 0 { f64::NAN } else { f64::INFINITY }];
            row
        })
        .collect();
    let mut incomplete = Vec::new();
    for row in &complete {
        incomplete.push(row.clone());
        for (feature, objective) in [
            (f64::NAN, 999.0),
            (f64::INFINITY, 999.0),
            (f64::NEG_INFINITY, 999.0),
            (999.0, f64::NAN),
        ] {
            let mut bad = row.clone();
            bad.param_display.insert("x".to_string(), feature);
            bad.objective_values[1] = objective;
            incomplete.push(bad);
        }
    }
    let clean = setup_df(complete, &["x", "cat"], &["a", "b"]);
    let filtered = setup_df(incomplete, &["x", "cat"], &["a", "b"]);
    let expected = compute_sobol_from_df(&clean, 128).unwrap();
    let actual = compute_sobol_from_df(&filtered, 128).unwrap();
    assert_eq!(actual.param_names, expected.param_names);
    assert_eq!(actual.objective_names, expected.objective_names);
    assert_eq!(actual.first_order, expected.first_order);
    assert_eq!(actual.total_effect, expected.total_effect);
    assert_eq!(actual.r_squared, expected.r_squared);
}

#[test]
fn tc_801_01_spearman_perfect_positive() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];

    let r = compute_spearman(&x, &y);

    assert!(
        (r - 1.0).abs() < 1e-9,
        "Spearman correlation should be ~1.0: {}",
        r
    );
}

#[test]
fn tc_801_02_spearman_perfect_negative() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![5.0, 4.0, 3.0, 2.0, 1.0];

    let r = compute_spearman(&x, &y);

    assert!(
        (r + 1.0).abs() < 1e-9,
        "Spearman correlation should be ~-1.0: {}",
        r
    );
}

#[test]
fn tc_801_03_spearman_known_example() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let y = vec![4.0, 1.0, 2.0, 5.0, 6.0, 3.0];

    let r = compute_spearman(&x, &y);

    let expected = 13.0 / 35.0;
    assert!(
        (r - expected).abs() < 1e-9,
        "Spearman correlation mismatch: expected={}, got={}",
        expected,
        r
    );
}

#[test]
fn tc_801_04_spearman_tied_ranks() {
    let x = vec![1.0, 2.0, 2.0, 3.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];

    let r = compute_spearman(&x, &y);

    assert!(
        r > 0.9,
        "Spearman correlation should be positive for tied ranks: {}",
        r
    );
}

#[test]
fn tc_801_05_spearman_n_less_than_2_returns_zero() {
    let r1 = compute_spearman(&[], &[]);
    let r2 = compute_spearman(&[1.0], &[1.0]);

    assert_eq!(
        r1, 0.0,
        "Spearman correlation should be 0.0 for empty input"
    );
    assert_eq!(r2, 0.0, "Spearman correlation should be 0.0 for n=1");
}

#[test]
fn tc_801_16_spearman_nan_pairs_filtered_pairwise() {
    // Rows at index 2 and 4 contain a NaN on one side; they must be dropped
    // before ranking rather than corrupting the tie structure.
    let x = vec![1.0, 2.0, f64::NAN, 4.0, 5.0, 3.0];
    let y = vec![2.0, 4.0, 6.0, f64::NAN, 10.0, 6.0];

    let r = compute_spearman(&x, &y);

    let x_clean = vec![1.0, 2.0, 5.0, 3.0];
    let y_clean = vec![2.0, 4.0, 10.0, 6.0];
    let expected = compute_spearman(&x_clean, &y_clean);

    assert!(
        r.is_finite(),
        "NaN-contaminated pairs must not leak NaN into result"
    );
    assert!(
        (r - expected).abs() < 1e-9,
        "expected pairwise-deleted result {expected}, got {r}"
    );
}

#[test]
fn tc_801_17_spearman_inf_pairs_filtered_pairwise() {
    // Rows at index 1 and 3 contain an infinity on one side; they must be
    // dropped before ranking rather than corrupting the tie structure.
    let x = vec![1.0, f64::INFINITY, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 6.0, f64::NEG_INFINITY, 10.0];

    let r = compute_spearman(&x, &y);

    let x_clean = vec![1.0, 3.0, 5.0];
    let y_clean = vec![2.0, 6.0, 10.0];
    let expected = compute_spearman(&x_clean, &y_clean);

    assert!(
        r.is_finite(),
        "Inf-contaminated pairs must not leak NaN/Inf into result"
    );
    assert!(
        (r - expected).abs() < 1e-9,
        "expected pairwise-deleted result {expected}, got {r}"
    );
}

#[test]
fn tc_801_18_spearman_fewer_than_two_valid_pairs_returns_nan() {
    // Total length is >= 2, but pairwise deletion leaves fewer than 2 valid
    // pairs; this must be distinguished from the "n < 2 -> 0.0" raw-input
    // sentinel and instead signal via NaN, matching pearson_correlation's
    // convention for degenerate input.
    let x = vec![1.0, f64::NAN];
    let y = vec![f64::NAN, 2.0];

    let r = compute_spearman(&x, &y);

    assert!(
        r.is_nan(),
        "expected NaN for fewer than 2 valid pairs, got {r}"
    );
}

#[test]
fn tc_801_06_ridge_perfect_linear_r_squared_near_1() {
    let n = 50;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
    let y: Vec<f64> = (0..n).map(|i| 2.0 * i as f64 + 1.0).collect();

    let result = compute_ridge_from_vecs(&x_matrix, &y, 0.001);

    assert!(
        result.r_squared > 0.99,
        "R² should be ~1.0 for perfect linear fit: {}",
        result.r_squared
    );
}

#[test]
fn tc_801_08_ridge_two_params_identifies_stronger() {
    let n = 50;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 5) as f64]).collect();
    let y: Vec<f64> = (0..n).map(|i| i as f64 + 0.1 * (i % 5) as f64).collect();

    let result = compute_ridge_from_vecs(&x_matrix, &y, 0.01);

    assert_eq!(result.beta.len(), 2, "beta should have 2 entries");
    assert!(
        result.beta[0].abs() > result.beta[1].abs(),
        "x1 beta should exceed x2 beta: beta={:?}",
        result.beta
    );
}

#[test]
fn tc_801_14_rf_anova_importances_sum_to_one_per_objective() {
    let rows: Vec<TrialRow> = (0..80)
        .map(|i| {
            let x1 = i as f64 / 80.0;
            let x2 = (i as f64 / 7.0).sin();
            let y = 2.0 * x1 + 0.1 * x2;
            make_row_multi(i as u32, &[("x1", x1), ("x2", x2)], vec![y])
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    let results = compute_sensitivity_single_obj(&df, vec![Box::new(RfAnovaMetric)], 0);
    let rf_anova = results[0]
        .rf_anova
        .clone()
        .expect("rf_anova should be present");

    assert_eq!(rf_anova.0.importances.len(), 2);
    assert_eq!(rf_anova.0.importances[0].len(), 1);

    let sum: f64 = rf_anova.0.importances.iter().map(|row| row[0]).sum();
    assert!(
        (sum - 1.0).abs() < 1e-6,
        "rf_anova importances should sum to 1.0, got {}",
        sum
    );
}

#[test]
fn tc_801_15_rf_anova_small_dataset_non_zero() {
    // Small dataset (15 rows): x1 dominates, x2 is noise.
    // With holdout evaluation the RF cannot memorise eval samples,
    // so x1 should receive a clearly non-zero importance score.
    let rows: Vec<TrialRow> = (0..15)
        .map(|i| {
            let x1 = i as f64;
            let x2 = (i % 3) as f64;
            let y = x1 * 2.0 + x2 * 0.1;
            make_row_multi(i as u32, &[("x1", x1), ("x2", x2)], vec![y])
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);
    let results = compute_sensitivity_single_obj(&df, vec![Box::new(RfAnovaMetric)], 0);
    let rf = results[0]
        .rf_anova
        .clone()
        .expect("rf_anova should be present");
    let sum: f64 = rf.0.importances.iter().map(|row| row[0]).sum();
    assert!(
        sum > 0.1,
        "importances should be non-zero on small data, got sum={}",
        sum
    );
    assert!(
        rf.0.importances[0][0] > rf.0.importances[1][0],
        "x1 importance should exceed x2: x1={}, x2={}",
        rf.0.importances[0][0],
        rf.0.importances[1][0]
    );
}

#[test]
fn tc_801_p01_spearman_50000_x_30_x_4_at_scale() {
    #[cfg(debug_assertions)]
    let (n, n_params, n_objs) = (5_000usize, 10usize, 4usize);
    #[cfg(not(debug_assertions))]
    let (n, n_params, n_objs) = (50_000usize, 30usize, 4usize);

    let param_cols: Vec<Vec<f64>> = (0..n_params)
        .map(|p| (0..n).map(|i| (i + p) as f64).collect())
        .collect();
    let obj_cols: Vec<Vec<f64>> = (0..n_objs)
        .map(|o| (0..n).map(|i| (i * (o + 1)) as f64).collect())
        .collect();

    // All columns are monotonically increasing, so Spearman must be exactly 1.0 for every param x obj pair.
    // Computing every pair also serves as a smoke test that nothing breaks at scale on large input.
    for param_column in &param_cols {
        for objective_column in &obj_cols {
            let r = compute_spearman(param_column, objective_column);
            assert!(
                (r - 1.0).abs() < 1e-9,
                "monotonic columns must give Spearman ≈ 1, got {r} (n={n})"
            );
        }
    }
}

#[test]
fn tc_801_p02_ridge_50000_x_30_at_scale() {
    #[cfg(debug_assertions)]
    let (n, n_params, n_objs) = (5_000usize, 10usize, 4usize);
    #[cfg(not(debug_assertions))]
    let (n, n_params, n_objs) = (50_000usize, 30usize, 4usize);

    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n_params).map(|p| (i + p) as f64).collect())
        .collect();
    let y_vecs: Vec<Vec<f64>> = (0..n_objs)
        .map(|o| (0..n).map(|i| (i * (o + 1)) as f64).collect())
        .collect();

    // Solve Ridge for each objective and confirm exactly one coefficient is returned per parameter.
    for y in &y_vecs {
        let r = compute_ridge_from_vecs(&x_matrix, y, 1.0);
        assert_eq!(
            r.beta.len(),
            n_params,
            "ridge must return one coefficient per parameter (n={n})"
        );
    }
}

#[test]
fn tc_1610_01_build_quad_features_output_length() {
    let x = vec![1.0, 2.0, 3.0];
    let feats = build_quad_features(&x);
    assert_eq!(feats, vec![1.0, 2.0, 3.0, 1.0, 4.0, 9.0, 2.0, 3.0, 6.0]);
}

#[test]
fn tc_1610_03_compute_sobol_insufficient_data_returns_none() {
    let rows: Vec<TrialRow> = vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])];
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);
    let result = compute_sobol(&df, 1024);
    assert!(result.is_none(), "n<2 の場合 None を返すこと");
}

#[test]
fn tc_1610_04_sobol_indices_in_range() {
    let rows: Vec<TrialRow> = (0..50)
        .map(|i| {
            let x1 = i as f64;
            let x2 = (i * 2) as f64;
            let y = x1 * 2.0;
            make_row_multi(i as u32, &[("x1", x1), ("x2", x2)], vec![y])
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);
    assert!(compute_sobol(&df, 0).is_none());
    let result = compute_sobol(&df, 1024);
    assert!(result.is_some());
    let r = result.unwrap();
    for pi in 0..r.param_names.len() {
        for k in 0..r.objective_names.len() {
            assert!(r.first_order[pi][k] >= 0.0 && r.first_order[pi][k] <= 1.0);
            assert!(r.total_effect[pi][k] >= 0.0 && r.total_effect[pi][k] <= 1.0);
        }
    }
}

#[test]
fn tc_1610_05_compute_sobol_index_pair_zero_variance() {
    let fa = vec![1.0, 1.0, 1.0, 1.0];
    let fb = vec![0.5, 0.5, 0.5, 0.5];
    let fab = vec![1.2, 1.2, 1.2, 1.2];
    let (fo, te) = compute_sobol_index_pair(&fa, &fb, &fab);
    assert_eq!(fo, 0.0);
    assert_eq!(te, 0.0);
}

#[test]
fn tc_1610_06_compute_sobol_index_pair_known_values() {
    let fa = vec![1.0, 3.0];
    let fb = vec![2.0, 4.0];
    let fab = vec![1.5, 3.5];
    let (fo, te) = compute_sobol_index_pair(&fa, &fb, &fab);

    // var_y = 1, unclamped s_i = 1.5 → clamped 1.0, raw st_i = 0.125
    // To enforce ST_i >= S_i, max(0.125, 1.5) = 1.5 → clamp → 1.0
    assert!((fo - 1.0).abs() < 1e-12, "first-order mismatch: {fo}");
    assert!((te - 1.0).abs() < 1e-12, "total-effect mismatch: {te}");
}

#[test]
fn tc_1610_integration_output_shape() {
    let rows: Vec<TrialRow> = (0..100)
        .map(|i| {
            make_row_multi(
                i as u32,
                &[
                    ("p0", i as f64),
                    ("p1", (i + 1) as f64),
                    ("p2", (i + 2) as f64),
                    ("p3", (i + 3) as f64),
                    ("p4", (i + 4) as f64),
                ],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2", "p3", "p4"], &["obj0"]);
    let result = compute_sobol(&df, 512);
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.first_order.len(), 5);
    assert_eq!(r.first_order[0].len(), 1);
    assert_eq!(r.total_effect.len(), 5);
}

// --- Permutation Feature Importance tests ---

#[test]
fn tc_pfi_001_01_normal_case() {
    let n = 50;
    let p = 5;
    // Use varied data so permuting a column actually changes predictions and sum ≈ 1.0.
    let x: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..p).map(|j| i as f64 + j as f64 * 0.1).collect())
        .collect();
    let y: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let (imp, _r2) = super::tree::permutation::compute_permutation_importances(&x, &y);
    assert_eq!(imp.len(), p);
    let sum: f64 = imp.iter().sum();
    assert!((sum - 1.0).abs() < 1e-3, "expected sum ≈ 1.0, got {sum}");
}

#[test]
fn tc_pfi_001_02_single_feature() {
    let x: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64]).collect();
    let y: Vec<f64> = (0..20).map(|i| i as f64 * 2.0).collect();
    let (imp, _) = super::tree::permutation::compute_permutation_importances(&x, &y);
    assert_eq!(imp.len(), 1);
    assert!((imp[0] - 1.0).abs() < 1e-6);
}

#[test]
fn tc_pfi_001_b01_min_valid_rows() {
    let x = vec![vec![1.0], vec![2.0]];
    let y = vec![1.0, 2.0];
    let (imp, _) = super::tree::permutation::compute_permutation_importances(&x, &y);
    assert_eq!(imp.len(), 1);
}

#[test]
fn tc_pfi_001_e02_empty_input() {
    let (imp, r2) = super::tree::permutation::compute_permutation_importances(&[], &[]);
    assert!(imp.is_empty());
    assert_eq!(r2, 0.0);
}

// --- Permutation integration tests (TC-PFI-INT) ---

#[test]
fn tc_pfi_int_02_result_shape() {
    let rows: Vec<TrialRow> = (0..30)
        .map(|i| {
            make_row_multi(
                i,
                &[
                    ("p0", i as f64),
                    ("p1", (i * 2) as f64),
                    ("p2", (i % 5) as f64),
                ],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2"], &["obj0"]);
    let results = compute_sensitivity_single_obj(&df, vec![Box::new(PermutationMetric)], 0);
    assert!(!results.is_empty());
    let perm = results.into_iter().next().unwrap().permutation.unwrap();
    assert_eq!(
        perm.0.importances.len(),
        3,
        "importances should have one entry per param"
    );
    assert_eq!(
        perm.0.r_squared.len(),
        1,
        "r_squared should have one entry per objective"
    );
}

// ===========================================================================
// TASK-2263: compute_sensitivity_single_obj simplification tests
// ===========================================================================

#[test]
fn tc_2263_01_multiple_metrics_all_returned() {
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("p0", i as f64), ("p1", (i * 2) as f64)],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1"], &["obj0"]);
    let results = compute_sensitivity_single_obj(
        &df,
        vec![Box::new(SpearmanMetric), Box::new(RidgeMetric)],
        0,
    );
    assert_eq!(results.len(), 2, "both metrics should produce a result");
    assert!(
        !results[0].spearman.is_empty(),
        "first result should have spearman"
    );
    assert!(
        !results[1].ridge.is_empty(),
        "second result should have ridge"
    );
}

#[test]
fn tc_2263_02_none_excluded_from_results() {
    // n=1 → too small for any metric → all return None → empty Vec
    let rows = vec![make_row_multi(0, &[("p0", 1.0)], vec![1.0])];
    let df = setup_df(rows, &["p0"], &["obj0"]);
    let results = compute_sensitivity_single_obj(
        &df,
        vec![Box::new(SpearmanMetric), Box::new(RidgeMetric)],
        0,
    );
    assert!(
        results.is_empty(),
        "None results should be filtered: got {} results",
        results.len()
    );
}

#[test]
fn tc_2263_03_invalid_obj_idx_excluded() {
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| make_row_multi(i, &[("p0", i as f64)], vec![i as f64]))
        .collect();
    let df = setup_df(rows, &["p0"], &["obj0"]);
    let results = compute_sensitivity_single_obj(
        &df,
        vec![Box::new(SpearmanMetric)],
        99, // invalid obj_idx
    );
    assert!(
        results.is_empty(),
        "invalid obj_idx should produce no results"
    );
}

#[test]
fn public_ridge_rejects_non_finite_features_and_objectives() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let x = faer::Mat::from_fn(3, 1, |i, _| if i == 1 { bad } else { i as f64 });
        assert!(super::compute_ridge(&x, &[0.0, 1.0, 2.0], 1.0)
            .beta
            .is_empty());
        let x = faer::Mat::from_fn(3, 1, |i, _| i as f64);
        assert!(super::compute_ridge(&x, &[0.0, bad, 2.0], 1.0)
            .beta
            .is_empty());
    }
}
#[test]
fn missing_parameters_do_not_change_sensitivity_of_complete_rows() {
    let complete: Vec<TrialRow> = (0..8)
        .map(|i| TrialRow {
            trial_id: i,
            trial_number: i,
            param_display: [("x".to_string(), i as f64)].into(),
            distribution_metadata: Default::default(),
            param_category_label: Default::default(),
            objective_values: vec![2.0 * i as f64],
            user_attrs_numeric: Default::default(),
            user_attrs_string: Default::default(),
            user_attrs_json: Default::default(),
            constraint_values: vec![],
        })
        .collect();
    let mut incomplete = complete.clone();
    let mut missing = complete[0].clone();
    missing.param_display.clear();
    missing.objective_values = vec![9999.0];
    incomplete.insert(1, missing);
    let clean = setup_df(complete, &["x"], &["obj"]);
    let missing = setup_df(incomplete, &["x"], &["obj"]);
    let a = RidgeMetric.compute(&clean, 0).unwrap();
    let b = RidgeMetric.compute(&missing, 0).unwrap();
    assert_eq!(a.ridge[0].beta, b.ridge[0].beta);
    assert_eq!(a.ridge[0].r_squared, b.ridge[0].r_squared);
    let a = SpearmanMetric.compute(&clean, 0).unwrap();
    let b = SpearmanMetric.compute(&missing, 0).unwrap();
    assert_eq!(a.spearman, b.spearman);
}

#[test]
fn nominal_categories_never_receive_spearman_or_ridge_scores() {
    // Three identities, bijective relabeling, and changed first appearance.
    // Compare Ridge to the numeric-only model for each row order: holdout is seeded,
    // so row permutation is not required to preserve the numerical model's R².
    for labels in [["steel", "wood", "glass"], ["10", "2", "99"]] {
        for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
            let rows: Vec<_> = (0..30)
                .map(|i| {
                    let mut row = make_row_multi(
                        i,
                        &[("x", i as f64), ("cat", order[i as usize % 3] as f64)],
                        vec![2.0 * i as f64],
                    );
                    row.param_category_label
                        .insert("cat".into(), labels[order[i as usize % 3]].into());
                    row
                })
                .collect();
            for permuted in [false, true] {
                let mut rows = rows.clone();
                if permuted {
                    rows.rotate_left(1);
                }
                let mixed = setup_df(rows.clone(), &["cat", "x"], &["obj"]);
                let numeric = setup_df(rows.clone(), &["x"], &["obj"]);
                let categorical = setup_df(rows, &["cat"], &["obj"]);
                let a = SpearmanMetric.compute(&mixed, 0).unwrap();
                let b = SpearmanMetric.compute(&numeric, 0).unwrap();
                assert_eq!(a.param_names, vec!["x"]);
                assert_eq!(a.unsupported_categorical, vec!["cat"]);
                assert_eq!(a.spearman, b.spearman);
                let a = RidgeMetric.compute(&mixed, 0).unwrap();
                let b = RidgeMetric.compute(&numeric, 0).unwrap();
                assert_eq!(a.param_names, vec!["x"]);
                assert_eq!(a.unsupported_categorical, vec!["cat"]);
                assert_eq!(a.ridge[0].beta, b.ridge[0].beta);
                assert_eq!(a.ridge[0].r_squared, b.ridge[0].r_squared);
                for metric in [&SpearmanMetric as &dyn SensitivityMetric, &RidgeMetric] {
                    let r = metric.compute(&categorical, 0).unwrap();
                    assert!(r.param_names.is_empty());
                    assert_eq!(r.unsupported_categorical, vec!["cat"]);
                    assert!(r.spearman.is_empty());
                    assert!(r.ridge.is_empty());
                }
            }
        }
    }
}

#[test]
fn numerical_sensitivity_matches_existing_primitives_and_keeps_nan_supported() {
    let rows: Vec<_> = (0..30)
        .map(|i| {
            make_row_multi(
                i,
                &[("x", i as f64), ("z", (i % 5) as f64)],
                vec![2.0 * i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x", "z"], &["obj"]);
    let x: Vec<_> = (0..30).map(|i| vec![i as f64, (i % 5) as f64]).collect();
    let y: Vec<_> = (0..30).map(|i| 2.0 * i as f64).collect();
    let r = RidgeMetric.compute(&df, 0).unwrap();
    let expected = super::ridge::compute_ridge_result(&x, &y);
    assert_eq!(r.ridge[0].beta, expected.beta);
    assert_eq!(r.ridge[0].r_squared, expected.r_squared);
    assert!(r.unsupported_categorical.is_empty());
    let r = SpearmanMetric.compute(&df, 0).unwrap();
    for (j, name) in ["x", "z"].iter().enumerate() {
        assert_eq!(
            r.spearman[j][0],
            compute_spearman(df.get_numeric_column(name).unwrap(), &y)
        );
    }
    let df = setup_df(
        vec![
            make_row_multi(0, &[("x", f64::NAN)], vec![0.0]),
            make_row_multi(1, &[("x", f64::NAN)], vec![1.0]),
        ],
        &["x"],
        &["obj"],
    );
    let r = SpearmanMetric.compute(&df, 0).unwrap();
    assert_eq!(r.param_names, vec!["x"]);
    assert!(r.unsupported_categorical.is_empty());
    assert!(r.spearman[0][0].is_nan());
}
