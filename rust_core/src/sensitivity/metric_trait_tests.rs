//! TASK-2259 Red phase: SensitivityMetric trait implementation tests for SpearmanMetric and RidgeMetric
//!
//! Test case definitions: docs/implements/rust-core-refactoring/TASK-2259/spearman-ridge-metric-impl-testcases.md
//! Requirements definition: docs/implements/rust-core-refactoring/TASK-2259/spearman-ridge-metric-impl-requirements.md

use super::compute_sensitivity_single_obj;
use super::metric_trait::SensitivityMetric;
use super::ridge::RidgeMetric;
use super::spearman::SpearmanMetric;
use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Test utilities
// ---------------------------------------------------------------------------

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
    select_study(0).expect("study 0 exists");
    df
}

// ===========================================================================
// Normal-path test cases
// ===========================================================================

#[test]
fn tc_2259_01_spearman_metric_name() {
    // 【Test purpose】: Verify that SpearmanMetric::name() returns the correct string "Spearman"
    // 【Test description】: Instantiate SpearmanMetric and call name()
    // 【Expected behavior】: Returns the &'static str "Spearman"
    // 🔵 Reliability level: Blue (explicitly stated in requirements doc section 2, interfaces.rs)

    // 【Test data preparation】: Confirm instantiation of the zero-sized struct
    // 【Initial condition setup】: No special preconditions
    let metric = SpearmanMetric;

    // 【Actual execution】: Call the name() method
    // 【Process description】: Check the return value of the trait method name()
    let name = metric.name();

    // 【Result verification】: The return value must exactly match "Spearman"
    // 【Expected value check】: Requirements doc section 2, name() return value table
    assert_eq!(name, "Spearman"); // 【Verification point】: The exact string is returned 🔵
}

#[test]
fn tc_2259_02_ridge_metric_name() {
    // 【Test purpose】: Verify that RidgeMetric::name() returns the correct string "Ridge"
    // 【Test description】: Instantiate RidgeMetric and call name()
    // 【Expected behavior】: Returns the &'static str "Ridge"
    // 🔵 Reliability level: Blue (explicitly stated in requirements doc section 2, interfaces.rs)

    // 【Test data preparation】: Confirm instantiation of the zero-sized struct
    // 【Initial condition setup】: No special preconditions
    let metric = RidgeMetric;

    // 【Actual execution】: Call the name() method
    // 【Process description】: Check the return value of the trait method name()
    let name = metric.name();

    // 【Result verification】: The return value must exactly match "Ridge"
    // 【Expected value check】: Requirements doc section 2, name() return value table
    assert_eq!(name, "Ridge"); // 【Verification point】: The exact string is returned 🔵
}

#[test]
fn tc_2259_03_spearman_positive_correlation() {
    // 【Test purpose】: SpearmanMetric::compute() returns a correct SensitivityResult for positively correlated data
    // 【Test description】: With a 20-row, 2-parameter DataFrame, verify the Spearman sensitivity value is computed correctly for obj_idx=0
    // 【Expected behavior】: x1-obj0 is positively correlated (>0.99), x2-obj0 is negatively correlated (<-0.99), other fields are empty/None
    // 🔵 Reliability level: Blue (requirements doc section 2 SensitivityResult definition, full.rs L57-76)

    // 【Test data preparation】: Same pattern as tc_801_11. Perfectly correlated data with x1=i, x2=20-i, y=i
    // 【Initial condition setup】: DataFrame with 20 rows, 2 parameters, 1 objective
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

    // 【Actual execution】: Call SpearmanMetric::compute()
    // 【Process description】: Spearman sensitivity computation via the trait method compute()
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: Check every field of SensitivityResult
    // 【Expected value check】: Requirements doc section 2, "SensitivityResult contents (SpearmanMetric)"
    assert!(
        result.is_some(),
        "SpearmanMetric::compute() should return Some"
    ); // 【Verification point】: Computation completes successfully and returns Some 🔵
    let r = result.unwrap();
    assert_eq!(r.param_names, vec!["x1", "x2"]); // 【Verification point】: Parameter names are set correctly 🔵
    assert_eq!(r.objective_names, vec!["obj0"]); // 【Verification point】: Only one objective name 🔵
    assert_eq!(r.spearman.len(), 2); // 【Verification point】: Sensitivity values are set for each parameter 🔵
    assert!(
        r.spearman[0][0] > 0.99,
        "x1-obj0 should be positively correlated: {}",
        r.spearman[0][0]
    ); // 【Verification point】: x1-obj0 positively correlated 🔵
    assert!(
        r.spearman[1][0] < -0.99,
        "x2-obj0 should be negatively correlated: {}",
        r.spearman[1][0]
    ); // 【Verification point】: x2-obj0 negatively correlated 🔵
    assert!(r.ridge.is_empty()); // 【Verification point】: Ridge field is empty 🔵
    assert!(r.rf_anova.is_none()); // 【Verification point】: rf_anova is None 🔵
    assert!(r.mdi.is_none()); // 【Verification point】: mdi is None 🔵
    assert!(r.shap.is_none()); // 【Verification point】: shap is None 🔵
    assert!(r.permutation.is_none()); // 【Verification point】: permutation is None 🔵
}

