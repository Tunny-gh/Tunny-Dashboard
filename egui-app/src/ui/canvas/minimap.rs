//! Canvas minimap.
//!
//! Keeps the pure computation logic ([`compute_minimap_layout`],
//! [`world_to_map`], [`map_to_world`]) and drawing/interaction
//! ([`show_minimap`]) in the same module.
//! The computation part doesn't depend on the egui drawing context, so it's unit-testable.

use egui::emath::TSTransform;

use crate::state::layout_state::CanvasItem;
use crate::ui::canvas::viewport::{items_bbox, pan_to_center};

// ─────────────────────────────────────────────────────────────────────────────
// Layout constants
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum minimap width (screen pixels)
const MAX_W: f32 = 100.0;
/// Maximum minimap height (screen pixels)
const MAX_H: f32 = 75.0;
/// Margin from the area edge (screen pixels)
const MARGIN: f32 = 12.0;

/// Fit button size (same value as BTN_SIZE in canvas_view.rs)
pub(crate) const BTN_SIZE: f32 = 28.0;
/// Fit button margin (same value as BTN_MARGIN in canvas_view.rs)
pub(crate) const BTN_MARGIN: f32 = 12.0;

/// Area size (screen px) occupied by the fit button (bottom-right).
/// Referenced by the layout side to exclude this corner from the right
/// panel's hover open/close detection.
pub(crate) fn fit_button_footprint() -> egui::Vec2 {
    let s = BTN_SIZE + BTN_MARGIN * 2.0;
    egui::vec2(s, s)
}

