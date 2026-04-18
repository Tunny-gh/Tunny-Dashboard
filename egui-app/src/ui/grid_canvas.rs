use std::sync::mpsc;

use crate::state::app_state::AppState;
use crate::state::layout_state::{DragPayload, GridCell, LayoutState, PanelItem};
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::trial_table::TrialTableWidget;

const CLOSE_BUTTON_SIZE: f32 = 16.0;
const HANDLE_THICKNESS: f32 = 6.0;
const DRAG_HANDLE_HEIGHT: f32 = 24.0;

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
                ui.painter()
                    .rect_filled(cell_rect, 0.0, egui::Color32::WHITE);
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
                let (inner_resp, payload) = child_ui.dnd_drop_zone::<DragPayload, _>(frame, |ui| {
                    should_clear = render_cell_content(ui, app_state, widgets, cell, r, c, tx);
                });

                // ホバー中はハイライト
                if inner_resp.response.contains_pointer()
                    && egui::DragAndDrop::has_any_payload(ui.ctx())
                {
                    ui.painter().rect_filled(
                        cell_rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(37, 99, 235, 40),
                    );
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
                            ui.painter().rect_filled(
                                right_handle_rect,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(37, 99, 235, 80),
                            );
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
                            ui.painter().rect_filled(
                                bottom_handle_rect,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(37, 99, 235, 80),
                            );
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
fn show_cell_toolbar(
    ui: &mut egui::Ui,
    row: usize,
    col: usize,
    item: PanelItem,
    title: &'static str,
) -> bool {
    let drag_id = egui::Id::new("cell_drag_handle").with(row).with(col);
    let payload = DragPayload::MoveFromCell { item, row, col };
    let mut should_clear = false;

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

                    let spacer = (ui.available_width() - CLOSE_BUTTON_SIZE).max(0.0);
                    ui.add_space(spacer);

                    let close_resp = ui.add_sized(
                        egui::vec2(CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE),
                        egui::Button::new(
                            egui::RichText::new("x")
                                .small()
                                .color(egui::Color32::from_gray(180)),
                        )
                        .frame(false),
                    );
                    if close_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if close_resp.clicked() {
                        should_clear = true;
                    }
                },
            );
        });

    should_clear
}

/// セルのコンテンツを描画する。
/// コンテンツがある場合は上部ハンドルのみを dnd_drag_source として扱い、内部UI操作と競合しないようにする。
fn render_cell_content(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    cell: &GridCell,
    row: usize,
    col: usize,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    match &cell.content {
        Some(PanelItem::Chart(id)) => {
            let chart_id = id.clone();
            let item = PanelItem::Chart(chart_id.clone());
            let title = item.label();
            let should_clear = show_cell_toolbar(ui, row, col, item, title);
            egui::Frame::default()
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.push_id((row, col), |ui| {
                        crate::ui::chart_registry::show_chart(
                            ui, app_state, widgets, &chart_id, tx,
                        );
                    });
                });
            should_clear
        }
        Some(PanelItem::TrialTable) => {
            let item = PanelItem::TrialTable;
            let title = item.label();
            let should_clear = show_cell_toolbar(ui, row, col, item, title);
            egui::Frame::default()
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.push_id((row, col), |ui| {
                        TrialTableWidget.show(ui, app_state);
                    });
                });
            should_clear
        }
        None => {
            let _ = (row, col);
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("— No chart selected —").weak());
            });
            false
        }
    }
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
}
