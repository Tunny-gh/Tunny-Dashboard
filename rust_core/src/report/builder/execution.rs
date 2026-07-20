//! Execution-section construction (state counts, pruning, timing).

use std::collections::BTreeMap;

use crate::data::extras::{StudyExtras, TrialState};
use crate::statistics::quantile;

use crate::report::model::*;

pub(super) fn build_execution(
    ex: &StudyExtras,
    state_counts: &BTreeMap<String, usize>,
    wall_clock: Option<f64>,
) -> ExecutionSection {
    let complete = state_counts.get("COMPLETE").copied().unwrap_or(0);
    let pruned = state_counts.get("PRUNED").copied().unwrap_or(0);
    let fail = state_counts.get("FAIL").copied().unwrap_or(0);
    let finished = complete + pruned + fail;
    let pruned_rate = if finished > 0 {
        pruned as f64 / finished as f64
    } else {
        0.0
    };

    // Median of the final intermediate-value step among PRUNED trials.
    let mut prune_steps: Vec<f64> = ex
        .trials
        .iter()
        .filter(|t| t.state == TrialState::Pruned)
        .filter_map(|t| t.intermediate_values.iter().map(|&(s, _)| s).max())
        .map(|s| s as f64)
        .collect();
    let median_prune_step = if prune_steps.is_empty() {
        None
    } else {
        prune_steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Some(quantile(&prune_steps, 0.5))
    };

    // Trial durations.
    let durations: Vec<f64> = ex
        .trials
        .iter()
        .filter_map(|t| match (t.datetime_start, t.datetime_complete) {
            (Some(s), Some(c)) if s.is_finite() && c.is_finite() && c >= s => Some(c - s),
            _ => None,
        })
        .collect();
    let (mean_trial_seconds, std_trial_seconds) = if durations.is_empty() {
        (None, None)
    } else {
        let mean = durations.iter().sum::<f64>() / durations.len() as f64;
        let var =
            durations.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / durations.len() as f64;
        (Some(mean), Some(var.sqrt()))
    };

    ExecutionSection {
        state_counts: state_counts.clone(),
        pruned_rate,
        median_prune_step,
        mean_trial_seconds,
        std_trial_seconds,
        total_seconds: wall_clock,
    }
}
