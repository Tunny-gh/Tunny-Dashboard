use std::collections::HashMap;
use std::sync::mpsc;

use egui::emath::TSTransform;

use crate::state::app_state::AppState;
use crate::state::layout_state::{DragPayload, LayoutState, PanelItem};
use crate::state::messages::AppMessage;
use crate::theme::chart_colors::COLOR_SELECTION_HIGHLIGHT;
use crate::ui::canvas::minimap::{show_minimap, BTN_MARGIN, BTN_SIZE};
use crate::ui::canvas::viewport::{fit_view, items_bbox, ZOOM_MAX, ZOOM_MIN};
use crate::ui::chart_cell::{
    handle_toolbar_action, render_panel_item_body, CellToolbarAction, CLOSE_BUTTON_SIZE,
    DRAG_HANDLE_HEIGHT,
};
use crate::ui::widget_states::{CaptureDest, WidgetStates};

/// Dot grid spacing in world coordinates
const GRID_WORLD: f32 = 40.0;
/// Default size for new widgets (world coordinates).
/// Sized to fit the chart's top toolbar while not being too large when placed.
const DEFAULT_W: f32 = 640.0;
const DEFAULT_H: f32 = 376.0;
/// Minimum size when resizing
const MIN_W: f32 = 160.0;
const MIN_H: f32 = 120.0;
/// Side length of the resize handle
const RESIZE_HANDLE: f32 = 14.0;

/// Per-item frame result (an action applied in bulk after borrows are released)
enum CanvasAction {
    Move(u64, egui::Vec2),
    /// Sets an absolute size (w, h) while keeping the top-left corner fixed
    Resize(u64, f32, f32),
    Remove(u64),
    /// Maximizes the widget's display via a double-click on the bar
    Maximize(PanelItem),
}

