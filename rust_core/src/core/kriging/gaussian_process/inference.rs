use super::kernel_ops::matern52_ard;
use super::model::GpModel;

/// Predict the GP posterior mean at `x_test`.
pub(super) fn predict_mean(model: &GpModel, x_test: &[f64]) -> f64 {
    model
        .x_train
        .iter()
        .zip(model.alpha.iter())
        .map(|(x_train, alpha)| alpha * matern52_ard(x_test, x_train, &model.log_ls, model.log_sf))
        .sum()
}
