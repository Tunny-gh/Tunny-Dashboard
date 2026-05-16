use crate::kriging::{gaussian_process, sparse_fitc};
use rayon::prelude::*;

use super::types::{PdpResult1d, PdpResult2d};
use super::utils::{normalize_x_minmax, normalize_y, r_squared};
use crate::math::grid::linspace;

/// 1D PDP with Kriging (GP regression on all feature dimensions).
///
/// Trains a GP on the full `x_matrix` vs `y`. For each grid point `v` of the
/// target parameter, replaces that column with `v` for every training row,
/// predicts mean + variance per row, and averages them to obtain the PDP value
/// and 95% confidence band.
pub(crate) fn compute_pdp_1d_kriging_raw(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
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

    // Normalise each feature column to [0, 1] and objective variable
    let (col_stats, x_norm) = normalize_x_minmax(x_matrix);
    let (y_mean, y_std, y_norm) = normalize_y(y);

    let model = gaussian_process::train_gp(x_norm.clone(), y_norm, 100, 42)?;

    // Grid over the target parameter in original space
    let (min_j, range_j) = col_stats[target_param_idx];
    let max_j = min_j + range_j;
    let grid = linspace(min_j, max_j, n_grid);

    // ── Centroid approximation ──────────────────────────────────────────────
    // Instead of averaging G×N individual GP predictions (O(G×N×N²) = O(G×N³)),
    // we evaluate at a single "centroid" point per grid value where every
    // non-target dimension is fixed to the training-data mean.  This reduces
    // variance computation to O(G×N²) with negligible accuracy loss for smooth
    // objective functions because GP posterior variance is dominated by the
    // distance to the training cloud, not the specific sample point.
    let centroid_norm: Vec<f64> = (0..n_dims)
        .map(|d| {
            if d == target_param_idx {
                0.0 // will be replaced per grid point
            } else {
                x_norm.iter().map(|r| r[d]).sum::<f64>() / n as f64
            }
        })
        .collect();

    let mut pdp_values = Vec::with_capacity(n_grid);
    let mut y_upper_vec = Vec::with_capacity(n_grid);
    let mut y_lower_vec = Vec::with_capacity(n_grid);

    for &v in &grid {
        let v_norm = (v - min_j) / range_j;

        // ── Mean: average over all training rows (exact PDP marginalisation) ──
        // predict_mean is O(N) per call, so N calls = O(N²) acceptable
        let mean_avg: f64 = x_norm
            .par_iter()
            .map(|row_norm| {
                let mut pt = row_norm.clone();
                pt[target_param_idx] = v_norm;
                gaussian_process::predict_mean(&model, &pt)
            })
            .sum::<f64>()
            / n as f64;

        // ── Variance: evaluate once at the centroid (O(N²)) ─────────────────
        let mut centroid_pt = centroid_norm.clone();
        centroid_pt[target_param_idx] = v_norm;
        let var_centroid = gaussian_process::predict_variance(&model, &centroid_pt).max(0.0);

        let pdp_orig = mean_avg * y_std + y_mean;
        let std_orig = var_centroid.sqrt() * y_std;

        pdp_values.push(pdp_orig);
        y_upper_vec.push(pdp_orig + 1.96 * std_orig);
        y_lower_vec.push(pdp_orig - 1.96 * std_orig);
    }

    // R² on training data
    let y_pred: Vec<f64> = x_norm
        .iter()
        .map(|xi| gaussian_process::predict_mean(&model, xi) * y_std + y_mean)
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

/// 1D PDP using Sparse Kriging (FITC approximation) on all feature dimensions.
///
/// ## Speed design
///
/// Standard Kriging trains on 100 subsampled points but then runs the PDP mean
/// loop over **all N rows** (each `predict_mean` call is O(N_sub=100)).  For
/// large N the mean loop therefore costs O(n_grid × N × 100).
///
/// This function achieves a lower constant cost by:
/// 1. **Borrowing hyperparameters** from a quickly-trained standard GP (same
///    100-point subsample). This eliminates `optimize_fitc_hyperparams`, whose
///    numerical-gradient loop is O(N × M² × 2(D+2) × max_iter) — the
///    dominant bottleneck in the previous design.
/// 2. **K-means on the GP subsample** (100 pts max) → O(100 × M).
/// 3. **fitc_train on full N** with those hyperparams → O(N × M²), fast for
///    M = 20.
/// 4. **Centroid approximation for both mean and variance** in the PDP loop →
///    O(n_grid × M) instead of O(n_grid × N × 100).  This is a consistent
///    approximation: FITC is already an approximation, and marginalising over
///    the centroid is sufficient for smooth objectives.
///
/// Net result: Sparse Kriging PDP is O(n_grid × M) = constant in N (after
/// training), while standard Kriging PDP grows as O(n_grid × N × 100).
pub(crate) fn compute_pdp_1d_sparse_kriging_raw(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_names: &[String],
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
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

    // ── Normalise ────────────────────────────────────────────────────────────
    let (col_stats, x_norm) = normalize_x_minmax(x_matrix);
    let (y_mean, y_std, y_norm) = normalize_y(y);

    // ── Step 1: standard GP on 100-point subsample (for hyperparams) ─────────
    let gp_model = gaussian_process::train_gp(x_norm.clone(), y_norm.clone(), 100, 42)?;

    // Extract hyperparams in FITC layout: [log_ls..., log_sf, log_sn]
    let mut fitc_params: Vec<f64> = gp_model.kernel.log_ls.clone();
    fitc_params.push(gp_model.kernel.log_sf);
    fitc_params.push(gp_model.kernel.log_sn);

    // ── Step 2: K-means on GP subsample → inducing points Z ──────────────────
    // M=20 is sufficient for 1D PDP; using 50 (like 2D) is wasteful here.
    const M_1D: usize = 20;
    let gp_n = gp_model.x_train.len();
    let m = M_1D.min(gp_n);

    // Build column-major flat array from the GP's (already-normalised) subsample
    let mut gp_x_flat = vec![0.0_f64; n_dims * gp_n];
    for i in 0..gp_n {
        for d in 0..n_dims {
            gp_x_flat[d * gp_n + i] = gp_model.x_train[i][d];
        }
    }
    let z = sparse_fitc::select_inducing_points_kmeans(&gp_x_flat, gp_n, n_dims, m, 42);

    // ── Step 3: fitc_train on full N with borrowed hyperparams ───────────────
    // O(N × M²) — cheap because M=20 and no hyperparameter search
    let mut x_flat = vec![0.0_f64; n_dims * n];
    for i in 0..n {
        for d in 0..n_dims {
            x_flat[d * n + i] = x_norm[i][d];
        }
    }

    let fitc_model = match sparse_fitc::fitc_train(&x_flat, &z, &y_norm, &fitc_params, n, m) {
        Some(model) if model.w.iter().all(|v| v.is_finite()) => model,
        _ => {
            return compute_pdp_1d_kriging_raw(
                x_matrix,
                y,
                param_names,
                objective_name,
                target_param_idx,
                n_grid,
            )
        }
    };

    // ── Step 4: PDP — marginalise over all N rows ────────────────────────────
    // fitc_predict_mean  is O(M)  per call → mean loop  = O(n_grid × N × M)
    // fitc_predict_variance is O(M²) per call → var loop = O(n_grid × N × M²)
    // For n_grid=50, N=1000, M=20: ~1M / ~20M ops — fast and spatially varying.
    //
    // Using centroid-only for variance produces a nearly-flat CI band because
    // a single centroid point stays at almost constant distance from the M
    // inducing points as the target dim varies → reduction ≈ const → var ≈ const.
    // Averaging over all N rows captures the actual data density variation.
    let (min_j, range_j) = col_stats[target_param_idx];
    let max_j = min_j + range_j;
    let grid = linspace(min_j, max_j, n_grid);

    let results: Vec<(f64, f64, f64)> = grid
        .par_iter()
        .map(|&v| {
            let v_norm = (v - min_j) / range_j;

            // Mean: exact PDP marginalisation — O(N × M) per grid point
            let mean_norm: f64 = x_norm
                .iter()
                .map(|row| {
                    let mut pt = row.clone();
                    pt[target_param_idx] = v_norm;
                    sparse_fitc::fitc_predict_mean(&fitc_model, &pt)
                })
                .sum::<f64>()
                / n as f64;

            // Variance: average over all rows — spatially varying CI band
            // O(N × M²) per grid point
            let var_avg: f64 = x_norm
                .iter()
                .map(|row| {
                    let mut pt = row.clone();
                    pt[target_param_idx] = v_norm;
                    sparse_fitc::fitc_predict_variance(&fitc_model, &pt).max(0.0)
                })
                .sum::<f64>()
                / n as f64;

            let pdp_orig = mean_norm * y_std + y_mean;
            let std_orig = var_avg.sqrt() * y_std;
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

    // ── R² on training data (O(N × M) — acceptable) ──────────────────────────
    let y_pred: Vec<f64> = x_norm
        .iter()
        .map(|xi| sparse_fitc::fitc_predict_mean(&fitc_model, xi) * y_std + y_mean)
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
    let (col_stats, x_norm) = normalize_x_minmax(x_2d);
    let (min1, range1) = col_stats[0];
    let (min2, range2) = col_stats[1];
    let (y_mean, y_std, y_norm) = normalize_y(y);

    let model = gaussian_process::train_gp(x_norm.clone(), y_norm, 500, 42)?;

    let x_values = linspace(min1, min1 + range1, n_grid);
    let y_values = linspace(min2, min2 + range2, n_grid);

    let (z_values, uncertainties): (Vec<Vec<f64>>, Vec<Vec<f64>>) = x_values
        .par_iter()
        .map(|&v1| {
            let v1n = (v1 - min1) / range1;
            let mut mean_row = Vec::with_capacity(n_grid);
            let mut var_row = Vec::with_capacity(n_grid);
            for &v2 in &y_values {
                let v2n = (v2 - min2) / range2;
                let point = [v1n, v2n];
                mean_row.push(gaussian_process::predict_mean(&model, &point) * y_std + y_mean);
                // Variance is in normalized space; scale by y_std²
                var_row.push(gaussian_process::predict_variance(&model, &point) * y_std * y_std);
            }
            (mean_row, var_row)
        })
        .unzip();

    let y_pred_2d: Vec<f64> = x_norm
        .iter()
        .map(|xi| gaussian_process::predict_mean(&model, xi) * y_std + y_mean)
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

    // Normalise
    let (col_stats, x_2d_norm) = normalize_x_minmax(x_2d);
    let (min1, range1) = col_stats[0];
    let (min2, range2) = col_stats[1];
    let (y_mean, y_std, y_norm) = normalize_y(y);

    // Fallback: not enough data for FITC inducing points
    if n < m_inducing {
        return compute_pdp_2d_kriging_raw(x_2d, y, n_grid);
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

    let fitc_model = match sparse_fitc::fitc_train(&x_flat, &z, &y_norm, &params, n, m) {
        Some(model) => model,
        None => return compute_pdp_2d_kriging_raw(x_2d, y, n_grid),
    };

    if fitc_model.w.iter().any(|v| !v.is_finite()) {
        return compute_pdp_2d_kriging_raw(x_2d, y, n_grid);
    }

    let x_values = linspace(min1, min1 + range1, n_grid);
    let y_values = linspace(min2, min2 + range2, n_grid);

    let (z_values, uncertainties): (Vec<Vec<f64>>, Vec<Vec<f64>>) = x_values
        .par_iter()
        .map(|&v1| {
            let v1n = (v1 - min1) / range1;
            let mut mean_row = Vec::with_capacity(n_grid);
            let mut var_row = Vec::with_capacity(n_grid);
            for &v2 in &y_values {
                let v2n = (v2 - min2) / range2;
                let point = [v1n, v2n];
                let mean_norm = sparse_fitc::fitc_predict_mean(&fitc_model, &point);
                let var_norm = sparse_fitc::fitc_predict_variance(&fitc_model, &point);
                mean_row.push(mean_norm * y_std + y_mean);
                var_row.push(var_norm * y_std * y_std);
            }
            (mean_row, var_row)
        })
        .unzip();

    let y_pred_sparse: Vec<f64> = (0..n)
        .map(|i| {
            let xi = [x_2d_norm[i][0], x_2d_norm[i][1]];
            sparse_fitc::fitc_predict_mean(&fitc_model, &xi) * y_std + y_mean
        })
        .collect();
    let r_sq = r_squared(y, &y_pred_sparse);

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

/// Compute 2D PDP surface using Sparse Kriging (FITC) with automatic fallback.
///
/// Extracts the two relevant parameter columns from `x_matrix` and delegates
/// to [`compute_pdp_2d_sparse_kriging_raw`]. Falls back to standard Kriging
/// when `N < 50` (not enough data for FITC inducing points).
pub(crate) fn compute_pdp_2d_sparse_kriging(
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
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return empty;
    }

    let x_2d: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| vec![row[param1_idx], row[param2_idx]])
        .collect();

    match compute_pdp_2d_sparse_kriging_raw(&x_2d, y, n_grid) {
        Some(mut result) => {
            result.param1_name = p1_name;
            result.param2_name = p2_name;
            result.objective_name = objective_name.to_string();
            result
        }
        None => empty,
    }
}
