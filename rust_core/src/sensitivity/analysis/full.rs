use crate::dataframe::DataFrame;

use super::super::{
    compute_spearman, data::get_param_numeric_values, metric_trait::SensitivityMetric,
    metrics::MdiMetric, metrics::RfAnovaMetric, MdiResult, RfAnovaResult, SensitivityResult,
};
use super::common::{
    build_standardized_param_columns, compute_ridge_from_standardized_columns, empty_result,
    run_tree_metric_for_all_objectives,
};

/// Computes sensitivity for a single objective using each provided metric.
/// Metrics that return `None` (insufficient data, etc.) are silently excluded.
pub fn compute_sensitivity_single_obj(
    df: &DataFrame,
    metrics: Vec<Box<dyn SensitivityMetric>>,
    obj_idx: usize,
) -> Vec<SensitivityResult> {
    metrics.iter()
        .filter_map(|m| m.compute(df, obj_idx))
        .collect()
}

/// Computes sensitivity analysis without MDI; the returned `result.mdi` is always `None`.
pub fn compute_sensitivity_without_mdi(df: &DataFrame) -> SensitivityResult {
    compute_sensitivity_impl(df, false)
}

pub fn compute_sensitivity_all(df: &DataFrame) -> SensitivityResult {
    compute_sensitivity_impl(df, true)
}

fn compute_sensitivity_impl(df: &DataFrame, include_mdi: bool) -> SensitivityResult {
    let param_names = df.param_col_names().to_vec();
    let objective_names = df.objective_col_names().to_vec();
    let n = df.row_count();

    if n < 2 || param_names.is_empty() || objective_names.is_empty() {
        return empty_result(param_names, objective_names);
    }

    let spearman: Vec<Vec<f64>> = param_names
        .iter()
        .map(|param_name| {
            let x = match get_param_numeric_values(df, param_name, n) {
                Some(col) => col,
                None => return vec![0.0; objective_names.len()],
            };
            objective_names
                .iter()
                .map(|objective_name| {
                    let y = match df.get_numeric_column(objective_name) {
                        Some(col) => col,
                        None => return 0.0,
                    };
                    compute_spearman(&x, y)
                })
                .collect()
        })
        .collect();

    let x_cols_flat = build_standardized_param_columns(df, &param_names, n);

    // Use get_param_numeric_values so categorical string params are ordinal-encoded (0,1,2,…)
    // instead of always returning 0.0 from get_numeric_column.
    let param_cols: Vec<Vec<f64>> = param_names
        .iter()
        .map(|name| get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]))
        .collect();
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|row_index| {
            param_cols
                .iter()
                .map(|col| col.get(row_index).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    let ridge = objective_names
        .iter()
        .map(|objective_name| {
            let y: Vec<f64> = df
                .get_numeric_column(objective_name)
                .map(|col| col[..n].to_vec())
                .unwrap_or_else(|| vec![0.0; n]);
            compute_ridge_from_standardized_columns(&x_cols_flat, n, &y)
        })
        .collect();

    let objective_columns: Vec<Vec<f64>> = objective_names
        .iter()
        .map(|objective_name| {
            df.get_numeric_column(objective_name)
                .map(|col| col[..n].to_vec())
                .unwrap_or_else(|| vec![0.0; n])
        })
        .collect();

    let rf_anova = RfAnovaResult(run_tree_metric_for_all_objectives(
        &RfAnovaMetric,
        &x_matrix,
        &objective_columns,
        param_names.len(),
        objective_names.len(),
    ));

    let mdi = if include_mdi {
        Some(MdiResult(run_tree_metric_for_all_objectives(
            &MdiMetric,
            &x_matrix,
            &objective_columns,
            param_names.len(),
            objective_names.len(),
        )))
    } else {
        None
    };

    SensitivityResult {
        param_names: param_names.clone(),
        objective_names: objective_names.clone(),
        spearman,
        ridge,
        rf_anova: Some(rf_anova),
        mdi,
        shap: None,
        permutation: None,
    }
}
