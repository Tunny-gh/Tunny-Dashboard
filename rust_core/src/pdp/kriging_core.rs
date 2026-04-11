use crate::core::kriging::{gaussian_process, sparse_fitc};

use super::types::PdpResult2d;
use super::utils::linspace;

/// Core Kriging computation without global state.
///
/// Takes pre-extracted 2D input where `x_2d[i] = [param1_val, param2_val]`.
/// Returns `None` if training fails or input is insufficient.
/// The `param1_name`, `param2_name`, `objective_name` fields in the result are
/// empty strings - callers should set them as needed.
pub(crate) fn compute_pdp_2d_kriging_raw(
    x_2d: &[Vec<f64>],
    y: &[f64],
    n_grid: usize,
) -> Option<PdpResult2d> {
    let n = y.len();
    if n < 3 || n_grid == 0 || x_2d.is_empty() {
        return None;
    }

    // Normalize data ranges to make GP hyper-parameter optimization stable.
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

    let x_norm: Vec<Vec<f64>> = x_2d
        .iter()
        .map(|r| vec![(r[0] - min1) / range1, (r[1] - min2) / range2])
        .collect();

    let model = gaussian_process::train_gp(x_norm.clone(), y_norm, 500, 42)?;

    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    let values: Vec<Vec<f64>> = grid1
        .iter()
        .map(|&v1| {
            let v1n = (v1 - min1) / range1;
            grid2
                .iter()
                .map(|&v2| {
                    let v2n = (v2 - min2) / range2;
                    gaussian_process::predict_mean(&model, &[v1n, v2n]) * y_std + y_mean
                })
                .collect()
        })
        .collect();

    let ss_tot: f64 = y.iter().map(|&v| (v - y_mean).powi(2)).sum();
    let ss_res: f64 = x_norm
        .iter()
        .zip(y.iter())
        .map(|(xi, &yi)| {
            let pred = gaussian_process::predict_mean(&model, xi) * y_std + y_mean;
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
/// Name fields in the result are empty strings - callers should set them.
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

    if n < m_inducing {
        return compute_pdp_2d_kriging_raw(&x_2d_norm, &y_norm, n_grid).map(|mut r| {
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

    let mut x_flat = vec![0.0_f64; n * n_dims];
    for i in 0..n {
        x_flat[i] = x_2d_norm[i][0];
        x_flat[n + i] = x_2d_norm[i][1];
    }

    let z = sparse_fitc::select_inducing_points_kmeans(&x_flat, n, n_dims, m_inducing, 42);
    let m = m_inducing;

    let max_fitc_iter = if n >= 2000 {
        3
    } else if n >= 500 {
        10
    } else {
        20
    };
    let params = sparse_fitc::optimize_fitc_hyperparams(&x_flat, &z, &y_norm, n, m, max_fitc_iter);

    let (w, _) = match sparse_fitc::fitc_predict_weights(&x_flat, &z, &y_norm, &params, n, m) {
        Some(r) => r,
        None => return compute_pdp_2d_kriging_raw(x_2d, y, n_grid),
    };

    if w.iter().any(|v| !v.is_finite()) {
        return compute_pdp_2d_kriging_raw(x_2d, y, n_grid);
    }

    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    let log_ls = &params[..n_dims];
    let log_sf = params[n_dims];

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
                            gaussian_process::matern52_ard(&x_star, &zj, log_ls, log_sf) * w[j]
                        })
                        .sum();
                    pred_norm * y_std + y_mean
                })
                .collect()
        })
        .collect();

    let ss_tot: f64 = y.iter().map(|&v| (v - y_mean).powi(2)).sum();
    let ss_res: f64 = (0..n)
        .map(|i| {
            let xi = [x_flat[i], x_flat[n + i]];
            let pred_norm: f64 = (0..m)
                .map(|j| {
                    let zj = [z[j], z[m + j]];
                    gaussian_process::matern52_ard(&xi, &zj, log_ls, log_sf) * w[j]
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

/// Compute 2D PDP surface using Kriging (GP with ARD Matern 5/2 kernel).
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
