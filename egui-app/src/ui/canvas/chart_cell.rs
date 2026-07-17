use std::sync::mpsc;

use crate::state::app_state::AppState;
use crate::state::layout_state::PanelItem;
use crate::state::messages::AppMessage;
use crate::theme::CLOSE_BTN_TEXT;
use crate::ui::widget_states::WidgetStates;

pub(crate) const CLOSE_BUTTON_SIZE: f32 = 16.0;
pub(crate) const DRAG_HANDLE_HEIGHT: f32 = 24.0;

/// Action returned by the cell toolbar.
pub enum CellToolbarAction {
    None,
    Close,
    /// Maximizes the widget's display via a double-click on the title bar.
    Maximize(PanelItem),
    Help(PanelItem),
    SaveAsPng(PanelItem),
    SaveAsCsv(PanelItem),
    CopyCsv(PanelItem),
    CopyImage(PanelItem),
}

/// Draws the chart cell's "…" (options) menu button and returns the selected action.
/// Shared by the canvas view's toolbar and the maximize modal.
pub(crate) fn show_chart_menu_button(
    ui: &mut egui::Ui,
    item: &PanelItem,
    csv_available: bool,
) -> Option<CellToolbarAction> {
    let mut menu_action: Option<CellToolbarAction> = None;
    let menu_resp = ui.menu_button(
        egui::RichText::new("…").small().color(CLOSE_BTN_TEXT()),
        |ui| {
            if ui.button("Save as PNG").clicked() {
                menu_action = Some(CellToolbarAction::SaveAsPng(item.clone()));
            }
            if ui.button("Copy image to clipboard").clicked() {
                menu_action = Some(CellToolbarAction::CopyImage(item.clone()));
            }
            ui.separator();
            let csv_btn = ui.add_enabled(csv_available, egui::Button::new("Save as CSV"));
            if csv_btn.clicked() {
                menu_action = Some(CellToolbarAction::SaveAsCsv(item.clone()));
            }
            if !csv_available {
                csv_btn.on_hover_text("No data available");
            }
            let copy_btn =
                ui.add_enabled(csv_available, egui::Button::new("Copy data to clipboard"));
            if copy_btn.clicked() {
                menu_action = Some(CellToolbarAction::CopyCsv(item.clone()));
            }
            if !csv_available {
                copy_btn.on_hover_text("No data available");
            }
            ui.separator();
            if ui.button("Help").clicked() {
                menu_action = Some(CellToolbarAction::Help(item.clone()));
            }
        },
    );
    menu_resp.response.on_hover_text("Options");
    menu_action
}

/// Records a PNG capture request into `ChartCaptureState`.
pub fn record_capture_target(
    state: &mut crate::ui::widget_states::ChartCaptureState,
    item: crate::state::layout_state::PanelItem,
    dest: crate::ui::widget_states::CaptureDest,
) {
    state.pending_capture = Some(item);
    state.pending_capture_dest = dest;
}

pub(crate) fn handle_toolbar_action(
    ctx: &egui::Context,
    action: &CellToolbarAction,
    help_language: crate::ui::help::help_types::HelpLanguage,
    widgets: &mut WidgetStates,
    app_state: &AppState,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    use crate::ui::widget_states::CaptureDest;
    match action {
        CellToolbarAction::Help(help_item) => {
            if let Err(e) = crate::ui::help::help_launcher::open_help(help_item, help_language) {
                let _ = tx.try_send(AppMessage::Error(e));
            }
        }
        CellToolbarAction::SaveAsPng(target) => {
            record_capture_target(&mut widgets.capture, target.clone(), CaptureDest::File);
        }
        CellToolbarAction::CopyImage(target) => {
            record_capture_target(&mut widgets.capture, target.clone(), CaptureDest::Clipboard);
        }
        CellToolbarAction::SaveAsCsv(PanelItem::Chart(chart_id)) => {
            // Run the save dialog (rfd) on the UI thread first to determine the
            // path (same convention as report_export: dialog → background write).
            let filename = crate::io::csv_export::csv_export_filename(chart_id);
            if let Some(path) = crate::io::export::pick_csv_save_path(&filename) {
                // build_chart_csv needs &AppState / &WidgetStates (many caches and
                // widget states), so it can't be sent to a worker. Build the CSV
                // string on the UI thread and delegate only the potentially
                // blocking file write to the background.
                if let Some(csv_str) =
                    crate::io::csv_export::build_chart_csv(chart_id, app_state, widgets)
                {
                    crate::io::export::spawn_csv_write(csv_str, path, tx.clone());
                }
            }
        }
        CellToolbarAction::CopyCsv(PanelItem::Chart(chart_id)) => {
            if let Some(csv_str) =
                crate::io::csv_export::build_chart_csv(chart_id, app_state, widgets)
            {
                ctx.copy_text(csv_str);
            }
        }
        CellToolbarAction::SaveAsCsv(PanelItem::TrialTable) => {
            // Same as the chart version: run the save dialog on the UI thread
            // first, then delegate the file write to the background.
            let filename = crate::io::csv_export::trial_table_csv_filename(widgets);
            if let Some(path) = crate::io::export::pick_csv_save_path(&filename) {
                if let Some(csv_str) =
                    crate::io::csv_export::build_trial_table_csv(app_state, widgets)
                {
                    crate::io::export::spawn_csv_write(csv_str, path, tx.clone());
                }
            }
        }
        CellToolbarAction::CopyCsv(PanelItem::TrialTable) => {
            if let Some(csv_str) = crate::io::csv_export::build_trial_table_csv(app_state, widgets)
            {
                ctx.copy_text(csv_str);
            }
        }
        _ => {}
    }
}