#[test]
fn tc_2259_04_ridge_linear_data() {
    // 【Test purpose】: RidgeMetric::compute() returns a correct SensitivityResult for linearly related data
    // 【Test description】: Verify R^2 > 0.99 for a perfectly linear 50-row, 1-parameter dataset
    // 【Expected behavior】: R^2 is 0.99 or higher, and beta has a positive sign
    // 🔵 Reliability level: Blue (requirements doc section 2, full.rs L77-89)

    // 【Test data preparation】: Same pattern as tc_801_06. Perfectly linear relationship x1=i, y=2*i+1
    // 【Initial condition setup】: DataFrame with 50 rows, 1 parameter, 1 objective
    let rows: Vec<TrialRow> = (0..50)
        .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![2.0 * i as f64 + 1.0]))
        .collect();
    let df = setup_df(rows, &["x1"], &["obj0"]);

    // 【Actual execution】: Call RidgeMetric::compute()
    // 【Process description】: Ridge sensitivity computation via the trait method compute()
    let metric = RidgeMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: Check every field of SensitivityResult
    // 【Expected value check】: Requirements doc section 2, "SensitivityResult contents (RidgeMetric)"
    assert!(
        result.is_some(),
        "RidgeMetric::compute() should return Some"
    ); // 【Verification point】: Computation completes successfully 🔵
    let r = result.unwrap();
    assert_eq!(r.param_names, vec!["x1"]); // 【Verification point】: Parameter name 🔵
    assert_eq!(r.objective_names, vec!["obj0"]); // 【Verification point】: One objective name 🔵
    assert!(r.spearman.is_empty()); // 【Verification point】: Spearman field is empty 🔵
    assert_eq!(r.ridge.len(), 1); // 【Verification point】: One element in the ridge field 🔵
    assert_eq!(r.ridge[0].beta.len(), 1); // 【Verification point】: beta has one entry per parameter 🔵
    assert!(
        r.ridge[0].r_squared > 0.99,
        "R² should be close to 1.0: {}",
        r.ridge[0].r_squared
    ); // 【Verification point】: R^2 is high 🔵
    assert!(
        r.ridge[0].beta[0] > 0.0,
        "beta should be positive: {}",
        r.ridge[0].beta[0]
    ); // 【Verification point】: beta has a positive sign 🔵
    assert!(r.rf_anova.is_none()); // 【Verification point】: rf_anova is None 🔵
    assert!(r.mdi.is_none()); // 【Verification point】: mdi is None 🔵
    assert!(r.shap.is_none()); // 【Verification point】: shap is None 🔵
    assert!(r.permutation.is_none()); // 【Verification point】: permutation is None 🔵
}

