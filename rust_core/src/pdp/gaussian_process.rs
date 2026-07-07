use crate::gaussian_process::{GpMethod, GpModel};
use rayon::prelude::*;

use super::types::{PdpResult1d, PdpResult2d};
use super::utils::{normalize_x_minmax, normalize_y, r_squared};
use crate::math::grid::linspace;

/// 1D PDP with Gaussian Process (one of three methods: FITC / VFE / mixture-of-experts).
///
/// All methods use M = min(N, 100) inducing points. For [`GpMethod::Moe`], if fitting
/// fails (e.g. degenerate cluster structure) the function retries once with
/// [`GpMethod::Fitc`] before giving up. For FITC / VFE a `None` from `GpModel::fit`
/// is final.
///
/// For each grid point `v` of the target parameter, replaces that column with `v`
/// for every training row, predicts mean via `predict_mean_batch`, and averages them
/// to obtain the PDP value. Variance uses the centroid approximation (single point
/// per grid position) for speed, with rayon parallelism over grid points.
pub(crate) fn compute_pdp_1d_gp_raw(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
    method: GpMethod,
) -> Option<PdpResult1d> {
    let n = y.len();
    let n_dims = x_matrix.first()?.len();

    if n < 3 || n_grid == 0 || target_param_idx >= n_dims {
        return None;
    }

    let param_name = param_names
        .get(target_param_idx)
        .cloned()
        .unwrap_or_default();

    // Normalise each feature column to [0, 1] and objective variable.
    let (col_stats, x_norm) = normalize_x_minmax(x_matrix);
    let (y_mean, y_std, y_norm) = normalize_y(y);

    // Train GP model; MoE falls back to FITC on training failure.
    let model = fit_with_moe_fallback(&x_norm, &y_norm, method, 100, 42)?;

    // Grid over the target parameter in original space.
    let (min_j, range_j) = col_stats[target_param_idx];
    let max_j = min_j + range_j;
    let grid = linspace(min_j, max_j, n_grid);

    // ── Centroid approximation ──────────────────────────────────────────────
    // Instead of averaging G×N individual GP variance predictions (O(G×N×N²)),
    // evaluate variance at a single "centroid" point per grid value where every
    // non-target dimension is fixed to the training-data mean.
    let centroid_norm: Vec<f64> = (0..n_dims)
        .map(|d| {
            if d == target_param_idx {
                0.0 // will be replaced per grid point
            } else {
                x_norm.iter().map(|r| r[d]).sum::<f64>() / n as f64
            }
        })
        .collect();

    let results: Vec<(f64, f64, f64)> = grid
        .par_iter()
        .map(|&v| {
            let v_norm = (v - min_j) / range_j;

            // ── Mean: average over all training rows via batch prediction ──
            let rows: Vec<Vec<f64>> = x_norm
                .iter()
                .map(|row_norm| {
                    let mut pt = row_norm.clone();
                    pt[target_param_idx] = v_norm;
                    pt
                })
                .collect();
            let preds = model.predict_mean_batch(&rows);
            let mean_avg = preds.iter().sum::<f64>() / n as f64;

            // ── Variance: evaluate once at the centroid ─────────────────
            let mut centroid_pt = centroid_norm.clone();
            centroid_pt[target_param_idx] = v_norm;
            let var_centroid = model.predict_variance(&centroid_pt).max(0.0);

            let pdp_orig = mean_avg * y_std + y_mean;
            let std_orig = var_centroid.sqrt() * y_std;
            (
                pdp_orig,
                pdp_orig + 1.96 * std_orig,
                pdp_orig - 1.96 * std_orig,
            )
        })
        .collect();

    let (pdp_values, y_upper_vec, y_lower_vec) = results.into_iter().fold(
        (
            Vec::with_capacity(n_grid),
            Vec::with_capacity(n_grid),
            Vec::with_capacity(n_grid),
        ),
        |(mut p, mut u, mut l), (pdp, upper, lower)| {
            p.push(pdp);
            u.push(upper);
            l.push(lower);
            (p, u, l)
        },
    );

    // R² on training data.
    let y_pred: Vec<f64> = model
        .predict_mean_batch(&x_norm)
        .into_iter()
        .map(|v| v * y_std + y_mean)
        .collect();
    let r_sq = r_squared(y, &y_pred);

    Some(PdpResult1d {
        param_name,
        objective_name: objective_name.to_string(),
        grid,
        values: pdp_values,
        r_squared: r_sq,
        y_upper: Some(y_upper_vec),
        y_lower: Some(y_lower_vec),
    })
}

