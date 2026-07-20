//! Construction of [`StudyReport`] (composition of existing analysis APIs).
//!
//! No new analysis logic is invented here; the model is filled by composing
//! existing public functions from `multi_objective` / `mcdm` / `statistics` /
//! `math` / `convergence`. All series needed for charts are computed here so
//! the renderer only has to draw them.
//!
//! ## Handling of direction
//!
//! [`OptimizationDirection`] is respected per objective. `is_minimize[j]` is
//! `matches!(directions[j], Minimize)`, matching the calling convention of
//! the existing MCDM / pareto / convergence functions.

use crate::data::dataframe::DataFrame;
use crate::data::extras::StudyExtras;
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};
use crate::multi_objective::pareto::nd_sort;

use convergence::{build_convergence_multi, build_convergence_single};
use correlations::build_correlations;
use execution::build_execution;
use facts::{best_single_fact, feasibility_fact, pruning_fact, trade_off_fact};
use importance::build_importance;
use outcome::{build_outcome_multi, build_outcome_single};
use stats::build_objective_stats;
use trial_summary::state_summary;

use super::findings::{self, FindingInputs};
use super::model::*;
use super::{ReportOptions, ReportSource};

mod convergence;
mod correlations;
mod execution;
mod facts;
mod importance;
mod mcdm;
mod outcome;
mod pareto;
mod stats;
mod trial_summary;

/// VIKOR compromise-solution parameter (standard value), used for the
/// last-20% convergence check.
const VIKOR_V: f64 = 0.5;
/// Maximum number of points in the convergence series (downsampling cap).
const MAX_SERIES_POINTS: usize = 500;
/// Maximum number of histogram bins.
const MAX_HIST_BINS: usize = 20;