#[test]
fn tc_2259_05_spearman_matches_legacy() {
    // 【Test purpose】: SpearmanMetric::compute() returns the same result as compute_sensitivity_single_obj(Spearman)
    // 【Test description】: Compare both computation results with a 20-row, 3-parameter DataFrame using a 1e-10 floating-point tolerance
    // 【Expected behavior】: The spearman value for every parameter matches within a difference of < 1e-10
    // 🔵 Reliability level: Blue (NFR-102, direct comparison with full.rs L57-76)

    // 【Test data preparation】: Diverse data patterns for 3 parameters
    // 【Initial condition setup】: DataFrame with 20 rows, 3 parameters, 1 objective
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[
                    ("p0", i as f64),
                    ("p1", (i as f64 * 0.5).sin()),
                    ("p2", (i % 7) as f64),
                ],
                vec![i as f64 * 2.0 + 1.0],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2"], &["obj0"]);

    // 【Actual execution】: Compute with both the trait implementation and the new API, then compare
    let metric = SpearmanMetric;
    let metric_result = metric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(SpearmanMetric)], 0);

    // 【Result verification】: Confirm both computation results are identical
    assert!(metric_result.is_some()); // 【Verification point】: The trait implementation returns Some 🔵
    assert!(!api_results.is_empty()); // 【Verification point】: The new API also returns a result 🔵
    let mr = metric_result.unwrap();
    let ar = &api_results[0];
    for i in 0..mr.spearman.len() {
        let diff = (mr.spearman[i][0] - ar.spearman[i][0]).abs();
        assert!(
            diff < 1e-10,
            "Spearman mismatch at param {}: metric={}, api={}, diff={}",
            i,
            mr.spearman[i][0],
            ar.spearman[i][0],
            diff
        ); // 【Verification point】: Each parameter's sensitivity value matches within a difference of < 1e-10 🔵
    }
}

