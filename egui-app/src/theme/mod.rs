use std::sync::atomic::{AtomicBool, Ordering};

use egui::{Stroke, Visuals};

pub mod chart_colors;
pub mod color_compute;
pub mod colormap;
pub mod colormap_name;
pub mod ui_colors;

pub use ui_colors::*;

/// The current theme (whether it's dark). A simple global flag assumed to be written
/// only from the rendering thread, referenced every frame by the color functions in
/// `ui_colors` / `chart_colors`. Always update it via [`set_dark_mode`] at the same time
/// as switching egui's `Visuals` ([`tunny_visuals`]).
static DARK_MODE: AtomicBool = AtomicBool::new(false);

/// Switches the theme (`true` = dark).
pub fn set_dark_mode(dark: bool) {
    DARK_MODE.store(dark, Ordering::Relaxed);
}

/// Whether the current theme is dark.
pub fn is_dark_mode() -> bool {
    DARK_MODE.load(Ordering::Relaxed)
}

/// Builds the `Visuals` for the Tunny theme, shared by light/dark.
///
/// Since the colors reference the same-named function group in `ui_colors` (theme-
/// following), always call [`set_dark_mode`] before rebuilding `Visuals` with this
/// function.
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

/// A macro defining a theme-following color.
///
/// Expands into an uppercase-snake-case function (called as `NAME()`) to preserve a
/// usage feel equivalent to `pub const NAME: Color32`. The light/dark actual values are
/// listed side by side at the definition site, and resolved every time via [`is_dark_mode`].
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
