mod analysis;
mod constants;
mod data;
mod mdi;
mod metric_trait;
mod metrics;
mod permutation;
mod rf_anova;
mod ridge;
mod shap;
mod sobol;
mod spearman;
mod tree_common;
mod types;

pub use analysis::{
    compute_sensitivity, compute_sensitivity_all, compute_sensitivity_selected,
    compute_sensitivity_single_obj, compute_sensitivity_without_mdi,
};
pub use mdi::compute_mdi_importances;
pub use metric_trait::SensitivityMetric;
pub use metrics::{MdiMetric, PermutationMetric, RfAnovaMetric, ShapMetric};
pub use permutation::compute_permutation_importances;
pub use ridge::RidgeMetric;
pub use spearman::SpearmanMetric;
pub use rf_anova::compute_rf_anova_importances;
pub use ridge::compute_ridge;
pub(crate) use ridge::compute_ridge_from_vecs;
pub use shap::compute_shap_importances;
pub use sobol::{compute_sobol, compute_sobol_from_df};
pub use spearman::compute_spearman;
pub use types::{
    MdiResult, PermutationResult, RfAnovaResult, RidgeResult, SensitivityKind, SensitivityResult,
    ShapResult, SobolResult, TreeImportanceResult,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod metric_trait_tests;

#[cfg(test)]
mod tree_metric_tests;
