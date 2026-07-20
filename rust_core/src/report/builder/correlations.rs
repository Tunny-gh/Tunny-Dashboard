//! Correlation-section construction.

use crate::data::dataframe::DataFrame;
use crate::io::journal::parser::StudyMeta;
use crate::statistics::CorrelationMethod;

use super::ReportOptions;
use crate::report::model::*;

/// Pairwise Spearman correlation between two series (uses only rows finite
/// in both; NaN if fewer than 2 rows).
///
/// Implementation delegates to the shared helper in `statistics::correlation`
/// (avoids duplicate implementations).
pub(super) fn spearman_pairwise(x: &[f64], y: &[f64]) -> f64 {
    crate::statistics::correlation::pairwise_correlation(x, y, CorrelationMethod::Spearman)
}

pub(super) fn build_correlations(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    n: usize,
    opts: &ReportOptions,
) -> Option<CorrelationSection> {
    if n < 2 || meta.objective_names.is_empty() {
        return None;
    }
    // Numeric parameter columns only. Column slices are resolved here
    // exactly once and reused afterward without re-lookup (avoids
    // resolving twice across the filter and the loop).
    let numeric_params: Vec<(String, &[f64])> = meta
        .param_names
        .iter()
        .filter_map(|name| df.get_numeric_column(name).map(|col| (name.clone(), col)))
        .collect();
    if numeric_params.is_empty() {
        return None;
    }

    let obj_cols: Vec<Vec<f64>> = (0..meta.objective_names.len())
        .map(|j| objectives.iter().map(|o| o[j]).collect())
        .collect();

    // Spearman matrix for each parameter x each objective, plus the max |ρ|.
    let mut rows: Vec<(String, Vec<f64>, f64)> = numeric_params
        .iter()
        .map(|(name, x)| {
            let row: Vec<f64> = obj_cols.iter().map(|y| spearman_pairwise(x, y)).collect();
            let max_abs = row
                .iter()
                .filter(|v| v.is_finite())
                .fold(0.0f64, |acc, v| acc.max(v.abs()));
            (name.clone(), row, max_abs)
        })
        .collect();

    // Cap by descending max |ρ| (ties broken by ascending name).
    rows.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    rows.truncate(opts.max_heatmap_params.max(1));

    Some(CorrelationSection {
        method: "spearman".to_string(),
        params: rows.iter().map(|(n, _, _)| n.clone()).collect(),
        objectives: meta.objective_names.clone(),
        matrix: rows.into_iter().map(|(_, r, _)| r).collect(),
    })
}
