use std::sync::mpsc;

use egui::emath::TSTransform;

use crate::state::app_state::AppState;
use crate::state::layout_state::{DragPayload, LayoutState, PanelItem};
use crate::state::messages::AppMessage;
use crate::theme::chart_colors::COLOR_SELECTION_HIGHLIGHT;
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
    /// 左上を固定したまま絶対サイズ (w, h) を設定する
    Resize(u64, f32, f32),
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
    /// 左上固定でのリサイズ後の絶対サイズ (w, h)
    resize_to: Option<(f32, f32)>,
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
    // スクロール/ズームは「背景（空白部分）をホバーしているとき」のみキャンバスへ適用する。
    // bg.hovered() はアイテム（Area）に遮蔽されると false になるため、チャート上での
    // スクロール/ズームはチャート側だけに効き、キャンバスは動かない。
    if bg.hovered() {
        if let Some(ptr) = ui.ctx().input(|i| i.pointer.hover_pos()) {
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
            // 画面矩形へのクランプを無効化（無限キャンバス上のどこにでも配置可能にする）。
            // constrain が true だと、レイヤー変換適用前の world 座標が画面外のアイテムが
            // 画面内へクランプされ、一定範囲より外へ移動できなくなる。
            .constrain(false)
            .show(ui.ctx(), |ui| {
                // アイテムの枠（ワールド座標）。fixed_pos によりレイヤー原点が (item.x, item.y)。
                let item_rect = egui::Rect::from_min_size(
                    egui::pos2(item.x, item.y),
                    egui::vec2(item.w, item.h),
                );
                // 枠とビューポートの交差でクリップ。これにより内部コンテンツ（チャート/
                // ツールバー）が枠外へはみ出して描画されない。
                let viewport = to_screen.inverse().mul_rect(area);
                ui.set_clip_rect(item_rect.intersect(viewport));

                // 枠領域を確保（Area のサイズ・応答領域＝クリックで最前面化、背景パンの遮蔽）
                ui.allocate_rect(item_rect, egui::Sense::click());
                // 背景・枠線
                ui.painter()
                    .rect_filled(item_rect, 4.0, crate::theme::CENTRAL_BG);
                ui.painter().rect_stroke(
                    item_rect,
                    4.0,
                    egui::Stroke::new(1.0, crate::theme::BORDER_COLOR),
                );

                // item_rect に収まる子 UI（grid と同じ new_child 方式でサイズを拘束）
                let mut content_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(item_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );

                let csv_available = match &item.content {
                    PanelItem::Chart(chart_id) => {
                        crate::io::csv_export::has_csv_data(chart_id, app_state, widgets)
                    }
                    PanelItem::TrialTable => false,
                };
                let tb = show_canvas_item_toolbar(
                    &mut content_ui,
                    &item.content,
                    item.content.label(),
                    csv_available,
                );

                // grid と同じ順序: handle_toolbar_action(キャプチャ要求の登録) → body 描画。
                let had_no_capture = widgets.capture.pending_capture.is_none();
                let ctx = content_ui.ctx().clone();
                handle_toolbar_action(
                    &ctx,
                    &tb.action,
                    app_state.help_language,
                    widgets,
                    app_state,
                    tx,
                );
                let body_rect = render_panel_item_body(
                    &mut content_ui,
                    app_state,
                    widgets,
                    &item.content,
                    item.id,
                    tx,
                );
                // キャプチャ矩形は画面座標で記録する（レイヤー変換を適用）。
                if had_no_capture && widgets.capture.pending_capture.is_some() {
                    widgets.capture.pending_capture_rect = Some(to_screen.mul_rect(body_rect));
                }

                // リサイズハンドル（item_rect 右下）。content の後に外側 ui へ登録して最前面に。
                let handle_rect = egui::Rect::from_min_size(
                    item_rect.max - egui::vec2(RESIZE_HANDLE, RESIZE_HANDLE),
                    egui::vec2(RESIZE_HANDLE, RESIZE_HANDLE),
                );
                let handle_id = egui::Id::new("canvas_resize").with(item.id);
                let rh = ui.interact(handle_rect, handle_id, egui::Sense::click_and_drag());
                let active = rh.hovered() || rh.dragged();
                if active {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                }
                // 左上 (item_rect.min) を固定し、ポインタ位置（ワールド座標）から絶対サイズを算出する。
                // ドラッグ開始時にハンドル内のつかみ位置オフセットを記録し、開始時の飛びを防ぐ。
                let mut resize_to: Option<(f32, f32)> = None;
                if rh.drag_started() {
                    if let Some(p) = rh.interact_pointer_pos() {
                        let grab = p - item_rect.max; // 角からのオフセット
                        ui.ctx().data_mut(|d| d.insert_temp(handle_id, grab));
                    }
                }
                if rh.dragged() {
                    if let Some(p) = rh.interact_pointer_pos() {
                        let grab: egui::Vec2 =
                            ui.ctx().data(|d| d.get_temp(handle_id)).unwrap_or_default();
                        let corner = p - grab;
                        let new_w = (corner.x - item_rect.min.x).max(MIN_W);
                        let new_h = (corner.y - item_rect.min.y).max(MIN_H);
                        resize_to = Some((new_w, new_h));
                    }
                }
                // 右下隅にグリップ（斜線）を常時描画し、リサイズ可能だと一目で分かるようにする。
                // ホバー/ドラッグ時はアクセント色で強調する。
                let grip_color = if active {
                    crate::theme::ACCENT_BLUE
                } else {
                    egui::Color32::from_gray(160)
                };
                let br = item_rect.max;
                for i in 0..3 {
                    let off = 3.0 + i as f32 * 4.0;
                    ui.painter().line_segment(
                        [
                            egui::pos2(br.x - off, br.y - 2.0),
                            egui::pos2(br.x - 2.0, br.y - off),
                        ],
                        egui::Stroke::new(1.5, grip_color),
                    );
                }
                CanvasItemOutput {
                    move_delta: tb.move_delta,
                    resize_to,
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
        if let Some((w, h)) = out.resize_to {
            actions.push(CanvasAction::Resize(item.id, w, h));
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
            CanvasAction::Resize(id, w, h) => {
                if let Some(it) = layout.canvas.items.iter_mut().find(|i| i.id == id) {
                    // 左上 (x, y) は変更せず、サイズのみ更新する。
                    it.w = w;
                    it.h = h;
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

/// アイテム上部バーのボタン（⋯ / ✕）にトップバーと同じ水色スタイルを適用する。
fn apply_item_button_visuals(vis: &mut egui::Visuals) {
    use crate::theme::{TOOLBAR_BG, TOOLBAR_BTN_ACTIVE, TOOLBAR_BTN_HOVER, TOOLBAR_TEXT};
    vis.override_text_color = Some(TOOLBAR_TEXT);
    for w in [&mut vis.widgets.inactive, &mut vis.widgets.open] {
        w.weak_bg_fill = TOOLBAR_BG;
        w.bg_fill = TOOLBAR_BG;
    }
    vis.widgets.hovered.weak_bg_fill = TOOLBAR_BTN_HOVER;
    vis.widgets.hovered.bg_fill = TOOLBAR_BTN_HOVER;
    vis.widgets.active.weak_bg_fill = TOOLBAR_BTN_ACTIVE;
    vis.widgets.active.bg_fill = TOOLBAR_BTN_ACTIVE;
}

/// キャンバスアイテム上部のバーを描画する。
/// バー自体のドラッグで移動量を返し、`⋯`/`✕` は grid と同じ `CellToolbarAction` を返す。
/// ボタンはトップバーと同じ水色で表示してバー背景と区別できるようにする。
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

    // バー矩形（内側マージン 6x4 を含めた高さ）。
    let bar_h = DRAG_HANDLE_HEIGHT + 8.0;
    let bar_rect =
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), bar_h));

    // バー領域を確保（body をバーの下へ配置）し、背景・枠線を描画。
    ui.allocate_rect(bar_rect, egui::Sense::hover());
    ui.painter()
        .rect_filled(bar_rect, 0.0, crate::theme::CELL_TOOLBAR_BG);
    ui.painter().rect_stroke(
        bar_rect,
        0.0,
        egui::Stroke::new(1.0, crate::theme::BORDER_COLOR),
    );

    // バーのドラッグで移動（ボタンより先に登録 → ボタンのクリックが優先される）。
    let drag_resp = ui.interact(
        bar_rect,
        egui::Id::new("canvas_item_bar"),
        egui::Sense::click_and_drag(),
    );
    if drag_resp.dragged() {
        result.move_delta = drag_resp.drag_delta();
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if drag_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    // バー内コンテンツ（タイトル＋ボタン）をバーの上に描画。
    let inner_rect = bar_rect.shrink2(egui::vec2(6.0, 4.0));
    let mut bar_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    bar_ui.strong(title);

    // 右寄せでボタン群（✕ / ⋯）を水色で配置。
    bar_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        apply_item_button_visuals(ui.visuals_mut());

        let close_resp = ui.add_sized(
            egui::vec2(CLOSE_BUTTON_SIZE + 8.0, DRAG_HANDLE_HEIGHT),
            egui::Button::new(
                egui::RichText::new("✕")
                    .small()
                    .color(crate::theme::TOOLBAR_TEXT),
            ),
        );
        if close_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if close_resp.clicked() {
            result.action = CellToolbarAction::Close;
        }

        ui.add_space(4.0);

        if let Some(a) = crate::ui::grid_canvas::show_chart_menu_button(ui, item, csv_available) {
            result.action = a;
        }
    });

    result
}
