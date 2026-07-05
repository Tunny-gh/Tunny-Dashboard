//! 2D チャート共通のナビゲーション設定。
//!
//! 全 2D チャート（egui_plot ベース）で操作系を統一する:
//! - 左ドラッグ: 矩形ズーム（範囲選択で拡大）
//! - 右ドラッグ: パン
//! - 左ダブルクリック: デフォルトビュー（自動範囲）へリセット
//! - スクロールホイール: カーソル位置を中心にズーム
//!   （egui_plot 既定の「ホイール = 平行移動」は無効化し、[`apply_wheel_zoom`] で
//!   ズームに割り当て直す）

/// 統一ナビゲーション設定を `egui_plot::Plot` へ適用する拡張トレイト。
///
/// 2D チャートを追加する際は `Plot::new(..)` のビルダーチェーンに
/// `.unified_nav()` を必ず挟み、`show` クロージャ先頭で [`apply_wheel_zoom`]
/// を呼ぶこと。
pub trait UnifiedNav {
    fn unified_nav(self) -> Self;
}

impl UnifiedNav for egui_plot::Plot<'_> {
    fn unified_nav(self) -> Self {
        self.boxed_zoom_pointer_button(egui::PointerButton::Primary)
            .pan_pointer_button(egui::PointerButton::Secondary)
            .allow_double_click_reset(true)
            .allow_scroll(false)
    }
}

/// ホイール操作をカーソル位置中心のズームとして適用する。
///
/// egui ではホイール単体は `smooth_scroll_delta`（平行移動用）になり、
/// `zoom_delta` になるのは Ctrl+ホイール / ピンチのみ。チャート上では
/// ホイール = 拡大縮小に統一したいため、プロットへのホバー中はスクロール
/// 入力を消費してズームへ変換する（親コンテナのスクロールも抑止される）。
pub fn apply_wheel_zoom(plot_ui: &mut egui_plot::PlotUi<'_>) {
    if !plot_ui.response().hovered() {
        return;
    }
    let scroll_y = plot_ui.ctx().input_mut(|i| {
        let y = i.smooth_scroll_delta.y;
        i.smooth_scroll_delta = egui::Vec2::ZERO;
        y
    });
    if scroll_y != 0.0 {
        // egui の Ctrl+ホイールズームと同じ感度（2^(delta/200)）。
        let factor = 2.0_f32.powf(scroll_y / 200.0);
        plot_ui.zoom_bounds_around_hovered(egui::Vec2::splat(factor));
    }
}
