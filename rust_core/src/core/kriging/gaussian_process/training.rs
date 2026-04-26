use super::kernel_ops::build_kernel_matrix;
use super::model::GpModel;
use super::optimization::optimize_hyperparams;
use super::solvers::{cholesky, compute_alpha};

/// Train a GP model (with optional subsampling for large datasets).
pub(super) fn train_gp(
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
    subsample_n: usize,
    seed: u64,
) -> Option<GpModel> {
    let (x_sub, y_sub) = if x.len() > subsample_n {
        let mut rng = crate::core::random_forest::Lcg::new(seed);
        let n = x.len();
        let mut indices: Vec<usize> = (0..n).collect();
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
        l,
        log_sn,
    })
}
