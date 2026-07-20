//! Key Finding fact extraction.

use crate::convergence::build_best_trial_history;
use crate::data::dataframe::DataFrame;
use crate::data::extras::{StudyExtras, TrialState};
use crate::io::journal::parser::StudyMeta;
use crate::statistics::quantile;

use super::convergence::last_improve_index;
use super::correlations::spearman_pairwise;
use super::findings;

pub(super) fn best_single_fact(
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
) -> Option<(f64, u32, f64)> {
    let minimize = is_minimize.first().copied().unwrap_or(true);
    let mut seq: Vec<usize> = (0..objectives.len())
        .filter(|&r| valid_row[r] && !objectives[r].is_empty())
        .collect();
    if seq.is_empty() {
        return None;
    }
    seq.sort_by_key(|&r| trial_numbers[r]);
    let vals: Vec<f64> = seq.iter().map(|&r| objectives[r][0]).collect();
    let ids: Vec<u32> = seq.iter().map(|&r| trial_numbers[r]).collect();
    let history = build_best_trial_history(&ids, &vals, minimize);
    let best_series: Vec<f64> = history.iter().map(|&(_, v)| v).collect();
    let last_idx = last_improve_index(&best_series, minimize);
    let best = best_series[last_idx];
    let found_pct = (last_idx + 1) as f64 / history.len() as f64 * 100.0;
    Some((best, history[last_idx].0, found_pct))
}

pub(super) fn trade_off_fact(
    objectives: &[Vec<f64>],
    meta: &StudyMeta,
    m: usize,
) -> Option<(String, String, f64)> {
    let mut worst: Option<(String, String, f64)> = None;
    let cols: Vec<Vec<f64>> = (0..m)
        .map(|j| objectives.iter().map(|o| o[j]).collect())
        .collect();
    for a in 0..m {
        for b in (a + 1)..m {
            let rho = spearman_pairwise(&cols[a], &cols[b]);
            if rho.is_finite() && rho < worst.as_ref().map(|w| w.2).unwrap_or(f64::INFINITY) {
                worst = Some((
                    meta.objective_names[a].clone(),
                    meta.objective_names[b].clone(),
                    rho,
                ));
            }
        }
    }
    worst.filter(|w| w.2 < findings::TRADEOFF_RHO_THRESHOLD)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn feasibility_fact(
    df: &DataFrame,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
    is_multi: bool,
    n: usize,
) -> Option<(f64, usize, usize, Option<u32>)> {
    let feas = df.feasibility();
    let feasible: Vec<usize> = (0..n).filter(|&r| feas.is_feasible(r)).collect();
    let feasible_count = feasible.len();
    let rate = if n > 0 {
        feasible_count as f64 / n as f64
    } else {
        0.0
    };
    // Only for single-objective: find the best feasible trial.
    let best_trial = if !is_multi {
        let minimize = is_minimize.first().copied().unwrap_or(true);
        let mut best: Option<(f64, u32)> = None;
        for &r in &feasible {
            if !valid_row[r] {
                continue;
            }
            let v = objectives[r][0];
            let better = match best {
                None => true,
                Some((bv, _)) => {
                    if minimize {
                        v < bv
                    } else {
                        v > bv
                    }
                }
            };
            if better {
                best = Some((v, trial_numbers[r]));
            }
        }
        best.map(|(_, t)| t)
    } else {
        None
    };
    Some((rate, feasible_count, n, best_trial))
}

pub(super) fn pruning_fact(ex: &StudyExtras) -> (f64, usize, Option<f64>) {
    let mut complete = 0usize;
    let mut pruned = 0usize;
    let mut fail = 0usize;
    for t in &ex.trials {
        match t.state {
            TrialState::Complete => complete += 1,
            TrialState::Pruned => pruned += 1,
            TrialState::Fail => fail += 1,
            _ => {}
        }
    }
    let finished = complete + pruned + fail;
    let rate = if finished > 0 {
        pruned as f64 / finished as f64
    } else {
        0.0
    };
    let mut steps: Vec<f64> = ex
        .trials
        .iter()
        .filter(|t| t.state == TrialState::Pruned)
        .filter_map(|t| t.intermediate_values.iter().map(|&(s, _)| s).max())
        .map(|s| s as f64)
        .collect();
    let median = if steps.is_empty() {
        None
    } else {
        steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Some(quantile(&steps, 0.5))
    };
    (rate, pruned, median)
}