/// Draws the freely-placed canvas.
///
/// Widgets can be freely placed on an infinite plane; drag empty space to pan,
/// scroll/pinch to zoom. Each widget is drawn as an `egui::Area`, and applying a
/// `TSTransform` to the layer uniformly scales everything, including charts and
/// text (the same technique as egui's official pan_zoom demo).
/// `widgets` holds state shared across the whole canvas (color cache, capture).
/// `item_widgets` keeps independent chart UI state per item (keyed by item.id),
/// so that placing the same widget multiple times doesn't mix up settings
/// (objective selection, toggles, etc.).
pub fn show_canvas_view(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    layout: &mut LayoutState,
    widgets: &mut WidgetStates,
    item_widgets: &mut HashMap<u64, WidgetStates>,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let area = ui.available_rect_before_wrap();
    ui.set_clip_rect(area);

    // World-to-screen transform. Combine pan/zoom with an offset of area.min.
    let offset = TSTransform::from_translation(area.min.to_vec2());
    let mut to_screen = offset
        * TSTransform::new(
            egui::vec2(layout.canvas.pan_x, layout.canvas.pan_y),
            layout.canvas.zoom,
        );

    // ── Background interaction (pan / zoom / double-click to reset) ──────────────
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
        // Fit to items if any exist, otherwise reset to the default (pan=0, zoom=1)
        if let Some(bbox) = items_bbox(&layout.canvas.items) {
            let (zoom, pan) = fit_view(area, bbox);
            to_screen = offset * TSTransform::new(pan, zoom);
        } else {
            to_screen = offset; // reset to pan=0, zoom=1
        }
    }
    // Apply scroll/zoom to the canvas only while hovering the background (empty area).
    // bg.hovered() becomes false when occluded by an item (Area), so scroll/zoom
    // over a chart affects only that chart and doesn't move the canvas.
    if bg.hovered() {
        if let Some(ptr) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Regular scroll → pan
            let scroll = ui.ctx().input(|i| i.smooth_scroll_delta);
            if scroll != egui::Vec2::ZERO {
                to_screen.translation += scroll;
            }
            // Pinch / Ctrl+scroll → zoom anchored at the pointer
            let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
            if zoom_delta != 1.0 {
                let current = to_screen.scaling;
                let target = (current * zoom_delta).clamp(ZOOM_MIN, ZOOM_MAX);
                let eff = if current > 0.0 { target / current } else { 1.0 };
                if (eff - 1.0).abs() > f32::EPSILON {
                    let pil = to_screen.inverse() * ptr; // Pointer's world coordinates
                    to_screen = to_screen
                        * TSTransform::from_translation(pil.to_vec2())
                        * TSTransform::from_scaling(eff)
                        * TSTransform::from_translation(-pil.to_vec2());
                }
            }
        }
    }

    let zoom = to_screen.scaling;

    // ── Background fill + dot grid (moves with pan/zoom) ─────────────
    let painter = ui.painter().clone();
    painter.rect_filled(area, 0.0, crate::theme::CANVAS_BG());
    {
        let step = (GRID_WORLD * zoom).max(8.0);
        let origin = to_screen * egui::pos2(0.0, 0.0); // Screen position of the world origin
        let start_x = area.left() - (area.left() - origin.x).rem_euclid(step);
        let start_y = area.top() - (area.top() - origin.y).rem_euclid(step);
        let r = (1.2 * zoom).clamp(0.6, 2.2);
        let color = crate::theme::CANVAS_DOT();
        // Tens of thousands of circle_filled calls are expensive to tessellate,
        // so batch the visible points into a single Mesh (each point drawn as a
        // small square made of 2 triangles).
        let mut mesh = egui::Mesh::default();
        let mut gx = start_x;
        while gx <= area.right() {
            let mut gy = start_y;
            while gy <= area.bottom() {
                let idx = mesh.vertices.len() as u32;
                let rect =
                    egui::Rect::from_center_size(egui::pos2(gx, gy), egui::vec2(r * 2.0, r * 2.0));
                mesh.colored_vertex(rect.left_top(), color);
                mesh.colored_vertex(rect.right_top(), color);
                mesh.colored_vertex(rect.right_bottom(), color);
                mesh.colored_vertex(rect.left_bottom(), color);
                mesh.add_triangle(idx, idx + 1, idx + 2);
                mesh.add_triangle(idx, idx + 2, idx + 3);
                gy += step;
            }
            gx += step;
        }
        painter.add(egui::Shape::mesh(mesh));
    }

    // ── Item rendering (z-order is managed automatically by egui's Area) ────────────────
    let mut actions: Vec<CanvasAction> = Vec::new();

    for item in &layout.canvas.items {
        // Per-item dedicated WidgetStates (independent UI state).
        let iw = item_widgets.entry(item.id).or_default();
        // Capture requests are consumed globally (by the screenshot handling), so
        // receive it inside the closure and propagate it to the global side after show.
        let mut item_capture: Option<(PanelItem, egui::Rect, CaptureDest)> = None;

        let ir = egui::Area::new(egui::Id::new("canvas_item").with(item.id))
            .order(egui::Order::Middle)
            .fixed_pos(egui::pos2(item.x, item.y))
            // Disable clamping to the screen rect (allow placement anywhere on
            // the infinite canvas). If constrain were true, items whose world
            // coordinates (before the layer transform) fall outside the screen
            // would get clamped back onscreen, preventing movement beyond a
            // certain range.
            .constrain(false)
            .show(ui.ctx(), |ui| {
                // The item's frame (world coordinates). fixed_pos makes the layer origin (item.x, item.y).
                let item_rect = egui::Rect::from_min_size(
                    egui::pos2(item.x, item.y),
                    egui::vec2(item.w, item.h),
                );
                // Clip to the intersection of the frame and the viewport. This
                // keeps internal content (chart/toolbar) from being drawn
                // outside the frame.
                let viewport = to_screen.inverse().mul_rect(area);
                ui.set_clip_rect(item_rect.intersect(viewport));

                // Allocate the frame area (Area's size / response region = brings to front on click, occludes background panning)
                ui.allocate_rect(item_rect, egui::Sense::click());
                // Background / border
                ui.painter()
                    .rect_filled(item_rect, 4.0, crate::theme::CENTRAL_BG());
                ui.painter().rect_stroke(
                    item_rect,
                    4.0,
                    egui::Stroke::new(1.0, crate::theme::BORDER_COLOR()),
                    egui::StrokeKind::Inside,
                );

                // Child UI constrained to item_rect (same new_child approach used by grid to constrain size)
                let mut content_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(item_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );

                let csv_available = match &item.content {
                    PanelItem::Chart(chart_id) => {
                        crate::io::csv_export::has_csv_data(chart_id, app_state, iw)
                    }
                    PanelItem::TrialTable => {
                        crate::io::csv_export::has_trial_table_csv(app_state, iw)
                    }
                };
                let (move_delta, tb_action) = show_canvas_item_toolbar(
                    &mut content_ui,
                    item.id,
                    &item.content,
                    item.content.label(),
                    item.content.subtitle(),
                    csv_available,
                );

                // Same order as grid: handle_toolbar_action (registers capture requests) → draw body.
                let had_no_capture = iw.capture.pending_capture.is_none();
                let ctx = content_ui.ctx().clone();
                handle_toolbar_action(&ctx, &tb_action, app_state.help_language, iw, app_state, tx);
                let body_rect = render_panel_item_body(
                    &mut content_ui,
                    app_state,
                    iw,
                    &item.content,
                    item.id,
                    tx,
                );
                // If a capture request was newly raised, record the screen-coordinate
                // rect to hand off to the global side (take it out of the per-item iw and clear it).
                if had_no_capture && iw.capture.pending_capture.is_some() {
                    if let Some(pc) = iw.capture.pending_capture.take() {
                        item_capture = Some((
                            pc,
                            to_screen.mul_rect(body_rect),
                            iw.capture.pending_capture_dest,
                        ));
                    }
                }

                // Resize handle (bottom-right of item_rect). Registered on the outer ui after content so it's on top.
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
                // Keep the top-left (item_rect.min) fixed and compute the absolute
                // size from the pointer position (world coordinates). Record the
                // grab offset within the handle at drag start to avoid a jump at
                // the beginning.
                let mut resize_to: Option<(f32, f32)> = None;
                if rh.drag_started() {
                    if let Some(p) = rh.interact_pointer_pos() {
                        let grab = p - item_rect.max; // Offset from the corner
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
                // Always draw a grip (diagonal lines) in the bottom-right corner
                // so it's clear at a glance that resizing is possible.
                // Highlight with the accent color while hovering/dragging.
                let grip_color = if active {
                    crate::theme::ACCENT_BLUE()
                } else {
                    crate::theme::chart_colors::COLOR_GRID_STROKE()
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

                // Push interaction results directly onto actions (applied in bulk after borrows are released).
                if move_delta != egui::Vec2::ZERO {
                    actions.push(CanvasAction::Move(item.id, move_delta));
                }
                if let Some((w, h)) = resize_to {
                    actions.push(CanvasAction::Resize(item.id, w, h));
                }
                if matches!(tb_action, CellToolbarAction::Close) {
                    actions.push(CanvasAction::Remove(item.id));
                }
                if let CellToolbarAction::Maximize(target) = &tb_action {
                    actions.push(CanvasAction::Maximize(target.clone()));
                }
            });

        // Apply the transform to this item's layer (scales text along with everything else)
        ui.ctx()
            .set_transform_layer(ir.response.layer_id, to_screen);

        // Propagate capture requests raised by the item to the global side
        // (screenshot handling reads from app.widget_states.capture).
        if let Some((pc, rect, dest)) = item_capture {
            widgets.capture.pending_capture = Some(pc);
            widgets.capture.pending_capture_rect = Some(rect);
            widgets.capture.pending_capture_dest = dest;
        }
    }

    // ── New drop from the right panel (judged by area, independent of layer) ──────────
    let mut pending_add: Option<(PanelItem, egui::Pos2)> = None;
    if let Some(payload) = egui::DragAndDrop::payload::<DragPayload>(ui.ctx()) {
        let new_item = payload.item();
        // Highlight indicating a drop is possible
        ui.painter().rect_stroke(
            area,
            0.0,
            egui::Stroke::new(2.0, COLOR_SELECTION_HIGHLIGHT()),
            egui::StrokeKind::Inside,
        );
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(sp) = ui.ctx().pointer_interact_pos() {
                if area.contains(sp) {
                    let wp = to_screen.inverse() * sp;
                    pending_add = Some((new_item.clone(), wp));
                }
            }
        }
    }

    // ── Apply collected actions ─────────────────────────────────────────────
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
                    // Keep the top-left (x, y) unchanged and update only the size.
                    it.w = w;
                    it.h = h;
                }
            }
            CanvasAction::Remove(id) => layout.canvas.remove(id),
            CanvasAction::Maximize(target) => widgets.maximized_item = Some(target),
        }
    }
    if let Some((it, wp)) = pending_add {
        layout.canvas.add(it, wp.x, wp.y, DEFAULT_W, DEFAULT_H);
        egui::DragAndDrop::clear_payload(ui.ctx());
    }

    // Discard dedicated WidgetStates for removed items to prevent memory leaks.
    item_widgets.retain(|id, _| layout.canvas.items.iter().any(|it| it.id == *id));

    // ── Minimap overlay (above the fit button) ──────────────────────────────
    // Because a Foreground Area occludes background clicks, show_minimap is
    // called after the item loop and before writing values back.
    show_minimap(ui, area, &mut to_screen, offset, &layout.canvas.items);

    // ── Fit button (bottom-right overlay) ────────────────────────────────────
    // Drawing it after the item loop keeps it always shown in front of the charts.
    // BTN_SIZE / BTN_MARGIN are defined as pub(crate) constants in the minimap
    // module, and are also referenced by the minimap's placement calculations.
    let btn_pos = egui::pos2(
        area.right() - BTN_MARGIN - BTN_SIZE,
        area.bottom() - BTN_MARGIN - BTN_SIZE,
    );
    let fit_clicked = egui::Area::new(egui::Id::new("canvas_fit_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(btn_pos)
        .show(ui.ctx(), |ui| {
            apply_item_button_visuals(ui.visuals_mut());
            let resp = ui
                .add_sized(
                    egui::vec2(BTN_SIZE, BTN_SIZE),
                    egui::Button::new(egui::RichText::new("⛶").color(crate::theme::TOOLBAR_TEXT())),
                )
                .on_hover_text("Fit view to charts");
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            resp.clicked()
        })
        .inner;

    if fit_clicked {
        if let Some(bbox) = items_bbox(&layout.canvas.items) {
            let (zoom, pan) = fit_view(area, bbox);
            to_screen = offset * TSTransform::new(pan, zoom);
        }
    }

    // ── Write the viewport transform back (pan is unbounded = infinite canvas, zoom is clamped) ──
    let logical = offset.inverse() * to_screen;
    layout.canvas.pan_x = logical.translation.x;
    layout.canvas.pan_y = logical.translation.y;
    layout.canvas.zoom = logical.scaling.clamp(ZOOM_MIN, ZOOM_MAX);
}

/// Applies the same light-blue style used by the top bar to the item's top bar buttons (… / ×).
fn apply_item_button_visuals(vis: &mut egui::Visuals) {
    use crate::theme::{TOOLBAR_BG, TOOLBAR_BTN_ACTIVE, TOOLBAR_BTN_HOVER, TOOLBAR_TEXT};
    vis.override_text_color = Some(TOOLBAR_TEXT());
    for w in [&mut vis.widgets.inactive, &mut vis.widgets.open] {
        w.weak_bg_fill = TOOLBAR_BG();
        w.bg_fill = TOOLBAR_BG();
    }
    vis.widgets.hovered.weak_bg_fill = TOOLBAR_BTN_HOVER();
    vis.widgets.hovered.bg_fill = TOOLBAR_BTN_HOVER();
    vis.widgets.active.weak_bg_fill = TOOLBAR_BTN_ACTIVE();
    vis.widgets.active.bg_fill = TOOLBAR_BTN_ACTIVE();
}

/// Draws the bar at the top of a canvas item.
/// Returns the movement delta from dragging the bar itself; `…`/`×` return the
/// same `CellToolbarAction` as grid.
/// Buttons are shown in the same light blue as the top bar so they stand out
/// from the bar background.
fn show_canvas_item_toolbar(
    ui: &mut egui::Ui,
    id: u64,
    item: &PanelItem,
    title: &str,
    subtitle: Option<&'static str>,
    csv_available: bool,
) -> (egui::Vec2, CellToolbarAction) {
    let mut move_delta = egui::Vec2::ZERO;
    let mut action = CellToolbarAction::None;

    // The bar's rect (height includes the inner margin of 6x4).
    let bar_h = DRAG_HANDLE_HEIGHT + 8.0;
    let bar_rect =
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), bar_h));

    // Allocate the bar area (positions body below the bar) and draw the background/border.
    ui.allocate_rect(bar_rect, egui::Sense::hover());
    ui.painter()
        .rect_filled(bar_rect, 0.0, crate::theme::CELL_TOOLBAR_BG());
    ui.painter().rect_stroke(
        bar_rect,
        0.0,
        egui::Stroke::new(1.0, crate::theme::BORDER_COLOR()),
        egui::StrokeKind::Inside,
    );

    // Move via dragging the bar (registered before the buttons → button clicks take priority).
    let drag_resp = ui.interact(
        bar_rect,
        egui::Id::new("canvas_item_bar").with(id),
        egui::Sense::click_and_drag(),
    );
    if drag_resp.dragged() {
        move_delta = drag_resp.drag_delta();
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if drag_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    // Maximize on double-click of the bar (excluding buttons, which are
    // registered later and sit on top in z-order).
    if drag_resp.double_clicked() {
        action = CellToolbarAction::Maximize(item.clone());
    }

    // Draw the bar's content (title + buttons) on top of the bar.
    let inner_rect = bar_rect.shrink2(egui::vec2(6.0, 4.0));
    let mut bar_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    bar_ui.strong(title);
    // Show chart-specific supplementary text (legend) next to the title in a faint font
    if let Some(subtitle) = subtitle {
        bar_ui.add_space(8.0);
        bar_ui.label(egui::RichText::new(subtitle).weak().size(11.0));
    }

    // Right-align the button group (× / …) in light blue.
    bar_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        apply_item_button_visuals(ui.visuals_mut());

        let close_resp = ui
            .add_sized(
                egui::vec2(CLOSE_BUTTON_SIZE + 8.0, DRAG_HANDLE_HEIGHT),
                egui::Button::new(
                    egui::RichText::new("×")
                        .small()
                        .color(crate::theme::TOOLBAR_TEXT()),
                ),
            )
            .on_hover_text("Close");
        if close_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if close_resp.clicked() {
            action = CellToolbarAction::Close;
        }

        ui.add_space(4.0);

        if let Some(a) = crate::ui::chart_cell::show_chart_menu_button(ui, item, csv_available) {
            action = a;
        }
    });

    (move_delta, action)
}
