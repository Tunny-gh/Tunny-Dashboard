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
    select_study(0).expect("study 0 translated");
    df
}

#[test]
fn tc_801_01_spearman_perfect_positive() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];

    let r = compute_spearman(&x, &y);

    assert!(
        (r - 1.0).abs() < 1e-9,
        "translatedSpearmantranslated1.0translated: {}",
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
        "translatedSpearmantranslated-1.0translated: {}",
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
        "Spearmantranslated: expected={}, got={}",
        expected,
        r
    );
}

#[test]
fn tc_801_04_spearman_tied_ranks() {
    let x = vec![1.0, 2.0, 2.0, 3.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];

    let r = compute_spearman(&x, &y);

    assert!(r > 0.9, "translated: {}", r);
}

#[test]
fn tc_801_05_spearman_n_less_than_2_returns_zero() {
    let r1 = compute_spearman(&[], &[]);
    let r2 = compute_spearman(&[1.0], &[1.0]);

    assert_eq!(r1, 0.0, "translated0.0translated");
    assert_eq!(r2, 0.0, "n=1translated0.0translated");
}

#[test]
fn tc_801_06_ridge_perfect_linear_r_squared_near_1() {
    let n = 50;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
    let y: Vec<f64> = (0..n).map(|i| 2.0 * i as f64 + 1.0).collect();

    let result = compute_ridge_from_vecs(&x_matrix, &y, 0.001);

    assert!(
        result.r_squared > 0.99,
        "translatedR²translated1.0translated: {}",
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
        "translatedβ>0translated: {}",
        result.beta[0]
    );
}

#[test]
fn tc_801_08_ridge_two_params_identifies_stronger() {
    let n = 50;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 5) as f64]).collect();
    let y: Vec<f64> = (0..n).map(|i| i as f64 + 0.1 * (i % 5) as f64).collect();

    let result = compute_ridge_from_vecs(&x_matrix, &y, 0.01);

    assert_eq!(result.beta.len(), 2, "βtranslated2translated");
    assert!(
        result.beta[0].abs() > result.beta[1].abs(),
        "x1translatedx2translated: beta={:?}",
        result.beta
    );
}

#[test]
fn tc_801_09_ridge_empty_returns_zero_r_squared() {
    let empty: Vec<Vec<f64>> = vec![];
    let result = compute_ridge_from_vecs(&empty, &[], 1.0);

    assert_eq!(result.beta.len(), 0, "translatedβtranslated");
    assert_eq!(result.r_squared, 0.0, "translatedR²=0.0");
}

#[test]
fn tc_801_10_sensitivity_all_correct_structure() {
    let rows: Vec<TrialRow> = (0..10)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (10 - i) as f64)],
                vec![i as f64, (10 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

    let result = compute_sensitivity_all(&df);

    assert_eq!(result.param_names.len(), 2);
    assert_eq!(result.objective_names.len(), 2);
    assert_eq!(result.spearman.len(), 2);
    assert_eq!(result.spearman[0].len(), 2);
    assert_eq!(result.ridge.len(), 2);
    assert!(result.rf_anova.is_some(), "rf_anova should be present");
    let rf_anova = result.rf_anova.as_ref().unwrap();
    assert_eq!(rf_anova.0.importances.len(), 2);
    assert_eq!(rf_anova.0.importances[0].len(), 2);
}

#[test]
fn tc_801_11_sensitivity_all_known_correlations() {
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    let result = compute_sensitivity_all(&df);

    assert!(
        result.spearman[0][0] > 0.99,
        "x1-obj0translated: {}",
        result.spearman[0][0]
    );
    assert!(
        result.spearman[1][0] < -0.99,
        "x2-obj0translated: {}",
        result.spearman[1][0]
    );
}

#[test]
fn tc_801_11b_sensitivity_all_categorical_param_non_zero() {
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
    let result = compute_sensitivity_all(&df);

    assert_eq!(result.param_names, vec!["cat"]);
    assert_eq!(result.objective_names, vec!["obj0"]);
    assert!(
        result.spearman[0][0].abs() > 0.7,
        "categorical param should contribute to sensitivity: {}",
        result.spearman[0][0]
    );
    assert!(
        result.ridge[0].beta[0].abs() > 0.0,
        "categorical param beta should not be zero"
    );
}

#[test]
fn tc_801_12_sensitivity_selected_subset() {
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![i as f64]))
        .collect();
    setup_df(rows, &["x1"], &["obj0"]);

    let indices: Vec<u32> = (0..10).collect();
    let result = compute_sensitivity_selected(&indices).expect("translated");

    assert_eq!(result.param_names, vec!["x1"]);
    assert_eq!(result.objective_names, vec!["obj0"]);
    assert!(
        result.spearman[0][0] > 0.99,
        "translated: {}",
        result.spearman[0][0]
    );
    assert!(result.rf_anova.is_some(), "rf_anova should be present");
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

    let result = compute_sensitivity_all(&df);
    let rf_anova = result.rf_anova.expect("rf_anova should be present");

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
    let result = compute_sensitivity_all(&df);
    let rf = result.rf_anova.expect("rf_anova should be present");
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
fn tc_801_13_sensitivity_selected_empty_indices() {
    let rows: Vec<TrialRow> = (0..5)
        .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![i as f64]))
        .collect();
    setup_df(rows, &["x1"], &["obj0"]);

    let result = compute_sensitivity_selected(&[]).expect("translated");

    assert!(result.spearman.is_empty(), "translatedspearmantranslated");
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

    // 全列が単調増加なので、どの param×obj ペアでも Spearman は厳密に 1.0 になる。
    // 全ペアを計算することで大規模入力でも破綻しないこと（スケール時のスモーク）も兼ねる。
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

    // 各目的について Ridge を解き、パラメータごとに係数が 1 つ返ることを確認する。
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
fn tc_801_p03_sensitivity_selected_at_scale() {
    // RF-ANOVA trains a random forest on this path; use a smaller debug dataset
    // so the (unoptimised) run stays reasonable while still exercising the path.
    #[cfg(debug_assertions)]
    let n = 200usize;
    #[cfg(not(debug_assertions))]
    let n = 2_000usize;

    let rows: Vec<TrialRow> = (0..n)
        .map(|i| make_row_multi(i as u32, &[("x1", i as f64)], vec![i as f64; 4]))
        .collect();
    setup_df(rows, &["x1"], &["obj0", "obj1", "obj2", "obj3"]);

    let indices: Vec<u32> = (0..n as u32).collect();
    let result = compute_sensitivity_selected(&indices).expect("sensitivity must compute");

    assert_eq!(
        result.param_names.len(),
        1,
        "one parameter (x1) was provided"
    );
    assert!(
        !result.spearman.is_empty(),
        "Spearman scores must be produced (n={n})"
    );
}

#[test]
fn tc_1610_01_build_quad_features_output_length() {
    let x = vec![1.0, 2.0, 3.0];
    let feats = build_quad_features(&x);
    assert_eq!(feats.len(), 9);
}

#[test]
fn tc_301_06_sobol_regression_after_seeded_rng_migration() {
    // 【テスト目的】: lcg_next → SeededRng 移行後も Sobol 感度解析が正常動作することを確認
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

    // var_y = 1, unclamped first-order = 1.5 -> clamped 1.0, total-effect = 0.125
    assert!((fo - 1.0).abs() < 1e-12, "first-order mismatch: {fo}");
    assert!((te - 0.125).abs() < 1e-12, "total-effect mismatch: {te}");
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
// TASK-2263: compute_sensitivity_single_obj 簡略化テスト
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
