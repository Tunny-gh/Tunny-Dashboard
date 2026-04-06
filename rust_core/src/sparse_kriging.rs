//! Sparse Kriging (FITC approximation) for large-N Kriging.
//!
//! Pure-Rust implementation — no external crates.
//! Uses inducing points selected via K-means clustering to approximate
//! the full Gaussian Process with O(N·M²) instead of O(N³).

// =============================================================================
// K-means inducing point selection
// =============================================================================

/// Select M inducing points from training data using K-means (Lloyd's algorithm).
///
/// # Arguments
/// - `x`: training data in column-major flat layout: `x[dim * n_samples + i]`
/// - `n_samples`: number of training samples (N)
/// - `n_dims`: input dimensionality (typically 2 for 2D PDP)
/// - `m`: number of inducing points (typically 50)
/// - `seed`: random seed for reproducibility
///
/// # Returns
/// Inducing points in column-major flat layout: `result[dim * m + j]`
/// Length = `m * n_dims`.
pub(crate) fn select_inducing_points_kmeans(
    x: &[f64],
    n_samples: usize,
    n_dims: usize,
    m: usize,
    seed: u64,
) -> Vec<f64> {
    assert!(
        m > 0 && m <= n_samples,
        "Inducing point count M={} must satisfy 0 < M <= N={}",
        m,
        n_samples
    );
    assert!(n_dims > 0, "n_dims must be > 0");

    let mut rng = crate::rf::Lcg::new(seed);

    // --- Step 1: initialise centers by random sampling without replacement ---
    let mut indices: Vec<usize> = (0..n_samples).collect();
    // Fisher-Yates partial shuffle to pick first m elements
    for i in (1..n_samples).rev() {
        let j = rng.next_usize(i + 1);
        indices.swap(i, j);
    }
    // Copy the first m shuffled points as initial centers
    let mut centers = vec![0.0_f64; m * n_dims];
    for j in 0..m {
        let sample_idx = indices[j];
        for d in 0..n_dims {
            centers[d * m + j] = x[d * n_samples + sample_idx];
        }
    }

    // --- Step 2: Lloyd's iterations (max 100) ---
    for _ in 0..100 {
        let assignments = assign_clusters(&centers, x, n_samples, n_dims, m);
        let new_centers =
            compute_centroids(&assignments, &centers, x, n_samples, n_dims, m, &mut rng);

        if has_converged(&centers, &new_centers, m, n_dims) {
            return new_centers;
        }
        centers = new_centers;
    }

    centers
}

// =============================================================================
// Helper functions
// =============================================================================

/// Assign each training sample to its nearest cluster center.
fn assign_clusters(
    centers: &[f64], // column-major: centers[d * m + j]
    x: &[f64],       // column-major: x[d * n_samples + i]
    n_samples: usize,
    n_dims: usize,
    m: usize,
) -> Vec<usize> {
    (0..n_samples)
        .map(|i| {
            let mut best_j = 0;
            let mut best_dist = f64::INFINITY;
            for j in 0..m {
                let sq_dist: f64 = (0..n_dims)
                    .map(|d| {
                        let diff = x[d * n_samples + i] - centers[d * m + j];
                        diff * diff
                    })
                    .sum();
                if sq_dist < best_dist {
                    best_dist = sq_dist;
                    best_j = j;
                }
            }
            best_j
        })
        .collect()
}