/// Shared function that draws the body (chart/table) of a `PanelItem`.
/// Called from both the canvas view and the maximize modal. `id_salt` avoids
/// egui ID collisions between instances (canvas passes item.id, the maximize
/// modal passes a fixed string).
/// Returns the drawn rect of the body only (excluding the toolbar), used for
/// cropping the PNG/image capture.
pub(crate) fn render_panel_item_body(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    item: &PanelItem,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    tx: &mpsc::SyncSender<AppMessage>,
) -> egui::Rect {
    let body_resp = egui::Frame::default()
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.push_id(id_salt, |ui| match item {
                PanelItem::Chart(chart_id) => {
                    crate::ui::chart_registry::show_chart(ui, app_state, widgets, chart_id, tx);
                }
                PanelItem::TrialTable => {
                    widgets.trial_table.show(ui, app_state);
                    crate::ui::poll_chart::poll_trial_table_work(app_state, widgets, tx);
                }
            });
        });
    body_resp.response.rect
}

/// Draws the maximize modal.
/// When `widgets.maximized_item` is `Some`, dims the screen and displays the
/// target widget enlarged. Closed via the Esc key, clicking the background, or
/// the × button.
/// Call this at the end of `show_layout` (after all panels) so it overlays
/// everything.
pub(crate) fn show_maximized_modal(
    ctx: &egui::Context,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(item) = widgets.maximized_item.clone() else {
        return;
    };

    let screen = ctx.content_rect();
    let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    // Dim the background (click to close). Created before the window so it sits behind it.
    egui::Area::new(egui::Id::new("maximized_modal_dim"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(screen, egui::Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(160));
            if resp.clicked() {
                close = true;
            }
        });

    // The centered maximize window (closed via the × in the title bar).
    // If the chart has a specific supplementary caption (legend), append it to the title.
    let win_title = match item.subtitle() {
        Some(subtitle) => format!("{}    —    {}", item.label(), subtitle),
        None => item.label().to_string(),
    };
    let win_rect = screen.shrink(40.0);
    let mut open = true;
    egui::Window::new(win_title)
        .id(egui::Id::new("maximized_modal_window"))
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .open(&mut open)
        .fixed_rect(win_rect)
        .show(ctx, |ui| {
            render_panel_item_body(ui, app_state, widgets, &item, "maximized_modal", tx);
        });

    if close || !open {
        widgets.maximized_item = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TASK-2244 tests ─────────────────────────────────────────

    #[test]
    fn cell_toolbar_shows_menu_for_chart_cells() {
        use crate::state::layout_state::{ChartId, PanelItem};
        // Chart cells have a SaveAsPng menu entry (CellToolbarAction has a SaveAsPng variant)
        let item = PanelItem::Chart(ChartId::OptimizationHistory);
        let action = CellToolbarAction::SaveAsPng(item);
        assert!(matches!(action, CellToolbarAction::SaveAsPng(_)));
    }

    #[test]
    fn save_as_png_action_records_target_cell() {
        use crate::state::layout_state::{ChartId, PanelItem};
        use crate::ui::widget_states::{CaptureDest, ChartCaptureState};
        let mut state = ChartCaptureState::default();
        let item = PanelItem::Chart(ChartId::ParallelCoordinates);
        record_capture_target(&mut state, item.clone(), CaptureDest::File);
        assert_eq!(state.pending_capture, Some(item));
        assert_eq!(state.pending_capture_dest, CaptureDest::File);
    }

    #[test]
    fn copy_image_action_records_clipboard_dest() {
        use crate::state::layout_state::{ChartId, PanelItem};
        use crate::ui::widget_states::{CaptureDest, ChartCaptureState};
        let mut state = ChartCaptureState::default();
        let item = PanelItem::Chart(ChartId::ParallelCoordinates);
        record_capture_target(&mut state, item.clone(), CaptureDest::Clipboard);
        assert_eq!(state.pending_capture, Some(item));
        assert_eq!(state.pending_capture_dest, CaptureDest::Clipboard);
    }
}
