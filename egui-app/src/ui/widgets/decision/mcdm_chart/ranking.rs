use crate::state::results::McdmResult;
use crate::state::types::StudyView;

/// Common extracted data for the top-N ranking entries.
pub(super) struct RankingEntry {
    pub(super) rank: usize,
    pub(super) trial_idx: usize,
    pub(super) score: f64,
}

/// Generates the top-N ranking entries from a McdmResult.
pub(super) fn enumerate_ranked(result: &McdmResult, top_n: usize) -> Vec<RankingEntry> {
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let count = top_n.min(ranked.len());

    (0..count)
        .map(|rank| {
            let trial_idx = ranked[rank] as usize;
            let score = scores.get(trial_idx).copied().unwrap_or(0.0);
            RankingEntry {
                rank: rank + 1,
                trial_idx,
                score,
            }
        })
        .collect()
}

/// Table row data.
pub struct RankingRow {
    pub rank: usize,
    /// Global trial_id used for pinning/highlighting.
    pub trial_id: u32,
    /// Optuna trial.number for display (0-based creation order within the Study).
    pub trial_number: u32,
    pub score: f64,
    pub parameters: Vec<f64>,
    pub objectives: Vec<f64>,
}

/// Generates the top-N table row data from a McdmResult.
pub fn build_ranking_rows(
    result: &McdmResult,
    view: &StudyView,
    param_names: &[String],
    obj_names: &[String],
    top_n: usize,
) -> Vec<RankingRow> {
    let param_cols = view.numeric_columns(param_names);
    let obj_cols = view.numeric_columns(obj_names);
    enumerate_ranked(result, top_n)
        .into_iter()
        .map(|e| {
            let parameters: Vec<f64> = param_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(e.trial_idx)).copied().unwrap_or(0.0))
                .collect();
            let objectives: Vec<f64> = obj_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(e.trial_idx)).copied().unwrap_or(0.0))
                .collect();
            RankingRow {
                rank: e.rank,
                trial_id: view
                    .trial_ids
                    .get(e.trial_idx)
                    .copied()
                    .unwrap_or(e.trial_idx as u32),
                // Display the Optuna trial.number rather than the row index
                // (they diverge in a Study that includes pruned/failed trials).
                trial_number: view
                    .df
                    .get_trial_number(e.trial_idx)
                    .unwrap_or(e.trial_idx as u32),
                score: e.score,
                parameters,
                objectives,
            }
        })
        .collect()
}
