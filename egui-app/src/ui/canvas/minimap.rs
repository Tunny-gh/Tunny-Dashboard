//! キャンバスミニマップ。
//!
//! 純粋な計算ロジック（[`compute_minimap_layout`]、[`world_to_map`]、[`map_to_world`]）と
//! 描画・インタラクション（[`show_minimap`]）を同一モジュールに置く。
//! 計算部分は egui 描画コンテキストに依存しないためユニットテスト可能。

use egui::emath::TSTransform;

use crate::state::layout_state::CanvasItem;
use crate::ui::canvas::viewport::{items_bbox, pan_to_center};

// ─────────────────────────────────────────────────────────────────────────────
// レイアウト定数
// ─────────────────────────────────────────────────────────────────────────────

/// ミニマップの最大幅（スクリーンピクセル）
const MAX_W: f32 = 100.0;
/// ミニマップの最大高さ（スクリーンピクセル）
const MAX_H: f32 = 75.0;
/// エリア端からのマージン（スクリーンピクセル）
const MARGIN: f32 = 12.0;

/// フィットボタンのサイズ（canvas_view.rs の BTN_SIZE と同じ値）
pub(crate) const BTN_SIZE: f32 = 28.0;
/// フィットボタンのマージン（canvas_view.rs の BTN_MARGIN と同じ値）
pub(crate) const BTN_MARGIN: f32 = 12.0;

/// フィットボタン（右下）が占有する領域サイズ（スクリーンpx）。
/// レイアウト側で右パネルのホバー開閉判定からこの隅を除外するために参照する。
pub(crate) fn fit_button_footprint() -> egui::Vec2 {
    let s = BTN_SIZE + BTN_MARGIN * 2.0;
    egui::vec2(s, s)
}

