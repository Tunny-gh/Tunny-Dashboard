use super::kernel_ops::{build_kernel_matrix, matern52_ard, matern52_ard_grad_ld};
use super::solvers::{backward_sub, cholesky, compute_alpha, forward_sub};

/// Log marginal likelihood:
///   L = −½ y^T α − Σ_i log(L_ii) − n/2 log(2π)
pub(super) fn log_marginal_likelihood(
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

/// Compute log marginal likelihood and its gradient in a single pass.
pub(super) fn log_ml_with_gradient(x: &[Vec<f64>], y: &[f64], params: &[f64]) -> (f64, Vec<f64>) {
    if x.is_empty() {
        return (f64::NEG_INFINITY, vec![0.0; params.len()]);
    }
    let ndim = x[0].len();
    let log_ls = &params[..ndim];
    let log_sf = params[ndim];
    let log_sn = params[ndim + 1];
    let n = y.len();

    let k = build_kernel_matrix(x, log_ls, log_sf, log_sn);
    let l = match cholesky(&k) {
        Some(l) => l,
        None => return (f64::NEG_INFINITY, vec![0.0; params.len()]),
    };

    let alpha = compute_alpha(&l, y);
    let data_fit: f64 = y.iter().zip(alpha.iter()).map(|(yi, ai)| yi * ai).sum();
    let log_det: f64 = l.iter().enumerate().map(|(i, row)| row[i].ln()).sum();
    let lml = -0.5 * data_fit - log_det - 0.5 * n as f64 * (2.0 * std::f64::consts::PI).ln();

    let k_inv: Vec<Vec<f64>> = (0..n)
        .map(|j| {
            let e_j: Vec<f64> = (0..n).map(|i| if i == j { 1.0 } else { 0.0 }).collect();
            let v = forward_sub(&l, &e_j);
            backward_sub(&l, &v)
        })
        .collect();

    let mut grad = vec![0.0; params.len()];

    for (d, grad_slot) in grad.iter_mut().enumerate().take(ndim) {
        let mut trace = 0.0;
        for i in 0..n {
            for j in 0..n {
                let w_ij = alpha[i] * alpha[j] - k_inv[j][i];
                let dk_ij = matern52_ard_grad_ld(&x[i], &x[j], log_ls, log_sf, d);
                trace += w_ij * dk_ij;
            }
        }
        *grad_slot = 0.5 * trace;
    }

    {
        let mut trace = 0.0;
        for i in 0..n {
            for j in 0..n {
                let w_ij = alpha[i] * alpha[j] - k_inv[j][i];
                let dk_ij = 2.0 * matern52_ard(&x[i], &x[j], log_ls, log_sf);
                trace += w_ij * dk_ij;
            }
        }
        grad[ndim] = 0.5 * trace;
    }

    {
        let sigma_n2 = (2.0 * log_sn).exp();
        let tr_w: f64 = (0..n).map(|i| alpha[i].powi(2) - k_inv[i][i]).sum();
        grad[ndim + 1] = sigma_n2 * tr_w;
    }

    (lml, grad)
}
