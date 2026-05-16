/// ARD Matérn 5/2 kernel wrapper.
pub(super) fn matern52_ard(x1: &[f64], x2: &[f64], log_ls: &[f64], log_sf: f64) -> f64 {
    super::super::kernel::matern52_ard(x1, x2, log_ls, log_sf)
}

/// ∂k/∂log(l_d) wrapper.
pub(super) fn matern52_ard_grad_ld(
    x1: &[f64],
    x2: &[f64],
    log_ls: &[f64],
    log_sf: f64,
    dim: usize,
) -> f64 {
    super::super::kernel::matern52_ard_grad_ld(x1, x2, log_ls, log_sf, dim)
}

/// Build the N×N kernel matrix with noise.
pub(super) fn build_kernel_matrix(
    x: &[Vec<f64>],
    log_ls: &[f64],
    log_sf: f64,
    log_sn: f64,
) -> Vec<Vec<f64>> {
    super::super::kernel::build_kernel_matrix(x, log_ls, log_sf, log_sn)
}
