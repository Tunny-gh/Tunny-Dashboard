//! Canvas viewport utility functions.
//!
//! The pure functions defined here are unit-testable and do not depend on
//! egui's rendering context. Future features such as a minimap can also
//! reuse this module's functions.

use crate::state::layout_state::CanvasItem;

/// Zoom lower bound (single source referenced by both manual zoom-out and fit calculation)
pub(crate) const ZOOM_MIN: f32 = 0.1;
/// Zoom upper bound (limit for manual zoom-in)
pub(crate) const ZOOM_MAX: f32 = 3.0;

/// Screen margin used when fitting (each side)
const FIT_MARGIN: f32 = 24.0;

/// Returns the bounding box (in world coordinates) of a set of items.
///
/// Returns `None` if the items are empty.
pub(crate) fn items_bbox(items: &[CanvasItem]) -> Option<egui::Rect> {
    let mut iter = items.iter();
    let first = iter.next()?;
    let mut bbox =
        egui::Rect::from_min_size(egui::pos2(first.x, first.y), egui::vec2(first.w, first.h));
    for item in iter {
        let r = egui::Rect::from_min_size(egui::pos2(item.x, item.y), egui::vec2(item.w, item.h));
        bbox = bbox.union(r);
    }
    Some(bbox)
}

/// Returns the pan such that `world_center` ends up at `area.center()`.
///
/// Based on the transform `screen_pos = area.min + pan + zoom * world_pos`,
/// solves for the `pan` that satisfies `screen_pos == area.center()`.
pub(crate) fn pan_to_center(area: egui::Rect, zoom: f32, world_center: egui::Pos2) -> egui::Vec2 {
    area.center().to_vec2() - area.min.to_vec2() - zoom * world_center.to_vec2()
}

/// Returns the (zoom, pan) that fits the entire `bbox` within `area`.
///
/// - Margin: `FIT_MARGIN` screen pixels on each side
/// - zoom is clamped to `[ZOOM_MIN, 1.0]` (a fit operation never zooms in
///   past 100%)
/// - In the degenerate case where `bbox`'s width or height is 0 or less,
///   uses zoom = 1.0
pub(crate) fn fit_view(area: egui::Rect, bbox: egui::Rect) -> (f32, egui::Vec2) {
    let avail_w = area.width() - 2.0 * FIT_MARGIN;
    let avail_h = area.height() - 2.0 * FIT_MARGIN;

    let zoom = if bbox.width() > 0.0 && bbox.height() > 0.0 {
        let zx = avail_w / bbox.width();
        let zy = avail_h / bbox.height();
        zx.min(zy).clamp(ZOOM_MIN, 1.0)
    } else {
        1.0
    };

    let pan = pan_to_center(area, zoom, bbox.center());
    (zoom, pan)
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::layout_state::{CanvasItem, PanelItem};

    fn make_item(id: u64, x: f32, y: f32, w: f32, h: f32) -> CanvasItem {
        CanvasItem {
            id,
            content: PanelItem::TrialTable,
            x,
            y,
            w,
            h,
        }
    }

    // ── items_bbox ────────────────────────────────────────────────────────────

    /// Returns None when the items are empty.
    #[test]
    fn items_bbox_empty_returns_none() {
        assert!(items_bbox(&[]).is_none());
    }

    /// The bounding box of a single item is the item itself.
    #[test]
    fn items_bbox_single_item() {
        let items = [make_item(0, 10.0, 20.0, 100.0, 80.0)];
        let bbox = items_bbox(&items).unwrap();
        assert_eq!(bbox.min, egui::pos2(10.0, 20.0));
        assert_eq!(bbox.max, egui::pos2(110.0, 100.0));
    }

    /// The enclosing rectangle of multiple items is computed correctly.
    #[test]
    fn items_bbox_multiple_items() {
        let items = [
            make_item(0, 0.0, 0.0, 100.0, 100.0),
            make_item(1, 200.0, 150.0, 50.0, 50.0),
            make_item(2, -30.0, 10.0, 40.0, 40.0),
        ];
        let bbox = items_bbox(&items).unwrap();
        // min: x=-30, y=0; max: x=250, y=200
        assert_eq!(bbox.min.x, -30.0);
        assert_eq!(bbox.min.y, 0.0);
        assert_eq!(bbox.max.x, 250.0);
        assert_eq!(bbox.max.y, 200.0);
    }

    // ── fit_view ─────────────────────────────────────────────────────────────

    /// A bbox wide in the horizontal direction yields zoom < 1.0, and the
    /// bbox center ends up at the area center.
    #[test]
    fn fit_view_wide_bbox_produces_zoom_below_1() {
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        // bbox is 4x the width of area -> zoom should end up below 0.5
        let bbox = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(3000.0, 400.0));
        let (zoom, pan) = fit_view(area, bbox);

        // zoom is greater than the lower bound and at most 1.0
        assert!((ZOOM_MIN..=1.0).contains(&zoom), "zoom={zoom}");
        // zoom should be < 1.0 (since the width is large)
        assert!(
            zoom < 1.0,
            "wide bbox should produce zoom < 1.0, got {zoom}"
        );

        // Verify that the bbox center ends up at the screen center:
        // screen = area.min + pan + zoom * world -> at world=bbox.center(), screen==area.center()
        let world_center = bbox.center();
        let screen_center = area.min.to_vec2() + pan + zoom * world_center.to_vec2();
        let expected = area.center().to_vec2();
        assert!(
            (screen_center.x - expected.x).abs() < 0.01,
            "cx mismatch: {screen_center:?} vs {expected:?}"
        );
        assert!(
            (screen_center.y - expected.y).abs() < 0.01,
            "cy mismatch: {screen_center:?} vs {expected:?}"
        );
    }

    /// A small bbox has its zoom clamped to 1.0 (never zooms in).
    #[test]
    fn fit_view_small_bbox_caps_zoom_at_1() {
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let bbox = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(50.0, 50.0));
        let (zoom, _pan) = fit_view(area, bbox);
        assert_eq!(zoom, 1.0, "small bbox must not zoom in beyond 100%");
    }

    /// A degenerate bbox (width 0) does not produce NaN or inf.
    #[test]
    fn fit_view_degenerate_bbox_no_nan() {
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let bbox = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(0.0, 0.0));
        let (zoom, pan) = fit_view(area, bbox);
        assert!(zoom.is_finite(), "zoom must be finite");
        assert!(pan.x.is_finite() && pan.y.is_finite(), "pan must be finite");
    }
}