/// Compute new cluster centroids. Empty clusters reuse a random training sample.
fn compute_centroids(
    assignments: &[usize],
    _old_centers: &[f64], // kept for API symmetry (not needed after random fallback)
    x: &[f64],
    n_samples: usize,
    n_dims: usize,
    m: usize,
    rng: &mut crate::rf::Lcg,
) -> Vec<f64> {
    let mut sums = vec![0.0_f64; m * n_dims];
    let mut counts = vec![0_usize; m];

    for i in 0..n_samples {
        let c = assignments[i];
        counts[c] += 1;
        for d in 0..n_dims {
            sums[d * m + c] += x[d * n_samples + i];
        }
    }

    let mut centers = vec![0.0_f64; m * n_dims];
    for j in 0..m {
        if counts[j] > 0 {
            for d in 0..n_dims {
                centers[d * m + j] = sums[d * m + j] / counts[j] as f64;
            }
        } else {
            // Empty cluster: reinitialise from a random training sample
            let random_idx = rng.next_usize(n_samples);
            for d in 0..n_dims {
                centers[d * m + j] = x[d * n_samples + random_idx];
            }
        }
    }

    centers
}

/// Return true if all centers moved less than 1e-6 (Euclidean distance).
fn has_converged(old: &[f64], new: &[f64], m: usize, n_dims: usize) -> bool {
    for j in 0..m {
        let shift: f64 = (0..n_dims)
            .map(|d| {
                let diff = old[d * m + j] - new[d * m + j];
                diff * diff
            })
            .sum::<f64>()
            .sqrt();
        if shift >= 1e-6 {
            return false;
        }
    }
    true
}

// =============================================================================
// FITC kernel matrix construction (TASK-1651)
// =============================================================================

/// Build the M×M kernel matrix K_ZZ between inducing points.
///
/// `z`: inducing points in column-major flat layout: `z[dim * m + j]`
/// `m`: number of inducing points
/// `params`: `[log_ls_0, …, log_ls_{d-1}, log_sf, log_sn]`  (len = n_dims + 2)
///
/// Returns row-major flat M×M matrix. A jitter of 1e-6 is added to the diagonal
/// for numerical stability.
pub(crate) fn build_kzz(z: &[f64], m: usize, params: &[f64]) -> Vec<f64> {
    assert!(
        params.len() >= 3,
        "params must have at least n_dims(>=1) + 2 entries"
    );
    let n_dims = params.len() - 2;
    let log_ls = &params[..n_dims];
    let log_sf = params[n_dims];

    let mut kzz = vec![0.0_f64; m * m];
    for i in 0..m {
        let zi: Vec<f64> = (0..n_dims).map(|d| z[d * m + i]).collect();
        for j in i..m {
            let zj: Vec<f64> = (0..n_dims).map(|d| z[d * m + j]).collect();
            let k = crate::kriging::matern52_ard(&zi, &zj, log_ls, log_sf);
            kzz[i * m + j] = k;
            kzz[j * m + i] = k;
        }
        kzz[i * m + i] += 1e-6; // jitter for numerical stability
    }
    kzz
}

/// Build the N×M kernel matrix K_XZ between training points and inducing points.
///
/// `x`: training data in column-major flat layout: `x[dim * n + i]`
/// `z`: inducing points in column-major flat layout: `z[dim * m + j]`
/// `params`: `[log_ls_0, …, log_ls_{d-1}, log_sf, log_sn]`
///
/// Returns row-major flat N×M matrix.
pub(crate) fn build_kxz(x: &[f64], z: &[f64], n: usize, m: usize, params: &[f64]) -> Vec<f64> {
    assert!(
        params.len() >= 3,
        "params must have at least n_dims(>=1) + 2 entries"
    );
    let n_dims = params.len() - 2;
    let log_ls = &params[..n_dims];
    let log_sf = params[n_dims];

    let mut kxz = vec![0.0_f64; n * m];
    for i in 0..n {
        let xi: Vec<f64> = (0..n_dims).map(|d| x[d * n + i]).collect();
        for j in 0..m {
            let zj: Vec<f64> = (0..n_dims).map(|d| z[d * m + j]).collect();
            kxz[i * m + j] = crate::kriging::matern52_ard(&xi, &zj, log_ls, log_sf);
        }
    }
    kxz
}

