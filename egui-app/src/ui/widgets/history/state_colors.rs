//! Display colors and legend per trial state (shared by Intermediate Values / Timeline).
//!
//! Both widgets color-code by trial state (RUNNING/COMPLETE/PRUNED/FAIL/WAITING),
//! so color resolution and legend drawing are factored out here to avoid duplication.

use tunny_core::extras::TrialState;

use crate::theme::chart_colors::{
    COLOR_EMPTY_STATE, COLOR_STATE_COMPLETE, COLOR_STATE_FAIL, COLOR_STATE_PRUNED,
    COLOR_STATE_RUNNING, COLOR_STATE_WAITING,
};

/// Builds a list of states in order of appearance (no duplicates). Shared helper
/// for building the state set shown in the Intermediate Values / Timeline legend (D-12).
pub fn distinct_states_in_order<I: IntoIterator<Item = TrialState>>(states: I) -> Vec<TrialState> {
    let mut present: Vec<TrialState> = Vec::new();
    for s in states {
        if !present.contains(&s) {
            present.push(s);
        }
    }
    present
}

/// Dims curves/bars that are not currently hovered (drops alpha only).
/// Shared by Intermediate Values / Timeline (D-12).
pub fn dim(color: egui::Color32) -> egui::Color32 {
    let [r, g, b, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, 90)
}

/// Displays an empty-state message centered (shared by Intermediate Values / Timeline, D-12).
pub fn empty_state(ui: &mut egui::Ui, message: &str) {
    ui.centered_and_justified(|ui| {
        ui.colored_label(COLOR_EMPTY_STATE(), message);
    });
}

/// Returns the display color for a trial state.
pub fn state_color(state: TrialState) -> egui::Color32 {
    match state {
        TrialState::Complete => COLOR_STATE_COMPLETE(),
        TrialState::Pruned => COLOR_STATE_PRUNED(),
        TrialState::Running => COLOR_STATE_RUNNING(),
        TrialState::Fail => COLOR_STATE_FAIL(),
        TrialState::Waiting => COLOR_STATE_WAITING(),
    }
}

/// Draws a legend of color swatches + labels, in a fixed order
/// (Complete/Pruned/Running/Fail/Waiting), for only the states present in `present`.
pub fn show_state_legend(ui: &mut egui::Ui, present: &[TrialState]) {
    const ORDER: [TrialState; 5] = [
        TrialState::Complete,
        TrialState::Pruned,
        TrialState::Running,
        TrialState::Fail,
        TrialState::Waiting,
    ];
    ui.horizontal(|ui| {
        for state in ORDER {
            if !present.contains(&state) {
                continue;
            }
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, state_color(state));
            ui.label(
                egui::RichText::new(state.label())
                    .small()
                    .color(crate::theme::TEXT_SECONDARY()),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_color_is_distinct_per_state() {
        let colors = [
            state_color(TrialState::Complete),
            state_color(TrialState::Pruned),
            state_color(TrialState::Running),
            state_color(TrialState::Fail),
            state_color(TrialState::Waiting),
        ];
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "colors[{i}] == colors[{j}]");
            }
        }
    }
}
