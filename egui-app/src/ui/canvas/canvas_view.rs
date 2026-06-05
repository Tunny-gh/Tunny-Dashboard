use std::sync::mpsc;

use egui::emath::TSTransform;

use crate::state::app_state::AppState;
use crate::state::layout_state::{DragPayload, LayoutState, PanelItem};
use crate::state::messages::AppMessage;
use crate::theme::chart_colors::{COLOR_CELL_HIGHLIGHT, COLOR_SELECTION_HIGHLIGHT};
use crate::ui::grid_canvas::{handle_toolbar_action, render_panel_item_body, CellToolbarAction};
use crate::ui::widget_states::WidgetStates;

/// ワールド座標でのドットグリッド間隔
const GRID_WORLD: f32 = 40.0;
/// ズーム下限・上限
const ZOOM_MIN: f32 = 0.3;
const ZOOM_MAX: f32 = 3.0;
/// 新規ウィジェットのデフォルトサイズ（ワールド座標）
const DEFAULT_W: f32 = 360.0;
const DEFAULT_H: f32 = 280.0;
/// リサイズ時の最小サイズ
const MIN_W: f32 = 160.0;
const MIN_H: f32 = 120.0;
/// ツールバー要素サイズ
const CLOSE_BUTTON_SIZE: f32 = 16.0;
const DRAG_HANDLE_HEIGHT: f32 = 24.0;
/// リサイズハンドルの一辺
const RESIZE_HANDLE: f32 = 14.0;

/// アイテムごとのフレーム結果（借用解放後に一括適用するためのアクション）
enum CanvasAction {
    Move(u64, egui::Vec2),
    Resize(u64, egui::Vec2),
    Remove(u64),
}

/// アイテムツールバーの結果
struct ItemToolbarResult {
    move_delta: egui::Vec2,
    action: CellToolbarAction,
}

/// Area クロージャから返すアイテムの操作結果
struct CanvasItemOutput {
    move_delta: egui::Vec2,
    resize_delta: egui::Vec2,
    close: bool,
}

