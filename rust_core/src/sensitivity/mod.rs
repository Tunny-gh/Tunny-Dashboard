mod analysis;
mod data;
mod ridge;
mod sobol;
mod spearman;
mod types;

pub use analysis::{compute_sensitivity, compute_sensitivity_all, compute_sensitivity_selected};
pub use ridge::compute_ridge;
pub use sobol::compute_sobol;
pub use spearman::compute_spearman;
pub use types::{RidgeResult, SensitivityResult, SobolResult};

#[cfg(test)]
mod tests;
