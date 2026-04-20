use crate::dataframe::DataFrame;

use super::super::{
    compute_mdi_importances, compute_rf_anova_importances, compute_shap_importances,
    compute_spearman, data::get_param_numeric_values, SensitivityMetric, SensitivityResult,
};
use super::common::{
    build_standardized_param_columns, compute_ridge_from_standardized_columns, empty_result,
    transpose_mdi_importances, transpose_rf_anova_importances, transpose_shap_importances,
};

/// Computes sensitivity for a single objective and a single metric only.
/// The returned SensitivityResult has `objective_names = [selected_obj]` and
/// only the field corresponding to `metric` is populated; all others are empty/None.
pub fn compute_sensitivity_single_obj(
    df: &DataFrame,
    metric: &SensitivityMetric,
    obj_idx: usize,
) -> SensitivityResult {
    let param_names = df.param_col_names().to_vec();
    let objective_names = df.objective_col_names().to_vec();
    let n = df.row_count();

    let Some(objective_name) = objective_names.get(obj_idx).cloned() else {
        return empty_result(param_names, objective_names);
    };

    if n < 2 || param_names.is_empty() {
        return empty_result(param_names, vec![objective_name]);
    }

    let y: Vec<f64> = df
        .get_numeric_column(&objective_name)
        .map(|col| col[..n].to_vec())
        .unwrap_or_else(|| vec![0.0; n]);

    // Spearman and Ridge use different data layouts; build x_matrix only for tree-based metrics.
    let x_matrix: Option<Vec<Vec<f64>>> =
        match metric {
            SensitivityMetric::RfAnova | SensitivityMetric::Mdi | SensitivityMetric::Shap => {
                let param_cols: Vec<Vec<f64>> = param_names
                    .iter()
                    .map(|name| {
                        get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n])
                    })
                    .collect();
                Some(
                    (0..n)
                        .map(|row| param_cols.iter().map(|col| col[row]).collect())
                        .collect(),
                )
            }
            _ => None,
        };

    match metric {
        SensitivityMetric::Spearman => {
            let spearman: Vec<Vec<f64>> = param_names
                .iter()
                .map(|name| {
                    let x = get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]);
                    vec![compute_spearman(&x, &y)]
                })
                .collect();
            SensitivityResult {
                param_names,
                objective_names: vec![objective_name],
                spearman,
                ridge: vec![],
                rf_anova: None,
                mdi: None,
                shap: None,
            }
        }
        SensitivityMetric::Ridge => {
            let x_flat = build_standardized_param_columns(df, &param_names, n);
            let ridge = vec![compute_ridge_from_standardized_columns(&x_flat, n, &y)];
            SensitivityResult {
                param_names,
                objective_names: vec![objective_name],
                spearman: vec![],
                ridge,
                rf_anova: None,
                mdi: None,
                shap: None,
            }
        }
        SensitivityMetric::RfAnova => {
            let x_matrix = x_matrix.unwrap();
            let (imp, r2) = compute_rf_anova_importances(&x_matrix, &y);
            let rf_anova = Some(transpose_rf_anova_importances(
                &[imp],
                vec![r2],
                param_names.len(),
                1,
            ));
            SensitivityResult {
                param_names,
                objective_names: vec![objective_name],
                spearman: vec![],
                ridge: vec![],
                rf_anova,
                mdi: None,
                shap: None,
            }
        }
        SensitivityMetric::Mdi => {
            let x_matrix = x_matrix.unwrap();
            let (imp, r2) = compute_mdi_importances(&x_matrix, &y);
            let mdi = Some(transpose_mdi_importances(
                &[imp],
                vec![r2],
                param_names.len(),
                1,
            ));
            SensitivityResult {
                param_names,
                objective_names: vec![objective_name],
                spearman: vec![],
                ridge: vec![],
                rf_anova: None,
                mdi,
                shap: None,
            }
        }
        SensitivityMetric::Shap => {
            let x_matrix = x_matrix.unwrap();
            let (imp, r2) = compute_shap_importances(&x_matrix, &y);
            let shap = Some(transpose_shap_importances(
                &[imp],
                vec![r2],
                param_names.len(),
                1,
            ));
            SensitivityResult {
                param_names,
                objective_names: vec![objective_name],
                spearman: vec![],
                ridge: vec![],
                rf_anova: None,
                mdi: None,
                shap,
            }
        }
    }
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

    let rf_anova_by_obj: Vec<(Vec<f64>, f64)> = objective_names
        .iter()
        .map(|objective_name| {
            let y: Vec<f64> = df
                .get_numeric_column(objective_name)
                .map(|col| col[..n].to_vec())
                .unwrap_or_else(|| vec![0.0; n]);
            compute_rf_anova_importances(&x_matrix, &y)
        })
        .collect();
    let rf_anova_r_squared: Vec<f64> = rf_anova_by_obj.iter().map(|(_, r2)| *r2).collect();
    let rf_anova_importances: Vec<Vec<f64>> =
        rf_anova_by_obj.into_iter().map(|(imp, _)| imp).collect();

    let mdi = if include_mdi {
        let mdi_by_obj: Vec<(Vec<f64>, f64)> = objective_names
            .iter()
            .map(|objective_name| {
                let y: Vec<f64> = df
                    .get_numeric_column(objective_name)
                    .map(|col| col[..n].to_vec())
                    .unwrap_or_else(|| vec![0.0; n]);
                compute_mdi_importances(&x_matrix, &y)
            })
            .collect();
        let mdi_r_squared: Vec<f64> = mdi_by_obj.iter().map(|(_, r2)| *r2).collect();
        let mdi_importances: Vec<Vec<f64>> = mdi_by_obj.into_iter().map(|(imp, _)| imp).collect();
        Some(transpose_mdi_importances(
            &mdi_importances,
            mdi_r_squared,
            param_names.len(),
            objective_names.len(),
        ))
    } else {
        None
    };

    SensitivityResult {
        param_names: param_names.clone(),
        objective_names: objective_names.clone(),
        spearman,
        ridge,
        rf_anova: Some(transpose_rf_anova_importances(
            &rf_anova_importances,
            rf_anova_r_squared,
            param_names.len(),
            objective_names.len(),
        )),
        mdi,
        shap: None,
    }
}
