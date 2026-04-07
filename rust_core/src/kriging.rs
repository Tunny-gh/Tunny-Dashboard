//! Gaussian Process regression (Kriging) with ARD Matérn 5/2 kernel.
//!
//! Pure-Rust implementation — no external crates.
//! Used for 2D PDP surface computation in `pdp.rs`.

// =============================================================================
// GP Model struct
// =============================================================================

/// Trained Gaussian Process model.
pub(crate) struct GpModel {
    /// K^{-1} y
    pub alpha: Vec<f64>,
    /// Training data (after subsampling)
    pub x_train: Vec<Vec<f64>>,
    /// ARD log-length-scales (one per input dimension)
    pub log_ls: Vec<f64>,
    /// Log signal variance
    pub log_sf: f64,
}

// =============================================================================
// Cholesky decomposition and triangular solvers
// =============================================================================

/// Cholesky decomposition: A = L · L^T
///
/// Returns the lower triangular factor `L`, or `None` if the matrix is not
/// positive definite.  A jitter of `1e-6` is added to every diagonal element
/// for numerical stability.
pub(crate) fn cholesky(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut l = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            for k in 0..j {
                sum -= l[i][k] * l[j][k];
            }
            if i == j {
                let val = sum + 1e-6; // jitter for stability
                if val <= 0.0 {
                    return None;
                }
                l[i][j] = val.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Some(l)
}

/// Forward substitution: solve L · x = b  (L is lower triangular).
pub(crate) fn forward_sub(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut x = vec![0.0; n];
    for i in 0..n {
        let mut s = b[i];
        for j in 0..i {
            s -= l[i][j] * x[j];
        }
        x[i] = s / l[i][i];
    }
    x
}

/// Backward substitution: solve L^T · x = b  (L^T is upper triangular).
pub(crate) fn backward_sub(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in (i + 1)..n {
            s -= l[j][i] * x[j]; // l[j][i] == L^T[i][j]
        }
        x[i] = s / l[i][i];
    }
    x
}

// =============================================================================
// ARD Matérn 5/2 kernel
// =============================================================================

/// ARD Matérn 5/2 kernel:
///   k(x1,x2) = σ_f² · (1 + √5·r + 5r²/3) · exp(−√5·r)
///   r²        = Σ_d ((x1_d − x2_d) / l_d)²
///
/// `log_ls`: log-length-scales per dimension
/// `log_sf`: log signal standard deviation  (σ_f = exp(log_sf))
pub(crate) fn matern52_ard(x1: &[f64], x2: &[f64], log_ls: &[f64], log_sf: f64) -> f64 {
    let sigma_f2 = (2.0 * log_sf).exp();
    let r2: f64 = x1
        .iter()
        .zip(x2.iter())
        .zip(log_ls.iter())
        .map(|((a, b), &ll)| {
            let l = ll.exp();
            ((a - b) / l).powi(2)
        })
        .sum();
    let r = r2.sqrt();
    let sqrt5_r = 5.0_f64.sqrt() * r;
    sigma_f2 * (1.0 + sqrt5_r + 5.0 * r2 / 3.0) * (-sqrt5_r).exp()
}

/// ∂k/∂log(l_d) for ARD Matérn 5/2:
///   = σ_f² · (5/3) · (x1_d−x2_d)²/l_d² · (1 + √5·r) · exp(−√5·r)
fn matern52_ard_grad_ld(x1: &[f64], x2: &[f64], log_ls: &[f64], log_sf: f64, dim: usize) -> f64 {
    let sigma_f2 = (2.0 * log_sf).exp();
    let r2: f64 = x1
        .iter()
        .zip(x2.iter())
        .zip(log_ls.iter())
        .map(|((a, b), &ll)| ((a - b) / ll.exp()).powi(2))
        .sum();
    let r = r2.sqrt();
    let sqrt5_r = 5.0_f64.sqrt() * r;
    let l_d = log_ls[dim].exp();
    let d_sq = (x1[dim] - x2[dim]).powi(2) / l_d.powi(2);
    sigma_f2 * (5.0 / 3.0) * d_sq * (1.0 + sqrt5_r) * (-sqrt5_r).exp()
}