/// Core 2D PDP computation with a GP model, marginalising over all other params.
///
/// Trains on the FULL feature matrix (M = min(N, 100) inducing points) so the
/// surface is a genuine partial dependence plot: for each grid cell `(v1, v2)`
/// the two target columns are fixed to those values in every training row, the
/// mean is predicted for all rows via `predict_mean_batch`, and those means are
/// averaged — this marginalises out every non-target dimension. Without this
/// step the GP would only see the two selected features and report the other
/// parameters' variation as (spuriously large) predictive uncertainty.
///
/// Variance uses the same centroid approximation as the 1D path (evaluate once
/// per grid cell at a point where every non-target dimension is fixed to its
/// training-data mean) for speed, with rayon parallelism over the first axis.
///
/// For [`GpMethod::Moe`], retries once with [`GpMethod::Fitc`] on training
/// failure. For FITC / VFE a `None` is final.
///
/// The `param1_name`, `param2_name`, `objective_name` fields in the result are
/// empty strings — callers should set them as needed.
pub(crate) fn compute_pdp_2d_gp_raw(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
    method: GpMethod,
) -> Option<PdpResult2d> {
    let n = y.len();
    let n_dims = x_matrix.first()?.len();
    if n < 3 || n_grid == 0 || param1_idx >= n_dims || param2_idx >= n_dims {
        return None;
    }

    // Normalize data ranges for stable hyperparameter optimisation.
    let (col_stats, x_norm) = normalize_x_minmax(x_matrix);
    let (min1, range1) = col_stats[param1_idx];
    let (min2, range2) = col_stats[param2_idx];
    let (y_mean, y_std, y_norm) = normalize_y(y);

    // Train GP model; MoE falls back to FITC on training failure.
    let model = fit_with_moe_fallback(&x_norm, &y_norm, method, 100, 42)?;

    let x_values = linspace(min1, min1 + range1, n_grid);
    let y_values = linspace(min2, min2 + range2, n_grid);

    // ── Centroid approximation for variance ─────────────────────────────────
    // Non-target dimensions are fixed to the training-data mean; the two target
    // dimensions are replaced per grid cell below.
    let centroid_norm: Vec<f64> = (0..n_dims)
        .map(|d| {
            if d == param1_idx || d == param2_idx {
                0.0 // replaced per grid cell
            } else {
                x_norm.iter().map(|r| r[d]).sum::<f64>() / n as f64
            }
        })
        .collect();

    // Each first-axis value (v1) yields one row of (z, variance) pairs.
    let rows: Vec<(Vec<f64>, Vec<f64>)> = x_values
        .par_iter()
        .map(|&v1| {
            let v1n = (v1 - min1) / range1;
            let mut z_row = Vec::with_capacity(n_grid);
            let mut var_row = Vec::with_capacity(n_grid);
            // Reusable buffers: clone the training matrix once per v1 and fix the
            // first target column; per grid cell only the two target columns are
            // overwritten (instead of re-cloning every row for every cell).
            let mut pred_rows: Vec<Vec<f64>> = x_norm.to_vec();
            for pt in &mut pred_rows {
                pt[param1_idx] = v1n;
            }
            let mut centroid_pt = centroid_norm.clone();
            centroid_pt[param1_idx] = v1n;
            for &v2 in &y_values {
                let v2n = (v2 - min2) / range2;

                // ── Mean: marginalise over all training rows ──
                for pt in &mut pred_rows {
                    pt[param2_idx] = v2n;
                }
                let preds = model.predict_mean_batch(&pred_rows);
                let mean_avg = preds.iter().sum::<f64>() / n as f64;
                z_row.push(mean_avg * y_std + y_mean);

                // ── Variance: evaluate once at the centroid ──
                centroid_pt[param2_idx] = v2n;
                let var_centroid = model.predict_variance(&centroid_pt).max(0.0);
                var_row.push(var_centroid * y_std * y_std);
            }
            (z_row, var_row)
        })
        .collect();

    let mut z_values = Vec::with_capacity(n_grid);
    let mut uncertainties = Vec::with_capacity(n_grid);
    for (z_row, var_row) in rows {
        z_values.push(z_row);
        uncertainties.push(var_row);
    }

    let y_pred_2d: Vec<f64> = model
        .predict_mean_batch(&x_norm)
        .into_iter()
        .map(|v| v * y_std + y_mean)
        .collect();
    let r_sq = r_squared(y, &y_pred_2d);

    Some(PdpResult2d {
        param1_name: String::new(),
        param2_name: String::new(),
        objective_name: String::new(),
        x_values,
        y_values,
        z_values,
        r_squared: r_sq,
        uncertainties: Some(uncertainties),
    })
}