/// 自由配置キャンバスを描画する。
///
/// 無限平面上にウィジェットを自由配置し、空白ドラッグでパン、スクロール/ピンチでズームできる。
/// 各ウィジェットは `egui::Area` として描画し、レイヤーに `TSTransform` を適用することで
/// チャート・テキストまで一様に拡大縮小する（egui 公式 pan_zoom デモと同手法）。
pub fn show_canvas_view(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    layout: &mut LayoutState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let area = ui.available_rect_before_wrap();
    ui.set_clip_rect(area);

    // ワールド→画面 変換。area.min 分のオフセットに pan/zoom を合成する。
    let offset = TSTransform::from_translation(area.min.to_vec2());
    let mut to_screen = offset
        * TSTransform::new(
            egui::vec2(layout.canvas.pan_x, layout.canvas.pan_y),
            layout.canvas.zoom,
        );

    // ── 背景操作（パン / ズーム / ダブルクリックでリセット） ──────────────
    let bg = ui.interact(
        area,
        egui::Id::new("canvas_bg"),
        egui::Sense::click_and_drag(),
    );
    if bg.dragged() {
        to_screen.translation += bg.drag_delta();
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if bg.double_clicked() {
        to_screen = offset; // pan=0, zoom=1 にリセット
    }
    if let Some(ptr) = ui.ctx().input(|i| i.pointer.hover_pos()) {
        if area.contains(ptr) {
            // 通常スクロール → パン
            let scroll = ui.ctx().input(|i| i.smooth_scroll_delta);
            if scroll != egui::Vec2::ZERO {
                to_screen.translation += scroll;
            }
            // ピンチ / Ctrl+スクロール → ポインタ基点ズーム
            let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
            if zoom_delta != 1.0 {
                let current = to_screen.scaling;
                let target = (current * zoom_delta).clamp(ZOOM_MIN, ZOOM_MAX);
                let eff = if current > 0.0 { target / current } else { 1.0 };
                if (eff - 1.0).abs() > f32::EPSILON {
                    let pil = to_screen.inverse() * ptr; // ポインタのワールド座標
                    to_screen = to_screen
                        * TSTransform::from_translation(pil.to_vec2())
                        * TSTransform::from_scaling(eff)
                        * TSTransform::from_translation(-pil.to_vec2());
                }
            }
        }
    }

    let zoom = to_screen.scaling;

    // ── 背景塗り＋ドットグリッド（パン/ズームに追従して動く） ─────────────
    let painter = ui.painter().clone();
    painter.rect_filled(area, 0.0, crate::theme::CANVAS_BG);
    {
        let step = (GRID_WORLD * zoom).max(8.0);
        let origin = to_screen * egui::pos2(0.0, 0.0); // ワールド原点の画面位置
        let start_x = area.left() - (area.left() - origin.x).rem_euclid(step);
        let start_y = area.top() - (area.top() - origin.y).rem_euclid(step);
        let r = (1.2 * zoom).clamp(0.6, 2.2);
        let mut gx = start_x;
        while gx <= area.right() {
            let mut gy = start_y;
            while gy <= area.bottom() {
                painter.circle_filled(egui::pos2(gx, gy), r, crate::theme::CANVAS_DOT);
                gy += step;
            }
            gx += step;
        }
    }

    // ── アイテム描画（z-order は egui の Area が自動管理） ────────────────
    let mut actions: Vec<CanvasAction> = Vec::new();

    for item in &layout.canvas.items {
        let ir = egui::Area::new(egui::Id::new("canvas_item").with(item.id))
            .order(egui::Order::Middle)
            .fixed_pos(egui::pos2(item.x, item.y))
            .show(ui.ctx(), |ui| {
                // レイヤー（ワールド）座標系でキャンバス可視域にクリップ
                ui.set_clip_rect(to_screen.inverse().mul_rect(area));

                let content = ui.allocate_ui_with_layout(
                    egui::vec2(item.w, item.h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        let full = ui.max_rect();
                        ui.painter()
                            .rect_filled(full, 4.0, crate::theme::CENTRAL_BG);
                        ui.painter().rect_stroke(
                            full,
                            4.0,
                            egui::Stroke::new(1.0, crate::theme::BORDER_COLOR),
                        );

                        let csv_available = match &item.content {
                            PanelItem::Chart(chart_id) => {
                                crate::io::csv_export::has_csv_data(chart_id, app_state, widgets)
                            }
                            PanelItem::TrialTable => false,
                        };
                        let tb = show_canvas_item_toolbar(
                            ui,
                            &item.content,
                            item.content.label(),
                            csv_available,
                        );

                        // grid と同じ順序: handle_toolbar_action(キャプチャ要求の登録) → body 描画。
                        let had_no_capture = widgets.capture.pending_capture.is_none();
                        let ctx = ui.ctx().clone();
                        handle_toolbar_action(
                            &ctx,
                            &tb.action,
                            app_state.help_language,
                            widgets,
                            app_state,
                            tx,
                        );
                        let body_rect = render_panel_item_body(
                            ui,
                            app_state,
                            widgets,
                            &item.content,
                            item.id,
                            tx,
                        );
                        // キャプチャ矩形は画面座標で記録する（レイヤー変換を適用）。
                        if had_no_capture && widgets.capture.pending_capture.is_some() {
                            widgets.capture.pending_capture_rect =
                                Some(to_screen.mul_rect(body_rect));
                        }
                        tb
                    },
                );
                let tb = content.inner;
                let alloc_rect = content.response.rect;

                // リサイズハンドル（右下）。同レイヤーで body の後に登録して最前面に。
                let handle_rect = egui::Rect::from_min_size(
                    alloc_rect.max - egui::vec2(RESIZE_HANDLE, RESIZE_HANDLE),
                    egui::vec2(RESIZE_HANDLE, RESIZE_HANDLE),
                );
                let rh = ui.interact(
                    handle_rect,
                    egui::Id::new("canvas_resize").with(item.id),
                    egui::Sense::drag(),
                );
                let mut resize_delta = egui::Vec2::ZERO;
                if rh.hovered() || rh.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                    ui.painter()
                        .rect_filled(handle_rect, 0.0, COLOR_CELL_HIGHLIGHT);
                }
                if rh.dragged() {
                    resize_delta = rh.drag_delta();
                }

                CanvasItemOutput {
                    move_delta: tb.move_delta,
                    resize_delta,
                    close: matches!(tb.action, CellToolbarAction::Close),
                }
            });

        // このアイテムのレイヤーに変換を適用（テキストごと拡大縮小）
        ui.ctx()
            .set_transform_layer(ir.response.layer_id, to_screen);

        let out = ir.inner;
        if out.move_delta != egui::Vec2::ZERO {
            actions.push(CanvasAction::Move(item.id, out.move_delta));
        }
        if out.resize_delta != egui::Vec2::ZERO {
            actions.push(CanvasAction::Resize(item.id, out.resize_delta));
        }
        if out.close {
            actions.push(CanvasAction::Remove(item.id));
        }
    }

    // ── 右パネルからの新規ドロップ（レイヤーに依らず area で判定） ──────────
    let mut pending_add: Option<(PanelItem, egui::Pos2)> = None;
    if let Some(payload) = egui::DragAndDrop::payload::<DragPayload>(ui.ctx()) {
        if let DragPayload::NewWidget(new_item) = &*payload {
            // ドロップ可能であることを示すハイライト
            ui.painter()
                .rect_stroke(area, 0.0, egui::Stroke::new(2.0, COLOR_SELECTION_HIGHLIGHT));
            if ui.input(|i| i.pointer.any_released()) {
                if let Some(sp) = ui.ctx().pointer_interact_pos() {
                    if area.contains(sp) {
                        let wp = to_screen.inverse() * sp;
                        pending_add = Some((new_item.clone(), wp));
                    }
                }
            }
        }
    }

    // ── 収集アクションの適用 ─────────────────────────────────────────────
    for action in actions {
        match action {
            CanvasAction::Move(id, d) => {
                if let Some(it) = layout.canvas.items.iter_mut().find(|i| i.id == id) {
                    it.x += d.x;
                    it.y += d.y;
                }
            }
            CanvasAction::Resize(id, d) => {
                if let Some(it) = layout.canvas.items.iter_mut().find(|i| i.id == id) {
                    it.w = (it.w + d.x).max(MIN_W);
                    it.h = (it.h + d.y).max(MIN_H);
                }
            }
            CanvasAction::Remove(id) => layout.canvas.remove(id),
        }
    }
    if let Some((it, wp)) = pending_add {
        layout.canvas.add(it, wp.x, wp.y, DEFAULT_W, DEFAULT_H);
        egui::DragAndDrop::clear_payload(ui.ctx());
    }

    // ── ビューポート変換を書き戻す（pan は無制限＝無限キャンバス、zoom はクランプ） ──
    let logical = offset.inverse() * to_screen;
    layout.canvas.pan_x = logical.translation.x;
    layout.canvas.pan_y = logical.translation.y;
    layout.canvas.zoom = logical.scaling.clamp(ZOOM_MIN, ZOOM_MAX);
}

