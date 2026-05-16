//! Gaussian Process regression (Kriging) with ARD Matérn 5/2 kernel.
//!
//! Pure-Rust implementation — no external crates.
//! Used for 2D PDP surface computation in `pdp/kriging_core.rs`.

mod inference;
mod kernel_ops;
mod likelihood;
mod model;
mod optimization;
mod solvers;
mod training;

pub(crate) use model::GpModel;
#[cfg(test)]
pub(crate) use model::{GpFittedModel, GpKernel};

pub(crate) fn matern52_ard(x1: &[f64], x2: &[f64], log_ls: &[f64], log_sf: f64) -> f64 {
    kernel_ops::matern52_ard(x1, x2, log_ls, log_sf)
}

pub(crate) fn train_gp(
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
    subsample_n: usize,
    seed: u64,
) -> Option<GpModel> {
    training::train_gp(x, y, subsample_n, seed)
}

pub(crate) fn predict_mean(model: &GpModel, x_test: &[f64]) -> f64 {
    inference::predict_mean(model, x_test)
}

pub(crate) fn predict_variance(model: &GpModel, x_test: &[f64]) -> f64 {
    inference::predict_variance(model, x_test)
}

#[cfg(test)]
use kernel_ops::build_kernel_matrix;
#[cfg(test)]
use likelihood::{log_marginal_likelihood, log_ml_with_gradient};
#[cfg(test)]
use optimization::optimize_hyperparams;
#[cfg(test)]
use solvers::{cholesky, compute_alpha};

#[cfg(test)]
mod tests;
