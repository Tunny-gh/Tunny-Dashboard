//! TASK-2260: RfAnovaMetric・MdiMetric・ShapMetric・PermutationMetric の SensitivityMetric トレイト実装テスト

use super::metric_trait::SensitivityMetric;
use super::metrics::{MdiMetric, PermutationMetric, RfAnovaMetric, ShapMetric};
use super::compute_sensitivity_single_obj;
use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use std::collections::HashMap;

fn make_row(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
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

fn large_df(n: usize) -> DataFrame {
    let rows: Vec<TrialRow> = (0..n)
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

// ===========================================================================
// name() テスト
// ===========================================================================

#[test]
fn tc_2260_01_rf_anova_metric_name() {
    assert_eq!(RfAnovaMetric.name(), "RfAnova");
}

#[test]
fn tc_2260_02_mdi_metric_name() {
    assert_eq!(MdiMetric.name(), "Mdi");
}

#[test]
fn tc_2260_03_shap_metric_name() {
    assert_eq!(ShapMetric.name(), "Shap");
}

#[test]
fn tc_2260_04_permutation_metric_name() {
    assert_eq!(PermutationMetric.name(), "Permutation");
}

// ===========================================================================
// compute() 正常系: Some が返り正しいフィールドが設定される
// ===========================================================================

#[test]
fn tc_2260_05_rf_anova_compute_valid() {
    let df = large_df(50);
    let result = RfAnovaMetric.compute(&df, 0);
    assert!(result.is_some(), "RfAnovaMetric should return Some for 50-row data");
    let r = result.unwrap();
    assert_eq!(r.param_names, vec!["x1", "x2"]);
    assert_eq!(r.objective_names, vec!["obj0"]);
    assert!(r.rf_anova.is_some(), "rf_anova should be Some");
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.mdi.is_none());
    assert!(r.shap.is_none());
    assert!(r.permutation.is_none());
    let rf = r.rf_anova.unwrap().0;
    assert_eq!(rf.importances.len(), 2, "importances should have param_count entries");
    assert_eq!(rf.importances[0].len(), 1, "each entry should have 1 objective");
    assert_eq!(rf.r_squared.len(), 1);
}

#[test]
fn tc_2260_06_mdi_compute_valid() {
    let df = large_df(50);
    let result = MdiMetric.compute(&df, 0);
    assert!(result.is_some(), "MdiMetric should return Some for 50-row data");
    let r = result.unwrap();
    assert!(r.mdi.is_some(), "mdi should be Some");
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.rf_anova.is_none());
    assert!(r.shap.is_none());
    assert!(r.permutation.is_none());
}

#[test]
fn tc_2260_07_shap_compute_valid() {
    let df = large_df(50);
    let result = ShapMetric.compute(&df, 0);
    assert!(result.is_some(), "ShapMetric should return Some for 50-row data");
    let r = result.unwrap();
    assert!(r.shap.is_some(), "shap should be Some");
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.rf_anova.is_none());
    assert!(r.mdi.is_none());
    assert!(r.permutation.is_none());
}

#[test]
fn tc_2260_08_permutation_compute_valid() {
    let df = large_df(50);
    let result = PermutationMetric.compute(&df, 0);
    assert!(result.is_some(), "PermutationMetric should return Some for 50-row data");
    let r = result.unwrap();
    assert!(r.permutation.is_some(), "permutation should be Some");
    assert!(r.spearman.is_empty());
    assert!(r.ridge.is_empty());
    assert!(r.rf_anova.is_none());
    assert!(r.mdi.is_none());
    assert!(r.shap.is_none());
}

// ===========================================================================
// compute() 異常系: データ不足で None を返す
// ===========================================================================

#[test]
fn tc_2260_09_rf_anova_insufficient_data_n1() {
    let rows = vec![make_row(0, &[("x1", 1.0)], vec![1.0])];
    let df = setup_df(rows, &["x1"], &["obj0"]);
    assert!(RfAnovaMetric.compute(&df, 0).is_none(), "should return None when n=1");
}

#[test]
fn tc_2260_10_mdi_insufficient_data_n1() {
    let rows = vec![make_row(0, &[("x1", 1.0)], vec![1.0])];
    let df = setup_df(rows, &["x1"], &["obj0"]);
    assert!(MdiMetric.compute(&df, 0).is_none(), "should return None when n=1");
}

#[test]
fn tc_2260_11_shap_insufficient_data_n1() {
    let rows = vec![make_row(0, &[("x1", 1.0)], vec![1.0])];
    let df = setup_df(rows, &["x1"], &["obj0"]);
    assert!(ShapMetric.compute(&df, 0).is_none(), "should return None when n=1");
}

#[test]
fn tc_2260_12_permutation_insufficient_data_n1() {
    let rows = vec![make_row(0, &[("x1", 1.0)], vec![1.0])];
    let df = setup_df(rows, &["x1"], &["obj0"]);
    assert!(PermutationMetric.compute(&df, 0).is_none(), "should return None when n=1");
}

#[test]
fn tc_2260_13_all_metrics_invalid_obj_idx() {
    let df = large_df(20);
    assert!(RfAnovaMetric.compute(&df, 5).is_none(), "RfAnova: out-of-range obj_idx should return None");
    assert!(MdiMetric.compute(&df, 5).is_none(), "Mdi: out-of-range obj_idx should return None");
    assert!(ShapMetric.compute(&df, 5).is_none(), "Shap: out-of-range obj_idx should return None");
    assert!(PermutationMetric.compute(&df, 5).is_none(), "Permutation: out-of-range obj_idx should return None");
}

