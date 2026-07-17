use std::collections::HashMap;
use std::sync::mpsc;

use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;

/// Draws the main canvas.
/// Shows guidance when no study is selected, and delegates to canvas_view once one is selected.
pub fn show_main_canvas(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    layout: &mut LayoutState,
    widgets: &mut WidgetStates,
    canvas_widgets: &mut HashMap<u64, WidgetStates>,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    // Show guidance when no study is selected
    if app_state.current_study.is_none() {
        ui.centered_and_justified(|ui| {
            if app_state.all_studies.is_empty() {
                ui.label("Open a journal file to start.");
            } else {
                ui.label("Select a study from the toolbar.");
            }
        });
        return;
    }

    crate::ui::canvas_view::show_canvas_view(ui, app_state, layout, widgets, canvas_widgets, tx);
}