/// Cholesky decomposition on flat row-major M×M matrix (L lower triangular).
/// Returns flat L, or `None` if the matrix is not positive definite.
pub(crate) fn cholesky_flat(a: &[f64], m: usize) -> Option<Vec<f64>> {
    let mut l = vec![0.0_f64; m * m];
    for i in 0..m {
        for j in 0..=i {
            let mut s = a[i * m + j];
            for k in 0..j {
                s -= l[i * m + k] * l[j * m + k];
            }
            if i == j {
                if s <= 0.0 {
                    return None;
                }
                l[i * m + j] = s.sqrt();
            } else {
                l[i * m + j] = s / l[j * m + j];
            }
        }
    }
    Some(l)
}

/// Forward substitution: solve L · x = b where L is a flat lower-triangular M×M matrix.
pub(crate) fn forward_sub_flat(l: &[f64], b: &[f64], m: usize) -> Vec<f64> {
    let mut x = vec![0.0_f64; m];
    for i in 0..m {
        let mut s = b[i];
        for j in 0..i {
            s -= l[i * m + j] * x[j];
        }
        x[i] = s / l[i * m + i];
    }
    x
}

/// Build FITC diagonal matrices Q_diag and Λ_diag.
///
/// Algorithm:
/// 1. L_ZZ = cholesky(K_ZZ)
/// 2. For each i: v_i = L_ZZ^{-1} K_XZ[i,:]^T  (forward substitution)
/// 3. Q_diag[i] = ‖v_i‖²  (diagonal of Q = K_XZ · K_ZZ^{-1} · K_XZ^T)
/// 4. Λ_diag[i] = max(σ_f² − Q_diag[i], 1e-6) + σ_n²
///
/// Returns `Some((Q_diag, Lambda_diag))` of length N, or `None` if K_ZZ is not PD.
pub(crate) fn build_fitc_matrix(
    kzz: &[f64], // row-major flat M×M
    kxz: &[f64], // row-major flat N×M
    m: usize,
    n: usize,
    log_sf: f64,
    log_sn: f64,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let l_zz = cholesky_flat(kzz, m)?;

    let sigma_f2 = (2.0 * log_sf).exp();
    let sigma_n2 = (2.0 * log_sn).exp();

    let mut q_diag = vec![0.0_f64; n];
    for i in 0..n {
        let row = &kxz[i * m..(i + 1) * m];
        let v = forward_sub_flat(&l_zz, row, m);
        q_diag[i] = v.iter().map(|&vi| vi * vi).sum();
    }

    let lambda_diag: Vec<f64> = q_diag
        .iter()
        .map(|&qi| (sigma_f2 - qi).max(1e-6) + sigma_n2)
        .collect();

    Some((q_diag, lambda_diag))
}

// =============================================================================
// FITC LML optimization + prediction (TASK-1652)
// =============================================================================

/// Backward substitution: solve L^T · x = b where L is a flat lower-triangular M×M matrix.
pub(crate) fn backward_sub_flat(l: &[f64], b: &[f64], m: usize) -> Vec<f64> {
    let mut x = vec![0.0_f64; m];
    for i in (0..m).rev() {
        let mut s = b[i];
        for j in (i + 1)..m {
            s -= l[j * m + i] * x[j]; // l[j][i] = L^T[i][j]
        }
        x[i] = s / l[i * m + i];
    }
    x
}

