//! Parameter-importance section construction.

use crate::data::dataframe::DataFrame;
use crate::io::journal::parser::StudyMeta;

use super::correlations::spearman_pairwise;
use crate::report::model::*;

/// Uses |Spearman| against objective 0 as the importance score, for
/// numeric parameters only.
pub(super) fn build_importance(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    n: usize,
) -> Option<ImportanceSection> {
    if n < 2 || meta.objective_names.is_empty() {
        return None;
    }
    let y: Vec<f64> = objectives.iter().map(|o| o[0]).collect();
    let mut scores: Vec<(String, f64)> = Vec::new();
    for name in &meta.param_names {
        if let Some(col) = df.get_numeric_column(name) {
            let s = spearman_pairwise(col, &y).abs();
            if s.is_finite() {
                scores.push((name.clone(), s));
            }
        }
    }
    if scores.is_empty() {
        return None;
    }
    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    Some(ImportanceSection {
        method: "spearman_abs".to_string(),
        objective_name: meta.objective_names[0].clone(),
        scores,
    })
}
