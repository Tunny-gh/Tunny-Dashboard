use crate::dataframe::{self, DataFrame};

use super::{compute_ridge, compute_spearman, data::get_param_numeric_values, RidgeResult, SensitivityResult};

fn empty_result(param_names: Vec<String>, objective_names: Vec<String>) -> SensitivityResult {
    SensitivityResult {
        param_names,
        objective_names,
        spearman: vec![],
        ridge: vec![],
    }
}

fn gaussian_elimination(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let p = b.len();
    if p == 0 {
        return Some(vec![]);
    }

    for col in 0..p {
        let pivot_row = (col..p)
            .max_by(|&i, &j| {
                a[i][col]
                    .abs()
                    .partial_cmp(&a[j][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);

        a.swap(col, pivot_row);
        b.swap(col, pivot_row);

        let pivot = a[col][col];
        if pivot.abs() < 1e-12 {
            return None;
        }

        for row in (col + 1)..p {
            let factor = a[row][col] / pivot;
            for k in col..p {
                let v = a[col][k] * factor;
                a[row][k] -= v;
            }
            b[row] -= b[col] * factor;
        }
    }

    let mut x = vec![0.0f64; p];
    for i in (0..p).rev() {
        let mut sum = b[i];
        for j in (i + 1)..p {
            sum -= a[i][j] * x[j];
        }
        x[i] = sum / a[i][i];
    }

    Some(x)
}

fn build_standardized_param_columns(df: &DataFrame, param_names: &[String], n: usize) -> Vec<f64> {
    let num_params = param_names.len();
    let mut x_cols_flat = vec![0.0f64; n * num_params];

    for (j, p_name) in param_names.iter().enumerate() {
        if let Some(col) = get_param_numeric_values(df, p_name, n) {
            for (i, &v) in col.iter().enumerate().take(n) {
                x_cols_flat[j * n + i] = v;
            }
        }
    }

    let nf = n as f64;
    for j in 0..num_params {
        let col = &mut x_cols_flat[j * n..(j + 1) * n];
        let mean: f64 = col.iter().sum::<f64>() / nf;
        let std_dev = (col.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf).sqrt();
        let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
        for v in col.iter_mut() {
            *v = (*v - mean) / std_dev;
        }
    }

    x_cols_flat
}

fn compute_ridge_from_standardized_columns(x_cols_flat: &[f64], n: usize, y: &[f64]) -> RidgeResult {
    let num_params = if n == 0 { 0 } else { x_cols_flat.len() / n };
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_c: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();

    let mut xtx_flat = vec![0.0f64; num_params * num_params];
    for i in 0..num_params {
        for j in i..num_params {
            let col_i = &x_cols_flat[i * n..(i + 1) * n];
            let col_j = &x_cols_flat[j * n..(j + 1) * n];
            let val: f64 = col_i.iter().zip(col_j.iter()).map(|(a, b)| a * b).sum();
            xtx_flat[i * num_params + j] = val;
            xtx_flat[j * num_params + i] = val;
        }
    }
    for i in 0..num_params {
        xtx_flat[i * num_params + i] += 1.0;
    }

    let mut xty = vec![0.0f64; num_params];
    for j in 0..num_params {
        let col_j = &x_cols_flat[j * n..(j + 1) * n];
        xty[j] = col_j.iter().zip(y_c.iter()).map(|(x, yy)| x * yy).sum();
    }

    let xtx_2d: Vec<Vec<f64>> = (0..num_params)
        .map(|i| xtx_flat[i * num_params..(i + 1) * num_params].to_vec())
        .collect();
    let beta = match gaussian_elimination(xtx_2d, xty) {
        Some(beta) => beta,
        None => vec![0.0; num_params],
    };

    let y_hat: Vec<f64> = (0..n)
        .map(|i| {
            (0..num_params)
                .map(|j| x_cols_flat[j * n + i] * beta[j])
                .sum()
        })
        .collect();
    let ss_res: f64 = y_c
        .iter()
        .zip(y_hat.iter())
        .map(|(yi, yhi)| (yi - yhi).powi(2))
        .sum();
    let ss_tot: f64 = y_c.iter().map(|&yi| yi.powi(2)).sum();
    let r_squared = if ss_tot < f64::EPSILON {
        0.0
    } else {
        (1.0 - ss_res / ss_tot).max(0.0)
    };

    RidgeResult { beta, r_squared }
}

pub fn compute_sensitivity_all(df: &DataFrame) -> SensitivityResult {
    let param_names = df.param_col_names().to_vec();
    let objective_names = df.objective_col_names().to_vec();
    let n = df.row_count();

    if n < 2 || param_names.is_empty() || objective_names.is_empty() {
        return empty_result(param_names, objective_names);
    }

    let spearman: Vec<Vec<f64>> = param_names
        .iter()
        .map(|p_name| {
            let x = match get_param_numeric_values(df, p_name, n) {
                Some(col) => col,
                None => return vec![0.0; objective_names.len()],
            };
            objective_names
                .iter()
                .map(|o_name| {
                    let y = match df.get_numeric_column(o_name) {
                        Some(col) => col,
                        None => return 0.0,
                    };
                    compute_spearman(&x, y)
                })
                .collect()
        })
        .collect();

    let x_cols_flat = build_standardized_param_columns(df, &param_names, n);
    let ridge: Vec<RidgeResult> = objective_names
        .iter()
        .map(|o_name| {
            let y: Vec<f64> = df
                .get_numeric_column(o_name)
                .map(|col| col[..n].to_vec())
                .unwrap_or_else(|| vec![0.0; n]);
            compute_ridge_from_standardized_columns(&x_cols_flat, n, &y)
        })
        .collect();

    SensitivityResult {
        param_names,
        objective_names,
        spearman,
        ridge,
    }
}

pub fn compute_sensitivity() -> Option<SensitivityResult> {
    dataframe::with_active_df(compute_sensitivity_all)
}

pub fn compute_sensitivity_selected(indices: &[u32]) -> Option<SensitivityResult> {
    dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n_rows = df.row_count();

        if indices.is_empty() || param_names.is_empty() || objective_names.is_empty() {
            return empty_result(param_names, objective_names);
        }

        let valid_idx: Vec<usize> = indices
            .iter()
            .filter_map(|&i| {
                let u = i as usize;
                if u < n_rows {
                    Some(u)
                } else {
                    None
                }
            })
            .collect();

        if valid_idx.is_empty() {
            return empty_result(param_names, objective_names);
        }

        let spearman: Vec<Vec<f64>> = param_names
            .iter()
            .map(|p_name| {
                let full_x = match get_param_numeric_values(df, p_name, n_rows) {
                    Some(col) => col,
                    None => return vec![0.0; objective_names.len()],
                };
                let x_sub: Vec<f64> = valid_idx.iter().map(|&i| full_x[i]).collect();

                objective_names
                    .iter()
                    .map(|o_name| {
                        let full_y = match df.get_numeric_column(o_name) {
                            Some(col) => col,
                            None => return 0.0,
                        };
                        let y_sub: Vec<f64> = valid_idx.iter().map(|&i| full_y[i]).collect();
                        compute_spearman(&x_sub, &y_sub)
                    })
                    .collect()
            })
            .collect();

        let param_cols_all: Vec<Vec<f64>> = param_names
            .iter()
            .map(|p| get_param_numeric_values(df, p, n_rows).unwrap_or_else(|| vec![0.0; n_rows]))
            .collect();

        let x_matrix: Vec<Vec<f64>> = valid_idx
            .iter()
            .map(|&row_idx| {
                param_cols_all
                    .iter()
                    .map(|col| col.get(row_idx).copied().unwrap_or(0.0))
                    .collect()
            })
            .collect();

        let ridge: Vec<RidgeResult> = objective_names
            .iter()
            .map(|o_name| {
                let y_sub: Vec<f64> = valid_idx
                    .iter()
                    .map(|&row_idx| {
                        df.get_numeric_column(o_name)
                            .map(|col| col[row_idx])
                            .unwrap_or(0.0)
                    })
                    .collect();
                compute_ridge(&x_matrix, &y_sub, 1.0)
            })
            .collect();

        SensitivityResult {
            param_names,
            objective_names,
            spearman,
            ridge,
        }
    })
}
