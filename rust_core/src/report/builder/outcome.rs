//! Outcome-section construction (single- and multi-objective).

use crate::data::dataframe::DataFrame;
use crate::io::journal::parser::StudyMeta;
use crate::mcdm::topsis;

use super::mcdm::build_mcdm;
use super::pareto::{build_pareto_table, build_scatter_points, flatten_front};
use super::trial_summary::build_trial_summary;
use super::ReportOptions;
use crate::report::model::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_outcome_single(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
    opts: &ReportOptions,
) -> Outcome {
    if meta.objective_names.is_empty() {
        return Outcome::SingleObj {
            best_trial: None,
            top_n: Vec::new(),
        };
    }
    let minimize = is_minimize.first().copied().unwrap_or(true);
    let mut order: Vec<usize> = (0..objectives.len()).filter(|&r| valid_row[r]).collect();
    order.sort_by(|&a, &b| {
        let (va, vb) = (objectives[a][0], objectives[b][0]);
        let ord = va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal);
        let ord = if minimize { ord } else { ord.reverse() };
        ord.then(trial_numbers[a].cmp(&trial_numbers[b]))
    });

    let best_trial = order.first().map(|&r| build_trial_summary(df, meta, r));
    let top_n: Vec<TrialSummary> = order
        .iter()
        .take(opts.top_n)
        .map(|&r| build_trial_summary(df, meta, r))
        .collect();

    Outcome::SingleObj { best_trial, top_n }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_outcome_multi(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    on_front: &[bool],
    front_rows: &[usize],
    directions: &[Direction],
    is_minimize: &[bool],
    valid_count: usize,
    m: usize,
    opts: &ReportOptions,
) -> (Outcome, Option<McdmSection>) {
    let per_objective_extremes = build_objective_extremes(
        df,
        meta,
        objectives,
        trial_numbers,
        valid_row,
        directions,
        is_minimize,
        m,
    );
    let scatter = build_scatter_points(df, objectives, trial_numbers, valid_row, on_front, m);

    // MCDM input (equal weights, Pareto-front subset). No computation
    // needed if the front is empty.
    let mcdm_values: Option<(Vec<f64>, usize, Vec<f64>)> = if front_rows.is_empty() || m == 0 {
        None
    } else {
        let k = front_rows.len();
        let values = flatten_front(objectives, front_rows, m);
        let weights = vec![1.0 / m as f64; m];
        Some((values, k, weights))
    };
    // The equal-weight TOPSIS ranking is used both for ordering
    // pareto_table and, unless omitted by `skip_decision_sections`, for the
    // TOPSIS entries in the MCDM section, so it's computed once here and
    // shared (previously build_mcdm and pareto_table each ran the same
    // computation redundantly).
    let front_topsis: Option<topsis::TopsisResult> =
        mcdm_values.as_ref().and_then(|(values, k, weights)| {
            topsis::compute_topsis(values, *k, m, weights, is_minimize).ok()
        });

    let mcdm = if opts.skip_decision_sections {
        None
    } else {
        match (&mcdm_values, &front_topsis) {
            (Some((values, k, weights)), Some(ts)) => build_mcdm(
                ts,
                values,
                *k,
                weights,
                is_minimize,
                m,
                front_rows,
                trial_numbers,
                objectives,
            ),
            _ => None,
        }
    };

    let pareto_table = build_pareto_table(df, meta, front_rows, front_topsis.as_ref(), opts);

    // The violation count for the fallback note is counted over the entire
    // front before capping (counting from the capped pareto_table would be
    // clamped at top_n*2 and under-report the count).
    let feas = df.feasibility();
    let pareto_infeasible_count = front_rows.iter().filter(|&&r| !feas.is_feasible(r)).count();

    let outcome = Outcome::MultiObj {
        pareto_size: front_rows.len(),
        complete_count: valid_count,
        objective_count: m,
        per_objective_extremes,
        pareto_table,
        pareto_infeasible_count,
        scatter,
        scatter_axes: (0, 1),
    };

    (outcome, mcdm)
}

/// Builds per-objective extremes (best/worst along the direction, plus the
/// feasibility of the best trial) from all COMPLETE trials.
#[allow(clippy::too_many_arguments)]
fn build_objective_extremes(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    directions: &[Direction],
    is_minimize: &[bool],
    m: usize,
) -> Vec<ObjectiveExtreme> {
    let mut per_objective_extremes = Vec::with_capacity(m);
    for j in 0..m {
        let minimize = is_minimize[j];
        let mut best_v = if minimize {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
        let mut worst_v = if minimize {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut best_row: Option<usize> = None;
        for row in 0..objectives.len() {
            if !valid_row[row] {
                continue;
            }
            let v = objectives[row][j];
            let is_best = if minimize { v < best_v } else { v > best_v };
            if best_row.is_none() || is_best {
                best_v = v;
                best_row = Some(row);
            }
            let is_worst = if minimize { v > worst_v } else { v < worst_v };
            if is_worst {
                worst_v = v;
            }
        }
        if let Some(br) = best_row {
            per_objective_extremes.push(ObjectiveExtreme {
                objective_name: meta.objective_names[j].clone(),
                direction: directions[j],
                best_value: best_v,
                best_trial_number: trial_numbers[br],
                best_feasible: df.feasibility().is_feasible(br),
                worst_value: worst_v,
            });
        }
    }
    per_objective_extremes
}