#[test]
fn tc_2259_06_ridge_matches_legacy() {
    // 【Test purpose】: RidgeMetric::compute() returns the same result as compute_sensitivity_single_obj(Ridge)
    // 【Test description】: Compare beta and r_squared with a 30-row, 3-parameter DataFrame
    // 【Expected behavior】: Every beta element and r_squared match within a difference of < 1e-10
    // 🔵 Reliability level: Blue (NFR-102, direct comparison with full.rs L77-89)

    // 【Test data preparation】: Diverse data patterns for 3 parameters
    // 【Initial condition setup】: DataFrame with 30 rows, 3 parameters, 1 objective
    let rows: Vec<TrialRow> = (0..30)
        .map(|i| {
            make_row_multi(
                i,
                &[
                    ("p0", i as f64),
                    ("p1", (i as f64 * 0.3).cos()),
                    ("p2", (i % 5) as f64),
                ],
                vec![i as f64 * 1.5 + 3.0],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2"], &["obj0"]);

    // 【Actual execution】: Compute with both the trait implementation and the new API, then compare
    let metric = RidgeMetric;
    let metric_result = metric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(RidgeMetric)], 0);

    // 【Result verification】: Confirm both computation results are identical
    assert!(metric_result.is_some()); // 【Verification point】: The trait implementation returns Some 🔵
    assert!(!api_results.is_empty()); // 【Verification point】: The new API also returns a result 🔵
    let mr = metric_result.unwrap();
    let ar = &api_results[0];
    for i in 0..mr.ridge[0].beta.len() {
        let diff = (mr.ridge[0].beta[i] - ar.ridge[0].beta[i]).abs();
        assert!(
            diff < 1e-10,
            "Ridge beta mismatch at param {}: metric={}, api={}, diff={}",
            i,
            mr.ridge[0].beta[i],
            ar.ridge[0].beta[i],
            diff
        ); // 【Verification point】: Each beta element matches within a difference of < 1e-10 🔵
    }
    let r2_diff = (mr.ridge[0].r_squared - ar.ridge[0].r_squared).abs();
    assert!(
        r2_diff < 1e-10,
        "Ridge R² mismatch: metric={}, api={}, diff={}",
        mr.ridge[0].r_squared,
        ar.ridge[0].r_squared,
        r2_diff
    ); // 【Verification point】: R^2 matches within a difference of < 1e-10 🔵
}

#[test]
fn tc_2259_09_spearman_as_trait_object() {
    // 【Test purpose】: Verify SpearmanMetric can be used as a Box<dyn SensitivityMetric>
    // 【Test description】: Call compute() and name() through the trait object
    // 【Expected behavior】: Computation proceeds normally even through the trait object
    // 🔵 Reliability level: Blue (design doc architecture.md "dispatch", Send + Sync bound in metric_trait.rs)

    // 【Test data preparation】: A basic DataFrame
    // 【Initial condition setup】: 20 rows, 2 parameters, 1 objective
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

    // 【Actual execution】: Store as a trait object and call methods on it
    // 【Process description】: Verify the polymorphic usage pattern
    let metric: Box<dyn SensitivityMetric> = Box::new(SpearmanMetric);

    // 【Result verification】: Dynamic dispatch through the trait object works correctly
    // 【Expected value check】: compute() returns Some, and name() returns the correct value
    assert_eq!(metric.name(), "Spearman"); // 【Verification point】: name() returns the correct string 🔵
    let result = metric.compute(&df, 0);
    assert!(
        result.is_some(),
        "trait object compute() should return Some"
    ); // 【Verification point】: The computation result is returned 🔵
}

#[test]
fn tc_2259_10_ridge_as_trait_object() {
    // 【Test purpose】: Verify RidgeMetric can be used as a Box<dyn SensitivityMetric>
    // 【Test description】: Call compute() and name() through the trait object
    // 【Expected behavior】: Computation proceeds normally even through the trait object
    // 🔵 Reliability level: Blue (design doc architecture.md "dispatch")

    // 【Test data preparation】: A basic DataFrame
    // 【Initial condition setup】: 20 rows, 2 parameters, 1 objective
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

    // 【Actual execution】: Store as a trait object and call methods on it
    // 【Process description】: Verify the polymorphic usage pattern
    let metric: Box<dyn SensitivityMetric> = Box::new(RidgeMetric);

    // 【Result verification】: Dynamic dispatch through the trait object works correctly
    // 【Expected value check】: compute() returns Some, and name() returns the correct value
    assert_eq!(metric.name(), "Ridge"); // 【Verification point】: name() returns the correct string 🔵
    let result = metric.compute(&df, 0);
    assert!(
        result.is_some(),
        "trait object compute() should return Some"
    ); // 【Verification point】: The computation result is returned 🔵
}

#[test]
fn tc_2259_11_multiple_metrics_vector_dispatch() {
    // 【Test purpose】: Verify multiple metrics can be stored in a Vec<Box<dyn SensitivityMetric>> and processed uniformly
    // 【Test description】: Store SpearmanMetric and RidgeMetric in the same Vec and call compute() via iteration
    // 【Expected behavior】: Each metric independently returns the correct result
    // 🔵 Reliability level: Blue (requirements doc section 1 "what the feature does", REQ-A03)

    // 【Test data preparation】: A 20-row, 2-parameter DataFrame
    // 【Initial condition setup】: Data computable by both metrics
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

    // 【Actual execution】: Store in a Vec and iterate
    // 【Process description】: A unified dispatch pattern
    let metrics: Vec<Box<dyn SensitivityMetric>> =
        vec![Box::new(SpearmanMetric), Box::new(RidgeMetric)];

    // 【Result verification】: Each metric's name() and compute() work correctly
    // 【Expected value check】: REQ-A03 no dispatch-side changes needed when adding a new metric
    assert_eq!(metrics[0].name(), "Spearman"); // 【Verification point】: Name of the first metric 🔵
    assert_eq!(metrics[1].name(), "Ridge"); // 【Verification point】: Name of the second metric 🔵

    let spearman_result = metrics[0].compute(&df, 0);
    assert!(
        spearman_result.is_some(),
        "SpearmanMetric should return Some"
    ); // 【Verification point】: Spearman computation succeeds 🔵
    assert!(!spearman_result.unwrap().spearman.is_empty()); // 【Verification point】: The spearman field is not empty 🔵

    let ridge_result = metrics[1].compute(&df, 0);
    assert!(ridge_result.is_some(), "RidgeMetric should return Some"); // 【Verification point】: Ridge computation succeeds 🔵
    assert!(!ridge_result.unwrap().ridge.is_empty()); // 【Verification point】: The ridge field is not empty 🔵
}

#[test]
fn tc_2259_12_spearman_obj_idx_1() {
    // 【Test purpose】: Verify SpearmanMetric::compute() computes correctly for obj_idx=1 (the second objective)
    // 【Test description】: Specify obj_idx=1 with a 20-row, 2-parameter, 2-objective DataFrame
    // 【Expected behavior】: The Spearman sensitivity value for the second objective is computed correctly
    // 🔵 Reliability level: Blue (requirements doc section 2, obj_idx logic in full.rs)

    // 【Test data preparation】: Data with 2 objectives. obj0=i, obj1=20-i
    // 【Initial condition setup】: 20 rows, 2 parameters, 2 objectives
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64, (20 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

    // 【Actual execution】: Call SpearmanMetric::compute() with obj_idx=1
    // 【Process description】: Sensitivity computation for the second objective
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 1);

    // 【Result verification】: objective_names correctly has exactly one element
    // 【Expected value check】: When obj_idx=1, objective_names == ["obj1"]
    assert!(result.is_some()); // 【Verification point】: Computation succeeds 🔵
    let r = result.unwrap();
    assert_eq!(r.objective_names, vec!["obj1"]); // 【Verification point】: Only the second objective name 🔵
    assert_eq!(r.spearman.len(), 2); // 【Verification point】: Sensitivity values for each parameter 🔵
}

#[test]
fn tc_2259_13_ridge_obj_idx_1() {
    // 【Test purpose】: Verify RidgeMetric::compute() computes correctly for obj_idx=1 (the second objective)
    // 【Test description】: Specify obj_idx=1 with a 30-row, 2-parameter, 2-objective DataFrame
    // 【Expected behavior】: The RidgeResult for the second objective is computed correctly
    // 🔵 Reliability level: Blue (requirements doc section 2, full.rs L77-89)

    // 【Test data preparation】: Data with 2 objectives
    // 【Initial condition setup】: 30 rows, 2 parameters, 2 objectives
    let rows: Vec<TrialRow> = (0..30)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (30 - i) as f64)],
                vec![i as f64 * 2.0, (30 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

    // 【Actual execution】: Call RidgeMetric::compute() with obj_idx=1
    // 【Process description】: Ridge regression for the second objective
    let metric = RidgeMetric;
    let result = metric.compute(&df, 1);

    // 【Result verification】: Check the structure of objective_names and ridge
    // 【Expected value check】: When obj_idx=1, objective_names == ["obj1"]
    assert!(result.is_some()); // 【Verification point】: Computation succeeds 🔵
    let r = result.unwrap();
    assert_eq!(r.objective_names, vec!["obj1"]); // 【Verification point】: Only the second objective name 🔵
    assert_eq!(r.ridge.len(), 1); // 【Verification point】: One element in ridge 🔵
    assert_eq!(r.ridge[0].beta.len(), 2); // 【Verification point】: beta has one entry per parameter 🔵
}

// ===========================================================================
// Abnormal-path test cases
// ===========================================================================

#[test]
fn tc_2259_14_spearman_insufficient_data_n1() {
    // 【Test purpose】: Verify SpearmanMetric::compute() returns None on insufficient data (n=1)
    // 【Test description】: Call compute() with a 1-row DataFrame and confirm it returns None
    // 【Expected behavior】: Returns None without panicking
    // 🔵 Reliability level: Blue (EDGE-2259-01, completion criterion "returns None without panicking on insufficient data")

    // 【Test data preparation】: A DataFrame with only 1 row
    // 【Initial condition setup】: 1 row, 2 parameters, 1 objective
    let rows = vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])];
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【Actual execution】: Call SpearmanMetric::compute()
    // 【Process description】: Error handling for insufficient data
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: None is returned (no panic)
    // 【Expected value check】: None when n < 2
    assert!(
        result.is_none(),
        "SpearmanMetric should return None when n < 2"
    ); // 【Verification point】: None on insufficient data 🔵
}