/// Maximum area size (screen px) the minimap (bottom-left) can occupy.
/// Referenced by the layout side to exclude this corner from the left
/// panel's hover open/close detection.
pub(crate) fn minimap_footprint() -> egui::Vec2 {
    egui::vec2(MAX_W + MARGIN * 2.0, MAX_H + MARGIN * 2.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure computation — unit-testable
// ─────────────────────────────────────────────────────────────────────────────

/// The minimap's screen layout info.
///
/// - `map_rect` : the screen rect occupied by the minimap
/// - `scale`    : ratio of 1 world-coordinate unit to px within the map
/// - `world_origin` : the world coordinate corresponding to the minimap's top-left
#[derive(Debug, Clone)]
pub(crate) struct MinimapLayout {
    pub map_rect: egui::Rect,
    pub scale: f32,
    pub world_origin: egui::Pos2,
}

/// Computes the minimap layout (pure function).
///
/// # Arguments
/// - `area`         : the screen rect of the entire canvas
/// - `world_bounds` : the world rect the minimap represents (items bbox union
///   viewport, expanded by 5%)
///
/// # Placement
/// Placed in the bottom-left corner (doesn't interfere with the fit button,
/// which is in the bottom-right corner).
///
/// # Degenerate guard
/// If `world_bounds`'s width/height is 0 or less, treats it as scale = 1.0 /
/// map size = MAX.
pub(crate) fn compute_minimap_layout(area: egui::Rect, world_bounds: egui::Rect) -> MinimapLayout {
    // Scale: world -> map
    let scale = if world_bounds.width() > 0.0 && world_bounds.height() > 0.0 {
        let sx = MAX_W / world_bounds.width();
        let sy = MAX_H / world_bounds.height();
        sx.min(sy)
    } else {
        1.0
    };

    // Actual map size (preserves aspect ratio and stays within MAX)
    let map_w = if world_bounds.width() > 0.0 {
        (world_bounds.width() * scale).min(MAX_W)
    } else {
        MAX_W
    };
    let map_h = if world_bounds.height() > 0.0 {
        (world_bounds.height() * scale).min(MAX_H)
    } else {
        MAX_H
    };

    // Placed in the bottom-left corner.
    let map_bottom = area.bottom() - MARGIN;
    let map_left = area.left() + MARGIN;

    let map_min = egui::pos2(map_left, map_bottom - map_h);
    let map_rect = egui::Rect::from_min_size(map_min, egui::vec2(map_w, map_h));

    MinimapLayout {
        map_rect,
        scale,
        world_origin: world_bounds.min,
    }
}

/// Converts a world coordinate to a screen coordinate within the minimap.
pub(crate) fn world_to_map(layout: &MinimapLayout, p: egui::Pos2) -> egui::Pos2 {
    layout.map_rect.min + (p - layout.world_origin) * layout.scale
}

/// Converts a screen coordinate within the minimap to a world coordinate (the inverse of [`world_to_map`]).
pub(crate) fn map_to_world(layout: &MinimapLayout, p: egui::Pos2) -> egui::Pos2 {
    layout.world_origin + (p - layout.map_rect.min) / layout.scale
}

// ─────────────────────────────────────────────────────────────────────────────
// Drawing / interaction
// ─────────────────────────────────────────────────────────────────────────────

/// Draws the minimap overlay and handles viewport panning via dragging.
///
/// Draws nothing when items is empty.
/// Updating `to_screen` directly reflects the change into the caller's viewport transform.
///
/// # Overlay implementation
/// Uses an [`egui::Area`] with [`egui::Order::Foreground`], so it's always
/// drawn in front of the canvas background/charts, and also blocks background interaction.
pub(crate) fn show_minimap(
    ui: &mut egui::Ui,
    area: egui::Rect,
    to_screen: &mut TSTransform,
    offset: TSTransform,
    items: &[CanvasItem],
) {
    if items.is_empty() {
        return;
    }

    // ── World bounds computation ─────────────────────────────────────────
    let items_bb = match items_bbox(items) {
        Some(b) => b,
        None => return,
    };
    // Convert the current viewport into world coordinates
    let viewport_world = to_screen.inverse().mul_rect(area);
    // Add a 5% margin to the union of the items bbox and the viewport
    let union = items_bb.union(viewport_world);
    let world_bounds = union.expand2(union.size() * 0.05);

    let layout = compute_minimap_layout(area, world_bounds);

    // ── Interaction (registered before Foreground to separate from clicks outside the minimap) ──
    // Use the minimap's top-left as the Area's fixed_pos.
    let minimap_resp = egui::Area::new(egui::Id::new("canvas_minimap_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(layout.map_rect.min)
        .show(ui.ctx(), |ui| {
            let map_size = layout.map_rect.size();

            // Reserve the sense region over the entire map (hover detection + click/drag)
            let (resp, painter) = ui.allocate_painter(map_size, egui::Sense::click_and_drag());

            let mr = resp.rect; // The actual rect within the Area (should match map_rect)

            // ── Background panel ──────────────────────────────────────
            painter.rect(
                mr,
                4.0,
                egui::Color32::from_black_alpha(170),
                egui::Stroke::new(1.0, crate::theme::BORDER_COLOR()),
                egui::StrokeKind::Inside,
            );

            // ── Item rects ─────────────────────────────────────────────
            for item in items {
                let item_world_rect = egui::Rect::from_min_size(
                    egui::pos2(item.x, item.y),
                    egui::vec2(item.w, item.h),
                );
                // Rect within the map (adjust the offset since it's based on layout.map_rect.min)
                let map_min = world_to_map(&layout, item_world_rect.min)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let map_max = world_to_map(&layout, item_world_rect.max)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let item_map_rect = egui::Rect::from_min_max(map_min, map_max);
                // Clip to within the map (don't draw if the intersection is empty)
                let clipped = item_map_rect.intersect(mr);
                if clipped.is_positive() {
                    painter.rect(
                        clipped,
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(120, 120, 120, 180),
                        egui::Stroke::new(0.5, egui::Color32::from_gray(160)),
                        egui::StrokeKind::Inside,
                    );
                }
            }

            // ── Viewport rect ──────────────────────────────────────────
            {
                let vp_min = world_to_map(&layout, viewport_world.min)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let vp_max = world_to_map(&layout, viewport_world.max)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let vp_map_rect = egui::Rect::from_min_max(vp_min, vp_max).intersect(mr);
                let accent = crate::theme::ACCENT_BLUE();
                painter.rect(
                    vp_map_rect,
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 20),
                    egui::Stroke::new(1.5, accent),
                    egui::StrokeKind::Inside,
                );
            }

            // ── Cursor ─────────────────────────────────────────────────
            if resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            resp
        });

    // ── Pan the viewport on drag/click ──────────────────────────────────
    let resp = minimap_resp.inner;
    let do_pan = resp.clicked() || resp.dragged();
    if do_pan {
        if let Some(ptr) = resp.interact_pointer_pos() {
            // Pointer within the Area -> actual screen coordinate (add the map_rect.min offset)
            let map_ptr = ptr + layout.map_rect.min.to_vec2() - resp.rect.min.to_vec2();
            let world_center = map_to_world(&layout, map_ptr);
            let zoom = to_screen.scaling;
            let pan = pan_to_center(area, zoom, world_center);
            *to_screen = offset * TSTransform::new(pan, zoom);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_area() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0))
    }

    fn make_world_bounds(w: f32, h: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, h))
    }

    // ── compute_minimap_layout ────────────────────────────────────────────

    /// When width is the constraining dimension, the map width doesn't exceed MAX_W
    #[test]
    fn layout_respects_max_width() {
        let area = make_area();
        let wb = make_world_bounds(10000.0, 100.0);
        let layout = compute_minimap_layout(area, wb);
        assert!(
            layout.map_rect.width() <= MAX_W + f32::EPSILON,
            "map_w={} exceeds MAX_W={}",
            layout.map_rect.width(),
            MAX_W
        );
    }

    /// When height is the constraining dimension, the map height doesn't exceed MAX_H
    #[test]
    fn layout_respects_max_height() {
        let area = make_area();
        let wb = make_world_bounds(100.0, 10000.0);
        let layout = compute_minimap_layout(area, wb);
        assert!(
            layout.map_rect.height() <= MAX_H + f32::EPSILON,
            "map_h={} exceeds MAX_H={}",
            layout.map_rect.height(),
            MAX_H
        );
    }

    /// The aspect ratio is preserved (tolerance 1%)
    #[test]
    fn layout_preserves_aspect_ratio() {
        let area = make_area();
        let w = 800.0_f32;
        let h = 400.0_f32;
        let wb = make_world_bounds(w, h);
        let layout = compute_minimap_layout(area, wb);
        let expected_ratio = w / h;
        let actual_ratio = layout.map_rect.width() / layout.map_rect.height();
        assert!(
            (actual_ratio - expected_ratio).abs() < expected_ratio * 0.01,
            "aspect ratio mismatch: expected {expected_ratio:.3}, got {actual_ratio:.3}"
        );
    }

    /// The minimap is placed in the bottom-left corner (a different corner than the fit button's bottom-right)
    #[test]
    fn layout_anchored_bottom_left() {
        let area = make_area();
        let wb = make_world_bounds(500.0, 300.0);
        let layout = compute_minimap_layout(area, wb);
        assert!(
            (layout.map_rect.left() - (area.left() + MARGIN)).abs() < f32::EPSILON,
            "minimap left={} should be area.left()+MARGIN={}",
            layout.map_rect.left(),
            area.left() + MARGIN
        );
        assert!(
            (layout.map_rect.bottom() - (area.bottom() - MARGIN)).abs() < f32::EPSILON,
            "minimap bottom={} should be area.bottom()-MARGIN={}",
            layout.map_rect.bottom(),
            area.bottom() - MARGIN
        );
    }

    /// Returns finite values even for degenerate bounds (width 0, height 0)
    #[test]
    fn layout_degenerate_bounds_finite() {
        let area = make_area();
        let wb = make_world_bounds(0.0, 0.0);
        let layout = compute_minimap_layout(area, wb);
        assert!(layout.scale.is_finite(), "scale must be finite");
        assert!(layout.map_rect.min.x.is_finite());
        assert!(layout.map_rect.min.y.is_finite());
    }

    // ── world_to_map / map_to_world round-trip conversion ─────────────────

    /// world -> map -> world returns to the original coordinates (round-trip precision within 0.01 px)
    #[test]
    fn world_map_roundtrip() {
        let area = make_area();
        let wb = make_world_bounds(2000.0, 1000.0);
        let layout = compute_minimap_layout(area, wb);

        let pts = [
            egui::pos2(0.0, 0.0),
            egui::pos2(1000.0, 500.0),
            egui::pos2(2000.0, 1000.0),
            egui::pos2(-100.0, 200.0),
        ];
        for p in pts {
            let mp = world_to_map(&layout, p);
            let back = map_to_world(&layout, mp);
            assert!(
                (back.x - p.x).abs() < 0.01 && (back.y - p.y).abs() < 0.01,
                "roundtrip failed for {p:?}: got {back:?}"
            );
        }
    }

    /// map -> world -> map returns to the original map coordinates
    #[test]
    fn map_world_roundtrip() {
        let area = make_area();
        let wb = make_world_bounds(1500.0, 600.0);
        let layout = compute_minimap_layout(area, wb);

        let map_pts = [
            layout.map_rect.min,
            layout.map_rect.center(),
            layout.map_rect.max,
        ];
        for mp in map_pts {
            let w = map_to_world(&layout, mp);
            let back = world_to_map(&layout, w);
            assert!(
                (back.x - mp.x).abs() < 0.01 && (back.y - mp.y).abs() < 0.01,
                "roundtrip failed for {mp:?}: got {back:?}"
            );
        }
    }

    // ── Viewport rect containment check ────────────────────────────────

    /// The viewport always stays within the world bounds (after 5% expansion)
    #[test]
    fn viewport_rect_always_inside_map_bounds() {
        let area = make_area();
        // A typical viewport (equivalent to the full screen)
        let viewport_world =
            egui::Rect::from_min_size(egui::pos2(-100.0, -50.0), egui::vec2(1200.0, 800.0));
        let items_bb = egui::Rect::from_min_size(egui::pos2(50.0, 50.0), egui::vec2(800.0, 600.0));
        let union = items_bb.union(viewport_world);
        let world_bounds = union.expand2(union.size() * 0.05);

        let layout = compute_minimap_layout(area, world_bounds);

        // All 4 corners of the viewport stay within the world bounds
        for corner in [
            viewport_world.min,
            viewport_world.max,
            egui::pos2(viewport_world.min.x, viewport_world.max.y),
            egui::pos2(viewport_world.max.x, viewport_world.min.y),
        ] {
            // Should fit since world_bounds has already been expanded by 5%
            assert!(
                world_bounds.contains(corner),
                "corner {corner:?} outside world_bounds {world_bounds:?}"
            );
            // When converted from world coordinates to map coordinates, it should stay within the map rect
            let mp = world_to_map(&layout, corner);
            assert!(
                layout.map_rect.contains(mp) || {
                    // Tolerate floating-point error at the boundary
                    (mp.x - layout.map_rect.min.x).min(layout.map_rect.max.x - mp.x) > -0.1
                        && (mp.y - layout.map_rect.min.y).min(layout.map_rect.max.y - mp.y) > -0.1
                },
                "map point {mp:?} outside map_rect {:?}",
                layout.map_rect
            );
        }
    }
}
