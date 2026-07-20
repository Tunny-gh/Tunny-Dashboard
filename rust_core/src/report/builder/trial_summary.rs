//! Trial summary construction and trial-state bookkeeping.

use std::collections::{BTreeMap, HashMap};

use crate::data::dataframe::DataFrame;
use crate::data::extras::StudyExtras;
use crate::io::journal::parser::StudyMeta;
use crate::report::format_number;

use crate::report::model::*;

/// Returns the state breakdown (all states), FAIL count, and measured
/// wall-clock time (seconds).
pub(super) fn state_summary(
    extras: Option<&StudyExtras>,
    complete_n: usize,
) -> (BTreeMap<String, usize>, usize, Option<f64>) {
    let mut counts = BTreeMap::new();
    let Some(ex) = extras else {
        counts.insert("COMPLETE".to_string(), complete_n);
        return (counts, 0, None);
    };
    for t in &ex.trials {
        *counts.entry(t.state.label().to_string()).or_insert(0) += 1;
    }
    let fail = counts.get("FAIL").copied().unwrap_or(0);

    let mut min_start = f64::INFINITY;
    let mut max_complete = f64::NEG_INFINITY;
    for t in &ex.trials {
        if let Some(s) = t.datetime_start {
            if s.is_finite() {
                min_start = min_start.min(s);
            }
        }
        if let Some(c) = t.datetime_complete {
            if c.is_finite() {
                max_complete = max_complete.max(c);
            }
        }
    }
    let wall = if min_start.is_finite() && max_complete.is_finite() && max_complete >= min_start {
        Some(max_complete - min_start)
    } else {
        None
    };
    (counts, fail, wall)
}

/// Builds a [`TrialSummary`] for a single row.
pub(super) fn build_trial_summary(df: &DataFrame, meta: &StudyMeta, row: usize) -> TrialSummary {
    let trial_number = df.get_trial_number(row).unwrap_or(row as u32);
    let objectives: Vec<f64> = meta
        .objective_names
        .iter()
        .map(|name| {
            df.get_numeric_column(name)
                .and_then(|c| c.get(row))
                .copied()
                .unwrap_or(f64::NAN)
        })
        .collect();

    let params: Vec<(String, ParamValue)> = meta
        .param_names
        .iter()
        .map(|name| {
            if let Some(col) = df.get_numeric_column(name) {
                (
                    name.clone(),
                    ParamValue::Num(col.get(row).copied().unwrap_or(f64::NAN)),
                )
            } else if let Some(col) = df.get_string_column(name) {
                (
                    name.clone(),
                    ParamValue::Cat(col.get(row).cloned().unwrap_or_default()),
                )
            } else {
                (name.clone(), ParamValue::Cat(String::new()))
            }
        })
        .collect();

    // Row-wise max of the raw constraint values. We don't use the sum
    // (constraint_sum) because negative margins can cancel out positive
    // violations and make an infeasible row look feasible (max ≤ 0 ⟺ all
    // constraints satisfied).
    let max_constraint = if meta.has_constraints {
        df.constraint_col_names()
            .iter()
            .filter_map(|name| {
                df.get_numeric_column(name)
                    .and_then(|c| c.get(row))
                    .copied()
            })
            .reduce(f64::max)
    } else {
        None
    };

    let mut user_attrs: Vec<(String, String)> = Vec::new();
    for name in df.user_attr_string_col_names() {
        if let Some(col) = df.get_string_column(name) {
            user_attrs.push((name.clone(), col.get(row).cloned().unwrap_or_default()));
        }
    }
    for name in df.user_attr_numeric_col_names() {
        if let Some(col) = df.get_numeric_column(name) {
            let v = col.get(row).copied().unwrap_or(f64::NAN);
            user_attrs.push((name.clone(), format_number(v)));
        }
    }
    user_attrs.sort_by(|a, b| a.0.cmp(&b.0));

    TrialSummary {
        trial_number,
        objectives,
        params,
        max_constraint,
        user_attrs,
        duplicate_of: None,
    }
}

/// Marks trials with an identical objective-value vector by their first
/// occurrence (smallest trial number).
///
/// On the Pareto front, trials with exactly matching objective values can
/// occur, e.g. from resampling the same parameters. The smallest trial
/// number in each group is treated as canonical, and the others get a
/// positive trial number set in `duplicate_of`. Comparison is done
/// deterministically by bit-pattern equality (NaN objective values are
/// also treated as equal to each other; -0.0 and 0.0 are distinguished).
pub(super) fn mark_duplicate_objectives(table: &mut [TrialSummary]) {
    let mut first_of: HashMap<Vec<u64>, u32> = HashMap::new();
    for t in table.iter() {
        let key: Vec<u64> = t.objectives.iter().map(|v| v.to_bits()).collect();
        first_of
            .entry(key)
            .and_modify(|n| *n = (*n).min(t.trial_number))
            .or_insert(t.trial_number);
    }
    for t in table.iter_mut() {
        let key: Vec<u64> = t.objectives.iter().map(|v| v.to_bits()).collect();
        let first = first_of[&key];
        if first != t.trial_number {
            t.duplicate_of = Some(first);
        }
    }
}
