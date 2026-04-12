use crate::state::app_state::AppState;
use crate::state::layout_state::{ChartId, GridCell, LayoutState, PanelItem};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::trial_table::TrialTableWidget;

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

    // 各セルをクローンして借用エラーを回避
    let cells: Vec<Vec<GridCell>> = layout.grid.cells.clone();

    // ドロップ・コンテキストメニューアクションを収集してから一括適用（借用エラー回避）
    let mut pending_drops: Vec<(usize, usize, PanelItem)> = Vec::new();
    let mut pending_actions: Vec<CellAction> = Vec::new();

    for r in 0..rows {
        for c in 0..cols {
            let cell = &cells[r][c];

            // 結合先のセルは描画をスキップ（結合元が担当）
            if cell.merged_into.is_some() {
                continue;
            }

            let w = calc_cell_width(total_w, cols, cell.col_span);
            let h = calc_cell_height(grid_h, rows, cell.row_span);
            let min = grid_area.min + egui::vec2(c as f32 * cell_w, r as f32 * cell_h);
            let cell_rect = egui::Rect::from_min_size(min, egui::vec2(w, h));

            // セルの境界線を描画
            ui.painter().rect_stroke(
                cell_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
            );

            // セル内の子 UI を作成
            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(cell_rect)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );

            // D&D ドロップゾーンとしてラップ
            let frame = egui::Frame::default();
            let (inner_resp, payload) = child_ui.dnd_drop_zone::<PanelItem, _>(frame, |ui| {
                render_cell_content(ui, app_state, widgets, cell, r, c);
            });

            // ホバー中はハイライト
            if inner_resp.response.contains_pointer()
                && egui::DragAndDrop::has_any_payload(ui.ctx())
            {
                ui.painter().rect_filled(
                    cell_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(100, 150, 255, 40),
                );
            }

            // ドロップされた場合は pending リストに追加
            if let Some(dropped) = payload {
                pending_drops.push((r, c, (*dropped).clone()));
            }

            // 右クリックコンテキストメニュー
            let col_span = cell.col_span;
            let row_span = cell.row_span;
            let can_expand_right = (c + col_span as usize) < cols;
            let can_expand_down = (r + row_span as usize) < rows;
            let can_shrink_right = col_span > 1;
            let can_shrink_down = row_span > 1;
            let has_content = cell.content.is_some();

            inner_resp.response.context_menu(|ui| {
                ui.add_enabled_ui(can_expand_right, |ui| {
                    if ui.button("右に拡張").clicked() {
                        pending_actions.push(CellAction::ExpandRight(r, c));
                        ui.close_menu();
                    }
                });
                ui.add_enabled_ui(can_expand_down, |ui| {
                    if ui.button("下に拡張").clicked() {
                        pending_actions.push(CellAction::ExpandDown(r, c));
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.add_enabled_ui(can_shrink_right, |ui| {
                    if ui.button("縮小（右）").clicked() {
                        pending_actions.push(CellAction::ShrinkRight(r, c));
                        ui.close_menu();
                    }
                });
                ui.add_enabled_ui(can_shrink_down, |ui| {
                    if ui.button("縮小（下）").clicked() {
                        pending_actions.push(CellAction::ShrinkDown(r, c));
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.add_enabled_ui(has_content, |ui| {
                    if ui.button("クリア").clicked() {
                        pending_actions.push(CellAction::Clear(r, c));
                        ui.close_menu();
                    }
                });
            });
        }
    }

    // 収集したドロップを適用
    for (r, c, item) in pending_drops {
        layout.grid.place(r, c, item);
    }

    // 収集したコンテキストメニューアクションを適用
    for action in pending_actions {
        match action {
            CellAction::ExpandRight(r, c) => {
                layout.grid.expand_right(r, c);
            }
            CellAction::ExpandDown(r, c) => {
                layout.grid.expand_down(r, c);
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
    if button_ui.button("＋行").clicked() {
        layout.grid.add_row();
    }
    if button_ui
        .add_enabled(can_remove_row, egui::Button::new("－行"))
        .clicked()
    {
        layout.grid.try_remove_last_row();
    }
    button_ui.separator();
    if button_ui.button("＋列").clicked() {
        layout.grid.add_col();
    }
    if button_ui
        .add_enabled(can_remove_col, egui::Button::new("－列"))
        .clicked()
    {
        layout.grid.try_remove_last_col();
    }
}

/// セルのコンテンツを描画する
fn render_cell_content(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    cell: &GridCell,
    row: usize,
    col: usize,
) {
    match &cell.content {
        Some(PanelItem::Chart(id)) => {
            let id = id.clone();
            show_cell_chart(ui, app_state, widgets, &id);
        }
        Some(PanelItem::TrialTable) => {
            TrialTableWidget::default().show(ui, app_state);
        }
        None => {
            // 空セルのプレースホルダー
            let _ = (row, col); // 将来のD&Dドロップゾーン登録用
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("— No chart selected —").weak());
            });
        }
    }
}

/// タイトルと区切り線付きでチャートを描画する
fn show_cell_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
) {
    ui.label(egui::RichText::new(chart_id.label()).strong());
    ui.separator();
    show_chart(ui, app_state, widgets, chart_id);
}

/// ChartId に対応するチャートウィジェットを描画する
fn show_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
) {
    let Some(ctx) = app_state.current_study.as_ref() else {
        return;
    };
    let trial_rows = ctx.trial_rows.clone();
    let obj_names = ctx.meta.objective_names.clone();
    let param_names = ctx.meta.param_names.clone();
    let is_minimize = ctx
        .meta
        .directions
        .first()
        .map(|d| matches!(d, crate::state::app_state::Direction::Minimize))
        .unwrap_or(true);
    let sensitivity = app_state.sensitivity_result.clone();
    let hv_history = app_state.hv_history.clone();

    match chart_id {
        ChartId::ParetoScatter2D => {
            widgets.pareto_2d.show(ui, app_state);
        }
        ChartId::OptimizationHistory => {
            widgets.opt_history.show(ui, &trial_rows, is_minimize);
        }
        ChartId::HvHistory => {
            widgets.hv_history.hv_history = hv_history;
            widgets.hv_history.show(ui);
        }
        ChartId::ImportanceChart => {
            widgets
                .importance
                .show(ui, sensitivity.as_ref(), &obj_names);
        }
        ChartId::PdpChart => {
            widgets.pdp_chart.show(ui, &param_names, &obj_names);
        }
        ChartId::ParallelCoordinates => {
            widgets
                .parallel_coords
                .show(ui, &trial_rows, &param_names, &obj_names);
        }
        ChartId::ScatterMatrix => {
            widgets
                .scatter_matrix
                .show(ui, &trial_rows, &param_names, &obj_names);
        }
        ChartId::ParetoScatter3D => {
            ui.label("3D Pareto chart requires GPU rendering (not yet wired up).");
        }
        ChartId::SensitivityHeatmap => {
            widgets.sensitivity_heatmap.show(ui, sensitivity.as_ref());
        }
        ChartId::ClusterScatter => {
            widgets.cluster_scatter.show(
                ui,
                &trial_rows,
                app_state.cluster_result.as_ref(),
                &param_names,
            );
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
