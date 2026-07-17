use std::sync::mpsc;

use crate::state::app_state::AppState;
use crate::state::layout_state::ChartId;
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;

/// Renders the chart widget corresponding to a ChartId.
///
/// Internally calls, in order, [`crate::ui::render_chart::render_chart`]
/// (rendering only, no `tx` needed) and
/// [`crate::ui::poll_chart::poll_chart_work`] (async dispatch only, no `ui`
/// needed).
pub fn show_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    crate::ui::render_chart::render_chart(ui, app_state, widgets, chart_id);
    crate::ui::poll_chart::poll_chart_work(app_state, widgets, chart_id, tx);
}