/// Builds a [`StudyReport`] from `(StudyMeta, DataFrame, StudyExtras)`.
///
/// `df` is assumed to hold only COMPLETE trials, column-oriented. `extras`
/// carries auxiliary data for all states (if absent, the execution section
/// and pruning finding are omitted). For determinism, every collection that
/// appears in the output is sorted / backed by a BTree.
pub fn build_study_report(
    meta: &StudyMeta,
    df: &DataFrame,
    extras: Option<&StudyExtras>,
    source: &ReportSource,
    opts: &ReportOptions,
) -> StudyReport {
    let directions: Vec<Direction> = meta
        .directions
        .iter()
        .map(|d| match d {
            OptimizationDirection::Minimize => Direction::Minimize,
            OptimizationDirection::Maximize => Direction::Maximize,
        })
        .collect();
    let is_minimize: Vec<bool> = directions.iter().map(|d| d.is_minimize()).collect();
    let m = meta.objective_names.len();
    let is_multi = m >= 2;
    let n = df.row_count();

    // Objective columns and objective value matrix (NaN allowed).
    let obj_cols: Vec<Option<&[f64]>> = meta
        .objective_names
        .iter()
        .map(|name| df.get_numeric_column(name))
        .collect();
    let objectives: Vec<Vec<f64>> = (0..n)
        .map(|row| {
            obj_cols
                .iter()
                .map(|c| c.and_then(|c| c.get(row)).copied().unwrap_or(f64::NAN))
                .collect()
        })
        .collect();
    let trial_numbers: Vec<u32> = (0..n)
        .map(|row| df.get_trial_number(row).unwrap_or(row as u32))
        .collect();

    // Valid rows (all objectives finite).
    let valid_row: Vec<bool> = objectives
        .iter()
        .map(|o| o.len() == m && o.iter().all(|v| v.is_finite()))
        .collect();
    let valid_count = valid_row.iter().filter(|&&v| v).count();
    let nan_count = n - valid_count;

    // State breakdown, FAIL count, and measured wall-clock time.
    let (state_counts, fail_count, wall_clock) = state_summary(extras, n);

    // Pareto front (only meaningful for multi-objective). For constrained
    // studies, non-dominated sorting uses only feasible rows (matching
    // Optuna's constrained-optimization semantics). Only when there are no
    // feasible rows at all do we fall back to the non-dominated set over
    // all rows (this avoids an empty front, and the renderer's violation
    // note makes the fallback transparent).
    let feas = df.feasibility();
    let front_rows: Vec<usize> = if is_multi && n > 0 {
        let feasible_rows: Vec<usize> = (0..n)
            .filter(|&r| valid_row[r] && feas.is_feasible(r))
            .collect();
        if feas.has_constraints() && !feasible_rows.is_empty() {
            let sub: Vec<Vec<f64>> = feasible_rows
                .iter()
                .map(|&r| objectives[r].clone())
                .collect();
            let sub_ranks = nd_sort(&sub, &is_minimize);
            feasible_rows
                .iter()
                .zip(&sub_ranks)
                .filter(|&(_, &rank)| rank == 0)
                .map(|(&r, _)| r)
                .collect()
        } else {
            let ranks = nd_sort(&objectives, &is_minimize);
            (0..n).filter(|&r| ranks[r] == 0 && valid_row[r]).collect()
        }
    } else {
        Vec::new()
    };
    let mut on_front = vec![false; n];
    for &r in &front_rows {
        on_front[r] = true;
    }

    // ---- Overview ----
    let mut param_bounds: Vec<(String, f64, f64)> = meta
        .param_bounds
        .iter()
        .map(|(k, (lo, hi))| (k.clone(), *lo, *hi))
        .collect();
    param_bounds.sort_by(|a, b| a.0.cmp(&b.0));

    let overview = Overview {
        name: meta.name.clone(),
        directions: directions.clone(),
        objective_names: meta.objective_names.clone(),
        param_names: meta.param_names.clone(),
        user_attr_names: meta.user_attr_names.clone(),
        state_counts: state_counts.clone(),
        complete_trials: n,
        total_trials: meta.total_trials as usize,
        wall_clock_seconds: wall_clock,
        param_bounds,
        has_constraints: meta.has_constraints,
    };

    // ---- Convergence ----
    let convergence = if is_multi {
        build_convergence_multi(&objectives, &trial_numbers, &is_minimize, valid_count)
    } else {
        build_convergence_single(&objectives, &trial_numbers, &valid_row, &is_minimize, m)
    };

    // ---- Outcome ----
    let (outcome, mcdm) = if is_multi {
        build_outcome_multi(
            df,
            meta,
            &objectives,
            &trial_numbers,
            &valid_row,
            &on_front,
            &front_rows,
            &directions,
            &is_minimize,
            valid_count,
            m,
            opts,
        )
    } else {
        (
            build_outcome_single(
                df,
                meta,
                &objectives,
                &trial_numbers,
                &valid_row,
                &is_minimize,
                opts,
            ),
            None,
        )
    };

    // ---- Importance ----
    let importance = build_importance(df, meta, &objectives, n);

    // ---- Objective stats ----
    let objective_stats = build_objective_stats(&objectives, meta, &directions, m);

    // ---- Correlations ----
    // If `skip_decision_sections`, skip the computationally expensive
    // correlation calculation.
    let correlations = if opts.skip_decision_sections {
        None
    } else {
        build_correlations(df, meta, &objectives, n, opts)
    };

    // ---- Execution ----
    let execution = extras.map(|ex| build_execution(ex, &state_counts, wall_clock));

    // ---- Findings ----
    let best_single = if !is_multi {
        best_single_fact(&objectives, &trial_numbers, &valid_row, &is_minimize)
    } else {
        None
    };
    let trade_off = if is_multi {
        trade_off_fact(&objectives, meta, m)
    } else {
        None
    };
    let feasibility = if meta.has_constraints {
        feasibility_fact(
            df,
            &objectives,
            &trial_numbers,
            &valid_row,
            &is_minimize,
            is_multi,
            n,
        )
    } else {
        None
    };
    let pruning = extras.map(pruning_fact);
    let top_importance: Vec<(String, f64)> = importance
        .as_ref()
        .map(|s| s.scores.iter().take(3).cloned().collect())
        .unwrap_or_default();
    let importance_method = importance.as_ref().map(|s| s.method.clone());

    let finding_inputs = FindingInputs {
        is_multi,
        best_single,
        pareto: if is_multi {
            Some((front_rows.len(), valid_count))
        } else {
            None
        },
        convergence_status: convergence.status,
        top_importance,
        importance_method,
        trade_off,
        feasibility,
        pruning,
        data_quality: Some((nan_count, fail_count)),
    };
    let key_findings = findings::generate_findings(&finding_inputs);

    StudyReport {
        schema_version: SCHEMA_VERSION,
        source: ReportSourceInfo {
            storage_display: source.storage_display.clone(),
            generated_at_unix: source.generated_at_unix,
        },
        overview,
        key_findings,
        outcome,
        convergence,
        importance,
        objective_stats,
        correlations,
        mcdm,
        execution,
        reproduction: Reproduction {
            study_id: meta.study_id,
            storage_display: source.storage_display.clone(),
            top_n: opts.top_n,
            max_heatmap_params: opts.max_heatmap_params,
            schema_version: SCHEMA_VERSION,
        },
    }
}

// =============================================================================
// Common helpers
// =============================================================================

/// Evenly downsamples a series to at most `max` points (preserving first
/// and last).
///
/// Also shared with the convergence-series sampling in the `markdown`
/// renderer (`pub(crate)`).
pub(crate) fn downsample<T: Clone>(pts: &[T], max: usize) -> Vec<T> {
    if pts.len() <= max || max < 2 {
        return pts.to_vec();
    }
    let last = pts.len() - 1;
    (0..max)
        .map(|k| pts[k * last / (max - 1)].clone())
        .collect()
}

#[cfg(test)]
mod tests;