#[test]
fn tc_2259_15_ridge_insufficient_data_n1() {
    // 【Test purpose】: Verify RidgeMetric::compute() returns None on insufficient data (n=1)
    // 【Test description】: Call compute() with a 1-row DataFrame and confirm it returns None
    // 【Expected behavior】: Returns None without panicking
    // 🔵 Reliability level: Blue (EDGE-2259-01)

    // 【Test data preparation】: A DataFrame with only 1 row
    // 【Initial condition setup】: 1 row, 2 parameters, 1 objective
    let rows = vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])];
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【Actual execution】: Call RidgeMetric::compute()
    // 【Process description】: Error handling for insufficient data
    let metric = RidgeMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: None is returned (no panic)
    // 【Expected value check】: None when n < 2
    assert!(
        result.is_none(),
        "RidgeMetric should return None when n < 2"
    ); // 【Verification point】: None on insufficient data 🔵
}

#[test]
fn tc_2259_16_spearman_empty_data_n0() {
    // 【Test purpose】: Verify SpearmanMetric::compute() returns None on empty data (n=0)
    // 【Test description】: Call compute() with a 0-row DataFrame and confirm it returns None
    // 【Expected behavior】: Returns None without panicking
    // 🔵 Reliability level: Blue (EDGE-2259-01, the n < 2 check in full.rs L29)

    // 【Test data preparation】: A DataFrame with 0 rows
    // 【Initial condition setup】: An empty DataFrame (parameter and objective names are already set)
    let rows: Vec<TrialRow> = vec![];
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【Actual execution】: Call SpearmanMetric::compute()
    // 【Process description】: Error handling for empty data
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: None is returned (no panic)
    // 【Expected value check】: None when n = 0 < 2
    assert!(
        result.is_none(),
        "SpearmanMetric should return None when n = 0"
    ); // 【Verification point】: None on empty data 🔵
}

