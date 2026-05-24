use crate::math::stats::column_mean_std;
use crate::dataframe::DataFrame;
use rayon::prelude::*;

use super::super::{
    data::get_param_numeric_values, metrics::TreeMetric,
    ridge::compute_ridge_from_standardized_columns as ridge_from_standardized_columns_core,
    tree::common::{prepare_shared_x, prepare_training_data}, RidgeResult, SensitivityResult, TreeImportanceResult,
};

pub(super) fn empty_result(
    param_names: Vec<String>,
    objective_names: Vec<String>,
) -> SensitivityResult {
    SensitivityResult {
        param_names,
        objective_names,
        spearman: vec![],
        ridge: vec![],
        rf_anova: None,
        mdi: None,
        shap: None,
        permutation: None,
    }
}

pub(super) fn build_standardized_param_columns(
    df: &DataFrame,
    param_names: &[String],
    n: usize,
) -> Vec<f64> {
    let num_params = param_names.len();
    let mut x_cols_flat = vec![0.0f64; n * num_params];

    for (j, param_name) in param_names.iter().enumerate() {
        if let Some(col) = get_param_numeric_values(df, param_name, n) {
            for (i, &value) in col.iter().enumerate().take(n) {
                x_cols_flat[j * n + i] = value;
            }
        }
    }

    for j in 0..num_params {
        let col = &mut x_cols_flat[j * n..(j + 1) * n];
        let vals: Vec<f64> = col.to_vec();
        let (mean, std_dev) = column_mean_std(&vals);
        for value in col.iter_mut() {
            *value = (*value - mean) / std_dev;
        }
    }

    x_cols_flat
}

pub(super) fn compute_ridge_from_standardized_columns(
    x_cols_flat: &[f64],
    n: usize,
    y: &[f64],
) -> RidgeResult {
    ridge_from_standardized_columns_core(x_cols_flat, n, y, 1.0)
}

pub(super) fn collect_valid_indices(indices: &[u32], n_rows: usize) -> Vec<usize> {
    indices
        .iter()
        .filter_map(|&index| {
            let row = index as usize;
            if row < n_rows {
                Some(row)
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn build_param_columns(
    df: &DataFrame,
    param_names: &[String],
    n_rows: usize,
) -> Vec<Vec<f64>> {
    param_names
        .iter()
        .map(|param_name| {
            get_param_numeric_values(df, param_name, n_rows).unwrap_or_else(|| vec![0.0; n_rows])
        })
        .collect()
}

pub(super) fn build_param_matrix_from_columns(
    param_columns: &[Vec<f64>],
    row_indices: &[usize],
) -> Vec<Vec<f64>> {
    row_indices
        .iter()
        .map(|&row_index| {
            param_columns
                .iter()
                .map(|col| col.get(row_index).copied().unwrap_or(0.0))
                .collect()
        })
        .collect()
}

pub(super) fn collect_objective_subset(
    df: &DataFrame,
    objective_name: &str,
    row_indices: &[usize],
) -> Vec<f64> {
    row_indices
        .iter()
        .map(|&row_index| {
            df.get_numeric_column(objective_name)
                .map(|col| col[row_index])
                .unwrap_or(0.0)
        })
        .collect()
}

fn transpose_importances_matrix(
    importances_by_objective: &[Vec<f64>],
    param_count: usize,
    objective_count: usize,
) -> Vec<Vec<f64>> {
    let mut importances = vec![vec![0.0; objective_count]; param_count];
    for (objective_index, values) in importances_by_objective.iter().enumerate() {
        for (param_index, &value) in values.iter().enumerate() {
            if param_index < param_count {
                importances[param_index][objective_index] = value;
            }
        }
    }
    importances
}

pub(super) fn transpose_to_tree_result(
    importances_by_objective: &[Vec<f64>],
    r_squared: Vec<f64>,
    param_count: usize,
    objective_count: usize,
) -> TreeImportanceResult {
    TreeImportanceResult {
        importances: transpose_importances_matrix(
            importances_by_objective,
            param_count,
            objective_count,
        ),
        r_squared,
    }
}

/// Multi-objective TreeMetric dispatch: prepares x once via SharedX and reuses it per objective.
/// Falls back to `prepare_training_data` for objectives with NaN/Inf y values.
pub(super) fn run_tree_metric_for_all_objectives<M: TreeMetric + Send + Sync>(
    metric: &M,
    x_matrix: &[Vec<f64>],
    objective_columns: &[Vec<f64>],
    param_count: usize,
    objective_count: usize,
) -> TreeImportanceResult {
    let shared_x = prepare_shared_x(
        x_matrix,
        metric.max_rows(),
        metric.data_seed(),
        metric.split_seed(),
    );
    let results: Vec<(Vec<f64>, f64)> = objective_columns
        .par_iter()
        .map(|y| {
            let data = shared_x
                .as_ref()
                .and_then(|sx| sx.with_y(y))
                .or_else(|| prepare_training_data(
                    x_matrix, y, metric.max_rows(), metric.data_seed(), metric.split_seed(),
                ));
            match data {
                Some(d) => metric.compute_importances(&d).unwrap_or_else(|| (vec![0.0; param_count], 0.0)),
                None => (vec![0.0; param_count], 0.0),
            }
        })
        .collect();
    let (importances, r_squared): (Vec<Vec<f64>>, Vec<f64>) = results.into_iter().unzip();
    transpose_to_tree_result(&importances, r_squared, param_count, objective_count)
}
