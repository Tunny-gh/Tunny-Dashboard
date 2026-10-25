//! Unified trait interface for sensitivity analysis metrics.
//!
//! Each metric (Spearman, Ridge, RfAnova, Mdi, Shap, Permutation) implements
//! this trait so that `compute_sensitivity_single_obj` can iterate over a
//! heterogeneous collection of metrics instead of using a large `match` arm.

use super::types::SensitivityResult;
use crate::dataframe::DataFrame;

/// Common interface for all sensitivity analysis metrics.
///
/// Implementors must be `Send + Sync` so that metric computation can be
/// safely dispatched across threads (e.g. via rayon).
pub trait SensitivityMetric: Send + Sync {
    /// Compute sensitivity for a single objective identified by `obj_idx`.
    ///
    /// Returns `None` when the computation cannot be performed (e.g.
    /// insufficient data), never panics.
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult>;

    /// Compute without substituting zero scores for fitting failures. Consumers
    /// that distinguish unavailable results from measured zeros use this path;
    /// heatmaps retain their established fallback through `compute`.
    fn compute_checked(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult> {
        self.compute(df, obj_idx)
    }

    /// Human-readable identifier for the metric (used in logging / debugging).
    fn name(&self) -> &'static str;
}
