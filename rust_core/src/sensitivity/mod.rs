mod analysis;
mod data;
mod rf_anova;
mod ridge;
mod sobol;
mod spearman;
mod types;

pub use analysis::{compute_sensitivity, compute_sensitivity_all, compute_sensitivity_selected};
pub use rf_anova::compute_rf_anova_importances;
pub use ridge::compute_ridge;
pub use sobol::compute_sobol;
pub use spearman::compute_spearman;
pub use types::{RfAnovaResult, RidgeResult, SensitivityResult, SobolResult};

#[cfg(test)]
mod tests;
