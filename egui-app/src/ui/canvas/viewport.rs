//! キャンバスビューポートのユーティリティ関数。
//!
//! ここに定義した純粋関数はユニットテスト可能で、egui の描画コンテキストに依存しない。
//! ミニマップなど将来の機能もこのモジュールの関数を再利用できる。

use crate::state::layout_state::CanvasItem;

/// ズーム下限（手動ズームアウト・フィット計算の両方で参照する単一ソース）
pub(crate) const ZOOM_MIN: f32 = 0.1;
/// ズーム上限（手動ズームインの上限）
pub(crate) const ZOOM_MAX: f32 = 3.0;

/// フィット時の画面マージン（各辺）
const FIT_MARGIN: f32 = 24.0;

/// アイテム群のバウンディングボックス（ワールド座標）を返す。
///
/// アイテムが空の場合は `None` を返す。
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

/// `world_center` が `area.center()` に来るようなパンを返す。
///
/// 変換式: `screen_pos = area.min + pan + zoom * world_pos` に基づき、
/// `screen_pos == area.center()` となる `pan` を求める。
pub(crate) fn pan_to_center(area: egui::Rect, zoom: f32, world_center: egui::Pos2) -> egui::Vec2 {
    area.center().to_vec2() - area.min.to_vec2() - zoom * world_center.to_vec2()
}

/// `bbox` 全体が `area` 内に収まる (zoom, pan) を返す。
///
/// - マージン: 各辺 `FIT_MARGIN` スクリーンピクセル
/// - zoom は `[ZOOM_MIN, 1.0]` にクランプ（フィット操作は 100% を超えてズームインしない）
/// - `bbox` の幅または高さが 0 以下の縮退ケースでは zoom = 1.0 を使う
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
// ユニットテスト
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

    /// アイテムが空のとき None を返す
    #[test]
    fn items_bbox_empty_returns_none() {
        assert!(items_bbox(&[]).is_none());
    }

    /// 単一アイテムのバウンディングボックスはアイテム自身
    #[test]
    fn items_bbox_single_item() {
        let items = [make_item(0, 10.0, 20.0, 100.0, 80.0)];
        let bbox = items_bbox(&items).unwrap();
        assert_eq!(bbox.min, egui::pos2(10.0, 20.0));
        assert_eq!(bbox.max, egui::pos2(110.0, 100.0));
    }

    /// 複数アイテムの外接矩形が正しく計算される
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

    /// 幅方向に広い bbox は zoom < 1.0 になり、bbox 中心が area 中心に来る
    #[test]
    fn fit_view_wide_bbox_produces_zoom_below_1() {
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        // bbox が area の 4 倍の幅 → zoom が 0.5 未満になるはず
        let bbox = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(3000.0, 400.0));
        let (zoom, pan) = fit_view(area, bbox);

        // zoom が下限より大きく 1.0 以下
        assert!((ZOOM_MIN..=1.0).contains(&zoom), "zoom={zoom}");
        // zoom < 1.0 になるはず（幅が大きいので）
        assert!(
            zoom < 1.0,
            "wide bbox should produce zoom < 1.0, got {zoom}"
        );

        // bbox 中心がスクリーン中心に来るか検証:
        // screen = area.min + pan + zoom * world → world=bbox.center() で screen==area.center()
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

    /// 小さな bbox は zoom が 1.0 にクランプされる（ズームインしない）
    #[test]
    fn fit_view_small_bbox_caps_zoom_at_1() {
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let bbox = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(50.0, 50.0));
        let (zoom, _pan) = fit_view(area, bbox);
        assert_eq!(zoom, 1.0, "small bbox must not zoom in beyond 100%");
    }

    /// 縮退した bbox（幅 0）は NaN や inf を生まない
    #[test]
    fn fit_view_degenerate_bbox_no_nan() {
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let bbox = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(0.0, 0.0));
        let (zoom, pan) = fit_view(area, bbox);
        assert!(zoom.is_finite(), "zoom must be finite");
        assert!(pan.x.is_finite() && pan.y.is_finite(), "pan must be finite");
    }
}