/// FITC log marginal likelihood using the Woodbury identity.
///
/// `Σ = K_ZZ + K_XZ^T diag(1/Λ) K_XZ`
///
/// `LML = −½ (y^T K_FITC^{-1} y + log|K_FITC| + N log 2π)`
///
/// where:
/// - `y^T K_FITC^{-1} y = Σ y_i²/Λ_i − t^T Σ^{-1} t` (t = K_XZ^T (y/Λ))
/// - `log|K_FITC| = log|Σ| − log|K_ZZ| + Σ log(Λ_i)`
pub(crate) fn fitc_lml(
    x: &[f64], // training, column-major N×n_dims
    z: &[f64], // inducing, column-major M×n_dims
    y: &[f64],
    params: &[f64],
    n: usize,
    m: usize,
) -> f64 {
    let kzz = build_kzz(z, m, params);
    let kxz = build_kxz(x, z, n, m, params);

    let log_sf = params[params.len() - 2];
    let log_sn = params[params.len() - 1];
    let (_, lambda_diag) = match build_fitc_matrix(&kzz, &kxz, m, n, log_sf, log_sn) {
        Some(r) => r,
        None => return f64::NEG_INFINITY,
    };

    // Σ = K_ZZ + K_XZ^T diag(1/Λ) K_XZ  (symmetric, compute upper triangle only)
    let mut sigma = kzz.clone();
    for i in 0..m {
        for j in i..m {
            let s: f64 = (0..n)
                .map(|t| kxz[t * m + i] * kxz[t * m + j] / lambda_diag[t])
                .sum();
            sigma[i * m + j] += s;
            if i != j {
                sigma[j * m + i] += s;
            }
        }
        sigma[i * m + i] += 1e-6; // jitter on Σ diagonal
    }

    let l_zz = match cholesky_flat(&kzz, m) {
        Some(l) => l,
        None => return f64::NEG_INFINITY,
    };
    let l_sigma = match cholesky_flat(&sigma, m) {
        Some(l) => l,
        None => return f64::NEG_INFINITY,
    };

    // log|K_FITC| = log|Σ| − log|K_ZZ| + Σ log(Λ_i)
    let log_det_kzz: f64 = (0..m).map(|k| l_zz[k * m + k].ln()).sum::<f64>() * 2.0;
    let log_det_sigma: f64 = (0..m).map(|k| l_sigma[k * m + k].ln()).sum::<f64>() * 2.0;
    let log_lambda_sum: f64 = lambda_diag.iter().map(|&v| v.ln()).sum();
    let log_det_fitc = log_det_sigma - log_det_kzz + log_lambda_sum;

    // data fit: y^T K_FITC^{-1} y = Σ y_i²/Λ_i − ‖L_Σ^{-1} t‖²
    let u: Vec<f64> = y
        .iter()
        .zip(lambda_diag.iter())
        .map(|(&yi, &li)| yi / li)
        .collect();
    let t: Vec<f64> = (0..m)
        .map(|j| (0..n).map(|i| kxz[i * m + j] * u[i]).sum())
        .collect();
    let w = forward_sub_flat(&l_sigma, &t, m);
    let data_fit: f64 = y
        .iter()
        .zip(u.iter())
        .map(|(&yi, &ui)| yi * ui)
        .sum::<f64>()
        - w.iter().map(|&wi| wi * wi).sum::<f64>();

    let n_f = n as f64;
    -0.5 * (data_fit + log_det_fitc + n_f * (2.0 * std::f64::consts::PI).ln())
}

/// Compute FITC posterior mean weights `w = Σ^{-1} t` where:
/// - `Σ = K_ZZ + K_XZ^T diag(1/Λ) K_XZ`
/// - `t = K_XZ^T (y / Λ)`
///
/// Prediction: `μ(x*) = K_{x*,Z} · w`
///
/// Returns `Some((w, lambda_diag))` or `None` if Cholesky fails.
pub(crate) fn fitc_predict_weights(
    x: &[f64],
    z: &[f64],
    y: &[f64],
    params: &[f64],
    n: usize,
    m: usize,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let kzz = build_kzz(z, m, params);
    let kxz = build_kxz(x, z, n, m, params);

    let log_sf = params[params.len() - 2];
    let log_sn = params[params.len() - 1];
    let (_, lambda_diag) = build_fitc_matrix(&kzz, &kxz, m, n, log_sf, log_sn)?;

    let mut sigma = kzz.clone();
    for i in 0..m {
        for j in i..m {
            let s: f64 = (0..n)
                .map(|t| kxz[t * m + i] * kxz[t * m + j] / lambda_diag[t])
                .sum();
            sigma[i * m + j] += s;
            if i != j {
                sigma[j * m + i] += s;
            }
        }
        sigma[i * m + i] += 1e-6;
    }

    let l_sigma = cholesky_flat(&sigma, m)?;

    let u: Vec<f64> = y
        .iter()
        .zip(lambda_diag.iter())
        .map(|(&yi, &li)| yi / li)
        .collect();
    let t: Vec<f64> = (0..m)
        .map(|j| (0..n).map(|i| kxz[i * m + j] * u[i]).sum())
        .collect();

    let fw = forward_sub_flat(&l_sigma, &t, m);
    let w = backward_sub_flat(&l_sigma, &fw, m);

    Some((w, lambda_diag))
}