#[test]
fn tc_2260_14_all_metrics_empty_data_n0() {
    let df = setup_df(vec![], &["x1"], &["obj0"]);
    assert!(RfAnovaMetric.compute(&df, 0).is_none());
    assert!(MdiMetric.compute(&df, 0).is_none());
    assert!(ShapMetric.compute(&df, 0).is_none());
    assert!(PermutationMetric.compute(&df, 0).is_none());
}

// ===========================================================================
// compute() 結果が legacy compute_sensitivity_single_obj と一致する
// ===========================================================================

#[test]
fn tc_2260_15_rf_anova_matches_new_api() {
    let df = large_df(50);
    let metric_result = RfAnovaMetric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(RfAnovaMetric)], 0);

    assert!(metric_result.is_some());
    assert!(!api_results.is_empty());
    let mr = metric_result.unwrap().rf_anova.unwrap().0;
    let ar = api_results.into_iter().next().unwrap().rf_anova.unwrap().0;
    assert_eq!(mr.importances.len(), ar.importances.len());
    for i in 0..mr.importances.len() {
        let diff = (mr.importances[i][0] - ar.importances[i][0]).abs();
        assert!(
            diff < 1e-10,
            "RfAnova importances[{}][0] mismatch: {} vs {}, diff={}",
            i, mr.importances[i][0], ar.importances[i][0], diff
        );
    }
    let r2_diff = (mr.r_squared[0] - ar.r_squared[0]).abs();
    assert!(r2_diff < 1e-10, "RfAnova r_squared mismatch: {} vs {}", mr.r_squared[0], ar.r_squared[0]);
}

#[test]
fn tc_2260_16_mdi_matches_new_api() {
    let df = large_df(50);
    let metric_result = MdiMetric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(MdiMetric)], 0);

    assert!(metric_result.is_some());
    assert!(!api_results.is_empty());
    let mr = metric_result.unwrap().mdi.unwrap().0;
    let ar = api_results.into_iter().next().unwrap().mdi.unwrap().0;
    assert_eq!(mr.importances.len(), ar.importances.len());
    for i in 0..mr.importances.len() {
        let diff = (mr.importances[i][0] - ar.importances[i][0]).abs();
        assert!(
            diff < 1e-10,
            "Mdi importances[{}][0] mismatch: {} vs {}, diff={}",
            i, mr.importances[i][0], ar.importances[i][0], diff
        );
    }
    let r2_diff = (mr.r_squared[0] - ar.r_squared[0]).abs();
    assert!(r2_diff < 1e-10, "Mdi r_squared mismatch: {} vs {}", mr.r_squared[0], ar.r_squared[0]);
}

#[test]
fn tc_2260_17_shap_matches_new_api() {
    let df = large_df(50);
    let metric_result = ShapMetric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(ShapMetric)], 0);

    assert!(metric_result.is_some());
    assert!(!api_results.is_empty());
    let mr = metric_result.unwrap().shap.unwrap().0;
    let ar = api_results.into_iter().next().unwrap().shap.unwrap().0;
    assert_eq!(mr.importances.len(), ar.importances.len());
    for i in 0..mr.importances.len() {
        let diff = (mr.importances[i][0] - ar.importances[i][0]).abs();
        assert!(
            diff < 1e-10,
            "Shap importances[{}][0] mismatch: {} vs {}, diff={}",
            i, mr.importances[i][0], ar.importances[i][0], diff
        );
    }
    let r2_diff = (mr.r_squared[0] - ar.r_squared[0]).abs();
    assert!(r2_diff < 1e-10, "Shap r_squared mismatch: {} vs {}", mr.r_squared[0], ar.r_squared[0]);
}

#[test]
fn tc_2260_18_permutation_matches_new_api() {
    let df = large_df(50);
    let metric_result = PermutationMetric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(PermutationMetric)], 0);

    assert!(metric_result.is_some());
    assert!(!api_results.is_empty());
    let mr = metric_result.unwrap().permutation.unwrap().0;
    let ar = api_results.into_iter().next().unwrap().permutation.unwrap().0;
    assert_eq!(mr.importances.len(), ar.importances.len());
    for i in 0..mr.importances.len() {
        let diff = (mr.importances[i][0] - ar.importances[i][0]).abs();
        assert!(
            diff < 1e-10,
            "Permutation importances[{}][0] mismatch: {} vs {}, diff={}",
            i, mr.importances[i][0], ar.importances[i][0], diff
        );
    }
    let r2_diff = (mr.r_squared[0] - ar.r_squared[0]).abs();
    assert!(r2_diff < 1e-10, "Permutation r_squared mismatch: {} vs {}", mr.r_squared[0], ar.r_squared[0]);
}

// ===========================================================================
// トレイトオブジェクト経由の動的ディスパッチ
// ===========================================================================

#[test]
fn tc_2260_19_all_as_trait_objects() {
    let df = large_df(50);
    let metrics: Vec<Box<dyn SensitivityMetric>> = vec![
        Box::new(RfAnovaMetric),
        Box::new(MdiMetric),
        Box::new(ShapMetric),
        Box::new(PermutationMetric),
    ];
    let expected_names = ["RfAnova", "Mdi", "Shap", "Permutation"];
    for (metric, &name) in metrics.iter().zip(expected_names.iter()) {
        assert_eq!(metric.name(), name);
        assert!(metric.compute(&df, 0).is_some(), "{} should return Some via trait object", name);
    }
}

#[test]
fn tc_2260_20_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RfAnovaMetric>();
    assert_send_sync::<MdiMetric>();
    assert_send_sync::<ShapMetric>();
    assert_send_sync::<PermutationMetric>();
}