// =============================================================================
// Kernel matrix construction
// =============================================================================

/// Build the N×N kernel matrix K with noise:
///   K[i,j] = matern52_ard(x_i, x_j) + σ_n² · δ_{ij}
///
/// `log_sn`: log noise standard deviation  (σ_n = exp(log_sn))
pub(crate) fn build_kernel_matrix(
    x: &[Vec<f64>],
    log_ls: &[f64],
    log_sf: f64,
    log_sn: f64,
) -> Vec<Vec<f64>> {
    let n = x.len();
    let sigma_n2 = (2.0 * log_sn).exp();
    let mut k = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let kij = matern52_ard(&x[i], &x[j], log_ls, log_sf);
            k[i][j] = kij;
            k[j][i] = kij;
        }
        k[i][i] += sigma_n2;
    }
    k
}

// =============================================================================
// Log marginal likelihood and analytical gradient
// =============================================================================

/// Compute alpha = K^{-1} y via Cholesky factor L.
pub(crate) fn compute_alpha(l: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let v = forward_sub(l, y);
    backward_sub(l, &v)
}

/// Log marginal likelihood:
///   L = −½ y^T α − Σ_i log(L_ii) − n/2 log(2π)
pub(crate) fn log_marginal_likelihood(
    x: &[Vec<f64>],
    y: &[f64],
    log_ls: &[f64],
    log_sf: f64,
    log_sn: f64,
) -> f64 {
    let k = build_kernel_matrix(x, log_ls, log_sf, log_sn);
    let l = match cholesky(&k) {
        Some(l) => l,
        None => return f64::NEG_INFINITY,
    };
    let alpha = compute_alpha(&l, y);

    let data_fit: f64 = y.iter().zip(alpha.iter()).map(|(yi, ai)| yi * ai).sum();
    let log_det: f64 = l.iter().enumerate().map(|(i, row)| row[i].ln()).sum();
    let n = y.len() as f64;

    -0.5 * data_fit - log_det - 0.5 * n * (2.0 * std::f64::consts::PI).ln()
}

// =============================================================================
// Combined LML + gradient computation (Phase 1 optimization)
// =============================================================================

/// Compute log marginal likelihood and its gradient in a single pass.
///
/// More efficient than calling `log_marginal_likelihood` and `log_ml_gradient`
/// separately because it performs only one kernel matrix construction and
/// one Cholesky decomposition instead of two.
///
/// # Arguments
/// - `x`: training data (n_samples × n_dims)
/// - `y`: target values (n_samples,)
/// - `params`: hyperparameters [log_ls_0, …, log_ls_{d-1}, log_sf, log_sn]
///
/// # Returns
/// `(lml_value, gradient_vec)` — LML value (to be maximised) and 4-dim gradient
pub(crate) fn log_ml_with_gradient(x: &[Vec<f64>], y: &[f64], params: &[f64]) -> (f64, Vec<f64>) {
    if x.is_empty() {
        return (f64::NEG_INFINITY, vec![0.0; params.len()]);
    }
    let ndim = x[0].len();
    let log_ls = &params[..ndim];
    let log_sf = params[ndim];
    let log_sn = params[ndim + 1];
    let n = y.len();

    // 1. Build kernel matrix once
    let k = build_kernel_matrix(x, log_ls, log_sf, log_sn);

    // 2. Cholesky decomposition once
    let l = match cholesky(&k) {
        Some(l) => l,
        None => return (f64::NEG_INFINITY, vec![0.0; params.len()]),
    };

    // 3. Compute alpha = K^{-1} y once
    let alpha = compute_alpha(&l, y);

    // 4. Compute LML value: −½ y^T α − Σ log(L_ii) − n/2 log(2π)
    let data_fit: f64 = y.iter().zip(alpha.iter()).map(|(yi, ai)| yi * ai).sum();
    let log_det: f64 = l.iter().enumerate().map(|(i, row)| row[i].ln()).sum();
    let lml = -0.5 * data_fit - log_det - 0.5 * n as f64 * (2.0 * std::f64::consts::PI).ln();

    // 5. Compute K^{-1} column by column (needed for gradient)
    let k_inv: Vec<Vec<f64>> = (0..n)
        .map(|j| {
            let e_j: Vec<f64> = (0..n).map(|i| if i == j { 1.0 } else { 0.0 }).collect();
            let v = forward_sub(&l, &e_j);
            backward_sub(&l, &v)
        })
        .collect();

    // 6. Gradient: ∂L/∂θⱼ = ½ tr((αα^T − K^{-1}) · ∂K/∂θⱼ)
    let mut grad = vec![0.0; params.len()];

    // ∂L/∂log(l_d) for each length-scale dimension
    for d in 0..ndim {
        let mut tr = 0.0;
        for i in 0..n {
            for j in 0..n {
                let w_ij = alpha[i] * alpha[j] - k_inv[j][i];
                let dk_ij = matern52_ard_grad_ld(&x[i], &x[j], log_ls, log_sf, d);
                tr += w_ij * dk_ij;
            }
        }
        grad[d] = 0.5 * tr;
    }

    // ∂L/∂log(σ_f): ∂k/∂log(σ_f) = 2·k(x1,x2)
    {
        let mut tr = 0.0;
        for i in 0..n {
            for j in 0..n {
                let w_ij = alpha[i] * alpha[j] - k_inv[j][i];
                let dk_ij = 2.0 * matern52_ard(&x[i], &x[j], log_ls, log_sf);
                tr += w_ij * dk_ij;
            }
        }
        grad[ndim] = 0.5 * tr;
    }

    // ∂L/∂log(σ_n): ∂K/∂log(σ_n) = 2σ_n²·I → trace(W·2σ_n²·I) = 2σ_n²·tr(W)
    {
        let sigma_n2 = (2.0 * log_sn).exp();
        let tr_w: f64 = (0..n).map(|i| alpha[i].powi(2) - k_inv[i][i]).sum();
        grad[ndim + 1] = sigma_n2 * tr_w;
    }

    (lml, grad)
}