/// ミニマップ（左下）が占有しうる最大領域サイズ（スクリーンpx）。
/// レイアウト側で左パネルのホバー開閉判定からこの隅を除外するために参照する。
pub(crate) fn minimap_footprint() -> egui::Vec2 {
    egui::vec2(MAX_W + MARGIN * 2.0, MAX_H + MARGIN * 2.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// 純粋計算 — ユニットテスト可能
// ─────────────────────────────────────────────────────────────────────────────

/// ミニマップの画面レイアウト情報。
///
/// - `map_rect` : ミニマップが占有する画面矩形
/// - `scale`    : ワールド座標 1 単位 → マップ内 px の比率
/// - `world_origin` : ミニマップ左上に対応するワールド座標
#[derive(Debug, Clone)]
pub(crate) struct MinimapLayout {
    pub map_rect: egui::Rect,
    pub scale: f32,
    pub world_origin: egui::Pos2,
}

/// ミニマップのレイアウトを計算する（純粋関数）。
///
/// # 引数
/// - `area`         : キャンバス全体のスクリーン矩形
/// - `world_bounds` : ミニマップが表すワールド矩形（アイテム bbox ∪ ビューポートを 5% 拡張したもの）
///
/// # 配置
/// 左下隅に配置する（フィットボタンは右下隅のため干渉しない）。
///
/// # 縮退ガード
/// `world_bounds` の幅/高さが 0 以下の場合は scale = 1.0 / マップサイズ = MAX として扱う。
pub(crate) fn compute_minimap_layout(area: egui::Rect, world_bounds: egui::Rect) -> MinimapLayout {
    // スケール: ワールド → マップ
    let scale = if world_bounds.width() > 0.0 && world_bounds.height() > 0.0 {
        let sx = MAX_W / world_bounds.width();
        let sy = MAX_H / world_bounds.height();
        sx.min(sy)
    } else {
        1.0
    };

    // マップの実際のサイズ（縦横比を保ち MAX に収まる）
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

    // 左下隅に配置する。
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

/// ワールド座標をミニマップ内のスクリーン座標へ変換する。
pub(crate) fn world_to_map(layout: &MinimapLayout, p: egui::Pos2) -> egui::Pos2 {
    layout.map_rect.min + (p - layout.world_origin) * layout.scale
}

/// ミニマップ内のスクリーン座標をワールド座標へ変換する（[`world_to_map`] の逆写像）。
pub(crate) fn map_to_world(layout: &MinimapLayout, p: egui::Pos2) -> egui::Pos2 {
    layout.world_origin + (p - layout.map_rect.min) / layout.scale
}

// ─────────────────────────────────────────────────────────────────────────────
// 描画・インタラクション
// ─────────────────────────────────────────────────────────────────────────────

/// ミニマップオーバーレイを描画し、ドラッグによるビューポート移動を処理する。
///
/// アイテムが空のときは何も描画しない。
/// `to_screen` を直接更新することで呼び出し元のビューポート変換に反映される。
///
/// # オーバーレイ実装
/// [`egui::Order::Foreground`] の [`egui::Area`] を使用するため、
/// キャンバス背景・チャートより常に手前に描画され、背景インタラクションも遮蔽される。
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

    // ── ワールド境界の計算 ────────────────────────────────────────────────
    let items_bb = match items_bbox(items) {
        Some(b) => b,
        None => return,
    };
    // 現在のビューポートをワールド座標に変換
    let viewport_world = to_screen.inverse().mul_rect(area);
    // アイテム bbox とビューポートの和集合に 5% マージンを付加する
    let union = items_bb.union(viewport_world);
    let world_bounds = union.expand2(union.size() * 0.05);

    let layout = compute_minimap_layout(area, world_bounds);

    // ── インタラクション（Foreground より前に登録してミニマップ外クリックと分離） ──
    // Area の fixed_pos にミニマップ左上を使う。
    let minimap_resp = egui::Area::new(egui::Id::new("canvas_minimap_overlay"))
        .order(egui::Order::Foreground)
        .fixed_pos(layout.map_rect.min)
        .show(ui.ctx(), |ui| {
            let map_size = layout.map_rect.size();

            // センス領域をマップ全体に確保（ホバー判定 + クリック/ドラッグ）
            let (resp, painter) = ui.allocate_painter(map_size, egui::Sense::click_and_drag());

            let mr = resp.rect; // Area 内の実際の矩形（map_rect と一致するはず）

            // ── 背景パネル ─────────────────────────────────────────────
            painter.rect(
                mr,
                4.0,
                egui::Color32::from_black_alpha(170),
                egui::Stroke::new(1.0, crate::theme::BORDER_COLOR),
            );

            // ── アイテム矩形 ─────────────────────────────────────────
            for item in items {
                let item_world_rect = egui::Rect::from_min_size(
                    egui::pos2(item.x, item.y),
                    egui::vec2(item.w, item.h),
                );
                // マップ内の矩形（layout.map_rect.min 基点なのでオフセットを調整）
                let map_min = world_to_map(&layout, item_world_rect.min)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let map_max = world_to_map(&layout, item_world_rect.max)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let item_map_rect = egui::Rect::from_min_max(map_min, map_max);
                // マップ内にクリップ（交差が空なら描かない）
                let clipped = item_map_rect.intersect(mr);
                if clipped.is_positive() {
                    painter.rect(
                        clipped,
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(120, 120, 120, 180),
                        egui::Stroke::new(0.5, egui::Color32::from_gray(160)),
                    );
                }
            }

            // ── ビューポート矩形 ──────────────────────────────────────
            {
                let vp_min = world_to_map(&layout, viewport_world.min)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let vp_max = world_to_map(&layout, viewport_world.max)
                    - layout.map_rect.min.to_vec2()
                    + mr.min.to_vec2();
                let vp_map_rect = egui::Rect::from_min_max(vp_min, vp_max).intersect(mr);
                let accent = crate::theme::ACCENT_BLUE;
                painter.rect(
                    vp_map_rect,
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 20),
                    egui::Stroke::new(1.5, accent),
                );
            }

            // ── カーソル ─────────────────────────────────────────────
            if resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            resp
        });

    // ── ドラッグ/クリックでビューポートを移動 ───────────────────────────────
    let resp = minimap_resp.inner;
    let do_pan = resp.clicked() || resp.dragged();
    if do_pan {
        if let Some(ptr) = resp.interact_pointer_pos() {
            // Area 内ポインタ → 実際のスクリーン座標（map_rect.min オフセット分を加算）
            let map_ptr = ptr + layout.map_rect.min.to_vec2() - resp.rect.min.to_vec2();
            let world_center = map_to_world(&layout, map_ptr);
            let zoom = to_screen.scaling;
            let pan = pan_to_center(area, zoom, world_center);
            *to_screen = offset * TSTransform::new(pan, zoom);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ユニットテスト
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

    /// 幅方向が制約になる場合、マップ幅が MAX_W を超えない
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

    /// 高さ方向が制約になる場合、マップ高さが MAX_H を超えない
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

    /// アスペクト比が保持される（許容誤差 1%）
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

    /// ミニマップは左下隅に配置される（フィットボタンの右下とは別の隅）
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

    /// 縮退境界（幅 0 高さ 0）でも有限値を返す
    #[test]
    fn layout_degenerate_bounds_finite() {
        let area = make_area();
        let wb = make_world_bounds(0.0, 0.0);
        let layout = compute_minimap_layout(area, wb);
        assert!(layout.scale.is_finite(), "scale must be finite");
        assert!(layout.map_rect.min.x.is_finite());
        assert!(layout.map_rect.min.y.is_finite());
    }

    // ── world_to_map / map_to_world 往復変換 ─────────────────────────────

    /// world→map→world が元の座標に戻る（往復精度 0.01 px 以内）
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

    /// map→world→map が元のマップ座標に戻る
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

    // ── ビューポート矩形の包含確認 ────────────────────────────────────────

    /// ビューポートが常にワールド境界内に収まる（5% 拡張後）
    #[test]
    fn viewport_rect_always_inside_map_bounds() {
        let area = make_area();
        // 典型的なビューポート（画面全体相当）
        let viewport_world =
            egui::Rect::from_min_size(egui::pos2(-100.0, -50.0), egui::vec2(1200.0, 800.0));
        let items_bb = egui::Rect::from_min_size(egui::pos2(50.0, 50.0), egui::vec2(800.0, 600.0));
        let union = items_bb.union(viewport_world);
        let world_bounds = union.expand2(union.size() * 0.05);

        let layout = compute_minimap_layout(area, world_bounds);

        // ビューポートの4角がすべてワールド境界内に収まる
        for corner in [
            viewport_world.min,
            viewport_world.max,
            egui::pos2(viewport_world.min.x, viewport_world.max.y),
            egui::pos2(viewport_world.max.x, viewport_world.min.y),
        ] {
            // world_bounds に 5% 拡張済みなので収まるはず
            assert!(
                world_bounds.contains(corner),
                "corner {corner:?} outside world_bounds {world_bounds:?}"
            );
            // ワールド座標からマップ座標に変換したとき、マップ矩形内に収まる
            let mp = world_to_map(&layout, corner);
            assert!(
                layout.map_rect.contains(mp) || {
                    // 境界上の浮動小数誤差を許容
                    (mp.x - layout.map_rect.min.x).min(layout.map_rect.max.x - mp.x) > -0.1
                        && (mp.y - layout.map_rect.min.y).min(layout.map_rect.max.y - mp.y) > -0.1
                },
                "map point {mp:?} outside map_rect {:?}",
                layout.map_rect
            );
        }
    }
}
