use std::sync::mpsc;

use crate::state::app_state::AppState;
use crate::state::layout_state::{DragPayload, GridCell, LayoutState, PanelItem};
use crate::state::messages::AppMessage;
use crate::theme::chart_colors::{COLOR_CELL_HIGHLIGHT, COLOR_SELECTION_HIGHLIGHT};
use crate::theme::{CENTRAL_BG, CLOSE_BTN_TEXT};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::trial_table::TrialTableWidget;

pub(crate) const CLOSE_BUTTON_SIZE: f32 = 16.0;
const HANDLE_THICKNESS: f32 = 6.0;
pub(crate) const DRAG_HANDLE_HEIGHT: f32 = 24.0;

/// Action returned by the cell toolbar.
pub enum CellToolbarAction {
    None,
    Close,
    Help(PanelItem),
    SaveAsPng(PanelItem),
    SaveAsCsv(PanelItem),
    CopyCsv(PanelItem),
    CopyImage(PanelItem),
}

/// Returns the static list of items shown in the … (options) popup menu.
pub fn chart_cell_menu_items() -> &'static [&'static str] {
    &[
        "Save as PNG",
        "Copy image to clipboard",
        "Save as CSV",
        "Copy data to clipboard",
        "Help",
    ]
}

/// チャートセルの「…」(オプション) メニューボタンを描画し、選択されたアクションを返す。
/// grid・canvas の両ツールバーから共有する。
pub(crate) fn show_chart_menu_button(
    ui: &mut egui::Ui,
    item: &PanelItem,
    csv_available: bool,
) -> Option<CellToolbarAction> {
    let mut menu_action: Option<CellToolbarAction> = None;
    let menu_resp = ui.menu_button(
        egui::RichText::new("…").small().color(CLOSE_BTN_TEXT),
        |ui| {
            if ui.button("Save as PNG").clicked() {
                menu_action = Some(CellToolbarAction::SaveAsPng(item.clone()));
                ui.close_menu();
            }
            if ui.button("Copy image to clipboard").clicked() {
                menu_action = Some(CellToolbarAction::CopyImage(item.clone()));
                ui.close_menu();
            }
            ui.separator();
            let csv_btn = ui.add_enabled(csv_available, egui::Button::new("Save as CSV"));
            if csv_btn.clicked() {
                menu_action = Some(CellToolbarAction::SaveAsCsv(item.clone()));
                ui.close_menu();
            }
            if !csv_available {
                csv_btn.on_hover_text("No data available");
            }
            let copy_btn =
                ui.add_enabled(csv_available, egui::Button::new("Copy data to clipboard"));
            if copy_btn.clicked() {
                menu_action = Some(CellToolbarAction::CopyCsv(item.clone()));
                ui.close_menu();
            }
            if !csv_available {
                copy_btn.on_hover_text("No data available");
            }
            ui.separator();
            if ui.button("Help").clicked() {
                menu_action = Some(CellToolbarAction::Help(item.clone()));
                ui.close_menu();
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

/// セルの幅を計算する（テスト可能な純粋関数）
pub fn calc_cell_width(total_w: f32, cols: usize, col_span: u8) -> f32 {
    if cols == 0 {
        return 0.0;
    }
    (total_w / cols as f32) * col_span as f32
}

/// セルの高さを計算する（テスト可能な純粋関数）
pub fn calc_cell_height(total_h: f32, rows: usize, row_span: u8) -> f32 {
    if rows == 0 {
        return 0.0;
    }
    (total_h / rows as f32) * row_span as f32
}

/// セルに対するアクション（コンテキストメニュー操作を遅延適用するため）
enum CellAction {
    ExpandRight(usize, usize),
    ExpandDown(usize, usize),
    ShrinkRight(usize, usize),
    ShrinkDown(usize, usize),
    Clear(usize, usize),
}

/// グリッドキャンバスを描画する。
/// `layout.grid` の cells を2次元で走査し、各セルを描画する。
/// merged_into != None のセルはスキップする（結合元セルが描画担当）。
/// グリッド下部に「＋行」「－行」「＋列」「－列」ボタンを配置する。
pub fn show_grid_canvas(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    layout: &mut LayoutState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let available = ui.available_rect_before_wrap();
    let total_w = available.width();
    let total_h = available.height();

    // ボタンバー分（下部 28px）を確保してグリッド領域を縮小
    let button_bar_h = 28.0;
    let grid_h = (total_h - button_bar_h).max(0.0);

    let rows = layout.grid.rows;
    let cols = layout.grid.cols;

    if rows == 0 || cols == 0 {
        return;
    }

    let cell_w = total_w / cols as f32;
    let cell_h = grid_h / rows as f32;

    // グリッド描画領域（ボタンバーを除いた上部）
    let grid_area = egui::Rect::from_min_size(available.min, egui::vec2(total_w, grid_h));
    ui.allocate_rect(grid_area, egui::Sense::hover());

    // ドロップ・コンテキストメニューアクションを収集してから一括適用（借用エラー回避）
    let mut pending_drops: Vec<(usize, usize, PanelItem)> = Vec::new();
    let mut pending_actions: Vec<CellAction> = Vec::new();

    // スコープブロックで cells への不変参照を先に解放し、後段のミュータブルアクセスを許可
    {
        let cells = &layout.grid.cells;
        for (r, row_cells) in cells.iter().enumerate().take(rows) {
            for (c, cell) in row_cells.iter().enumerate().take(cols) {
                // 結合先のセルは描画をスキップ（結合元が担当）
                if cell.merged_into.is_some() {
                    continue;
                }

                let w = calc_cell_width(total_w, cols, cell.col_span);
                let h = calc_cell_height(grid_h, rows, cell.row_span);
                let min = grid_area.min + egui::vec2(c as f32 * cell_w, r as f32 * cell_h);
                let cell_rect = egui::Rect::from_min_size(min, egui::vec2(w, h));

                // セルの背景と境界線を描画
                ui.painter().rect_filled(cell_rect, 0.0, CENTRAL_BG);
                ui.painter().rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(1.0, crate::theme::BORDER_COLOR),
                );

                // セル内の子 UI を作成
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(cell_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );

                // Register before chart content so chart buttons win z-order for left-clicks;
                // context_menu's inner interact() early-returns (same sense) and registers nothing new.
                let bg_id = egui::Id::new("cell_bg_interact").with(r).with(c);
                let bg_resp = child_ui.interact(cell_rect, bg_id, egui::Sense::click());

                // D&D ドロップゾーンとしてラップ（DragPayload 型に変更）
                let frame = egui::Frame::default();
                let mut should_clear = false;
                let mut chart_rect = None;
                let had_no_capture = widgets.capture.pending_capture.is_none();
                let (inner_resp, payload) = child_ui.dnd_drop_zone::<DragPayload, _>(frame, |ui| {
                    let (clear, rect) = render_cell_content(ui, app_state, widgets, cell, r, c, tx);
                    should_clear = clear;
                    chart_rect = rect;
                });
                // SaveAsPng/CopyImage が新たにセットされた場合、チャート本体の rect を記録する。
                // ツールバー（Move/タイトル/⋯）を除くため cell_rect ではなく chart_rect を使う。
                if had_no_capture && widgets.capture.pending_capture.is_some() {
                    widgets.capture.pending_capture_rect = Some(chart_rect.unwrap_or(cell_rect));
                }

                // ホバー中はハイライト
                if inner_resp.response.contains_pointer()
                    && egui::DragAndDrop::has_any_payload(ui.ctx())
                {
                    ui.painter()
                        .rect_filled(cell_rect, 0.0, COLOR_SELECTION_HIGHLIGHT);
                }

                // ドロップされた場合は pending リストに追加
                if let Some(dropped) = payload {
                    let item = (*dropped).item().clone();
                    pending_drops.push((r, c, item));
                }

                if should_clear {
                    pending_actions.push(CellAction::Clear(r, c));
                }

                // リサイズハンドル（コンテンツがあるセルの右端・下端）
                // ドラッグ方向で拡張(右/下)・縮小(左/上)を判定する
                if cell.content.is_some() {
                    let can_expand_right = (c + cell.col_span as usize) < cols;
                    let can_shrink_right = cell.col_span > 1;
                    if can_expand_right || can_shrink_right {
                        let right_handle_rect = egui::Rect::from_min_size(
                            egui::pos2(cell_rect.right() - HANDLE_THICKNESS, cell_rect.top()),
                            egui::vec2(HANDLE_THICKNESS, cell_rect.height()),
                        );
                        let right_id = egui::Id::new("resize_right").with(r).with(c);
                        let right_resp =
                            ui.interact(right_handle_rect, right_id, egui::Sense::click_and_drag());
                        if right_resp.hovered() || right_resp.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            ui.painter()
                                .rect_filled(right_handle_rect, 0.0, COLOR_CELL_HIGHLIGHT);
                        }
                        if right_resp.dragged() {
                            let dx = right_resp.drag_delta().x;
                            ui.ctx().data_mut(|d| {
                                *d.get_temp_mut_or_default::<f32>(right_id) += dx;
                            });
                        }
                        if right_resp.drag_stopped() {
                            let acc = ui.ctx().data_mut(|d| {
                                let v: f32 = d.get_temp(right_id).unwrap_or(0.0);
                                d.remove::<f32>(right_id);
                                v
                            });
                            if acc > 30.0 && can_expand_right {
                                pending_actions.push(CellAction::ExpandRight(r, c));
                            } else if acc < -30.0 && can_shrink_right {
                                pending_actions.push(CellAction::ShrinkRight(r, c));
                            }
                        }
                    }

                    let can_expand_down = (r + cell.row_span as usize) < rows;
                    let can_shrink_down = cell.row_span > 1;
                    if can_expand_down || can_shrink_down {
                        let bottom_handle_rect = egui::Rect::from_min_size(
                            egui::pos2(cell_rect.left(), cell_rect.bottom() - HANDLE_THICKNESS),
                            egui::vec2(cell_rect.width(), HANDLE_THICKNESS),
                        );
                        let bottom_id = egui::Id::new("resize_bottom").with(r).with(c);
                        let bottom_resp = ui.interact(
                            bottom_handle_rect,
                            bottom_id,
                            egui::Sense::click_and_drag(),
                        );
                        if bottom_resp.hovered() || bottom_resp.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                            ui.painter()
                                .rect_filled(bottom_handle_rect, 0.0, COLOR_CELL_HIGHLIGHT);
                        }
                        if bottom_resp.dragged() {
                            let dy = bottom_resp.drag_delta().y;
                            ui.ctx().data_mut(|d| {
                                *d.get_temp_mut_or_default::<f32>(bottom_id) += dy;
                            });
                        }
                        if bottom_resp.drag_stopped() {
                            let acc = ui.ctx().data_mut(|d| {
                                let v: f32 = d.get_temp(bottom_id).unwrap_or(0.0);
                                d.remove::<f32>(bottom_id);
                                v
                            });
                            if acc > 30.0 && can_expand_down {
                                pending_actions.push(CellAction::ExpandDown(r, c));
                            } else if acc < -30.0 && can_shrink_down {
                                pending_actions.push(CellAction::ShrinkDown(r, c));
                            }
                        }
                    }
                }

                // 右クリックコンテキストメニュー
                let col_span = cell.col_span;
                let row_span = cell.row_span;
                let can_expand_right = (c + col_span as usize) < cols;
                let can_expand_down = (r + row_span as usize) < rows;
                let can_shrink_right = col_span > 1;
                let can_shrink_down = row_span > 1;
                let has_content = cell.content.is_some();

                bg_resp.context_menu(|ui| {
                    ui.add_enabled_ui(can_expand_right, |ui| {
                        if ui.button("Expand Right").clicked() {
                            pending_actions.push(CellAction::ExpandRight(r, c));
                            ui.close_menu();
                        }
                    });
                    ui.add_enabled_ui(can_expand_down, |ui| {
                        if ui.button("Expand Down").clicked() {
                            pending_actions.push(CellAction::ExpandDown(r, c));
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    ui.add_enabled_ui(can_shrink_right, |ui| {
                        if ui.button("Shrink Right").clicked() {
                            pending_actions.push(CellAction::ShrinkRight(r, c));
                            ui.close_menu();
                        }
                    });
                    ui.add_enabled_ui(can_shrink_down, |ui| {
                        if ui.button("Shrink Down").clicked() {
                            pending_actions.push(CellAction::ShrinkDown(r, c));
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    ui.add_enabled_ui(has_content, |ui| {
                        if ui.button("Clear").clicked() {
                            pending_actions.push(CellAction::Clear(r, c));
                            ui.close_menu();
                        }
                    });
                });
            }
        }
    } // cells の不変参照をここで解放

    // 収集したドロップを適用
    for (r, c, item) in pending_drops {
        layout.grid.place(r, c, item);
    }

    // 収集したコンテキストメニューアクションを適用
    for action in pending_actions {
        match action {
            CellAction::ExpandRight(r, c) => {
                layout.grid.safe_expand_right(r, c);
            }
            CellAction::ExpandDown(r, c) => {
                layout.grid.safe_expand_down(r, c);
            }
            CellAction::ShrinkRight(r, c) => {
                layout.grid.shrink_right(r, c);
            }
            CellAction::ShrinkDown(r, c) => {
                layout.grid.shrink_down(r, c);
            }
            CellAction::Clear(r, c) => {
                layout.grid.cells[r][c].content = None;
            }
        }
    }

    // 行・列追加/削除ボタンバー（グリッド下部）
    let button_bar_rect = egui::Rect::from_min_size(
        available.min + egui::vec2(0.0, grid_h),
        egui::vec2(total_w, button_bar_h),
    );
    let mut button_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(button_bar_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let can_remove_row = layout.grid.can_remove_last_row();
    let can_remove_col = layout.grid.can_remove_last_col();
    if button_ui.button("+Row").clicked() {
        layout.grid.add_row();
    }
    if button_ui
        .add_enabled(can_remove_row, egui::Button::new("-Row"))
        .clicked()
    {
        layout.grid.try_remove_last_row();
    }
    button_ui.separator();
    if button_ui.button("+Col").clicked() {
        layout.grid.add_col();
    }
    if button_ui
        .add_enabled(can_remove_col, egui::Button::new("-Col"))
        .clicked()
    {
        layout.grid.try_remove_last_col();
    }
}

/// セル上部のツールバーを描画する。
/// 左上の Move ボタンだけがセル移動を開始し、右上の ✕ ボタンはセル内容をクリアする。
/// ? ボタンはヘルプモーダルを開く。
fn show_cell_toolbar(
    ui: &mut egui::Ui,
    row: usize,
    col: usize,
    item: PanelItem,
    title: &'static str,
    csv_available: bool,
) -> CellToolbarAction {
    let drag_id = egui::Id::new("cell_drag_handle").with(row).with(col);
    let payload = DragPayload::MoveFromCell {
        item: item.clone(),
        row,
        col,
    };
    let mut action = CellToolbarAction::None;

    egui::Frame::default()
        .fill(crate::theme::CELL_TOOLBAR_BG)
        .stroke(egui::Stroke::new(1.0, crate::theme::BORDER_COLOR))
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), DRAG_HANDLE_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let drag_resp = ui.dnd_drag_source(drag_id, payload, |ui| {
                        ui.add_sized(
                            egui::vec2(56.0, DRAG_HANDLE_HEIGHT),
                            egui::Button::new(egui::RichText::new("Move").small()),
                        );
                    });
                    if drag_resp.response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    } else if drag_resp.response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }

                    ui.add_space(8.0);
                    ui.strong(title);

                    let spacer = (ui.available_width() - CLOSE_BUTTON_SIZE * 2.0 - 4.0).max(0.0);
                    ui.add_space(spacer);

                    if let Some(a) = show_chart_menu_button(ui, &item, csv_available) {
                        action = a;
                    }

                    ui.add_space(4.0);

                    let close_resp = ui.add_sized(
                        egui::vec2(CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE),
                        egui::Button::new(egui::RichText::new("x").small().color(CLOSE_BTN_TEXT))
                            .frame(false),
                    );
                    if close_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if close_resp.clicked() {
                        action = CellToolbarAction::Close;
                    }
                },
            );
        });

    action
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
            let csv = crate::io::csv_export::build_chart_csv(chart_id, app_state, widgets);
            if let Some(csv_str) = csv {
                let filename = crate::io::csv_export::csv_export_filename(chart_id);
                if let Err(e) = crate::io::export::save_csv_to_file_named(&csv_str, &filename) {
                    let _ = tx.try_send(AppMessage::Error(e));
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
        _ => {}
    }
}

/// セルのコンテンツを描画する。
/// コンテンツがある場合は上部ハンドルのみを dnd_drag_source として扱い、内部UI操作と競合しないようにする。
///
/// 戻り値は `(クリア要求, チャート本体の矩形)`。チャート本体の矩形はツールバーを
/// 含まない描画領域で、PNG/画像クリップボードのクロップに使う。
fn render_cell_content(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    cell: &GridCell,
    row: usize,
    col: usize,
    tx: &mpsc::SyncSender<AppMessage>,
) -> (bool, Option<egui::Rect>) {
    match &cell.content {
        Some(item) => {
            let item = item.clone();
            let title = item.label();
            let csv_available = match &item {
                PanelItem::Chart(chart_id) => {
                    crate::io::csv_export::has_csv_data(chart_id, app_state, widgets)
                }
                PanelItem::TrialTable => false,
            };
            let toolbar_action =
                show_cell_toolbar(ui, row, col, item.clone(), title, csv_available);
            let ctx = ui.ctx().clone();
            handle_toolbar_action(
                &ctx,
                &toolbar_action,
                app_state.help_language,
                widgets,
                app_state,
                tx,
            );
            // チャート本体（ツールバーを除く）の描画領域をキャプチャ矩形に使う。
            let chart_rect = render_panel_item_body(ui, app_state, widgets, &item, (row, col), tx);
            (
                matches!(toolbar_action, CellToolbarAction::Close),
                Some(chart_rect),
            )
        }
        None => {
            let _ = (row, col);
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("— No chart selected —").weak());
            });
            (false, None)
        }
    }
}

