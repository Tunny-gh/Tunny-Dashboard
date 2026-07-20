//! Pareto scatter / table construction.

use crate::data::dataframe::DataFrame;
use crate::io::journal::parser::StudyMeta;
use crate::mcdm::topsis;

use super::trial_summary::{build_trial_summary, mark_duplicate_objectives};
use super::ReportOptions;
use crate::report::model::*;

/// Builds scatter points (all COMPLETE, first two objective axes, with
/// front / feasible flags).
pub(super) fn build_scatter_points(
    df: &DataFrame,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    on_front: &[bool],
    m: usize,
) -> Vec<ParetoPoint> {
    let feas = df.feasibility();
    (0..objectives.len())
        .filter(|&r| valid_row[r])
        .map(|r| ParetoPoint {
            trial_number: trial_numbers[r],
            x: objectives[r][0],
            y: if m >= 2 { objectives[r][1] } else { f64::NAN },
            on_front: on_front[r],
            feasible: feas.is_feasible(r),
        })
        .collect()
}

/// Builds the Pareto table (TOPSIS order, or front row order if ranking
/// wasn't computed; capped at `top_n*2`; with duplicate-solution marks).
pub(super) fn build_pareto_table(
    df: &DataFrame,
    meta: &StudyMeta,
    front_rows: &[usize],
    front_topsis: Option<&topsis::TopsisResult>,
    opts: &ReportOptions,
) -> Vec<TrialSummary> {
    let cap = opts.top_n.saturating_mul(2);
    let pareto_table_rows: Vec<usize> = match front_topsis {
        Some(ts) => ts
            .ranked_indices
            .iter()
            .map(|&sub| front_rows[sub as usize])
            .collect(),
        None => front_rows.to_vec(),
    };
    let mut pareto_table: Vec<TrialSummary> = pareto_table_rows
        .iter()
        .take(cap.max(1))
        .map(|&r| build_trial_summary(df, meta, r))
        .collect();
    mark_duplicate_objectives(&mut pareto_table);
    pareto_table
}

/// Flattens the objective values of the Pareto-front subset into row-major
/// order.
pub(super) fn flatten_front(objectives: &[Vec<f64>], front_rows: &[usize], m: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(front_rows.len() * m);
    for &r in front_rows {
        values.extend_from_slice(&objectives[r][..m]);
    }
    values
}