#[test]
fn tc_2259_18_spearman_invalid_obj_idx() {
    // 【Test purpose】: Verify SpearmanMetric::compute() returns None for an invalid (out-of-range) obj_idx
    // 【Test description】: Specify obj_idx=5 with a 2-objective DataFrame
    // 【Expected behavior】: Returns None without panicking
    // 🔵 Reliability level: Blue (EDGE-2259-03, the get(obj_idx) check in full.rs L25-27)

    // 【Test data preparation】: A 10-row, 2-parameter, 2-objective DataFrame
    // 【Initial condition setup】: obj_idx=5 specified while objective_col_names.len() == 2
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

    // 【Actual execution】: Call compute() with the out-of-range obj_idx=5
    // 【Process description】: Verify the index bounds check
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 5);

    // 【Result verification】: None is returned (no panic)
    // 【Expected value check】: None when obj_idx >= objective_names.len()
    assert!(
        result.is_none(),
        "SpearmanMetric should return None for out-of-range obj_idx"
    ); // 【Verification point】: None when out of range 🔵
}

#[test]
fn tc_2259_19_ridge_invalid_obj_idx() {
    // 【Test purpose】: Verify RidgeMetric::compute() returns None for an invalid (out-of-range) obj_idx
    // 【Test description】: Specify obj_idx=100 with a 2-objective DataFrame
    // 【Expected behavior】: Returns None without panicking
    // 🔵 Reliability level: Blue (EDGE-2259-03)

    // 【Test data preparation】: A 10-row, 2-parameter, 2-objective DataFrame
    // 【Initial condition setup】: obj_idx=100 specified while objective_col_names.len() == 2
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

    // 【Actual execution】: Call compute() with the out-of-range obj_idx=100
    // 【Process description】: Verify the index bounds check
    let metric = RidgeMetric;
    let result = metric.compute(&df, 100);

    // 【Result verification】: None is returned (no panic)
    // 【Expected value check】: None when obj_idx >= objective_names.len()
    assert!(
        result.is_none(),
        "RidgeMetric should return None for out-of-range obj_idx"
    ); // 【Verification point】: None when out of range 🔵
}