/// PanelItem の本体（チャート/テーブル）を描画する共有関数。
/// grid・canvas の両ビューから呼ばれる。`id_salt` はインスタンスごとの egui ID 衝突回避用
/// （grid は (row, col)、canvas は item.id を渡す）。
/// 戻り値はツールバーを含まない本体の描画矩形で、PNG/画像キャプチャのクロップに使う。
pub(crate) fn render_panel_item_body(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    item: &PanelItem,
    id_salt: impl std::hash::Hash,
    tx: &mpsc::SyncSender<AppMessage>,
) -> egui::Rect {
    let body_resp = egui::Frame::default()
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.push_id(id_salt, |ui| match item {
                PanelItem::Chart(chart_id) => {
                    crate::ui::chart_registry::show_chart(ui, app_state, widgets, chart_id, tx);
                }
                PanelItem::TrialTable => {
                    TrialTableWidget.show(ui, app_state);
                }
            });
        });
    body_resp.response.rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_cell_width_equal_division() {
        let w = calc_cell_width(800.0, 2, 1);
        assert_eq!(w, 400.0);
    }

    #[test]
    fn calc_cell_width_col_span_2() {
        let w = calc_cell_width(800.0, 2, 2);
        assert_eq!(w, 800.0);
    }

    #[test]
    fn calc_cell_width_3_cols() {
        let w = calc_cell_width(900.0, 3, 1);
        assert!((w - 300.0).abs() < 0.01);
    }

    #[test]
    fn calc_cell_height_equal_division() {
        let h = calc_cell_height(600.0, 2, 1);
        assert_eq!(h, 300.0);
    }

    #[test]
    fn calc_cell_height_row_span_2() {
        let h = calc_cell_height(600.0, 2, 2);
        assert_eq!(h, 600.0);
    }

    #[test]
    fn calc_cell_width_zero_cols_returns_zero() {
        let w = calc_cell_width(800.0, 0, 1);
        assert_eq!(w, 0.0);
    }

    #[test]
    fn calc_cell_height_zero_rows_returns_zero() {
        let h = calc_cell_height(600.0, 0, 1);
        assert_eq!(h, 0.0);
    }

    // ── TASK-2244 tests ─────────────────────────────────────────

    #[test]
    fn cell_toolbar_shows_menu_for_chart_cells() {
        use crate::state::layout_state::{ChartId, PanelItem};
        // Chart セルは SaveAsPng メニューを持つ（CellToolbarAction に SaveAsPng バリアントがある）
        let item = PanelItem::Chart(ChartId::OptimizationHistory);
        let action = CellToolbarAction::SaveAsPng(item);
        assert!(matches!(action, CellToolbarAction::SaveAsPng(_)));
    }

    #[test]
    fn menu_contains_save_as_png_and_help() {
        let items = chart_cell_menu_items();
        assert!(items.contains(&"Save as PNG"));
        assert!(items.contains(&"Copy image to clipboard"));
        assert!(items.contains(&"Save as CSV"));
        assert!(items.contains(&"Copy data to clipboard"));
        assert!(items.contains(&"Help"));
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