// =============================================================================
// L-BFGS optimizer
// =============================================================================

/// L-BFGS Two-loop recursion: compute search direction d = −H^{-1} · grad.
///
/// `s_hist[k]` = x_{k+1} − x_k
/// `y_hist[k]` = grad_{k+1} − grad_k
pub(crate) fn lbfgs_direction(grad: &[f64], s_hist: &[Vec<f64>], y_hist: &[Vec<f64>]) -> Vec<f64> {
    let m = s_hist.len();
    let mut q = grad.to_vec();
    let mut rho = vec![0.0; m];
    let mut alpha = vec![0.0; m];

    // First loop (backward)
    for i in (0..m).rev() {
        let sy: f64 = s_hist[i]
            .iter()
            .zip(y_hist[i].iter())
            .map(|(s, y)| s * y)
            .sum();
        if sy.abs() < 1e-15 {
            continue;
        }
        rho[i] = 1.0 / sy;
        alpha[i] = rho[i]
            * s_hist[i]
                .iter()
                .zip(q.iter())
                .map(|(s, qi)| s * qi)
                .sum::<f64>();
        for (qi, yi) in q.iter_mut().zip(y_hist[i].iter()) {
            *qi -= alpha[i] * yi;
        }
    }

    // Initial Hessian approximation: H_0 = (s_{m-1}^T y_{m-1}) / (y_{m-1}^T y_{m-1}) · I
    let gamma = if m > 0 {
        let sy: f64 = s_hist[m - 1]
            .iter()
            .zip(y_hist[m - 1].iter())
            .map(|(s, y)| s * y)
            .sum();
        let yy: f64 = y_hist[m - 1].iter().map(|y| y * y).sum();
        if yy > 1e-15 {
            sy / yy
        } else {
            1.0
        }
    } else {
        1.0
    };
    let mut r: Vec<f64> = q.iter().map(|qi| gamma * qi).collect();

    // Second loop (forward)
    for i in 0..m {
        let yr: f64 = y_hist[i].iter().zip(r.iter()).map(|(y, ri)| y * ri).sum();
        let beta = rho[i] * yr;
        for (ri, si) in r.iter_mut().zip(s_hist[i].iter()) {
            *ri += (alpha[i] - beta) * si;
        }
    }

    r.iter_mut().for_each(|v| *v = -*v); // d = −H^{-1} grad
    r
}

