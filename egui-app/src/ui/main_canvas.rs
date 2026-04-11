use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;

/// メインキャンバスを描画する（各チャートタスクで完全実装予定）
pub fn show_main_canvas(ui: &mut egui::Ui, _app_state: &mut AppState, _layout: &mut LayoutState) {
    ui.centered_and_justified(|ui| {
        ui.label("Open a journal file to start.");
    });
}