/// Optimise FITC hyperparameters via L-BFGS with numerical gradients.
///
/// `params` layout: `[log_ls_0, …, log_ls_{d-1}, log_sf, log_sn]`
/// Returns optimised params.
pub(crate) fn optimize_fitc_hyperparams(
    x: &[f64],
    z: &[f64],
    y: &[f64],
    n: usize,
    m: usize,
    max_iter: usize,
) -> Vec<f64> {
    let n_dims = 2_usize; // 2D PDP
    let n_params = n_dims + 2;
    let mut params = vec![0.0_f64; n_params];
    params[n_params - 1] = -2.0; // initial log_sn

    let mut s_hist: Vec<Vec<f64>> = Vec::new();
    let mut y_hist: Vec<Vec<f64>> = Vec::new();
    let mut lml_history: std::collections::VecDeque<f64> =
        std::collections::VecDeque::with_capacity(6);

    let eps = 1e-4_f64;

    for _ in 0..max_iter {
        let lml = fitc_lml(x, z, y, &params, n, m);
        if !lml.is_finite() {
            break;
        }

        // Early stopping: LML span over last 5 iterations
        lml_history.push_back(lml);
        if lml_history.len() > 5 {
            lml_history.pop_front();
        }
        if lml_history.len() == 5 {
            let span = lml_history.back().unwrap() - lml_history.front().unwrap();
            if span.abs() < 1e-3 {
                break;
            }
        }

        // Numerical gradient (central difference)
        let mut grad = vec![0.0_f64; n_params];
        for d in 0..n_params {
            let mut p_plus = params.clone();
            p_plus[d] += eps;
            let mut p_minus = params.clone();
            p_minus[d] -= eps;
            grad[d] = (fitc_lml(x, z, y, &p_plus, n, m) - fitc_lml(x, z, y, &p_minus, n, m))
                / (2.0 * eps);
        }

        // Negate for minimisation
        let grad_neg: Vec<f64> = grad.iter().map(|g| -g).collect();
        let grad_norm: f64 = grad_neg.iter().map(|g| g * g).sum::<f64>().sqrt();
        if grad_norm < 1e-5 {
            break;
        }

        let d = crate::kriging::lbfgs_direction(&grad_neg, &s_hist, &y_hist);

        // Armijo line search
        let f_x = -lml;
        let neg_lml = |p: &[f64]| -fitc_lml(x, z, y, p, n, m);
        let alpha =
            crate::kriging::armijo_line_search(f_x, &grad_neg, &d, neg_lml, &params, 1e-4, 20);

        // Clamp to prevent extreme log-scale params causing numerical instability
        let x_new: Vec<f64> = params
            .iter()
            .zip(d.iter())
            .map(|(p, di)| (p + alpha * di).clamp(-6.0, 6.0))
            .collect();

        // Gradient at new point for L-BFGS history
        let mut grad_new = vec![0.0_f64; n_params];
        for dd in 0..n_params {
            let mut p_plus = x_new.clone();
            p_plus[dd] += eps;
            let mut p_minus = x_new.clone();
            p_minus[dd] -= eps;
            grad_new[dd] = (fitc_lml(x, z, y, &p_plus, n, m) - fitc_lml(x, z, y, &p_minus, n, m))
                / (2.0 * eps);
        }
        let grad_new_neg: Vec<f64> = grad_new.iter().map(|g| -g).collect();

        let s: Vec<f64> = x_new
            .iter()
            .zip(params.iter())
            .map(|(xn, xo)| xn - xo)
            .collect();
        let yv: Vec<f64> = grad_new_neg
            .iter()
            .zip(grad_neg.iter())
            .map(|(gn, go)| gn - go)
            .collect();

        params = x_new;

        if s_hist.len() >= 5 {
            s_hist.remove(0);
            y_hist.remove(0);
        }
        s_hist.push(s);
        y_hist.push(yv);
    }

    params
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a simple column-major flat array from row-major input.
    fn make_x_flat(data: &[[f64; 2]]) -> Vec<f64> {
        let n = data.len();
        let mut x = vec![0.0; n * 2];
        for i in 0..n {
            x[i] = data[i][0]; // dim 0
            x[n + i] = data[i][1]; // dim 1
        }
        x
    }

    // -------------------------------------------------------------------------
    // TASK-1650 unit tests
    // -------------------------------------------------------------------------

    /// Return length is exactly M * n_dims
    #[test]
    fn tc_1650_01_output_length_is_m_times_n_dims() {
        let n = 200;
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64 / (n * 2) as f64).collect();
        let m = 50;
        let result = select_inducing_points_kmeans(&x, n, 2, m, 42);
        assert_eq!(
            result.len(),
            m * 2,
            "Expected {} elements, got {}",
            m * 2,
            result.len()
        );
    }

    /// All inducing points lie within the [0.0, 1.0] data range
    #[test]
    fn tc_1650_02_inducing_points_within_data_range() {
        let n = 200;
        // x in [0.0, 1.0] for both dims
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64 / (n * 2) as f64).collect();
        let m = 20;
        let result = select_inducing_points_kmeans(&x, n, 2, m, 42);

        for &v in &result {
            assert!(
                v >= 0.0 && v <= 1.0,
                "Inducing point value {} is out of range [0.0, 1.0]",
                v
            );
        }
    }

    /// Same seed produces identical output (reproducibility)
    #[test]
    fn tc_1650_03_reproducible_with_same_seed() {
        let n = 100;
        let data: Vec<[f64; 2]> = (0..n)
            .map(|i| [i as f64 / n as f64, (i as f64 * 0.3).cos()])
            .collect();
        let x = make_x_flat(&data);

        let m = 10;
        let r1 = select_inducing_points_kmeans(&x, n, 2, m, 42);
        let r2 = select_inducing_points_kmeans(&x, n, 2, m, 42);
        assert_eq!(r1, r2, "Same seed should produce identical inducing points");
    }

    /// Different seeds produce different outputs
    #[test]
    fn tc_1650_04_different_seeds_give_different_results() {
        let n = 100;
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64 / (n * 2) as f64).collect();
        let m = 10;
        let r1 = select_inducing_points_kmeans(&x, n, 2, m, 1);
        let r2 = select_inducing_points_kmeans(&x, n, 2, m, 2);
        assert_ne!(
            r1, r2,
            "Different seeds should generally produce different results"
        );
    }

    /// M == N edge case: all points become centers (trivial)
    #[test]
    fn tc_1650_b01_m_equals_n() {
        let n = 5;
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64).collect();
        let result = select_inducing_points_kmeans(&x, n, 2, n, 42);
        assert_eq!(result.len(), n * 2);
    }

    /// M == 1 edge case
    #[test]
    fn tc_1650_b02_m_equals_one() {
        let n = 10;
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64 / n as f64).collect();
        let result = select_inducing_points_kmeans(&x, n, 2, 1, 42);
        assert_eq!(result.len(), 2);
    }

    // -------------------------------------------------------------------------
    // TASK-1651 unit tests
    // -------------------------------------------------------------------------

    /// Default params for 2D: [log_ls_0, log_ls_1, log_sf, log_sn]
    fn default_params() -> Vec<f64> {
        vec![0.0, 0.0, 0.0, -1.0]
    }

    /// Build column-major inducing points from [[x0,x1], ...] rows.
    fn make_z_flat(data: &[[f64; 2]]) -> Vec<f64> {
        let m = data.len();
        let mut z = vec![0.0_f64; m * 2];
        for j in 0..m {
            z[j] = data[j][0];
            z[m + j] = data[j][1];
        }
        z
    }

    /// K_ZZ output length is M×M
    #[test]
    fn tc_1651_01_kzz_output_length_is_m_times_m() {
        let m = 10;
        let z: Vec<f64> = (0..m * 2).map(|i| i as f64 / (m * 2) as f64).collect();
        let kzz = build_kzz(&z, m, &default_params());
        assert_eq!(kzz.len(), m * m);
    }

    /// K_ZZ is symmetric: K_ZZ[i,j] == K_ZZ[j,i]
    #[test]
    fn tc_1651_02_kzz_is_symmetric() {
        let m = 5;
        let data: Vec<[f64; 2]> = (0..m).map(|i| [i as f64 * 0.2, i as f64 * 0.3]).collect();
        let z = make_z_flat(&data);
        let kzz = build_kzz(&z, m, &default_params());
        for i in 0..m {
            for j in 0..m {
                let diff = (kzz[i * m + j] - kzz[j * m + i]).abs();
                assert!(diff < 1e-12, "K_ZZ[{},{}] != K_ZZ[{},{}]", i, j, j, i);
            }
        }
    }

    /// K_ZZ is positive definite (Cholesky succeeds)
    #[test]
    fn tc_1651_03_kzz_is_positive_definite() {
        let m = 10;
        let z: Vec<f64> = (0..m * 2).map(|i| i as f64 / (m * 2) as f64).collect();
        let kzz = build_kzz(&z, m, &default_params());
        assert!(
            cholesky_flat(&kzz, m).is_some(),
            "Cholesky of K_ZZ should succeed"
        );
    }

    /// K_XZ output length is N×M
    #[test]
    fn tc_1651_04_kxz_output_length_is_n_times_m() {
        let n = 100;
        let m = 10;
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64 / (n * 2) as f64).collect();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let kxz = build_kxz(&x, &z, n, m, &default_params());
        assert_eq!(kxz.len(), n * m);
    }

    /// Lambda_diag entries are all positive
    #[test]
    fn tc_1651_05_lambda_diag_all_positive() {
        let n = 20;
        let m = 5;
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64 / (n * 2) as f64).collect();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let params = default_params();
        let kzz = build_kzz(&z, m, &params);
        let kxz = build_kxz(&x, &z, n, m, &params);
        let result = build_fitc_matrix(&kzz, &kxz, m, n, params[2], params[3]);
        assert!(result.is_some(), "build_fitc_matrix should succeed");
        let (_, lambda_diag) = result.unwrap();
        for (i, &v) in lambda_diag.iter().enumerate() {
            assert!(v > 0.0, "Lambda_diag[{}]={} should be positive", i, v);
        }
    }

    /// Jitter ensures PD even when all inducing points are at the same location
    #[test]
    fn tc_1651_b01_jitter_ensures_pd_for_degenerate_inducing_points() {
        let m = 5;
        let z = vec![0.5_f64; m * 2]; // all at (0.5, 0.5)
        let kzz = build_kzz(&z, m, &default_params());
        assert!(
            cholesky_flat(&kzz, m).is_some(),
            "Cholesky should succeed for degenerate inducing points due to jitter"
        );
    }
}
