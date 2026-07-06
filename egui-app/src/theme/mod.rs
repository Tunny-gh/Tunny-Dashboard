use std::sync::atomic::{AtomicBool, Ordering};

use egui::{Stroke, Visuals};

pub mod chart_colors;
pub mod color_compute;
pub mod colormap;
pub mod colormap_name;
pub mod ui_colors;

pub use ui_colors::*;

/// 現在のテーマ（ダークか）。描画スレッドからのみ書き換える前提の単純な
/// グローバルフラグで、`ui_colors` / `chart_colors` の色関数が毎フレーム
/// 参照する。egui の `Visuals` 切替（[`tunny_visuals`]）と必ず同時に
/// [`set_dark_mode`] で更新すること。
static DARK_MODE: AtomicBool = AtomicBool::new(false);

/// テーマを切り替える（`true` = ダーク）。
pub fn set_dark_mode(dark: bool) {
    DARK_MODE.store(dark, Ordering::Relaxed);
}

/// 現在ダークテーマか。
pub fn is_dark_mode() -> bool {
    DARK_MODE.load(Ordering::Relaxed)
}

/// ライト/ダーク共通の Tunny テーマ `Visuals` を構築する。
///
/// 色は `ui_colors` の同名関数群（テーマ追従）を参照するため、必ず
/// [`set_dark_mode`] を呼んでから本関数で `Visuals` を作り直すこと。
pub fn tunny_visuals(dark: bool) -> Visuals {
    set_dark_mode(dark);
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    v.panel_fill = PANEL_BG();
    v.window_fill = CENTRAL_BG();
    v.window_stroke = Stroke::new(1.0, BORDER_COLOR());
    v.override_text_color = Some(TEXT_PRIMARY());
    v.extreme_bg_color = CENTRAL_BG();

    v.widgets.active.bg_fill = ACCENT_BLUE();
    v.widgets.active.fg_stroke = Stroke::new(1.5, TOOLBAR_BTN_FG());
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_BLUE());

    v.widgets.hovered.bg_fill = WIDGET_BG_HOVER();
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_BLUE());

    v.widgets.inactive.bg_fill = WIDGET_BG();
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_COLOR());

    v.widgets.noninteractive.bg_fill = PANEL_BG();
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_COLOR());

    v.selection.bg_fill = ACCENT_BLUE_MUTED();
    v.selection.stroke = Stroke::new(1.0, ACCENT_BLUE());

    v
}

/// テーマ追従色を定義するマクロ。
///
/// `pub const NAME: Color32` 相当の使用感を保つため、大文字スネークケースの
/// 関数として展開する（呼び出し側は `NAME()`）。ライト/ダークの実値は
/// 定義箇所に並記され、[`is_dark_mode`] で毎回解決される。
macro_rules! themed_color {
    ($(#[$doc:meta])* $name:ident, $light:expr, $dark:expr) => {
        $(#[$doc])*
        #[allow(non_snake_case)]
        #[inline]
        pub fn $name() -> egui::Color32 {
            if $crate::theme::is_dark_mode() {
                $dark
            } else {
                $light
            }
        }
    };
}
pub(crate) use themed_color;