/// キャンバスアイテム上部のツールバーを描画する。
/// 左の Move ハンドルはドラッグ移動量を返し、`⋯`/`x` は grid と同じ `CellToolbarAction` を返す。
fn show_canvas_item_toolbar(
    ui: &mut egui::Ui,
    item: &PanelItem,
    title: &str,
    csv_available: bool,
) -> ItemToolbarResult {
    let mut result = ItemToolbarResult {
        move_delta: egui::Vec2::ZERO,
        action: CellToolbarAction::None,
    };

    egui::Frame::default()
        .fill(crate::theme::CELL_TOOLBAR_BG)
        .stroke(egui::Stroke::new(1.0, crate::theme::BORDER_COLOR))
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), DRAG_HANDLE_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let move_resp = ui
                        .add_sized(
                            egui::vec2(56.0, DRAG_HANDLE_HEIGHT),
                            egui::Button::new(egui::RichText::new("Move").small()),
                        )
                        .interact(egui::Sense::click_and_drag());
                    if move_resp.dragged() {
                        result.move_delta = move_resp.drag_delta();
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    } else if move_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }

                    ui.add_space(8.0);
                    ui.strong(title);

                    let spacer = (ui.available_width() - CLOSE_BUTTON_SIZE * 2.0 - 4.0).max(0.0);
                    ui.add_space(spacer);

                    if let Some(a) =
                        crate::ui::grid_canvas::show_chart_menu_button(ui, item, csv_available)
                    {
                        result.action = a;
                    }

                    ui.add_space(4.0);

                    let close_resp = ui.add_sized(
                        egui::vec2(CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE),
                        egui::Button::new(
                            egui::RichText::new("x")
                                .small()
                                .color(crate::theme::CLOSE_BTN_TEXT),
                        )
                        .frame(false),
                    );
                    if close_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if close_resp.clicked() {
                        result.action = CellToolbarAction::Close;
                    }
                },
            );
        });

    result
}
