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
    /// Log noise std-dev
    pub log_sn: f64,
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
fn matern52_ard_grad_ld(
    x1: &[f64],
    x2: &[f64],
    log_ls: &[f64],
    log_sf: f64,
    dim: usize,
) -> f64 {
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

/// Analytical gradient of the log marginal likelihood:
///   ∂L/∂θⱼ = ½ tr((αα^T − K^{-1}) · ∂K/∂θⱼ)
///
/// `params` layout: [log_ls_0, …, log_ls_{d-1}, log_sf, log_sn]
pub(crate) fn log_ml_gradient(x: &[Vec<f64>], y: &[f64], params: &[f64]) -> Vec<f64> {
    let ndim = if x.is_empty() { return vec![]; } else { x[0].len() };
    let log_ls = &params[..ndim];
    let log_sf = params[ndim];
    let log_sn = params[ndim + 1];

    let k = build_kernel_matrix(x, log_ls, log_sf, log_sn);
    let l = match cholesky(&k) {
        Some(l) => l,
        None => return vec![0.0; params.len()],
    };
    let alpha = compute_alpha(&l, y);
    let n = y.len();

    // Compute K^{-1} column by column: K^{-1} e_j via forward/backward sub
    let k_inv: Vec<Vec<f64>> = (0..n)
        .map(|j| {
            let e_j: Vec<f64> = (0..n).map(|i| if i == j { 1.0 } else { 0.0 }).collect();
            let v = forward_sub(&l, &e_j);
            backward_sub(&l, &v)
        })
        .collect();

    let mut grad = vec![0.0; params.len()];

    // ∂L/∂log(l_d) for each dimension
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

    // ∂L/∂log(σ_n): ∂K/∂log(σ_n) = 2σ_n²·I
    {
        let sigma_n2 = (2.0 * log_sn).exp();
        let tr_w: f64 = (0..n).map(|i| alpha[i].powi(2) - k_inv[i][i]).sum();
        grad[ndim + 1] = sigma_n2 * tr_w;
    }

    grad
}

// =============================================================================
// L-BFGS optimizer
// =============================================================================

/// L-BFGS Two-loop recursion: compute search direction d = −H^{-1} · grad.
///
/// `s_hist[k]` = x_{k+1} − x_k
/// `y_hist[k]` = grad_{k+1} − grad_k
pub(crate) fn lbfgs_direction(
    grad: &[f64],
    s_hist: &[Vec<f64>],
    y_hist: &[Vec<f64>],
) -> Vec<f64> {
    let m = s_hist.len();
    let mut q = grad.to_vec();
    let mut rho = vec![0.0; m];
    let mut alpha = vec![0.0; m];

    // First loop (backward)
    for i in (0..m).rev() {
        let sy: f64 = s_hist[i].iter().zip(y_hist[i].iter()).map(|(s, y)| s * y).sum();
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
        if yy > 1e-15 { sy / yy } else { 1.0 }
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
        let x_new: Vec<f64> = x.iter().zip(d.iter()).map(|(xi, di)| xi + alpha * di).collect();
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
/// Returns the optimised parameter vector.
pub(crate) fn optimize_hyperparams(
    x: &[Vec<f64>],
    y: &[f64],
    n_iter: usize,
    m_history: usize,
) -> Vec<f64> {
    if x.is_empty() {
        return vec![];
    }
    let ndim = x[0].len();
    let mut params = vec![0.0; ndim + 2];
    params[ndim + 1] = -2.0; // initial log_sn: σ_n ≈ 0.135

    let mut s_hist: Vec<Vec<f64>> = Vec::new();
    let mut y_hist: Vec<Vec<f64>> = Vec::new();

    // Closure: −log marginal likelihood (we minimise)
    let neg_lml = |p: &[f64]| {
        -log_marginal_likelihood(x, y, &p[..ndim], p[ndim], p[ndim + 1])
    };

    for _ in 0..n_iter {
        // Gradient of −LML (negate the LML gradient)
        let grad_neg: Vec<f64> = log_ml_gradient(x, y, &params)
            .iter()
            .map(|g| -g)
            .collect();

        let grad_norm: f64 = grad_neg.iter().map(|g| g * g).sum::<f64>().sqrt();
        if grad_norm < 1e-5 {
            break;
        }

        let d = lbfgs_direction(&grad_neg, &s_hist, &y_hist);
        let f_x = neg_lml(&params);
        let alpha = armijo_line_search(f_x, &grad_neg, &d, &neg_lml, &params, 1e-4, 20);

        let x_new: Vec<f64> = params.iter().zip(d.iter()).map(|(p, di)| p + alpha * di).collect();
        let grad_new: Vec<f64> = log_ml_gradient(x, y, &x_new)
            .iter()
            .map(|g| -g)
            .collect();

        let s: Vec<f64> = x_new.iter().zip(params.iter()).map(|(xn, xo)| xn - xo).collect();
        let yv: Vec<f64> = grad_new.iter().zip(grad_neg.iter()).map(|(gn, go)| gn - go).collect();

        params = x_new;

        if s_hist.len() >= m_history {
            s_hist.remove(0);
            y_hist.remove(0);
        }
        s_hist.push(s);
        y_hist.push(yv);
    }
    params
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

    let params = optimize_hyperparams(&x_sub, &y_sub, 100, 5);
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
        log_sn,
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
                    i, j, reconstructed, i, j, expected
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
                i, ax, i, b[i]
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
        assert!((k - 1.0).abs() < 1e-12, "k(x,x) should be σ_f² = 1, got {}", k);
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
                    i, j, k[i][j], j, i, k[j][i]
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
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 * 0.1, (i as f64 * 0.2).sin()]).collect();
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
            let lml_plus = log_marginal_likelihood(
                &x,
                &y,
                &p_plus[..ndim],
                p_plus[ndim],
                p_plus[ndim + 1],
            );
            let lml_minus = log_marginal_likelihood(
                &x,
                &y,
                &p_minus[..ndim],
                p_minus[ndim],
                p_minus[ndim + 1],
            );
            let numerical = (lml_plus - lml_minus) / (2.0 * eps);
            let rel_err = (analytical[d] - numerical).abs() / (numerical.abs() + 1e-8);
            assert!(
                rel_err < 1e-3,
                "Gradient dim {} analytical={} numerical={} rel_err={}",
                d, analytical[d], numerical, rel_err
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
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 / n as f64])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] * 2.0 + (xi[0] * 6.0).sin() * 0.1).collect();
        let ndim = 1;

        let initial_params = vec![0.0_f64, 0.0, -2.0]; // [log_ls, log_sf, log_sn]
        let initial_lml = log_marginal_likelihood(
            &x,
            &y,
            &initial_params[..ndim],
            initial_params[ndim],
            initial_params[ndim + 1],
        );

        let opt_params = optimize_hyperparams(&x, &y, 20, 5);
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
            final_lml, initial_lml
        );
    }

    /// TC8: train_gp subsamples when n > subsample_n.
    #[test]
    fn tc_1635_02_train_gp_subsampling() {
        let n = 50; // use small n for speed
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64, 0.0]).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0]).collect();

        let model = train_gp(x, y, 30, 42).expect("train_gp should succeed");
        assert_eq!(model.x_train.len(), 30, "Model should be trained on 30 subsampled points");
    }

    /// TC9: GP predicts reasonably on training points (low RMSE for smooth data).
    #[test]
    fn tc_1635_03_gp_prediction_quality() {
        let n = 15;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| vec![i as f64 / n as f64])
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| (xi[0] * std::f64::consts::PI * 2.0).sin()).collect();

        let model = train_gp(x.clone(), y.clone(), 1000, 42).expect("train_gp should succeed");

        let mse: f64 = x.iter().zip(y.iter()).map(|(xi, &yi)| {
            let pred = predict_mean(&model, xi);
            (pred - yi).powi(2)
        }).sum::<f64>() / n as f64;
        let rmse = mse.sqrt();

        assert!(rmse < 0.5, "GP RMSE on training data should be < 0.5, got {}", rmse);
    }
}
