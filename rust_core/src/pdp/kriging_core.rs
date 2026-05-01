use crate::core::kriging::{gaussian_process, sparse_fitc};

use super::types::{PdpResult1d, PdpResult2d};
use crate::core::math::grid::linspace;

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

    // Normalise each feature column to [0, 1]
    let col_stats: Vec<(f64, f64)> = (0..n_dims)
        .map(|d| {
            let col: Vec<f64> = x_matrix.iter().map(|r| r[d]).collect();
            let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min, (max - min).max(f64::EPSILON))
        })
        .collect();

    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_std = (y.iter().map(|&v| (v - y_mean).powi(2)).sum::<f64>() / n as f64)
        .sqrt()
        .max(f64::EPSILON);
    let y_norm: Vec<f64> = y.iter().map(|&v| (v - y_mean) / y_std).collect();

    let x_norm: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(d, &v)| {
                    let (min, range) = col_stats[d];
                    (v - min) / range
                })
                .collect()
        })
        .collect();

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
        let mean_avg: f64 = {
            let sum: f64 = x_norm
                .iter()
                .map(|row_norm| {
                    let mut pt = row_norm.clone();
                    pt[target_param_idx] = v_norm;
                    gaussian_process::predict_mean(&model, &pt)
                })
                .sum();
            sum / n as f64
        };

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

    Some(PdpResult1d {
        param_name,
        objective_name: objective_name.to_string(),
        grid,
        values: pdp_values,
        r_squared,
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
    let col_stats: Vec<(f64, f64)> = (0..n_dims)
        .map(|d| {
            let col: Vec<f64> = x_matrix.iter().map(|r| r[d]).collect();
            let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min, (max - min).max(f64::EPSILON))
        })
        .collect();

    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_std = (y.iter().map(|&v| (v - y_mean).powi(2)).sum::<f64>() / n as f64)
        .sqrt()
        .max(f64::EPSILON);
    let y_norm: Vec<f64> = y.iter().map(|&v| (v - y_mean) / y_std).collect();

    let x_norm: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(d, &v)| {
                    let (min, range) = col_stats[d];
                    (v - min) / range
                })
                .collect()
        })
        .collect();

    // ── Step 1: standard GP on 100-point subsample (for hyperparams) ─────────
    let gp_model = gaussian_process::train_gp(x_norm.clone(), y_norm.clone(), 100, 42)?;

    // Extract hyperparams in FITC layout: [log_ls..., log_sf, log_sn]
    let mut fitc_params: Vec<f64> = gp_model.log_ls.clone();
    fitc_params.push(gp_model.log_sf);
    fitc_params.push(gp_model.log_sn);

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

    let mut pdp_values = Vec::with_capacity(n_grid);
    let mut y_upper_vec = Vec::with_capacity(n_grid);
    let mut y_lower_vec = Vec::with_capacity(n_grid);

    for &v in &grid {
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

        pdp_values.push(pdp_orig);
        y_upper_vec.push(pdp_orig + 1.96 * std_orig);
        y_lower_vec.push(pdp_orig - 1.96 * std_orig);
    }

    // ── R² on training data (O(N × M) — acceptable) ──────────────────────────
    let ss_tot: f64 = y.iter().map(|&v| (v - y_mean).powi(2)).sum();
    let ss_res: f64 = x_norm
        .iter()
        .zip(y.iter())
        .map(|(xi, &yi)| {
            let pred = sparse_fitc::fitc_predict_mean(&fitc_model, xi) * y_std + y_mean;
            (yi - pred).powi(2)
        })
        .sum();
    let r_squared = if ss_tot < f64::EPSILON {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Some(PdpResult1d {
        param_name,
        objective_name: objective_name.to_string(),
        grid,
        values: pdp_values,
        r_squared,
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

    let x_values = linspace(min1, max1, n_grid);
    let y_values = linspace(min2, max2, n_grid);

    let mut z_values: Vec<Vec<f64>> = Vec::with_capacity(n_grid);
    let mut uncertainties: Vec<Vec<f64>> = Vec::with_capacity(n_grid);

    for &v1 in &x_values {
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
        z_values.push(mean_row);
        uncertainties.push(var_row);
    }

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
        x_values,
        y_values,
        z_values,
        r_squared,
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
            r.x_values = r.x_values.iter().map(|&v| v * range1 + min1).collect();
            r.y_values = r.y_values.iter().map(|&v| v * range2 + min2).collect();
            for row in &mut r.z_values {
                for v in row.iter_mut() {
                    *v = *v * y_std + y_mean;
                }
            }
            if let Some(ref mut unc) = r.uncertainties {
                for row in unc.iter_mut() {
                    for v in row.iter_mut() {
                        *v *= y_std * y_std;
                    }
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

    let fitc_model = match sparse_fitc::fitc_train(&x_flat, &z, &y_norm, &params, n, m) {
        Some(model) => model,
        None => return compute_pdp_2d_kriging_raw(x_2d, y, n_grid),
    };

    if fitc_model.w.iter().any(|v| !v.is_finite()) {
        return compute_pdp_2d_kriging_raw(x_2d, y, n_grid);
    }

    let x_values = linspace(min1, max1, n_grid);
    let y_values = linspace(min2, max2, n_grid);

    let mut z_values: Vec<Vec<f64>> = Vec::with_capacity(n_grid);
    let mut uncertainties: Vec<Vec<f64>> = Vec::with_capacity(n_grid);

    for &v1 in &x_values {
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
        z_values.push(mean_row);
        uncertainties.push(var_row);
    }

    let ss_tot: f64 = y.iter().map(|&v| (v - y_mean).powi(2)).sum();
    let ss_res: f64 = (0..n)
        .map(|i| {
            let xi = [x_2d_norm[i][0], x_2d_norm[i][1]];
            let pred_norm = sparse_fitc::fitc_predict_mean(&fitc_model, &xi);
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
        x_values,
        y_values,
        z_values,
        r_squared,
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