/// Armijo backtracking line search.
///
/// Returns step size α satisfying: f(x + α·d) ≤ f(x) + c₁·α·(grad^T · d).
pub(crate) fn armijo_line_search(
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
        let x_new: Vec<f64> = x
            .iter()
            .zip(d.iter())
            .map(|(xi, di)| xi + alpha * di)
            .collect();
        if f(&x_new) <= f_x + c1 * alpha * slope {
            return alpha;
        }
        alpha *= 0.5;
    }
    alpha
}

/// Optimize GP hyperparameters via L-BFGS (maximise log marginal likelihood).
///
/// `params` layout: [log_ls_0, …, log_ls_{d-1}, log_sf, log_sn]
///
/// Returns `(optimised_params, actual_iterations_run)`.
pub(crate) fn optimize_hyperparams(
    x: &[Vec<f64>],
    y: &[f64],
    n_iter: usize,
    m_history: usize,
) -> (Vec<f64>, usize) {
    if x.is_empty() {
        return (vec![], 0);
    }
    let ndim = x[0].len();
    let mut params = vec![0.0; ndim + 2];
    params[ndim + 1] = -2.0; // initial log_sn: σ_n ≈ 0.135

    let mut s_hist: Vec<Vec<f64>> = Vec::new();
    let mut y_hist: Vec<Vec<f64>> = Vec::new();
    let mut lml_history: std::collections::VecDeque<f64> =
        std::collections::VecDeque::with_capacity(6);

    // Closure: −log marginal likelihood for Armijo line search only
    // (gradient not needed during line search, so we keep the lightweight version)
    let neg_lml = |p: &[f64]| -log_marginal_likelihood(x, y, &p[..ndim], p[ndim], p[ndim + 1]);

    let mut actual_iter = 0;
    for _ in 0..n_iter {
        // Combined LML + gradient in one Cholesky decomposition (Phase 1 optimization)
        let (lml, grad_raw) = log_ml_with_gradient(x, y, &params);
        actual_iter += 1;

        // Early stopping: LML history span over last 5 iterations
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

        // Gradient of −LML (negate for minimisation)
        let grad_neg: Vec<f64> = grad_raw.iter().map(|g| -g).collect();

        // Early stopping: gradient norm convergence
        let grad_norm: f64 = grad_neg.iter().map(|g| g * g).sum::<f64>().sqrt();
        if grad_norm < 1e-5 {
            break;
        }

        let d = lbfgs_direction(&grad_neg, &s_hist, &y_hist);
        let f_x = -lml; // reuse already-computed LML value (no extra Cholesky)
        let alpha = armijo_line_search(f_x, &grad_neg, &d, &neg_lml, &params, 1e-4, 20);

        let x_new: Vec<f64> = params
            .iter()
            .zip(d.iter())
            .map(|(p, di)| p + alpha * di)
            .collect();

        // Combined computation for x_new: get gradient for L-BFGS history update
        let (_, grad_new_raw) = log_ml_with_gradient(x, y, &x_new);
        let grad_new: Vec<f64> = grad_new_raw.iter().map(|g| -g).collect();

        let s: Vec<f64> = x_new
            .iter()
            .zip(params.iter())
            .map(|(xn, xo)| xn - xo)
            .collect();
        let yv: Vec<f64> = grad_new
            .iter()
            .zip(grad_neg.iter())
            .map(|(gn, go)| gn - go)
            .collect();

        params = x_new;

        if s_hist.len() >= m_history {
            s_hist.remove(0);
            y_hist.remove(0);
        }
        s_hist.push(s);
        y_hist.push(yv);
    }
    (params, actual_iter)
}

// =============================================================================
// GP training and prediction
// =============================================================================

