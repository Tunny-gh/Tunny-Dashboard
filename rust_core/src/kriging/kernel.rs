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
pub(super) fn matern52_ard_grad_ld(
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
