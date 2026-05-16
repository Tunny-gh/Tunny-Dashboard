//! Sparse Kriging (FITC approximation) for large-N Kriging.
//!
//! Uses inducing points selected via K-means clustering to approximate
//! the full Gaussian Process with O(N·M²) instead of O(N³).

// =============================================================================
// K-means inducing point selection
// =============================================================================

/// Select M inducing points from training data using K-means via the clustering module.
///
/// # Arguments
/// - `x`: training data in column-major flat layout: `x[dim * n_samples + i]`
/// - `n_samples`: number of training samples (N)
/// - `n_dims`: input dimensionality (typically 2 for 2D PDP)
/// - `m`: number of inducing points (typically 50)
/// - `_seed`: unused (clustering module derives its own seed from n and k)
///
/// # Returns
/// Inducing points in column-major flat layout: `result[dim * m + j]`
/// Length = `m * n_dims`.
pub(crate) fn select_inducing_points_kmeans(
    x: &[f64],
    n_samples: usize,
    n_dims: usize,
    m: usize,
    _seed: u64,
) -> Vec<f64> {
    assert!(
        m > 0 && m <= n_samples,
        "Inducing point count M={} must satisfy 0 < M <= N={}",
        m,
        n_samples
    );
    assert!(n_dims > 0, "n_dims must be > 0");

    // Column-major x[d * n_samples + i] → row-major flat_row[i * n_dims + d]
    let flat_row: Vec<f64> = (0..n_samples)
        .flat_map(|i| (0..n_dims).map(move |d| x[d * n_samples + i]))
        .collect();

    let result = crate::clustering::run_kmeans(
        m,
        &flat_row,
        n_dims,
        crate::clustering::InitStrategy::KMeansPlusPlus,
    );

    // centroids[j][d] → column-major out[d * m + j]
    let mut out = vec![0.0_f64; m * n_dims];
    for (j, centroid) in result.centroids.iter().enumerate() {
        for (d, &val) in centroid.iter().enumerate() {
            out[d * m + j] = val;
        }
    }
    out
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
            let k = crate::kriging::gaussian_process::matern52_ard(&zi, &zj, log_ls, log_sf);
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
            kxz[i * m + j] =
                crate::kriging::gaussian_process::matern52_ard(&xi, &zj, log_ls, log_sf);
        }
    }
    kxz
}

/// Cholesky decomposition on flat row-major M×M matrix (L lower triangular).
/// Returns flat L, or `None` if the matrix is not positive definite.
pub(crate) fn cholesky_flat(a: &[f64], m: usize) -> Option<Vec<f64>> {
    let mat = faer::Mat::<f64>::from_fn(m, m, |i, j| a[i * m + j]);
    let chol = mat.llt(faer::Side::Lower).ok()?;
    let l_ref = chol.L();
    let mut result = vec![0.0_f64; m * m];
    for i in 0..m {
        for j in 0..=i {
            result[i * m + j] = l_ref[(i, j)];
        }
    }
    Some(result)
}

