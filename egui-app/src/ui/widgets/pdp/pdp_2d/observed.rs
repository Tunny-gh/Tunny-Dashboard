//! Observed-data extraction for the 2D PDP 3D view: pulls (param1, param2, objective)
//! samples out of the study view for overlay rendering on the surface plot.

use crate::state::types::StudyView;
use crate::ui::widgets::pdp_chart::{classify_observed, ObservedKind};

/// Extracts observed data (row index, [param1, param2, objective], classification) from
/// the view (a testable pure function). The row index is used to identify the trial for
/// the hover tooltip / click detail.
///
/// Filtering rules match `extract_observed` in 1D PDP: all trials if `selected_indices`
/// is empty, otherwise only selected / pinned. Rows containing non-finite values are
/// skipped. Classification follows the same rule as the other scatter plots
/// (pareto_rank == 0 → Pareto, is_feasible <= 0.5 → Infeasible).
pub(crate) fn extract_observed_3d(
    view: &StudyView,
    param1: &str,
    param2: &str,
    objective: &str,
    selected_indices: &[u32],
    pinned: &[u32],
) -> Vec<(usize, [f64; 3], ObservedKind)> {
    let (Some(p1_col), Some(p2_col), Some(obj_col)) = (
        view.numeric_column(param1),
        view.numeric_column(param2),
        view.numeric_column(objective),
    ) else {
        return vec![];
    };
    let feas = view.feasibility();

    let use_filter = !selected_indices.is_empty();
    let selected_set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
    let pinned_set: std::collections::HashSet<u32> = pinned.iter().copied().collect();

    (0..view.row_count())
        .filter_map(|i| {
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            if use_filter && !selected_set.contains(&trial_id) && !pinned_set.contains(&trial_id) {
                return None;
            }
            let p1 = p1_col.get(i).copied()?;
            let p2 = p2_col.get(i).copied()?;
            let ov = obj_col.get(i).copied()?;
            if !p1.is_finite() || !p2.is_finite() || !ov.is_finite() {
                return None;
            }
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            Some((
                i,
                [p1, p2, ov],
                classify_observed(feas.is_feasible(i), rank),
            ))
        })
        .collect()
}
