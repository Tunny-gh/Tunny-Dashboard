use super::require_study;
use crate::state::app_state::AppState;

pub(super) fn build_trial_based_csv(app_state: &AppState) -> Option<String> {
    let study = require_study(app_state)?;
    let n = study.trial_count();
    let row_indices: Vec<usize> = (0..n).collect();
    Some(crate::io::export::build_csv_string_from_view(
        &study.view,
        &row_indices,
        &study.meta.param_names,
        &study.meta.objective_names,
    ))
}
