//! Spearman/Ridge adapters: payloads, objective selection and readiness.

use super::metric_trait::SensitivityMetric;
use super::ridge::RidgeMetric;
use super::spearman::SpearmanMetric;
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
fn tc_2259_03_spearman_positive_correlation() {
    let rows = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);
    assert_eq!(SpearmanMetric.name(), "Spearman");
    let r = SpearmanMetric.compute(&df, 0).unwrap();
    assert_eq!(r.param_names, vec!["x1", "x2"]);
    assert_eq!(r.objective_names, vec!["obj0"]);
    assert_eq!(r.spearman.len(), 2);
    assert!(r.spearman[0][0] > 0.99);
    assert!(r.spearman[1][0] < -0.99);
    assert!(r.ridge.is_empty());
    assert!(r.rf_anova.is_none());
    assert!(r.mdi.is_none());
    assert!(r.shap.is_none());
    assert!(r.permutation.is_none());
}

#[test]
fn tc_2259_04_ridge_linear_data() {
    let rows = (0..50)
        .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![2.0 * i as f64 + 1.0]))
        .collect();
    let df = setup_df(rows, &["x1"], &["obj0"]);
    assert_eq!(RidgeMetric.name(), "Ridge");
    let r = RidgeMetric.compute(&df, 0).unwrap();
    assert_eq!(r.param_names, vec!["x1"]);
    assert_eq!(r.objective_names, vec!["obj0"]);
    assert!(r.spearman.is_empty());
    assert_eq!(r.ridge.len(), 1);
    assert_eq!(r.ridge[0].beta.len(), 1);
    assert!(r.ridge[0].r_squared > 0.99);
    assert!(r.ridge[0].beta[0] > 0.0);
    assert!(r.rf_anova.is_none());
    assert!(r.mdi.is_none());
    assert!(r.shap.is_none());
    assert!(r.permutation.is_none());
}

#[test]
fn tc_2259_12_spearman_obj_idx_1() {
    let rows = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64, (20 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);
    let r = SpearmanMetric.compute(&df, 1).unwrap();
    assert_eq!(r.objective_names, vec!["obj1"]);
    assert_eq!(r.spearman.len(), 2);
}

#[test]
fn tc_2259_13_ridge_obj_idx_1() {
    let rows = (0..30)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (30 - i) as f64)],
                vec![i as f64 * 2.0, (30 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);
    let r = RidgeMetric.compute(&df, 1).unwrap();
    assert_eq!(r.objective_names, vec!["obj1"]);
    assert_eq!(r.ridge.len(), 1);
    assert_eq!(r.ridge[0].beta.len(), 2);
}

#[test]
fn tc_2259_14_spearman_insufficient_data_n1() {
    let df = setup_df(
        vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])],
        &["x1", "x2"],
        &["obj0"],
    );
    assert!(SpearmanMetric.compute(&df, 0).is_none());
}

#[test]
fn tc_2259_15_ridge_insufficient_data_n1() {
    let df = setup_df(
        vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])],
        &["x1", "x2"],
        &["obj0"],
    );
    assert!(RidgeMetric.compute(&df, 0).is_none());
}

#[test]
fn tc_2259_16_spearman_empty_data_n0() {
    let df = setup_df(vec![], &["x1", "x2"], &["obj0"]);
    assert!(SpearmanMetric.compute(&df, 0).is_none());
}

#[test]
fn tc_2259_18_spearman_invalid_obj_idx() {
    let rows = (0..10)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (10 - i) as f64)],
                vec![i as f64, (10 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);
    assert!(SpearmanMetric.compute(&df, 5).is_none());
}

#[test]
fn tc_2259_19_ridge_invalid_obj_idx() {
    let rows = (0..10)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (10 - i) as f64)],
                vec![i as f64, (10 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);
    assert!(RidgeMetric.compute(&df, 100).is_none());
}

#[test]
fn tc_2259_20_spearman_empty_params() {
    let rows = (0..10)
        .map(|i| make_row_multi(i, &[], vec![i as f64]))
        .collect();
    let df = setup_df(rows, &[], &["obj0"]);
    assert!(SpearmanMetric.compute(&df, 0).is_none());
}

#[test]
fn tc_2259_22_spearman_min_rows_n2() {
    let rows = vec![
        make_row_multi(0, &[("x1", 1.0)], vec![1.0]),
        make_row_multi(1, &[("x1", 2.0)], vec![2.0]),
    ];
    let df = setup_df(rows, &["x1"], &["obj0"]);
    let r = SpearmanMetric.compute(&df, 0).unwrap();
    assert_eq!(r.spearman.len(), 1);
    assert!((r.spearman[0][0] - 1.0).abs() < 1e-9);
}

#[test]
fn tc_2259_23_ridge_min_rows_n2() {
    let rows = vec![
        make_row_multi(0, &[("x1", 1.0)], vec![3.0]),
        make_row_multi(1, &[("x1", 2.0)], vec![5.0]),
    ];
    let df = setup_df(rows, &["x1"], &["obj0"]);
    let r = RidgeMetric.compute(&df, 0).unwrap();
    assert_eq!(r.ridge.len(), 1);
    assert_eq!(r.ridge[0].beta.len(), 1);
}

#[test]
fn tc_2259_27_spearman_large_data() {
    let rows = (0..1000)
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
    let r = SpearmanMetric.compute(&df, 0).unwrap();
    assert_eq!(r.spearman.len(), 5);
    assert!(r
        .spearman
        .iter()
        .flatten()
        .all(|v| (-1.0..=1.0).contains(v)));
}
