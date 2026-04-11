use crate::dataframe::DataFrame;

use super::super::{
    compute_rf_anova_importances, compute_spearman, data::get_param_numeric_values,
    SensitivityResult,
};
use super::common::{
    build_standardized_param_columns, compute_ridge_from_standardized_columns, empty_result,
    transpose_rf_anova_importances,
};

pub fn compute_sensitivity_all(df: &DataFrame) -> SensitivityResult {
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

    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|row_index| {
            param_names
                .iter()
                .map(|param_name| {
                    df.get_numeric_column(param_name)
                        .and_then(|col| col.get(row_index))
                        .copied()
                        .unwrap_or(0.0)
                })
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

    let rf_anova_importances_by_objective: Vec<Vec<f64>> = objective_names
        .iter()
        .map(|objective_name| {
            let y: Vec<f64> = df
                .get_numeric_column(objective_name)
                .map(|col| col[..n].to_vec())
                .unwrap_or_else(|| vec![0.0; n]);
            compute_rf_anova_importances(&x_matrix, &y)
        })
        .collect();

    SensitivityResult {
        param_names: param_names.clone(),
        objective_names: objective_names.clone(),
        spearman,
        ridge,
        rf_anova: Some(transpose_rf_anova_importances(
            &rf_anova_importances_by_objective,
            param_names.len(),
            objective_names.len(),
        )),
    }
}
