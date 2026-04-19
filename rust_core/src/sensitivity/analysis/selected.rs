use super::super::{
    compute_rf_anova_importances, compute_ridge, compute_spearman, data::get_param_numeric_values,
    SensitivityResult,
};
use super::common::{
    build_param_columns, build_param_matrix_from_columns, collect_objective_subset,
    collect_valid_indices, empty_result, transpose_rf_anova_importances,
};

pub fn compute_sensitivity_selected(indices: &[u32]) -> Option<SensitivityResult> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n_rows = df.row_count();

        if indices.is_empty() || param_names.is_empty() || objective_names.is_empty() {
            return empty_result(param_names, objective_names);
        }

        let valid_idx = collect_valid_indices(indices, n_rows);
        if valid_idx.is_empty() {
            return empty_result(param_names, objective_names);
        }

        let spearman: Vec<Vec<f64>> = param_names
            .iter()
            .map(|param_name| {
                let full_x = match get_param_numeric_values(df, param_name, n_rows) {
                    Some(col) => col,
                    None => return vec![0.0; objective_names.len()],
                };
                let x_subset: Vec<f64> = valid_idx
                    .iter()
                    .map(|&row_index| full_x[row_index])
                    .collect();

                objective_names
                    .iter()
                    .map(|objective_name| {
                        let full_y = match df.get_numeric_column(objective_name) {
                            Some(col) => col,
                            None => return 0.0,
                        };
                        let y_subset: Vec<f64> = valid_idx
                            .iter()
                            .map(|&row_index| full_y[row_index])
                            .collect();
                        compute_spearman(&x_subset, &y_subset)
                    })
                    .collect()
            })
            .collect();

        let param_columns = build_param_columns(df, &param_names, n_rows);
        let x_matrix = build_param_matrix_from_columns(&param_columns, &valid_idx);

        let ridge = objective_names
            .iter()
            .map(|objective_name| {
                let y_subset = collect_objective_subset(df, objective_name, &valid_idx);
                compute_ridge(&x_matrix, &y_subset, 1.0)
            })
            .collect();

        let rf_anova_by_obj: Vec<(Vec<f64>, f64)> = objective_names
            .iter()
            .map(|objective_name| {
                let y_subset = collect_objective_subset(df, objective_name, &valid_idx);
                compute_rf_anova_importances(&x_matrix, &y_subset)
            })
            .collect();
        let rf_anova_r_squared: Vec<f64> = rf_anova_by_obj.iter().map(|(_, r2)| *r2).collect();
        let rf_anova_importances: Vec<Vec<f64>> =
            rf_anova_by_obj.into_iter().map(|(imp, _)| imp).collect();

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
        }
    })
}
