use super::kernel_ops::matern52_ard;
use super::model::GpModel;
use super::solvers::forward_sub;

/// Predict the GP posterior mean at `x_test`.
pub(super) fn predict_mean(model: &GpModel, x_test: &[f64]) -> f64 {
    model
        .x_train
        .iter()
        .zip(model.alpha.iter())
        .map(|(x_train, alpha)| alpha * matern52_ard(x_test, x_train, &model.kernel.log_ls, model.kernel.log_sf))
        .sum()
}

/// Predict the GP posterior variance at `x_test`.
///
/// var(x*) = k(x*,x*) - ||L^{-1} k(X,x*)||²
pub(crate) fn predict_variance(model: &GpModel, x_test: &[f64]) -> f64 {
    // k(x*, x*) — prior variance
    let k_star_star = matern52_ard(x_test, x_test, &model.kernel.log_ls, model.kernel.log_sf);

    // k(X, x*) — cross-covariance vector
    let k_star: Vec<f64> = model
        .x_train
        .iter()
        .map(|x_tr| matern52_ard(x_tr, x_test, &model.kernel.log_ls, model.kernel.log_sf))
        .collect();

    // v = L^{-1} k_star
    let v = forward_sub(&model.l, &k_star);

    // posterior variance = k** - v^T v  (clamped to 0)
    let reduction: f64 = v.iter().map(|vi| vi * vi).sum();
    (k_star_star - reduction).max(0.0)
}
