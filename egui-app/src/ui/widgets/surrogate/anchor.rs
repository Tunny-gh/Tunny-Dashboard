//! Shared helper for resolving the anchor point (candidate design point).
//!
//! Both robustness analysis (`robustness.rs`) and the 3D response-surface viewer
//! (`response_surface.rs`) use the Best trial, or a pinned trial, as the center point.
//! `CenterChoice`, originally in `robustness.rs`, was moved here to be shared by both widgets.
//!
//! Since `CenterChoice` is a setting persisted to the session file (JSON), the variant names
//! (`BestTrial` / `Pinned`) must not be changed (serde's serialized form is based on the variant
//! name, so compatibility is preserved even if the type path changes).

use crate::state::types::{Direction, StudyView};
use tunny_core::surrogate_opt::TrainedSurrogate;

/// How the anchor point (candidate design point) is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CenterChoice {
    /// The best observed trial for the selected objective.
    #[default]
    BestTrial,
    /// A pinned trial (trial_id). Falls back to BestTrial if it no longer exists.
    Pinned(u32),
}

/// Label for the Center/Anchor combo. If the trial that Pinned refers to no longer exists,
/// simply displays "Best trial" so it doesn't disagree with the center point actually used
/// (after the fallback).
pub fn center_label(choice: CenterChoice, view: &StudyView) -> String {
    match choice {
        CenterChoice::BestTrial => "Best trial".to_string(),
        CenterChoice::Pinned(id) => match view.trial_ids.iter().position(|&t| t == id) {
            Some(row) => {
                let number = view.df.get_trial_number(row).unwrap_or(id);
                format!("Trial #{number}")
            }
            None => "Best trial".to_string(),
        },
    }
}

/// Returns the observed best row for the selected objective (direction-aware argmin/argmax).
pub fn best_trial_row(
    view: &StudyView,
    obj_names: &[String],
    directions: &[Direction],
    objective_name: &str,
) -> Option<usize> {
    let obj_idx = obj_names.iter().position(|n| n == objective_name)?;
    let col = view.numeric_column(objective_name)?;
    let minimize = directions
        .get(obj_idx)
        .map(|d| matches!(d, Direction::Minimize))
        .unwrap_or(true);

    let mut best_row = None;
    let mut best_val = if minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for i in 0..view.row_count() {
        let Some(v) = col.get(i).copied() else {
            continue;
        };
        if !v.is_finite() {
            continue;
        }
        let better = if minimize { v < best_val } else { v > best_val };
        if better {
            best_val = v;
            best_row = Some(i);
        }
    }
    best_row
}

/// Resolves the center point as a vector in original units (same order as `trained.param_names`).
/// Falls back to Best trial if the Pinned trial no longer exists.
pub fn resolve_center(
    trained: &TrainedSurrogate,
    choice: CenterChoice,
    view: &StudyView,
    obj_names: &[String],
    directions: &[Direction],
) -> Option<Vec<f64>> {
    let row = match choice {
        CenterChoice::Pinned(id) => view.trial_ids.iter().position(|&t| t == id),
        CenterChoice::BestTrial => None,
    }
    .or_else(|| best_trial_row(view, obj_names, directions, &trained.objective_name))?;

    trained
        .param_names
        .iter()
        .map(|name| view.numeric_column(name)?.get(row).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_choice_default_is_best_trial() {
        assert_eq!(CenterChoice::default(), CenterChoice::BestTrial);
    }

    #[test]
    fn center_choice_serde_round_trip_keeps_variant_names() {
        // Regression guard for session-file compatibility: verifies the variant name appears in the JSON.
        let best = serde_json::to_string(&CenterChoice::BestTrial).unwrap();
        assert_eq!(best, "\"BestTrial\"");
        let pinned = serde_json::to_string(&CenterChoice::Pinned(7)).unwrap();
        assert_eq!(pinned, "{\"Pinned\":7}");
    }
}
