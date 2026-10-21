//! Convergence-section construction (single- and multi-objective).

use crate::convergence::build_best_trial_history;
use crate::multi_objective::pareto::compute_hv_history_from_data;

use super::{downsample, findings, MAX_SERIES_POINTS};
use crate::report::model::*;

/// Returns the index of the last improvement (update) in a monotonic
/// best-so-far series. The first point (index 0) is always counted as an
/// improvement, since it's the first observation.
pub(super) fn last_improve_index(best_series: &[f64], minimize: bool) -> usize {
    let mut last = 0;
    for i in 1..best_series.len() {
        let improved = if minimize {
            best_series[i] < best_series[i - 1]
        } else {
            best_series[i] > best_series[i - 1]
        };
        if improved {
            last = i;
        }
    }
    last
}

pub(super) fn build_convergence_single(
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
    m: usize,
) -> ConvergenceSection {
    let minimize = is_minimize.first().copied().unwrap_or(true);
    // Sort valid rows by ascending trial.number.
    let mut seq: Vec<usize> = (0..objectives.len()).filter(|&r| valid_row[r]).collect();
    seq.sort_by_key(|&r| trial_numbers[r]);

    if m == 0 || seq.is_empty() {
        return ConvergenceSection {
            metric: ConvergenceMetric::BestSoFar,
            series: Vec::new(),
            found_at_trial_number: None,
            improved_in_last_20pct: false,
            status: findings::convergence_status(seq.len(), 0.0),
        };
    }

    let ids: Vec<u32> = seq.iter().map(|&r| trial_numbers[r]).collect();
    let vals: Vec<f64> = seq.iter().map(|&r| objectives[r][0]).collect();
    let history = build_best_trial_history(&ids, &vals, minimize);
    let best_series: Vec<f64> = history.iter().map(|&(_, v)| v).collect();

    let last_idx = last_improve_index(&best_series, minimize);
    let len = history.len();
    let frac = if len <= 1 {
        0.0
    } else {
        last_idx as f64 / (len - 1) as f64
    };

    let series: Vec<ConvergencePoint> = downsample(&history, MAX_SERIES_POINTS)
        .into_iter()
        .map(|(tn, v)| ConvergencePoint {
            trial_number: tn,
            value: v,
        })
        .collect();

    ConvergenceSection {
        metric: ConvergenceMetric::BestSoFar,
        series,
        found_at_trial_number: Some(history[last_idx].0),
        improved_in_last_20pct: frac >= findings::STILL_IMPROVING_FRACTION,
        status: findings::convergence_status(len, frac),
    }
}

pub(super) fn build_convergence_multi(
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    is_minimize: &[bool],
    valid_count: usize,
) -> ConvergenceSection {
    let n = objectives.len();
    // Sort by ascending trial.number and compute the HV trajectory.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&r| trial_numbers[r]);
    let ord_ids: Vec<u32> = order.iter().map(|&r| trial_numbers[r]).collect();
    // `compute_hv_history_from_data` (multi_objective::pareto, outside
    // report/) requires `&[Vec<f64>]`, so an owned copy of the rows is
    // unavoidable when reordering by trial.number (the borrowed
    // `objectives` must keep its original row order). This copy is m
    // columns x n rows, the same order of magnitude as the original data,
    // and cannot be reduced without changing the signature.
    let ord_objs: Vec<Vec<f64>> = order.iter().map(|&r| objectives[r].clone()).collect();

    let hv = compute_hv_history_from_data(&ord_ids, &ord_objs, is_minimize);

    if hv.hv_values.is_empty() {
        return ConvergenceSection {
            metric: ConvergenceMetric::Hypervolume,
            series: Vec::new(),
            found_at_trial_number: None,
            improved_in_last_20pct: false,
            status: findings::convergence_status(valid_count, 0.0),
        };
    }

    // HV is monotonically non-decreasing, so "improvement = greater than
    // the previous value" determines the last update position.
    let last_idx = last_improve_index(&hv.hv_values, false);
    let len = hv.hv_values.len();
    let frac = if len <= 1 {
        0.0
    } else {
        last_idx as f64 / (len - 1) as f64
    };

    let points: Vec<(u32, f64)> = ord_ids
        .iter()
        .copied()
        .zip(hv.hv_values.iter().copied())
        .collect();
    let series: Vec<ConvergencePoint> = downsample(&points, MAX_SERIES_POINTS)
        .into_iter()
        .map(|(tn, v)| ConvergencePoint {
            trial_number: tn,
            value: v,
        })
        .collect();

    ConvergenceSection {
        metric: ConvergenceMetric::Hypervolume,
        series,
        found_at_trial_number: Some(ord_ids[last_idx]),
        improved_in_last_20pct: frac >= findings::STILL_IMPROVING_FRACTION,
        status: findings::convergence_status(valid_count, frac),
    }
}