// ===========================================================================
// Boundary-value test cases
// ===========================================================================

#[test]
fn tc_2259_20_spearman_empty_params() {
    // 【Test purpose】: Verify SpearmanMetric::compute() returns None when there are no parameters (param_names is empty)
    // 【Test description】: Call compute() with a 0-parameter DataFrame
    // 【Expected behavior】: Returns None without panicking
    // 🔵 Reliability level: Blue (EDGE-2259-02, the param_names.is_empty() check in full.rs L29)

    // 【Test data preparation】: A DataFrame with no parameters
    // 【Initial condition setup】: 10 rows, 0 parameters, 1 objective
    let rows: Vec<TrialRow> = (0..10)
        .map(|i| make_row_multi(i, &[], vec![i as f64]))
        .collect();
    let df = setup_df(rows, &[], &["obj0"]);

    // 【Actual execution】: Call SpearmanMetric::compute()
    // 【Process description】: Error handling for an empty parameter list
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: None is returned
    // 【Expected value check】: None when param_names.is_empty()
    assert!(
        result.is_none(),
        "SpearmanMetric should return None when param_names is empty"
    ); // 【Verification point】: None with empty parameters 🔵
}

#[test]
fn tc_2259_22_spearman_min_rows_n2() {
    // 【Test purpose】: Verify SpearmanMetric::compute() returns a correct result at the minimum row count (n=2)
    // 【Test description】: Confirm Spearman = 1.0 with 2-row, 1-parameter perfectly positively correlated data
    // 【Expected behavior】: Returns Some at n=2, with an accurate spearman value
    // 🔵 Reliability level: Blue (the n < 2 check at spearman.rs L77-79, full.rs L29)

    // 【Test data preparation】: Minimal-row-count perfectly positively correlated data
    // 【Initial condition setup】: 2 rows, 1 parameter, 1 objective, x1=[1,2], y=[1,2]
    let rows = vec![
        make_row_multi(0, &[("x1", 1.0)], vec![1.0]),
        make_row_multi(1, &[("x1", 2.0)], vec![2.0]),
    ];
    let df = setup_df(rows, &["x1"], &["obj0"]);

    // 【Actual execution】: Call SpearmanMetric::compute()
    // 【Process description】: Computation at the minimum row count
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: Returns Some, and the perfect positive correlation is detected
    // 【Expected value check】: Normal computation at n=2, spearman = 1.0
    assert!(result.is_some()); // 【Verification point】: Returns Some at n=2 🔵
    let r = result.unwrap();
    assert_eq!(r.spearman.len(), 1); // 【Verification point】: One entry for the one parameter 🔵
    assert!(
        (r.spearman[0][0] - 1.0).abs() < 1e-9,
        "n=2 perfect positive: {}",
        r.spearman[0][0]
    ); // 【Verification point】: Perfect positive correlation 🔵
}

#[test]
fn tc_2259_23_ridge_min_rows_n2() {
    // 【Test purpose】: Verify RidgeMetric::compute() returns a correct RidgeResult at the minimum row count (n=2)
    // 【Test description】: Confirm the Ridge computation proceeds normally with 2-row, 1-parameter data
    // 【Expected behavior】: Returns Some at n=2, with a result set in the ridge field
    // 🔵 Reliability level: Blue (the n < 2 check at ridge.rs L136)

    // 【Test data preparation】: Minimal-row-count linear data
    // 【Initial condition setup】: 2 rows, 1 parameter, 1 objective, x1=[1,2], y=[3,5]
    let rows = vec![
        make_row_multi(0, &[("x1", 1.0)], vec![3.0]),
        make_row_multi(1, &[("x1", 2.0)], vec![5.0]),
    ];
    let df = setup_df(rows, &["x1"], &["obj0"]);

    // 【Actual execution】: Call RidgeMetric::compute()
    // 【Process description】: Ridge computation at the minimum row count
    let metric = RidgeMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: Returns Some, and RidgeResult is built correctly
    // 【Expected value check】: Normal computation at n=2
    assert!(result.is_some()); // 【Verification point】: Returns Some at n=2 🔵
    let r = result.unwrap();
    assert_eq!(r.ridge.len(), 1); // 【Verification point】: One element in ridge 🔵
    assert_eq!(r.ridge[0].beta.len(), 1); // 【Verification point】: beta has one entry per parameter 🔵
}