/// Compute 2D PDP surface using a GP model (FITC / VFE / mixture-of-experts).
///
/// All three methods use M = min(N, 100) inducing points. MoE falls back to FITC
/// on training failure (degenerate cluster structure). Delegates to
/// [`compute_pdp_2d_gp_raw`], which trains on the full feature matrix and
/// marginalises over every non-target dimension (a genuine partial dependence
/// plot).
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_pdp_2d_gp(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
    method: GpMethod,
) -> PdpResult2d {
    let p1_name = param_names.get(param1_idx).cloned().unwrap_or_default();
    let p2_name = param_names.get(param2_idx).cloned().unwrap_or_default();
    let empty = PdpResult2d {
        param1_name: p1_name.clone(),
        param2_name: p2_name.clone(),
        objective_name: objective_name.to_string(),
        x_values: vec![],
        y_values: vec![],
        z_values: vec![],
        r_squared: 0.0,
        uncertainties: None,
    };

    let n = y.len();
    if n < 3 || n_grid == 0 {
        return empty;
    }
    // Guard against an empty feature matrix (e.g. y populated but x rows missing).
    let Some(first_row) = x_matrix.first() else {
        return empty;
    };
    let p = first_row.len();
    if param1_idx >= p || param2_idx >= p {
        return empty;
    }

    match compute_pdp_2d_gp_raw(x_matrix, y, param1_idx, param2_idx, n_grid, method) {
        Some(mut result) => {
            result.param1_name = p1_name;
            result.param2_name = p2_name;
            result.objective_name = objective_name.to_string();
            result
        }
        None => empty,
    }
}

/// Train a GP model, retrying once with FITC when MoE training fails.
///
/// MoE cluster-finding can fail on degenerate data (e.g. all points co-linear
/// in the cluster-search subspace).  For FITC / VFE a `None` return is final.
fn fit_with_moe_fallback(
    x_norm: &[Vec<f64>],
    y_norm: &[f64],
    method: GpMethod,
    max_inducing: usize,
    seed: u64,
) -> Option<GpModel> {
    match GpModel::fit(x_norm, y_norm, method, max_inducing, seed) {
        Some(m) => Some(m),
        None if method == GpMethod::Moe => {
            GpModel::fit(x_norm, y_norm, GpMethod::Fitc, max_inducing, seed)
        }
        None => None,
    }
}
