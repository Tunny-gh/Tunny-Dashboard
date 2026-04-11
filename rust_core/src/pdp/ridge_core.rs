use crate::sensitivity::compute_ridge;

use super::types::{PdpResult1d, PdpResult2d};
use super::utils::{col_mean_std, linspace};

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

    let ridge = compute_ridge(x_matrix, y, 1.0);

    let param_col: Vec<f64> = x_matrix.iter().map(|row| row[target_param_idx]).collect();
    let (mean_j, std_j) = col_mean_std(&param_col);
    let y_mean = y.iter().sum::<f64>() / n as f64;

    let min_j = param_col
        .iter()
        .cloned()
        .fold(f64::INFINITY, |a, b| a.min(b));
    let max_j = param_col
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, |a, b| a.max(b));
    let grid = linspace(min_j, max_j, n_grid);

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

    let ridge = compute_ridge(x_matrix, y, 1.0);

    let col1: Vec<f64> = x_matrix.iter().map(|row| row[param1_idx]).collect();
    let col2: Vec<f64> = x_matrix.iter().map(|row| row[param2_idx]).collect();
    let (mean1, std1) = col_mean_std(&col1);
    let (mean2, std2) = col_mean_std(&col2);
    let y_mean = y.iter().sum::<f64>() / n as f64;

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