#[test]
fn tc_2259_27_spearman_large_data() {
    // 【Test purpose】: Verify SpearmanMetric::compute() returns a correct result on large-scale data (1000 rows)
    // 【Test description】: Confirm the computation completes normally with a 1000-row, 5-parameter DataFrame
    // 【Expected behavior】: All spearman values fall within [-1.0, 1.0]
    // 🔵 Reliability level: Blue (compute_spearman is an existing function whose accuracy is already guaranteed)

    // 【Test data preparation】: Large-scale multi-parameter data with 1000 rows
    // 【Initial condition setup】: 1000 rows, 5 parameters, 1 objective
    let rows: Vec<TrialRow> = (0..1000)
        .map(|i| {
            make_row_multi(
                i,
                &[
                    ("p0", i as f64),
                    ("p1", (i as f64 * 0.01).sin()),
                    ("p2", (i % 10) as f64),
                    ("p3", (i as f64).ln()),
                    ("p4", (i as f64 * 0.5).cos()),
                ],
                vec![i as f64 * 2.0],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2", "p3", "p4"], &["obj0"]);

    // 【Actual execution】: Call SpearmanMetric::compute()
    // 【Process description】: Computation on large-scale data
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【Result verification】: The computation completes normally and values are within range
    // 【Expected value check】: An accurate SensitivityResult is returned even for large-scale data
    assert!(result.is_some()); // 【Verification point】: Returns Some for large-scale data 🔵
    let r = result.unwrap();
    assert_eq!(r.spearman.len(), 5); // 【Verification point】: One entry per parameter (5) 🔵
    for (i, param_vals) in r.spearman.iter().enumerate() {
        for (j, val) in param_vals.iter().enumerate() {
            assert!(
                *val >= -1.0 && *val <= 1.0,
                "spearman[{}][{}] = {} out of [-1, 1]",
                i,
                j,
                val
            ); // 【Verification point】: The value is within [-1, 1] 🔵
        }
    }
}

#[test]
fn tc_2259_29_spearman_send_sync() {
    // 【Test purpose】: Confirm at compile time that SpearmanMetric automatically implements Send + Sync
    // 【Test description】: Verify the Send + Sync bound with a compile-time type-check function
    // 【Expected behavior】: Compilation succeeds (a zero-sized struct is automatically Send + Sync)
    // 🔵 Reliability level: Blue (Send + Sync bound in metric_trait.rs, automatically derived by the Rust type system)

    // 【Test data preparation】: Not required (compile-time check)
    // 【Initial condition setup】: A type-level check
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<SpearmanMetric>(); // 【Verification point】: SpearmanMetric satisfies Send + Sync 🔵
}

#[test]
fn tc_2259_30_ridge_send_sync() {
    // 【Test purpose】: Confirm at compile time that RidgeMetric automatically implements Send + Sync
    // 【Test description】: Verify the Send + Sync bound with a compile-time type-check function
    // 【Expected behavior】: Compilation succeeds
    // 🔵 Reliability level: Blue (Send + Sync bound in metric_trait.rs)

    // 【Test data preparation】: Not required (compile-time check)
    // 【Initial condition setup】: A type-level check
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<RidgeMetric>(); // 【Verification point】: RidgeMetric satisfies Send + Sync 🔵
}
