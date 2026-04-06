//! Module documentation.
//!
//! Module documentation.
//! Design:
//! Module documentation.
//!   - f̄_j(v) = y_mean + β_j * (v - mean_j) / std_j
//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-803

use crate::sensitivity::compute_ridge;
use crate::{kriging, rf, sparse_kriging};

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone)]
pub struct PdpResult1d {
    /// Documentation.
    pub param_name: String,
    /// Documentation.
    pub objective_name: String,
    /// Documentation.
    pub grid: Vec<f64>,
    /// Documentation.
    pub values: Vec<f64>,
    /// Documentation.
    pub r_squared: f64,
}

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdpResult2d {
    /// Documentation.
    pub param1_name: String,
    /// Documentation.
    pub param2_name: String,
    /// Documentation.
    pub objective_name: String,
    /// Documentation.
    pub grid1: Vec<f64>,
    /// Documentation.
    pub grid2: Vec<f64>,
    /// Documentation.
    pub values: Vec<Vec<f64>>,
    /// Documentation.
    pub r_squared: f64,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
fn col_mean_std(data: &[f64]) -> (f64, f64) {
    let n = data.len();
    if n == 0 {
        return (0.0, 1.0);
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let var = data.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = if var.sqrt() < f64::EPSILON {
        1.0
    } else {
        var.sqrt()
    };
    (mean, std_dev)
}

/// Documentation.
///
/// Documentation.
fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }
    (0..n)
        .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
        .collect()
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
///
/// Documentation.
/// Documentation.
///   PDP = (1/N) Σ_i ŷ_i|x_j=v = y_mean + β_j*(v-mean_j)/std_j + Σ_{k≠j} β_k * mean((x_ki-mean_k)/std_k)
/// Documentation.
pub(crate) fn compute_pdp_from_matrix(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
) -> PdpResult1d {
    // Documentation.
    let param_name = param_names
        .get(target_param_idx)
        .cloned()
        .unwrap_or_default();
    let empty = PdpResult1d {
        param_name: param_name.clone(),
        objective_name: objective_name.to_string(),
        grid: vec![],
        values: vec![],
        r_squared: 0.0,
    };

    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid == 0 {
        return empty;
    }
    if target_param_idx >= x_matrix[0].len() {
        return empty;
    }

    // Documentation.
    let ridge = compute_ridge(x_matrix, y, 1.0);

    // Documentation.
    let param_col: Vec<f64> = x_matrix.iter().map(|row| row[target_param_idx]).collect();
    let (mean_j, std_j) = col_mean_std(&param_col);
    let y_mean = y.iter().sum::<f64>() / n as f64;

    // Documentation.
    let min_j = param_col
        .iter()
        .cloned()
        .fold(f64::INFINITY, |a, b| a.min(b));
    let max_j = param_col
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let grid = linspace(min_j, max_j, n_grid);

    // Documentation.
    let beta_j = ridge.beta.get(target_param_idx).copied().unwrap_or(0.0);
    let values: Vec<f64> = grid
        .iter()
        .map(|&v| y_mean + beta_j * (v - mean_j) / std_j)
        .collect();

    PdpResult1d {
        param_name,
        objective_name: objective_name.to_string(),
        grid,
        values,
        r_squared: ridge.r_squared,
    }
}

/// Documentation.
///
/// Documentation.
/// Documentation.
///   f̄_{j1,j2}(v1, v2) = y_mean + β_j1*(v1-mean_j1)/std_j1 + β_j2*(v2-mean_j2)/std_j2
///
/// Documentation.
pub(crate) fn compute_pdp_2d_from_matrix(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> PdpResult2d {
    // Documentation.
    let p1_name = param_names.get(param1_idx).cloned().unwrap_or_default();
    let p2_name = param_names.get(param2_idx).cloned().unwrap_or_default();
    let empty = PdpResult2d {
        param1_name: p1_name.clone(),
        param2_name: p2_name.clone(),
        objective_name: objective_name.to_string(),
        grid1: vec![],
        grid2: vec![],
        values: vec![],
        r_squared: 0.0,
    };

    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid == 0 {
        return empty;
    }
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return empty;
    }

    // Documentation.
    let ridge = compute_ridge(x_matrix, y, 1.0);

    // Documentation.
    let col1: Vec<f64> = x_matrix.iter().map(|row| row[param1_idx]).collect();
    let col2: Vec<f64> = x_matrix.iter().map(|row| row[param2_idx]).collect();
    let (mean1, std1) = col_mean_std(&col1);
    let (mean2, std2) = col_mean_std(&col2);
    let y_mean = y.iter().sum::<f64>() / n as f64;

    // Documentation.
    let min1 = col1.iter().cloned().fold(f64::INFINITY, |a, b| a.min(b));
    let max1 = col1
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let min2 = col2.iter().cloned().fold(f64::INFINITY, |a, b| a.min(b));
    let max2 = col2
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    // Documentation.
    let beta1 = ridge.beta.get(param1_idx).copied().unwrap_or(0.0);
    let beta2 = ridge.beta.get(param2_idx).copied().unwrap_or(0.0);
    let values: Vec<Vec<f64>> = grid1
        .iter()
        .map(|&v1| {
            grid2
                .iter()
                .map(|&v2| y_mean + beta1 * (v1 - mean1) / std1 + beta2 * (v2 - mean2) / std2)
                .collect()
        })
        .collect();

    PdpResult2d {
        param1_name: p1_name,
        param2_name: p2_name,
        objective_name: objective_name.to_string(),
        grid1,
        grid2,
        values,
        r_squared: ridge.r_squared,
    }
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
pub fn compute_pdp(
    param_name: &str,
    objective_name: &str,
    n_grid: usize,
    _n_samples: usize,
) -> Option<PdpResult1d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        // Documentation.
        let target_idx = param_names.iter().position(|p| p == param_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        // Documentation.
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                param_names
                    .iter()
                    .map(|p| {
                        df.get_numeric_column(p)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        let y: Vec<f64> = (0..n)
            .map(|i| {
                df.get_numeric_column(objective_name)
                    .and_then(|c| c.get(i))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();

        Some(compute_pdp_from_matrix(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_idx,
            n_grid,
        ))
    })
    .flatten()
}

/// Documentation.
///
/// Documentation.
pub fn compute_pdp_2d(
    param1_name: &str,
    param2_name: &str,
    objective_name: &str,
    n_grid: usize,
    model_type: &str,
) -> Option<PdpResult2d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        let p1_idx = param_names.iter().position(|p| p == param1_name)?;
        let p2_idx = param_names.iter().position(|p| p == param2_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                param_names
                    .iter()
                    .map(|p| {
                        df.get_numeric_column(p)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        let y: Vec<f64> = (0..n)
            .map(|i| {
                df.get_numeric_column(objective_name)
                    .and_then(|c| c.get(i))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();

        match model_type {
            "random_forest" => {
                let (grid1, grid2, values, r_squared) =
                    rf::compute_pdp_2d_rf(&x_matrix, &y, p1_idx, p2_idx, n_grid)?;
                let p1_name = param_names.get(p1_idx).cloned().unwrap_or_default();
                let p2_name = param_names.get(p2_idx).cloned().unwrap_or_default();
                Some(PdpResult2d {
                    param1_name: p1_name,
                    param2_name: p2_name,
                    objective_name: objective_name.to_string(),
                    grid1,
                    grid2,
                    values,
                    r_squared,
                })
            }
            "kriging" => Some(compute_pdp_2d_kriging(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                p1_idx,
                p2_idx,
                n_grid,
            )),
            "sparse_kriging" => {
                let p1_name = param_names.get(p1_idx).cloned().unwrap_or_default();
                let p2_name = param_names.get(p2_idx).cloned().unwrap_or_default();
                let x_2d: Vec<Vec<f64>> = x_matrix
                    .iter()
                    .map(|row| vec![row[p1_idx], row[p2_idx]])
                    .collect();
                let mut result = compute_pdp_2d_sparse_kriging_raw(&x_2d, &y, n_grid)?;
                result.param1_name = p1_name;
                result.param2_name = p2_name;
                result.objective_name = objective_name.to_string();
                Some(result)
            }
            // "ridge" or unknown → fall back to Ridge
            _ => Some(compute_pdp_2d_from_matrix(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                p1_idx,
                p2_idx,
                n_grid,
            )),
        }
    })
    .flatten()
}

/// Core Kriging computation without global state.
///
/// Takes pre-extracted 2D input where `x_2d[i] = [param1_val, param2_val]`.
/// Returns `None` if training fails or input is insufficient.
/// The `param1_name`, `param2_name`, `objective_name` fields in the result are
/// empty strings — callers should set them as needed.
pub(crate) fn compute_pdp_2d_kriging_raw(
    x_2d: &[Vec<f64>],
    y: &[f64],
    n_grid: usize,
) -> Option<PdpResult2d> {
    let n = y.len();
    if n < 3 || n_grid == 0 || x_2d.is_empty() {
        return None;
    }

    // Compute data ranges for normalisation.
    // Without normalisation, log_ls is initialised to 0 (l=1) which is
    // inappropriate when x ranges differ greatly from [0,1], causing the
    // kernel matrix to become near-zero and the optimisation to fail.
    let col1: Vec<f64> = x_2d.iter().map(|r| r[0]).collect();
    let col2: Vec<f64> = x_2d.iter().map(|r| r[1]).collect();
    let min1 = col1.iter().cloned().fold(f64::INFINITY, f64::min);
    let max1 = col1.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min2 = col2.iter().cloned().fold(f64::INFINITY, f64::min);
    let max2 = col2.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range1 = (max1 - min1).max(f64::EPSILON);
    let range2 = (max2 - min2).max(f64::EPSILON);

    // Normalise y: subtract mean, divide by std.
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_std = (y.iter().map(|&v| (v - y_mean).powi(2)).sum::<f64>() / n as f64)
        .sqrt()
        .max(f64::EPSILON);
    let y_norm: Vec<f64> = y.iter().map(|&v| (v - y_mean) / y_std).collect();

    // Normalise x to [0, 1].
    let x_norm: Vec<Vec<f64>> = x_2d
        .iter()
        .map(|r| vec![(r[0] - min1) / range1, (r[1] - min2) / range2])
        .collect();

    // Train GP on normalised data (subsample to 500 if n > 500).
    let model = kriging::train_gp(x_norm.clone(), y_norm, 500, 42)?;

    // Build grid in original (display) space.
    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    // Predict on normalised grid points, denormalise output.
    let values: Vec<Vec<f64>> = grid1
        .iter()
        .map(|&v1| {
            let v1n = (v1 - min1) / range1;
            grid2
                .iter()
                .map(|&v2| {
                    let v2n = (v2 - min2) / range2;
                    kriging::predict_mean(&model, &[v1n, v2n]) * y_std + y_mean
                })
                .collect()
        })
        .collect();

    // R² on training data (original scale).
    let ss_tot: f64 = y.iter().map(|&v| (v - y_mean).powi(2)).sum();
    let ss_res: f64 = x_norm
        .iter()
        .zip(y.iter())
        .map(|(xi, &yi)| {
            let pred = kriging::predict_mean(&model, xi) * y_std + y_mean;
            (yi - pred).powi(2)
        })
        .sum();
    let r_squared = if ss_tot < f64::EPSILON {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Some(PdpResult2d {
        param1_name: String::new(),
        param2_name: String::new(),
        objective_name: String::new(),
        grid1,
        grid2,
        values,
        r_squared,
    })
}

/// Core Sparse Kriging (FITC) computation without global state.
///
/// Same interface as `compute_pdp_2d_kriging_raw`. Falls back to standard
/// Kriging when `N < M=50` (not enough data for inducing points).
/// Name fields in the result are empty strings — callers should set them.
pub(crate) fn compute_pdp_2d_sparse_kriging_raw(
    x_2d: &[Vec<f64>],
    y: &[f64],
    n_grid: usize,
) -> Option<PdpResult2d> {
    let n = y.len();
    let n_dims = 2_usize;
    let m_inducing = 50_usize;

    if n < 3 || n_grid == 0 || x_2d.is_empty() {
        return None;
    }

    // Normalise x and y (same reason as compute_pdp_2d_kriging_raw).
    let col1: Vec<f64> = x_2d.iter().map(|r| r[0]).collect();
    let col2: Vec<f64> = x_2d.iter().map(|r| r[1]).collect();
    let min1 = col1.iter().cloned().fold(f64::INFINITY, f64::min);
    let max1 = col1.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min2 = col2.iter().cloned().fold(f64::INFINITY, f64::min);
    let max2 = col2.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range1 = (max1 - min1).max(f64::EPSILON);
    let range2 = (max2 - min2).max(f64::EPSILON);

    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_std = (y.iter().map(|&v| (v - y_mean).powi(2)).sum::<f64>() / n as f64)
        .sqrt()
        .max(f64::EPSILON);
    let y_norm: Vec<f64> = y.iter().map(|&v| (v - y_mean) / y_std).collect();

    let x_2d_norm: Vec<Vec<f64>> = x_2d
        .iter()
        .map(|r| vec![(r[0] - min1) / range1, (r[1] - min2) / range2])
        .collect();

    // Fallback to standard Kriging when N < M (uses normalised data).
    if n < m_inducing {
        return compute_pdp_2d_kriging_raw(&x_2d_norm, &y_norm, n_grid).map(|mut r| {
            // Denormalise grid and values back to original scale.
            r.grid1 = r.grid1.iter().map(|&v| v * range1 + min1).collect();
            r.grid2 = r.grid2.iter().map(|&v| v * range2 + min2).collect();
            for row in &mut r.values {
                for v in row.iter_mut() {
                    *v = *v * y_std + y_mean;
                }
            }
            r
        });
    }

    // Convert normalised x_2d → column-major flat (n_dims × N).
    let mut x_flat = vec![0.0_f64; n * n_dims];
    for i in 0..n {
        x_flat[i] = x_2d_norm[i][0];
        x_flat[n + i] = x_2d_norm[i][1];
    }

    // Select inducing points via K-means on normalised space.
    let z = sparse_kriging::select_inducing_points_kmeans(&x_flat, n, n_dims, m_inducing, 42);
    let m = m_inducing;

    // Optimise FITC hyperparameters on normalised data.
    // Fewer iters for large N to keep total time under NFR-002 (< 5s at N=5000).
    let max_fitc_iter = if n >= 2000 { 3 } else if n >= 500 { 10 } else { 20 };
    let params =
        sparse_kriging::optimize_fitc_hyperparams(&x_flat, &z, &y_norm, n, m, max_fitc_iter);

    // Compute FITC posterior mean weights on normalised data.
    let (w, _) = match sparse_kriging::fitc_predict_weights(&x_flat, &z, &y_norm, &params, n, m) {
        Some(r) => r,
        None => return compute_pdp_2d_kriging_raw(x_2d, y, n_grid),
    };

    // Guard: fall back if weights are not finite.
    if w.iter().any(|v| !v.is_finite()) {
        return compute_pdp_2d_kriging_raw(x_2d, y, n_grid);
    }

    // Build grid in original (display) space.
    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    let log_ls = &params[..n_dims];
    let log_sf = params[n_dims];

    // μ(x*) = K_{x*,Z} · w, denormalised to original y scale.
    let values: Vec<Vec<f64>> = grid1
        .iter()
        .map(|&v1| {
            let v1n = (v1 - min1) / range1;
            grid2
                .iter()
                .map(|&v2| {
                    let v2n = (v2 - min2) / range2;
                    let x_star = [v1n, v2n];
                    let pred_norm: f64 = (0..m)
                        .map(|j| {
                            let zj = [z[j], z[m + j]];
                            kriging::matern52_ard(&x_star, &zj, log_ls, log_sf) * w[j]
                        })
                        .sum();
                    pred_norm * y_std + y_mean
                })
                .collect()
        })
        .collect();

    // R² on training data (original scale).
    let ss_tot: f64 = y.iter().map(|&v| (v - y_mean).powi(2)).sum();
    let ss_res: f64 = (0..n)
        .map(|i| {
            let xi = [x_flat[i], x_flat[n + i]];
            let pred_norm: f64 = (0..m)
                .map(|j| {
                    let zj = [z[j], z[m + j]];
                    kriging::matern52_ard(&xi, &zj, log_ls, log_sf) * w[j]
                })
                .sum();
            let pred = pred_norm * y_std + y_mean;
            (y[i] - pred).powi(2)
        })
        .sum();
    let r_squared = if ss_tot < f64::EPSILON {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Some(PdpResult2d {
        param1_name: String::new(),
        param2_name: String::new(),
        objective_name: String::new(),
        grid1,
        grid2,
        values,
        r_squared,
    })
}

/// Compute 2D PDP surface using Kriging (GP with ARD Matérn 5/2 kernel).
pub(crate) fn compute_pdp_2d_kriging(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> PdpResult2d {
    let p1_name = param_names.get(param1_idx).cloned().unwrap_or_default();
    let p2_name = param_names.get(param2_idx).cloned().unwrap_or_default();
    let empty = PdpResult2d {
        param1_name: p1_name.clone(),
        param2_name: p2_name.clone(),
        objective_name: objective_name.to_string(),
        grid1: vec![],
        grid2: vec![],
        values: vec![],
        r_squared: 0.0,
    };

    let n = y.len();
    if n < 3 || n_grid == 0 {
        return empty;
    }
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return empty;
    }

    // 2D subspace projection
    let x_2d: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| vec![row[param1_idx], row[param2_idx]])
        .collect();

    match compute_pdp_2d_kriging_raw(&x_2d, y, n_grid) {
        Some(mut result) => {
            result.param1_name = p1_name;
            result.param2_name = p2_name;
            result.objective_name = objective_name.to_string();
            result
        }
        None => empty,
    }
}

// =============================================================================
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    /// Documentation.
    ///
    /// Documentation.
    /// Documentation.
    fn make_linear_data_1d(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<String>) {
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let x1 = i as f64 / n as f64;
                let x2 = (i as f64 * 0.3).sin(); // Documentation.
                vec![x1, x2]
            })
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| row[0]).collect(); // y = x1
        let names = vec!["x1".to_string(), "x2".to_string()];
        (x_matrix, y, names)
    }

    /// Documentation.
    ///
    /// Documentation.
    fn make_linear_data_multi(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<String>) {
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, t * 1.3 + 0.1, 1.0 - t * 0.7]
            })
            .collect();
        let y: Vec<f64> = x_matrix
            .iter()
            .map(|row| 2.0 * row[0] - 0.5 * row[1] + 0.3 * row[2])
            .collect();
        let names = vec!["x1".to_string(), "x2".to_string(), "x3".to_string()];
        (x_matrix, y, names)
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_01_pdp_monotone_positive() {
        // Documentation.
        // Documentation.
        let n = 200;
        let (x_matrix, y, names) = make_linear_data_1d(n);

        // Documentation.
        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

        // Documentation.
        assert_eq!(result.grid.len(), 10, "translated 10 translated");
        assert_eq!(result.values.len(), 10, "PDPtranslated 10 translated");

        // Documentation.
        for i in 0..result.values.len() - 1 {
            assert!(
                result.values[i] < result.values[i + 1],
                "PDP[{}]={} translated PDP[{}]={} translated（translated）",
                i,
                result.values[i],
                i + 1,
                result.values[i + 1]
            );
        }
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_02_pdp_monotone_negative() {
        // Documentation.
        let n = 100;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| -row[0]).collect();
        let names = vec!["x1".to_string()];

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 8);

        // Documentation.
        for i in 0..result.values.len() - 1 {
            assert!(
                result.values[i] > result.values[i + 1],
                "PDP[{}]={} translated PDP[{}]={} translated（translated）",
                i,
                result.values[i],
                i + 1,
                result.values[i + 1]
            );
        }
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_03_pdp_midpoint_equals_ymean() {
        // Documentation.
        // Documentation.
        // Documentation.
        let n = 200;
        let (x_matrix, y, names) = make_linear_data_1d(n);
        let y_mean = y.iter().sum::<f64>() / n as f64;

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 11);

        // Documentation.
        let mid_idx = result.values.len() / 2;
        let mid_val = result.values[mid_idx];

        // Documentation.
        let tolerance = (y_mean.abs() + 0.01) * 0.05;
        assert!(
            (mid_val - y_mean).abs() < tolerance,
            "translatedPDPtranslated {} translated y_mean {} translated ±5% translated",
            mid_val,
            y_mean
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_04_pdp_r_squared_high_for_linear() {
        // Documentation.
        let n = 200;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = x_matrix.iter().map(|row| row[0] * 3.0 + 1.0).collect();
        let names = vec!["x1".to_string()];

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

        // Documentation.
        assert!(
            result.r_squared > 0.99,
            "translated R² {} translated 0.99 translated",
            result.r_squared
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_05_empty_data_returns_empty() {
        // Documentation.
        let x_matrix: Vec<Vec<f64>> = vec![vec![1.0]]; // n=1
        let y = vec![1.0f64];
        let names = vec!["x1".to_string()];

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

        // Documentation.
        assert!(result.grid.is_empty(), "n<2 translated");
        assert!(result.values.is_empty(), "n<2 translated");
        assert_eq!(
            result.r_squared, 0.0,
            "n<2 translated R² translated 0.0 translated"
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_06_pdp_2d_grid_shape() {
        // Documentation.
        let n = 100;
        let (x_matrix, y, names) = make_linear_data_multi(n);

        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 8);

        // Documentation.
        assert_eq!(result.grid1.len(), 8, "grid1 translated 8 translated");
        assert_eq!(result.grid2.len(), 8, "grid2 translated 8 translated");
        assert_eq!(result.values.len(), 8, "values translated 8 translated");
        for row in &result.values {
            assert_eq!(row.len(), 8, "values translated 8 translated");
        }
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_07_pdp_2d_empty_data() {
        // Documentation.
        let x_matrix: Vec<Vec<f64>> = vec![vec![1.0, 2.0]];
        let y = vec![1.0f64];
        let names = vec!["x1".to_string(), "x2".to_string()];

        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 5);

        // Documentation.
        assert!(result.grid1.is_empty(), "n<2 translated grid1 translated");
        assert!(result.grid2.is_empty(), "n<2 translated grid2 translated");
        assert!(result.values.is_empty(), "n<2 translated values translated");
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_08_pdp_2d_r_squared() {
        // Documentation.
        let n = 200;
        let (x_matrix, y, names) = make_linear_data_multi(n);

        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 5);

        // Documentation.
        assert!(
            result.r_squared > 0.95,
            "translated 2parameterPDP R² {} translated 0.95 translated",
            result.r_squared
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_09_result_names() {
        // Documentation.
        let n = 50;
        let (x_matrix, y, names) = make_linear_data_1d(n);

        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj_target", 0, 5);

        // Documentation.
        assert_eq!(
            result.param_name, "x1",
            "param_name translated 'x1' translated"
        );
        assert_eq!(
            result.objective_name, "obj_target",
            "objective_name translated 'obj_target' translated"
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_p01_pdp_1d_performance() {
        // Documentation.
        // Documentation.

        #[cfg(debug_assertions)]
        let (n, p) = (1_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 10);

        // Documentation.
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..p)
                    .map(|j| i as f64 / n as f64 + j as f64 * 0.1)
                    .collect()
            })
            .collect();
        let y: Vec<f64> = x_matrix
            .iter()
            .map(|row| row[0] * 2.0 + row[1] * 0.5)
            .collect();
        let names: Vec<String> = (0..p).map(|j| format!("x{}", j)).collect();

        let start = std::time::Instant::now();
        let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 20);
        let elapsed = start.elapsed();

        // Documentation.
        assert_eq!(result.grid.len(), 20, "translated 20 translated");
        assert!(
            elapsed.as_millis() < 20,
            "1parameterPDP translated 20ms translated: translated {}ms",
            elapsed.as_millis()
        );
    }

    // ------------------------------------------------------------------
    // Documentation.
    // ------------------------------------------------------------------

    #[test]
    fn tc_803_p02_pdp_2d_performance() {
        // Documentation.
        // Documentation.

        #[cfg(debug_assertions)]
        let (n, p) = (1_000, 4);
        #[cfg(not(debug_assertions))]
        let (n, p) = (50_000, 10);

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..p)
                    .map(|j| i as f64 / n as f64 + j as f64 * 0.1)
                    .collect()
            })
            .collect();
        let y: Vec<f64> = x_matrix
            .iter()
            .map(|row| row[0] * 2.0 + row[1] * 0.5)
            .collect();
        let names: Vec<String> = (0..p).map(|j| format!("x{}", j)).collect();

        let start = std::time::Instant::now();
        let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 15);
        let elapsed = start.elapsed();

        // Documentation.
        assert_eq!(result.values.len(), 15, "values translated 15 translated");
        assert_eq!(
            result.values[0].len(),
            15,
            "values translated 15 translated"
        );
        assert!(
            elapsed.as_millis() < 100,
            "2parameterPDP translated 100ms translated: translated {}ms",
            elapsed.as_millis()
        );
    }

    // -------------------------------------------------------------------------
    // TASK-1645: compute_pdp_2d_kriging_raw (グローバル状態非依存)
    // -------------------------------------------------------------------------

    /// TC-005-01: N=30 での kriging_raw 正常動作 — grid サイズと values 構造を確認
    #[test]
    fn tc_1645_01_kriging_raw_grid_shape() {
        let n = 30;
        let x_2d: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 / n as f64, (i as f64 * 0.3).sin()])
            .collect();
        let y: Vec<f64> = x_2d.iter().map(|xi| xi[0] + xi[1]).collect();
        let n_grid = 10;

        let result = compute_pdp_2d_kriging_raw(&x_2d, &y, n_grid)
            .expect("compute_pdp_2d_kriging_raw should succeed");

        assert_eq!(
            result.grid1.len(),
            n_grid,
            "grid1 should have n_grid points"
        );
        assert_eq!(
            result.grid2.len(),
            n_grid,
            "grid2 should have n_grid points"
        );
        assert_eq!(
            result.values.len(),
            n_grid,
            "values outer dim should be n_grid"
        );
        assert_eq!(
            result.values[0].len(),
            n_grid,
            "values inner dim should be n_grid"
        );
    }

    /// TC-005-E01: model_type="unknown" はエラーを返す（wasm_compute_kriging_raw 代替テスト）
    /// 不十分な入力（n < 3）で None が返ることを検証
    #[test]
    fn tc_1645_e01_insufficient_data_returns_none() {
        let x_2d = vec![vec![0.0, 0.0], vec![0.5, 0.5]]; // n=2 < 3
        let y = vec![0.0, 1.0];

        let result = compute_pdp_2d_kriging_raw(&x_2d, &y, 10);
        assert!(result.is_none(), "n < 3 should return None");
    }

    // -------------------------------------------------------------------------
    // TASK-1652: Sparse Kriging (FITC) - compute_pdp_2d_sparse_kriging_raw
    // -------------------------------------------------------------------------

    /// TC-005-02: N=100 で grid shape が n_grid × n_grid であること
    #[test]
    fn tc_1652_tc_005_02_sparse_kriging_n100_grid_shape() {
        let n = 100;
        let x_2d: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, (t * 3.0).sin() * 0.5 + 0.5]
            })
            .collect();
        let y: Vec<f64> = x_2d.iter().map(|r| r[0] + 0.3 * r[1]).collect();
        let n_grid = 10;

        let result = compute_pdp_2d_sparse_kriging_raw(&x_2d, &y, n_grid);
        assert!(result.is_some(), "Should succeed for N=100");
        let r = result.unwrap();
        assert_eq!(r.grid1.len(), n_grid, "grid1.len() should be n_grid");
        assert_eq!(r.grid2.len(), n_grid, "grid2.len() should be n_grid");
        assert_eq!(r.values.len(), n_grid, "values.len() should be n_grid");
        assert_eq!(
            r.values[0].len(),
            n_grid,
            "values[0].len() should be n_grid"
        );
    }

    /// TC-005-03: N < M=50 でフォールバック（エラーなし、合理的な結果）
    #[test]
    fn tc_1652_tc_005_03_fallback_when_n_lt_m() {
        let n = 30; // N < M=50
        let x_2d: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, 1.0 - t]
            })
            .collect();
        let y: Vec<f64> = x_2d.iter().map(|r| r[0] * 2.0).collect();
        let n_grid = 5;

        let result = compute_pdp_2d_sparse_kriging_raw(&x_2d, &y, n_grid);
        assert!(result.is_some(), "Fallback should succeed for N=30");
        let r = result.unwrap();
        // Grid values should be finite
        for row in &r.values {
            for &v in row {
                assert!(v.is_finite(), "Grid value should be finite: {}", v);
            }
        }
    }

    // -------------------------------------------------------------------------
    // TASK-1653: dispatch regression tests
    // -------------------------------------------------------------------------

    // -------------------------------------------------------------------------
    // TASK-1655: Performance tests (run with cargo test --release)
    // -------------------------------------------------------------------------

    /// TC-NFR-001-01: Kriging N=1000 < 10,000ms (release build)
    /// Run with: cargo test --release -- --ignored tc_nfr_001_01
    #[test]
    #[ignore]
    fn tc_nfr_001_01_kriging_n1000_under_10s() {
        let n = 1000;
        let x_2d: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, (t * 5.0).sin() * 0.5 + 0.5]
            })
            .collect();
        let y: Vec<f64> = x_2d.iter().map(|r| r[0] + 0.3 * r[1]).collect();

        let start = std::time::Instant::now();
        let result = compute_pdp_2d_kriging_raw(&x_2d, &y, 50);
        let elapsed = start.elapsed().as_millis();

        println!("Kriging N=1000: {}ms", elapsed);
        assert!(result.is_some(), "Should return Some for N=1000");
        // Timing assertion only in release builds (debug mode is ~50-100× slower)
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed < 10_000,
            "NFR-001 target missed: {}ms > 10,000ms",
            elapsed
        );
    }

    /// TC-NFR-002-01: Sparse Kriging N=5000 < 5,000ms (release build)
    /// Run with: cargo test --release -- --ignored tc_nfr_002_01
    #[test]
    #[ignore]
    fn tc_nfr_002_01_sparse_kriging_n5000_under_5s() {
        let n = 5000;
        let x_2d: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, (t * 7.0).cos() * 0.5 + 0.5]
            })
            .collect();
        let y: Vec<f64> = x_2d.iter().map(|r| r[0] * 2.0 + r[1] * 0.5).collect();

        let start = std::time::Instant::now();
        let result = compute_pdp_2d_sparse_kriging_raw(&x_2d, &y, 50);
        let elapsed = start.elapsed().as_millis();

        println!("Sparse Kriging N=5000: {}ms", elapsed);
        assert!(result.is_some(), "Should return Some for N=5000");
        // Timing assertion only in release builds (debug mode is ~50-100× slower)
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed < 5_000,
            "NFR-002 target missed: {}ms > 5,000ms",
            elapsed
        );
    }

    /// "sparse_kriging" dispatch via compute_pdp_2d_sparse_kriging_raw produces finite results
    #[test]
    fn tc_1653_01_sparse_kriging_dispatch_returns_finite_results() {
        let n = 60; // N >= M=50, no fallback
        let x_2d: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, 1.0 - t * 0.7]
            })
            .collect();
        let y: Vec<f64> = x_2d.iter().map(|r| r[0] * 1.5 + r[1] * 0.5).collect();
        let n_grid = 5;

        let result = compute_pdp_2d_sparse_kriging_raw(&x_2d, &y, n_grid);
        assert!(
            result.is_some(),
            "sparse_kriging dispatch should succeed for N=60"
        );
        let r = result.unwrap();
        assert_eq!(r.grid1.len(), n_grid);
        assert_eq!(r.grid2.len(), n_grid);
        for row in &r.values {
            for &v in row {
                assert!(v.is_finite(), "All grid values should be finite: {}", v);
            }
        }
    }
}
