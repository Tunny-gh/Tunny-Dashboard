use super::sobol::{build_quad_features, compute_sobol_index_pair};
use super::*;
use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use std::collections::HashMap;

fn make_row_multi(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        trial_number: trial_id,
        param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        param_category_label: HashMap::new(),
        objective_values: objectives,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![],
    }
}

fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
    let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
    let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
    let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
    store_dataframes(vec![df.clone()]);
    select_study(0).expect("study 0 should be selectable");
    df
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
fn tc_801_07_ridge_beta_sign_correct() {
    let n = 20;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
    let y: Vec<f64> = (0..n).map(|i| 3.0 * i as f64).collect();

    let result = compute_ridge_from_vecs(&x_matrix, &y, 0.01);

    assert!(
        result.beta[0] > 0.0,
        "beta should be positive: {}",
        result.beta[0]
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
fn tc_801_09_ridge_empty_returns_zero_r_squared() {
    let empty: Vec<Vec<f64>> = vec![];
    let result = compute_ridge_from_vecs(&empty, &[], 1.0);

    assert_eq!(result.beta.len(), 0, "beta should be empty for empty input");
    assert_eq!(result.r_squared, 0.0, "R² should be 0.0 for empty input");
}

#[test]
fn tc_801_11b_sensitivity_categorical_param_non_zero() {
    let labels = ["A", "B", "C", "A", "B", "C"];
    let y_vals = [1.0, 2.0, 3.0, 1.2, 2.2, 3.2];

    let rows: Vec<TrialRow> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let idx = match *label {
                "A" => 0.0,
                "B" => 1.0,
                _ => 2.0,
            };

            let mut param_display = HashMap::new();
            param_display.insert("cat".to_string(), idx);

            let mut param_category_label = HashMap::new();
            param_category_label.insert("cat".to_string(), (*label).to_string());

            TrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display,
                param_category_label,
                objective_values: vec![y_vals[i]],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            }
        })
        .collect();

    let df = setup_df(rows, &["cat"], &["obj0"]);
    let results = compute_sensitivity_single_obj(
        &df,
        vec![Box::new(SpearmanMetric), Box::new(RidgeMetric)],
        0,
    );

    assert_eq!(results.len(), 2, "both metrics should return results");
    let spearman_result = &results[0];
    let ridge_result = &results[1];
    assert_eq!(spearman_result.param_names, vec!["cat"]);
    assert!(
        spearman_result.spearman[0][0].abs() > 0.7,
        "categorical param should contribute to sensitivity: {}",
        spearman_result.spearman[0][0]
    );
    assert!(
        ridge_result.ridge[0].beta[0].abs() > 0.0,
        "categorical param beta should not be zero"
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
    assert_eq!(feats.len(), 9);
}

#[test]
fn tc_301_06_sobol_regression_after_seeded_rng_migration() {
    // [Test purpose]: Confirm Sobol sensitivity analysis still works correctly after the lcg_next -> SeededRng migration
    let rows: Vec<TrialRow> = (0..50)
        .map(|i| {
            let x1 = i as f64;
            let x2 = (i * 2) as f64;
            let y = x1 * 2.0;
            make_row_multi(i as u32, &[("x1", x1), ("x2", x2)], vec![y])
        })
        .collect();
    setup_df(rows, &["x1", "x2"], &["obj0"]);
    let result = compute_sobol(1024);
    assert!(
        result.is_some(),
        "SeededRng 移行後も compute_sobol が Some を返すこと"
    );
    let r = result.unwrap();
    for pi in 0..r.param_names.len() {
        for k in 0..r.objective_names.len() {
            assert!(
                r.first_order[pi][k] >= 0.0 && r.first_order[pi][k] <= 1.0,
                "first_order インデックスが [0,1] 範囲内"
            );
            assert!(
                r.total_effect[pi][k] >= 0.0 && r.total_effect[pi][k] <= 1.0,
                "total_effect インデックスが [0,1] 範囲内"
            );
        }
    }
}

#[test]
fn tc_1610_03_compute_sobol_insufficient_data_returns_none() {
    let rows: Vec<TrialRow> = vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])];
    setup_df(rows, &["x1", "x2"], &["obj0"]);
    let result = compute_sobol(1024);
    assert!(result.is_none(), "n<2 の場合 None を返すこと");
}

#[test]
fn tc_1610_03b_compute_sobol_zero_n_samples_returns_none() {
    // n_samples=0 must not let 0/0=NaN slip past the clamp and return a NaN-contaminated result.
    let rows: Vec<TrialRow> = (0..50)
        .map(|i| {
            let x1 = i as f64;
            let x2 = (i * 2) as f64;
            let y = x1 * 2.0;
            make_row_multi(i as u32, &[("x1", x1), ("x2", x2)], vec![y])
        })
        .collect();
    setup_df(rows, &["x1", "x2"], &["obj0"]);
    let result = compute_sobol(0);
    assert!(result.is_none(), "n_samples=0 の場合 None を返すこと");
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
    setup_df(rows, &["x1", "x2"], &["obj0"]);
    let result = compute_sobol(1024);
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
    setup_df(rows, &["p0", "p1", "p2", "p3", "p4"], &["obj0"]);
    let result = compute_sobol(512);
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
fn tc_pfi_001_e03_nan_filtering() {
    let mut x: Vec<Vec<f64>> = (0..50).map(|i| vec![i as f64, (i * 2) as f64]).collect();
    let mut y: Vec<f64> = (0..50).map(|i| i as f64).collect();
    x[5][0] = f64::NAN;
    y[10] = f64::INFINITY;
    let (imp, _) = super::tree::permutation::compute_permutation_importances(&x, &y);
    assert_eq!(imp.len(), 2);
    let sum: f64 = imp.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6 || sum < f64::EPSILON);
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
fn tc_pfi_int_01_single_obj_returns_some() {
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
    assert!(!results.is_empty(), "should return at least one result");
    assert!(
        results[0].permutation.is_some(),
        "permutation field should be Some"
    );
}

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
fn tc_2263_04_empty_metrics_vec_returns_empty() {
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| make_row_multi(i, &[("p0", i as f64)], vec![i as f64]))
        .collect();
    let df = setup_df(rows, &["p0"], &["obj0"]);
    let results: Vec<SensitivityResult> = compute_sensitivity_single_obj(&df, vec![], 0);
    assert!(results.is_empty());
}