/// Train a GP model (with optional subsampling for large datasets).
///
/// If `x.len() > subsample_n`, randomly subsample `subsample_n` points.
pub(crate) fn train_gp(
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
    subsample_n: usize,
    seed: u64,
) -> Option<GpModel> {
    let (x_sub, y_sub) = if x.len() > subsample_n {
        let mut rng = crate::rf::Lcg::new(seed);
        let n = x.len();
        let mut indices: Vec<usize> = (0..n).collect();
        // Fisher-Yates shuffle to pick first subsample_n elements
        for i in (1..n).rev() {
            let j = rng.next_usize(i + 1);
            indices.swap(i, j);
        }
        let idx = &indices[..subsample_n];
        let xs: Vec<Vec<f64>> = idx.iter().map(|&i| x[i].clone()).collect();
        let ys: Vec<f64> = idx.iter().map(|&i| y[i]).collect();
        (xs, ys)
    } else {
        (x, y)
    };

    if x_sub.is_empty() {
        return None;
    }
    let ndim = x_sub[0].len();

    // In debug builds use fewer iterations to keep test times manageable.
    // Release builds use the full 50 iterations for convergence quality.
    let n_iter = if cfg!(debug_assertions) { 5 } else { 50 };
    let (params, _) = optimize_hyperparams(&x_sub, &y_sub, n_iter, 5);
    if params.is_empty() {
        return None;
    }
    let log_ls = params[..ndim].to_vec();
    let log_sf = params[ndim];
    let log_sn = params[ndim + 1];

    let k = build_kernel_matrix(&x_sub, &log_ls, log_sf, log_sn);
    let l = cholesky(&k)?;
    let alpha = compute_alpha(&l, &y_sub);

    Some(GpModel {
        alpha,
        x_train: x_sub,
        log_ls,
        log_sf,
    })
}