/// Forward substitution: solve L · x = b where L is a flat lower-triangular M×M matrix.
pub(crate) fn forward_sub_flat(l: &[f64], b: &[f64], m: usize) -> Vec<f64> {
    let l_mat = faer::Mat::<f64>::from_fn(m, m, |i, j| l[i * m + j]);
    let mut x = faer::Mat::<f64>::from_fn(m, 1, |i, _| b[i]);
    faer::linalg::triangular_solve::solve_lower_triangular_in_place(
        l_mat.as_ref(),
        x.as_mut(),
        faer::Par::Seq,
    );
    (0..m).map(|i| x[(i, 0)]).collect()
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

/// Build Σ = K_ZZ + K_XZ^T diag(1/Λ) K_XZ with a 1e-6 diagonal jitter.
fn build_sigma(kzz: &[f64], kxz: &[f64], lambda_diag: &[f64], m: usize, n: usize) -> Vec<f64> {
    let mut sigma = kzz.to_vec();
    for i in 0..m {
        for j in i..m {
            let s: f64 =
                (0..n).map(|t| kxz[t * m + i] * kxz[t * m + j] / lambda_diag[t]).sum();
            sigma[i * m + j] += s;
            if i != j {
                sigma[j * m + i] += s;
            }
        }
        sigma[i * m + i] += 1e-6;
    }
    sigma
}

/// Backward substitution: solve L^T · x = b where L is a flat lower-triangular M×M matrix.
pub(crate) fn backward_sub_flat(l: &[f64], b: &[f64], m: usize) -> Vec<f64> {
    let l_mat = faer::Mat::<f64>::from_fn(m, m, |i, j| l[i * m + j]);
    let mut x = faer::Mat::<f64>::from_fn(m, 1, |i, _| b[i]);
    faer::linalg::triangular_solve::solve_upper_triangular_in_place(
        l_mat.transpose(),
        x.as_mut(),
        faer::Par::Seq,
    );
    (0..m).map(|i| x[(i, 0)]).collect()
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

    let sigma = build_sigma(&kzz, &kxz, &lambda_diag, m, n);

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
#[allow(dead_code)]
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
    let sigma = build_sigma(&kzz, &kxz, &lambda_diag, m, n);
    let l_sigma = cholesky_flat(&sigma, m)?;

    let u: Vec<f64> = y.iter().zip(lambda_diag.iter()).map(|(&yi, &li)| yi / li).collect();
    let t: Vec<f64> =
        (0..m).map(|j| (0..n).map(|i| kxz[i * m + j] * u[i]).sum()).collect();

    let fw = forward_sub_flat(&l_sigma, &t, m);
    let w = backward_sub_flat(&l_sigma, &fw, m);

    Some((w, lambda_diag))
}

/// L-BFGS two-loop recursion: compute search direction d = −H^{-1} · grad.
fn fitc_lbfgs_direction(
    grad: &[f64],
    s_hist: &[Vec<f64>],
    y_hist: &[Vec<f64>],
) -> Vec<f64> {
    let m = s_hist.len();
    let mut q = grad.to_vec();
    let mut rho = vec![0.0; m];
    let mut alpha = vec![0.0; m];

    for i in (0..m).rev() {
        let sy: f64 = s_hist[i].iter().zip(y_hist[i].iter()).map(|(s, y)| s * y).sum();
        if sy.abs() < 1e-15 {
            continue;
        }
        rho[i] = 1.0 / sy;
        alpha[i] = rho[i] * s_hist[i].iter().zip(q.iter()).map(|(s, qi)| s * qi).sum::<f64>();
        for (qi, yi) in q.iter_mut().zip(y_hist[i].iter()) {
            *qi -= alpha[i] * yi;
        }
    }

    let gamma = if m > 0 {
        let sy: f64 = s_hist[m - 1].iter().zip(y_hist[m - 1].iter()).map(|(s, y)| s * y).sum();
        let yy: f64 = y_hist[m - 1].iter().map(|y| y * y).sum();
        if yy > 1e-15 { sy / yy } else { 1.0 }
    } else {
        1.0
    };
    let mut r: Vec<f64> = q.iter().map(|qi| gamma * qi).collect();

    for i in 0..m {
        let yr: f64 = y_hist[i].iter().zip(r.iter()).map(|(y, ri)| y * ri).sum();
        let beta = rho[i] * yr;
        for (ri, si) in r.iter_mut().zip(s_hist[i].iter()) {
            *ri += (alpha[i] - beta) * si;
        }
    }

    r.iter_mut().for_each(|v| *v = -*v);
    r
}

/// Armijo backtracking line search for FITC optimizer.
fn fitc_armijo_line_search(
    f_x: f64,
    grad: &[f64],
    d: &[f64],
    f: impl Fn(&[f64]) -> f64,
    x: &[f64],
    c1: f64,
    max_iter: usize,
) -> f64 {
    let slope: f64 = grad.iter().zip(d.iter()).map(|(g, di)| g * di).sum();
    let mut alpha = 1.0;
    for _ in 0..max_iter {
        let x_new: Vec<f64> = x.iter().zip(d.iter()).map(|(xi, di)| xi + alpha * di).collect();
        if f(&x_new) <= f_x + c1 * alpha * slope {
            return alpha;
        }
        alpha *= 0.5;
    }
    alpha
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
    // Derive n_dims from column-major x layout (x.len() == n_dims * n)
    let n_dims = if n > 0 { x.len() / n } else { 2 };
    let n_params = n_dims + 2;
    let mut params = vec![0.0_f64; n_params];
    params[n_params - 1] = -2.0; // initial log_sn

    let mut s_hist: Vec<Vec<f64>> = Vec::new();
    let mut y_hist: Vec<Vec<f64>> = Vec::new();
    let mut lml_history: std::collections::VecDeque<f64> =
        std::collections::VecDeque::with_capacity(6);
    // Carry the gradient from the end of iteration k into the start of iteration k+1,
    // saving 2*n_params fitc_lml calls per iteration after the first.
    let mut prev_grad_neg: Option<Vec<f64>> = None;

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

        // Reuse gradient from previous iteration if available; otherwise compute fresh.
        let grad_neg: Vec<f64> = match prev_grad_neg.take() {
            Some(g) => g,
            None => {
                let mut grad = vec![0.0_f64; n_params];
                for d in 0..n_params {
                    let mut p_plus = params.clone();
                    p_plus[d] += eps;
                    let mut p_minus = params.clone();
                    p_minus[d] -= eps;
                    grad[d] = (fitc_lml(x, z, y, &p_plus, n, m)
                        - fitc_lml(x, z, y, &p_minus, n, m))
                        / (2.0 * eps);
                }
                grad.iter().map(|g| -g).collect()
            }
        };

        let grad_norm: f64 = grad_neg.iter().map(|g| g * g).sum::<f64>().sqrt();
        if grad_norm < 1e-5 {
            break;
        }

        let d = fitc_lbfgs_direction(&grad_neg, &s_hist, &y_hist);

        // Armijo line search
        let f_x = -lml;
        let neg_lml = |p: &[f64]| -fitc_lml(x, z, y, p, n, m);
        let alpha = fitc_armijo_line_search(f_x, &grad_neg, &d, neg_lml, &params, 1e-4, 20);

        // Clamp to prevent extreme log-scale params causing numerical instability
        let x_new: Vec<f64> = params
            .iter()
            .zip(d.iter())
            .map(|(p, di)| (p + alpha * di).clamp(-6.0, 6.0))
            .collect();

        // Gradient at new point for L-BFGS history and reuse on the next iteration.
        let mut grad_new = vec![0.0_f64; n_params];
        for dd in 0..n_params {
            let mut p_plus = x_new.clone();
            p_plus[dd] += eps;
            let mut p_minus = x_new.clone();
            p_minus[dd] -= eps;
            grad_new[dd] = (fitc_lml(x, z, y, &p_plus, n, m)
                - fitc_lml(x, z, y, &p_minus, n, m))
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
        prev_grad_neg = Some(grad_new_neg);

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
// FITC posterior model + prediction (TASK-2054)
// =============================================================================

/// Trained sparse FITC model storing all state needed for posterior prediction.
pub(crate) struct SparseFitcModel {
    /// Posterior mean weights: `w = Σ^{-1} t`
    pub w: Vec<f64>,
    /// Flat lower-triangular Cholesky factor of Σ (M×M), used for variance.
    pub l_sigma: Vec<f64>,
    /// Inducing points in column-major flat layout: `z[dim * m + j]`
    pub z: Vec<f64>,
    /// Hyperparameters: `[log_ls_0, …, log_ls_{d-1}, log_sf, log_sn]`
    pub params: Vec<f64>,
    /// Number of inducing points.
    pub m: usize,
}

/// Train a sparse FITC model and return posterior state needed for prediction.
///
/// Analogous to `fitc_predict_weights` but stores `l_sigma` for variance computation.
pub(crate) fn fitc_train(
    x: &[f64],
    z: &[f64],
    y: &[f64],
    params: &[f64],
    n: usize,
    m: usize,
) -> Option<SparseFitcModel> {
    let kzz = build_kzz(z, m, params);
    let kxz = build_kxz(x, z, n, m, params);

    let log_sf = params[params.len() - 2];
    let log_sn = params[params.len() - 1];
    let (_, lambda_diag) = build_fitc_matrix(&kzz, &kxz, m, n, log_sf, log_sn)?;
    let sigma = build_sigma(&kzz, &kxz, &lambda_diag, m, n);
    let l_sigma = cholesky_flat(&sigma, m)?;

    let u: Vec<f64> = y.iter().zip(lambda_diag.iter()).map(|(&yi, &li)| yi / li).collect();
    let t: Vec<f64> =
        (0..m).map(|j| (0..n).map(|i| kxz[i * m + j] * u[i]).sum()).collect();

    let fw = forward_sub_flat(&l_sigma, &t, m);
    let w = backward_sub_flat(&l_sigma, &fw, m);

    Some(SparseFitcModel {
        w,
        l_sigma,
        z: z.to_vec(),
        params: params.to_vec(),
        m,
    })
}

/// Predict the FITC posterior mean at `x_test` (a single point with `n_dims` dimensions).
///
/// `μ(x*) = K_{x*,Z} · w`
pub(crate) fn fitc_predict_mean(model: &SparseFitcModel, x_test: &[f64]) -> f64 {
    let n_dims = x_test.len();
    let log_ls = &model.params[..n_dims];
    let log_sf = model.params[n_dims];

    (0..model.m)
        .map(|j| {
            let zj: Vec<f64> = (0..n_dims).map(|d| model.z[d * model.m + j]).collect();
            let k =
                crate::kriging::gaussian_process::matern52_ard(x_test, &zj, log_ls, log_sf);
            k * model.w[j]
        })
        .sum()
}

/// Predict the FITC posterior variance at `x_test`.
///
/// `var(x*) = k(x*,x*) − ‖L_Σ^{-1} k(Z,x*)‖²`
///
/// Clamped to 0 to prevent negative values from numerical error.
pub(crate) fn fitc_predict_variance(model: &SparseFitcModel, x_test: &[f64]) -> f64 {
    let n_dims = x_test.len();
    let log_ls = &model.params[..n_dims];
    let log_sf = model.params[n_dims];

    // k(x*, x*) — prior variance at the test point
    let k_star_star =
        crate::kriging::gaussian_process::matern52_ard(x_test, x_test, log_ls, log_sf);

    // k(Z, x*) — cross-covariance between inducing points and test point
    let k_z_star: Vec<f64> = (0..model.m)
        .map(|j| {
            let zj: Vec<f64> = (0..n_dims).map(|d| model.z[d * model.m + j]).collect();
            crate::kriging::gaussian_process::matern52_ard(&zj, x_test, log_ls, log_sf)
        })
        .collect();

    // v = L_Σ^{-1} k(Z, x*)
    let v = forward_sub_flat(&model.l_sigma, &k_z_star, model.m);

    // posterior variance = k** - v^T v  (clamped to 0)
    let reduction: f64 = v.iter().map(|vi| vi * vi).sum();
    (k_star_star - reduction).max(0.0)
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

    // -------------------------------------------------------------------------
    // TASK-2054 unit tests: SparseFitcModel / fitc_train / fitc_predict_*
    // -------------------------------------------------------------------------

    fn make_simple_dataset() -> (Vec<f64>, Vec<f64>, usize, usize) {
        let n = 20_usize;
        let m = 5_usize;
        // Column-major 2D training data: x[0..n] = dim0, x[n..2n] = dim1
        let x: Vec<f64> = (0..n * 2).map(|i| i as f64 / (n * 2) as f64).collect();
        let y: Vec<f64> = (0..n).map(|i| (i as f64 / n as f64) * 2.0 - 0.5).collect();
        (x, y, n, m)
    }

    #[test]
    fn fitc_train_returns_some_for_valid_input() {
        let (x, y, n, m) = make_simple_dataset();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let params = default_params();
        let model = fitc_train(&x, &z, &y, &params, n, m);
        assert!(model.is_some(), "fitc_train should succeed for valid input");
    }

    #[test]
    fn fitc_predict_mean_is_finite() {
        let (x, y, n, m) = make_simple_dataset();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let params = default_params();
        let model = fitc_train(&x, &z, &y, &params, n, m).expect("fitc_train should succeed");
        let mean = fitc_predict_mean(&model, &[0.4, 0.6]);
        assert!(mean.is_finite(), "fitc_predict_mean should be finite");
    }

    #[test]
    fn fitc_predict_variance_is_nonnegative() {
        let (x, y, n, m) = make_simple_dataset();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let params = default_params();
        let model = fitc_train(&x, &z, &y, &params, n, m).expect("fitc_train should succeed");
        for xi in [[0.1_f64, 0.2], [0.5, 0.5], [0.9, 0.8]] {
            let var = fitc_predict_variance(&model, &xi);
            assert!(
                var >= 0.0,
                "fitc_predict_variance must be non-negative at {:?}, got {}",
                xi,
                var
            );
        }
    }

    #[test]
    fn fitc_predict_variance_is_finite() {
        let (x, y, n, m) = make_simple_dataset();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let params = default_params();
        let model = fitc_train(&x, &z, &y, &params, n, m).expect("fitc_train should succeed");
        let var = fitc_predict_variance(&model, &[0.5, 0.5]);
        assert!(var.is_finite(), "fitc_predict_variance should be finite");
    }

    // -------------------------------------------------------------------------
    // TASK-2307: faer Cholesky/triangular-solve replacement tests
    // -------------------------------------------------------------------------

    #[test]
    fn tc_102_01_fitc_lml_is_finite_and_negative() {
        let (x, y, n, m) = make_simple_dataset();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let lml = fitc_lml(&x, &z, &y, &default_params(), n, m);
        assert!(lml.is_finite(), "FITC LML should be finite, got {}", lml);
        assert!(lml < 0.0, "FITC LML should be negative (log prob), got {}", lml);
    }

    #[test]
    fn tc_102_02_fitc_predictions_within_data_range() {
        let (x, y, n, m) = make_simple_dataset();
        let z: Vec<f64> = (0..m * 2).map(|j| j as f64 / (m * 2) as f64).collect();
        let params = default_params();
        let model = fitc_train(&x, &z, &y, &params, n, m).expect("fitc_train should succeed");
        let mean = fitc_predict_mean(&model, &[0.5, 0.5]);
        let var = fitc_predict_variance(&model, &[0.5, 0.5]);
        let y_min = y.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let margin = (y_max - y_min).max(1.0);
        assert!(
            mean >= y_min - margin && mean <= y_max + margin,
            "FITC mean {} out of expected range [{}, {}]",
            mean,
            y_min - margin,
            y_max + margin
        );
        assert!(var >= 0.0, "variance must be non-negative");
    }

    #[test]
    fn tc_102_e01_cholesky_flat_returns_none_for_non_pd() {
        // Matrix [[1, 2], [2, 1]] has eigenvalues -1 and 3, not PD
        let non_pd = vec![1.0_f64, 2.0, 2.0, 1.0];
        assert!(
            cholesky_flat(&non_pd, 2).is_none(),
            "cholesky_flat should return None for non-PD matrix"
        );
    }
}
