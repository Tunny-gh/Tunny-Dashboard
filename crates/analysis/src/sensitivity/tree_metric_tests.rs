//! Tree metric adapters: payload routing and shared readiness boundaries.

use super::metric_trait::SensitivityMetric;
use super::metrics::{MdiMetric, PermutationMetric, RfAnovaMetric, ShapMetric};
use crate::dataframe::{DataFrame, TrialRow};
use std::collections::HashMap;

fn make_row(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
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

fn large_df(n: usize) -> DataFrame {
    let rows = (0..n)
        .map(|i| {
            make_row(
                i as u32,
                &[("x1", i as f64), ("x2", (n - i) as f64)],
                vec![i as f64 * 2.0 + 1.0],
            )
        })
        .collect();
    setup_df(rows, &["x1", "x2"], &["obj0"])
}

#[test]
fn tc_2260_05_rf_anova_compute_valid() {
    let df = large_df(50);
    assert_eq!(RfAnovaMetric.name(), "RfAnova");
    let r = RfAnovaMetric.compute(&df, 0).unwrap();
    assert_eq!(r.param_names, vec!["x1", "x2"]);
    assert_eq!(r.objective_names, vec!["obj0"]);
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.mdi.is_none());
    assert!(r.shap.is_none());
    assert!(r.permutation.is_none());
    let rf = r.rf_anova.unwrap().0;
    assert_eq!(rf.importances.len(), 2);
    assert_eq!(rf.importances[0].len(), 1);
    assert_eq!(rf.r_squared.len(), 1);
}

#[test]
fn tc_2260_06_mdi_compute_valid() {
    let df = large_df(50);
    assert_eq!(MdiMetric.name(), "Mdi");
    let r = MdiMetric.compute(&df, 0).unwrap();
    assert!(r.mdi.is_some());
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.rf_anova.is_none());
    assert!(r.shap.is_none());
    assert!(r.permutation.is_none());
}

#[test]
fn tc_2260_07_shap_compute_valid() {
    let df = large_df(50);
    assert_eq!(ShapMetric.name(), "Shap");
    let r = ShapMetric.compute(&df, 0).unwrap();
    assert!(r.shap.is_some());
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.rf_anova.is_none());
    assert!(r.mdi.is_none());
    assert!(r.permutation.is_none());
}

#[test]
fn tc_2260_08_permutation_compute_valid() {
    let df = large_df(50);
    assert_eq!(PermutationMetric.name(), "Permutation");
    let r = PermutationMetric.compute(&df, 0).unwrap();
    assert!(r.permutation.is_some());
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.rf_anova.is_none());
    assert!(r.mdi.is_none());
    assert!(r.shap.is_none());
}

#[test]
fn tree_metrics_reject_invalid_or_insufficient_inputs() {
    let single = setup_df(
        vec![make_row(0, &[("x1", 1.0)], vec![1.0])],
        &["x1"],
        &["obj0"],
    );
    let empty = setup_df(vec![], &["x1"], &["obj0"]);
    let valid = large_df(20);
    for metric in [
        &RfAnovaMetric as &dyn SensitivityMetric,
        &MdiMetric,
        &ShapMetric,
        &PermutationMetric,
    ] {
        assert!(metric.compute(&single, 0).is_none());
        assert!(metric.compute(&empty, 0).is_none());
        assert!(metric.compute(&valid, 5).is_none());
    }
}