/// Predict the GP posterior mean at `x_test`.
///
///   μ(x*) = k(x*, X) · alpha = Σ_i alpha_i · k(x*, x_i)
pub(crate) fn predict_mean(model: &GpModel, x_test: &[f64]) -> f64 {
    model
        .x_train
        .iter()
        .zip(model.alpha.iter())
        .map(|(xi, ai)| ai * matern52_ard(x_test, xi, &model.log_ls, model.log_sf))
        .sum()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // TASK-1633: Cholesky, forward/backward sub, kernel
    // -------------------------------------------------------------------------

    /// TC1: Cholesky decomposition correctness: L*L^T ≈ A
    #[test]
    fn tc_1633_01_cholesky_correctness() {
        let a = vec![
            vec![4.0, 2.0, 1.0],
            vec![2.0, 5.0, 2.0],
            vec![1.0, 2.0, 6.0],
        ];
        let l = cholesky(&a).expect("Should succeed on positive-definite matrix");
        let n = a.len();
        for i in 0..n {
            for j in 0..n {
                let reconstructed: f64 = (0..n).map(|k| l[i][k] * l[j][k]).sum();
                let expected = a[i][j] + if i == j { 1e-6 } else { 0.0 }; // jitter on diagonal
                assert!(
                    (reconstructed - expected).abs() < 1e-6,
                    "L*L^T[{},{}] = {} != A[{},{}] = {}",
                    i,
                    j,
                    reconstructed,
                    i,
                    j,
                    expected
                );
            }
        }
    }

    /// TC2: forward_sub + backward_sub solves K·x = b correctly.
    #[test]
    fn tc_1633_02_solve_linear_system() {
        let a = vec![
            vec![4.0, 2.0, 0.0],
            vec![2.0, 5.0, 1.0],
            vec![0.0, 1.0, 4.0],
        ];
        let b = vec![1.0, 2.0, 3.0];
        let l = cholesky(&a).expect("Cholesky should succeed");
        let x = {
            let v = forward_sub(&l, &b);
            backward_sub(&l, &v)
        };

        // Verify A_jitter · x ≈ b  (A_jitter = A + 1e-6·I)
        let n = a.len();
        for i in 0..n {
            let ax: f64 = (0..n)
                .map(|j| {
                    let aij = a[i][j] + if i == j { 1e-6 } else { 0.0 };
                    aij * x[j]
                })
                .sum();
            assert!(
                (ax - b[i]).abs() < 1e-6,
                "A·x[{}] = {} != b[{}] = {}",
                i,
                ax,
                i,
                b[i]
            );
        }
    }

    /// TC3: Matérn 5/2 at identical points equals σ_f².
    #[test]
    fn tc_1633_03_matern52_same_point() {
        let x = vec![1.0, 2.0];
        let log_ls = vec![0.0, 0.0];
        let log_sf = 0.0; // σ_f = 1 → σ_f² = 1
        let k = matern52_ard(&x, &x, &log_ls, log_sf);
        assert!(
            (k - 1.0).abs() < 1e-12,
            "k(x,x) should be σ_f² = 1, got {}",
            k
        );
    }

    /// TC4: Kernel matrix is symmetric.
    #[test]
    fn tc_1633_04_kernel_matrix_symmetric() {
        let x: Vec<Vec<f64>> = (0..5)
            .map(|i| vec![i as f64 * 0.3, i as f64 * 0.7 - 1.0])
            .collect();
        let log_ls = vec![0.0, 0.0];
        let k = build_kernel_matrix(&x, &log_ls, 0.0, -2.0);
        let n = x.len();
        for i in 0..n {
            for j in 0..n {
                assert!(
                    (k[i][j] - k[j][i]).abs() < 1e-12,
                    "K[{},{}]={} != K[{},{}]={}",
                    i,
                    j,
                    k[i][j],
                    j,
                    i,
                    k[j][i]
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // TASK-1634: Log marginal likelihood and gradient
    // -------------------------------------------------------------------------

    /// TC5: log_marginal_likelihood returns a finite value.
    #[test]
    fn tc_1634_01_lml_finite() {
        let n = 10;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 * 0.1, (i as f64 * 0.2).sin()])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] + xi[1]).collect();
        let lml = log_marginal_likelihood(&x, &y, &[0.0, 0.0], 0.0, -2.0);
        assert!(lml.is_finite(), "LML should be finite, got {}", lml);
    }

    /// TC6: Analytical gradient matches numerical finite difference.
    #[test]
    fn tc_1634_02_gradient_matches_finite_diff() {
        let n = 10;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 * 0.15, (i as f64 * 0.25 + 0.1).cos()])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] * 2.0 - xi[1] * 0.5).collect();
        let params = vec![0.0_f64, 0.0, 0.0, -2.0]; // [log_ls0, log_ls1, log_sf, log_sn]
        let ndim = 2;

        let analytical = log_ml_gradient(&x, &y, &params);
        let eps = 1e-5;

        for d in 0..params.len() {
            let mut p_plus = params.clone();
            p_plus[d] += eps;
            let mut p_minus = params.clone();
            p_minus[d] -= eps;
            let lml_plus =
                log_marginal_likelihood(&x, &y, &p_plus[..ndim], p_plus[ndim], p_plus[ndim + 1]);
            let lml_minus =
                log_marginal_likelihood(&x, &y, &p_minus[..ndim], p_minus[ndim], p_minus[ndim + 1]);
            let numerical = (lml_plus - lml_minus) / (2.0 * eps);
            let rel_err = (analytical[d] - numerical).abs() / (numerical.abs() + 1e-8);
            assert!(
                rel_err < 1e-3,
                "Gradient dim {} analytical={} numerical={} rel_err={}",
                d,
                analytical[d],
                numerical,
                rel_err
            );
        }
    }

    // -------------------------------------------------------------------------
    // TASK-1635: L-BFGS optimization and train_gp
    // -------------------------------------------------------------------------

    /// TC7: optimize_hyperparams improves log marginal likelihood.
    #[test]
    fn tc_1635_01_optimize_improves_lml() {
        let n = 20;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|xi| xi[0] * 2.0 + (xi[0] * 6.0).sin() * 0.1)
            .collect();
        let ndim = 1;

        let initial_params = vec![0.0_f64, 0.0, -2.0]; // [log_ls, log_sf, log_sn]
        let initial_lml = log_marginal_likelihood(
            &x,
            &y,
            &initial_params[..ndim],
            initial_params[ndim],
            initial_params[ndim + 1],
        );

        let (opt_params, _) = optimize_hyperparams(&x, &y, 20, 5);
        let final_lml = log_marginal_likelihood(
            &x,
            &y,
            &opt_params[..ndim],
            opt_params[ndim],
            opt_params[ndim + 1],
        );

        assert!(
            final_lml >= initial_lml - 0.1, // allow tiny regression due to numerics
            "Optimised LML {} should be >= initial LML {}",
            final_lml,
            initial_lml
        );
    }

    /// TC8: train_gp subsamples when n > subsample_n.
    #[test]
    fn tc_1635_02_train_gp_subsampling() {
        let n = 50; // use small n for speed
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64, 0.0]).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0]).collect();

        let model = train_gp(x, y, 30, 42).expect("train_gp should succeed");
        assert_eq!(
            model.x_train.len(),
            30,
            "Model should be trained on 30 subsampled points"
        );
    }

    // -------------------------------------------------------------------------
    // TASK-1642: log_ml_with_gradient 統合計算
    // -------------------------------------------------------------------------

    /// TC-002-01: log_ml_with_gradient の LML 値が log_marginal_likelihood と一致する
    #[test]
    fn tc_1642_01_lml_value_matches() {
        let n = 10;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 * 0.1, (i as f64 * 0.2).sin()])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] + xi[1]).collect();
        let params = vec![0.0f64, 0.0, 0.0, -2.0]; // [log_ls0, log_ls1, log_sf, log_sn]
        let ndim = 2;

        let reference_lml =
            log_marginal_likelihood(&x, &y, &params[..ndim], params[ndim], params[ndim + 1]);
        let (unified_lml, _) = log_ml_with_gradient(&x, &y, &params);

        assert!(
            (unified_lml - reference_lml).abs() < 1e-10,
            "LML mismatch: unified={} reference={} diff={}",
            unified_lml,
            reference_lml,
            (unified_lml - reference_lml).abs()
        );
    }

    /// TC-002-02: log_ml_with_gradient の勾配が log_ml_gradient と一致する
    #[test]
    fn tc_1642_02_gradient_matches() {
        let n = 10;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 * 0.1, (i as f64 * 0.2).sin()])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] + xi[1]).collect();
        let params = vec![0.0f64, 0.0, 0.0, -2.0];

        let reference_grad = log_ml_gradient(&x, &y, &params);
        let (_, unified_grad) = log_ml_with_gradient(&x, &y, &params);

        assert_eq!(
            unified_grad.len(),
            reference_grad.len(),
            "Gradient dimension mismatch"
        );
        for (d, (u, r)) in unified_grad.iter().zip(reference_grad.iter()).enumerate() {
            let rel_err = (u - r).abs() / (r.abs() + 1e-8);
            assert!(
                rel_err < 1e-8,
                "Gradient dim {} mismatch: unified={} reference={} rel_err={}",
                d,
                u,
                r,
                rel_err
            );
        }
    }

    /// TC-002-03: optimize_hyperparams（統合計算使用後）も収束する
    #[test]
    fn tc_1642_03_optimize_hyperparams_still_converges() {
        let n = 20;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 / n as f64, (i as f64 * 0.5).cos()])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] * 2.0 - xi[1]).collect();
        let ndim = 2;

        let initial_params = vec![0.0f64, 0.0, 0.0, -2.0];
        let initial_lml = log_marginal_likelihood(
            &x,
            &y,
            &initial_params[..ndim],
            initial_params[ndim],
            initial_params[ndim + 1],
        );

        let (opt_params, _) = optimize_hyperparams(&x, &y, 50, 5);
        let final_lml = log_marginal_likelihood(
            &x,
            &y,
            &opt_params[..ndim],
            opt_params[ndim],
            opt_params[ndim + 1],
        );

        assert!(
            final_lml >= initial_lml - 0.1,
            "Optimised LML {} should be >= initial LML {}",
            final_lml,
            initial_lml
        );
    }

    // -------------------------------------------------------------------------
    // TASK-1643: L-BFGS max_iter=50 + 早期停止
    // -------------------------------------------------------------------------

    /// TC-003-01: max_iter=50 での収束品質が max_iter=100 の 95% 以上
    #[test]
    fn tc_1643_01_max_iter_50_convergence_quality() {
        let n = 20;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 / n as f64, (i as f64 * 0.5).cos()])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] * 2.0 - xi[1]).collect();
        let ndim = 2;

        let (params_100, _) = optimize_hyperparams(&x, &y, 100, 5);
        let (params_50, _) = optimize_hyperparams(&x, &y, 50, 5);

        let lml_100 = log_marginal_likelihood(
            &x,
            &y,
            &params_100[..ndim],
            params_100[ndim],
            params_100[ndim + 1],
        );
        let lml_50 = log_marginal_likelihood(
            &x,
            &y,
            &params_50[..ndim],
            params_50[ndim],
            params_50[ndim + 1],
        );

        // 50 iterations should achieve at least 95% quality of 100 iterations.
        // Since LML can be negative, use: lml_50 >= lml_100 - |lml_100| * 0.05
        let tolerance = lml_100.abs() * 0.05 + 1.0; // +1 for near-zero safety margin
        assert!(
            lml_50 >= lml_100 - tolerance,
            "LML(50)={} should be within 5% of LML(100)={}",
            lml_50,
            lml_100
        );
    }

    /// TC-003-02: 早期停止が機能し、実行イテレーション数が max_iter より少ない
    #[test]
    fn tc_1643_02_early_stopping_triggers() {
        // Constant y: no signal → gradient norm converges to near-zero quickly.
        let n = 10;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y = vec![1.0_f64; n];

        let (_, iters) = optimize_hyperparams(&x, &y, 100, 5);
        assert!(
            iters < 100,
            "Early stopping should trigger before max_iter=100, but ran {} iterations",
            iters
        );
    }

    /// TC-003-B01: max_iter=0 の境界値テスト — 初期パラメータがそのまま返る
    #[test]
    fn tc_1643_b01_max_iter_zero_returns_initial() {
        let n = 5;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0]).collect();
        let ndim = 1;

        let initial_log_sn = -2.0_f64;
        let (params, iters) = optimize_hyperparams(&x, &y, 0, 5);

        assert_eq!(iters, 0, "max_iter=0 should run 0 iterations");
        assert_eq!(params.len(), ndim + 2, "Should return full param vector");
        assert!(
            (params[ndim + 1] - initial_log_sn).abs() < 1e-10,
            "Initial log_sn should be unchanged: expected {}, got {}",
            initial_log_sn,
            params[ndim + 1]
        );
    }

    // -------------------------------------------------------------------------
    // TASK-1644: subsample_n 1000→500 変更
    // -------------------------------------------------------------------------

    /// TC-004-01: N > 500 の場合に 500 点にサブサンプリングされる
    #[test]
    fn tc_1644_01_subsample_n_500_when_n_gt_500() {
        let n = 600;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0]).collect();

        let model = train_gp(x, y, 500, 42).expect("train_gp should succeed");
        assert_eq!(
            model.x_train.len(),
            500,
            "x_train should be subsampled to 500, got {}",
            model.x_train.len()
        );
    }

    /// TC-004-02: N ≤ 500 の場合はサブサンプリングされない
    #[test]
    fn tc_1644_02_no_subsample_when_n_le_500() {
        let n = 300;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0]).collect();

        let model = train_gp(x, y, 500, 42).expect("train_gp should succeed");
        assert_eq!(
            model.x_train.len(),
            300,
            "x_train should keep all 300 points, got {}",
            model.x_train.len()
        );
    }

    /// TC-004-03: N=500 の境界値でサブサンプリングなし
    #[test]
    fn tc_1644_03_boundary_n_equals_500() {
        let n = 500;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0]).collect();

        let model = train_gp(x, y, 500, 42).expect("train_gp should succeed");
        assert_eq!(
            model.x_train.len(),
            500,
            "x_train should keep all 500 points (boundary), got {}",
            model.x_train.len()
        );
    }

    /// TC9: GP predicts reasonably on training points (low RMSE for smooth data).
    #[test]
    fn tc_1635_03_gp_prediction_quality() {
        let n = 15;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|xi| (xi[0] * std::f64::consts::PI * 2.0).sin())
            .collect();

        let model = train_gp(x.clone(), y.clone(), 1000, 42).expect("train_gp should succeed");

        let mse: f64 = x
            .iter()
            .zip(y.iter())
            .map(|(xi, &yi)| {
                let pred = predict_mean(&model, xi);
                (pred - yi).powi(2)
            })
            .sum::<f64>()
            / n as f64;
        let rmse = mse.sqrt();

        assert!(
            rmse < 0.5,
            "GP RMSE on training data should be < 0.5, got {}",
            rmse
        );
    }
}
