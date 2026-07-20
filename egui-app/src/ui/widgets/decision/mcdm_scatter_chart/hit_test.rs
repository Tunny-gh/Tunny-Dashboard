//! Hit-testing candidate computation for the MCDM scatter chart.

use crate::state::results::McdmResult;
use crate::state::types::StudyView;

use super::points::extract_axis_values;

/// Computes candidates for hit testing (trial_id, row index, coordinates).
/// Only covers points with finite values drawn in the scatter plot (feasible or
/// infeasible).
pub(super) fn compute_hit_candidates(
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
    x_axis: &str,
    y_axis: &str,
) -> Vec<(u32, usize, [f64; 2])> {
    let (Ok(x_vals), Ok(y_vals)) = (
        extract_axis_values(x_axis, mcdm_result, view, obj_names),
        extract_axis_values(y_axis, mcdm_result, view, obj_names),
    ) else {
        return Vec::new();
    };
    (0..view.row_count())
        .filter_map(|i| {
            let x = x_vals.get(i).copied()?;
            let y = y_vals.get(i).copied()?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            Some((trial_id, i, [x, y]))
        })
        .collect()
}
