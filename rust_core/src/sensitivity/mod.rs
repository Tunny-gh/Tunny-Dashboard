mod analysis;
mod data;
mod mdi;
mod rf_anova;
mod ridge;
mod sobol;
mod spearman;
mod types;

pub use analysis::{
    compute_sensitivity, compute_sensitivity_all, compute_sensitivity_selected,
    compute_sensitivity_single_obj, compute_sensitivity_without_mdi,
};
pub use mdi::compute_mdi_importances;
pub use rf_anova::compute_rf_anova_importances;
pub use ridge::compute_ridge;
pub use sobol::{compute_sobol, compute_sobol_from_df};
pub use spearman::compute_spearman;
pub use types::{
    MdiResult, RfAnovaResult, RidgeResult, SensitivityMetric, SensitivityResult, SobolResult,
};

#[cfg(test)]
mod tests;
