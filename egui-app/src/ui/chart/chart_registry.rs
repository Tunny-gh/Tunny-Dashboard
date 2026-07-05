use std::sync::mpsc;

use crate::state::app_state::AppState;
use crate::state::layout_state::ChartId;
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;

/// ChartId に対応するチャートウィジェットを描画する。
///
/// 内部で [`crate::ui::render_chart::render_chart`]（描画のみ、`tx` 不要）と
/// [`crate::ui::poll_chart::poll_chart_work`]（非同期 dispatch のみ、`ui` 不要）を順に呼び出す。
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
