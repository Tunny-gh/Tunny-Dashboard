use crate::dataframe::DataFrame;

use super::super::{metric_trait::SensitivityMetric, SensitivityResult};

/// Computes sensitivity for a single objective using each provided metric.
/// Metrics that return `None` (insufficient data, etc.) are silently excluded.
pub fn compute_sensitivity_single_obj(
    df: &DataFrame,
    metrics: Vec<Box<dyn SensitivityMetric>>,
    obj_idx: usize,
) -> Vec<SensitivityResult> {
    metrics
        .iter()
        .filter_map(|m| m.compute(df, obj_idx))
        .collect()
}
