use super::constants::{
    MDI_MAX_ROWS, MDI_SEED, PFI_MAX_ROWS, PFI_SEED_BASE, PFI_SPLIT_SEED, RF_ANOVA_MAX_ROWS,
    RF_ANOVA_SEED, SHAP_MAX_ROWS, SHAP_SEED,
};
use super::data::get_param_numeric_values;
use super::metric_trait::SensitivityMetric;
use super::tree::common::{prepare_training_data, PreparedData};
use super::types::{
    MdiResult, PermutationResult, RfAnovaResult, SensitivityResult, ShapResult,
    TreeImportanceResult,
};
use crate::dataframe::DataFrame;

/// Common trait for tree-based sensitivity analysis metrics.
///
/// Receives `PreparedData` preprocessed by `prepare_training_data` and returns
/// (feature_importances, r_squared).
/// The importances must be normalized to sum to 1.0 (or be all 0.0).
/// Returns `None` when data is insufficient or LightGBM training fails.
pub(crate) trait TreeMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)>;
    /// Upper bound on rows after downsampling
    fn max_rows(&self) -> usize;
    /// Seed for data sampling
    fn data_seed(&self) -> u64;
    /// Seed for the holdout split shuffle
    fn split_seed(&self) -> u64;
    fn metric_name(&self) -> &'static str;
    fn wrap_result(
        &self,
        param_names: Vec<String>,
        objective_name: String,
        result: TreeImportanceResult,
    ) -> SensitivityResult;
}

pub struct RfAnovaMetric;
pub struct MdiMetric;
pub struct ShapMetric;
pub struct PermutationMetric;

impl TreeMetric for RfAnovaMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::tree::rf_anova::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        RF_ANOVA_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        RF_ANOVA_SEED
    }
    fn split_seed(&self) -> u64 {
        RF_ANOVA_SEED.wrapping_add(1)
    }
    fn metric_name(&self) -> &'static str {
        "RfAnova"
    }
    fn wrap_result(
        &self,
        param_names: Vec<String>,
        objective_name: String,
        result: TreeImportanceResult,
    ) -> SensitivityResult {
        SensitivityResult {
            param_names,
            objective_names: vec![objective_name],
            rf_anova: Some(RfAnovaResult(result)),
            ..Default::default()
        }
    }
}

impl TreeMetric for MdiMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::tree::mdi::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        MDI_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        MDI_SEED
    }
    fn split_seed(&self) -> u64 {
        MDI_SEED.wrapping_add(1)
    }
    fn metric_name(&self) -> &'static str {
        "Mdi"
    }
    fn wrap_result(
        &self,
        param_names: Vec<String>,
        objective_name: String,
        result: TreeImportanceResult,
    ) -> SensitivityResult {
        SensitivityResult {
            param_names,
            objective_names: vec![objective_name],
            mdi: Some(MdiResult(result)),
            ..Default::default()
        }
    }
}

impl TreeMetric for ShapMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::tree::shap::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        SHAP_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        SHAP_SEED
    }
    fn split_seed(&self) -> u64 {
        SHAP_SEED.wrapping_add(1)
    }
    fn metric_name(&self) -> &'static str {
        "Shap"
    }
    fn wrap_result(
        &self,
        param_names: Vec<String>,
        objective_name: String,
        result: TreeImportanceResult,
    ) -> SensitivityResult {
        SensitivityResult {
            param_names,
            objective_names: vec![objective_name],
            shap: Some(ShapResult(result)),
            ..Default::default()
        }
    }
}

impl TreeMetric for PermutationMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::tree::permutation::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        PFI_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        PFI_SEED_BASE
    }
    fn split_seed(&self) -> u64 {
        PFI_SPLIT_SEED
    }
    fn metric_name(&self) -> &'static str {
        "Permutation"
    }
    fn wrap_result(
        &self,
        param_names: Vec<String>,
        objective_name: String,
        result: TreeImportanceResult,
    ) -> SensitivityResult {
        SensitivityResult {
            param_names,
            objective_names: vec![objective_name],
            permutation: Some(PermutationResult(result)),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// SensitivityMetric blanket implementation for all TreeMetric implementors
// ---------------------------------------------------------------------------

/// Return value of `tree_extract_data`: (param names, objective name, input matrix, objective value vector).
type TreeExtractedData = (Vec<String>, String, Vec<Vec<f64>>, Vec<f64>);

fn tree_extract_data(df: &DataFrame, obj_idx: usize) -> Option<TreeExtractedData> {
    let param_names = df.param_col_names().to_vec();
    let objective_names = df.objective_col_names().to_vec();
    let n = df.row_count();

    let objective_name = objective_names.get(obj_idx)?.clone();
    if n < 2 || param_names.is_empty() {
        return None;
    }

    let y: Vec<f64> = df
        .get_numeric_column(&objective_name)
        .map(|col| col.iter().take(n).copied().collect())
        .unwrap_or_else(|| vec![0.0; n]);

    let param_cols: Vec<Vec<f64>> = param_names
        .iter()
        .map(|name| get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]))
        .collect();
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|row| {
            param_cols
                .iter()
                .map(|col| col.get(row).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    Some((param_names, objective_name, x_matrix, y))
}

fn to_tree_importance_result(imp: Vec<f64>, r2: f64) -> TreeImportanceResult {
    TreeImportanceResult {
        importances: imp.into_iter().map(|v| vec![v]).collect(),
        r_squared: vec![r2],
    }
}

impl<M: TreeMetric + Send + Sync> SensitivityMetric for M {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult> {
        let (param_names, objective_name, x_matrix, y) = tree_extract_data(df, obj_idx)?;
        let data = prepare_training_data(
            &x_matrix,
            &y,
            self.max_rows(),
            self.data_seed(),
            self.split_seed(),
        )?;
        let p = param_names.len();
        let (imp, r2) = self
            .compute_importances(&data)
            .unwrap_or_else(|| (vec![0.0; p], 0.0));
        Some(self.wrap_result(
            param_names,
            objective_name,
            to_tree_importance_result(imp, r2),
        ))
    }

    fn name(&self) -> &'static str {
        self.metric_name()
    }
}
