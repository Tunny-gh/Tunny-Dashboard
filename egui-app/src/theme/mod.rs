use egui::{Stroke, Visuals};

pub mod chart_colors;
pub mod color_compute;
pub mod colormap;
pub mod colormap_name;
pub mod ui_colors;

pub use ui_colors::*;

pub fn tunny_light_visuals() -> Visuals {
    let mut v = Visuals::light();

    v.panel_fill = PANEL_BG;
    v.window_fill = CENTRAL_BG;
    v.window_stroke = Stroke::new(1.0, BORDER_COLOR);
    v.override_text_color = Some(TEXT_PRIMARY);
    v.extreme_bg_color = CENTRAL_BG;

    v.widgets.active.bg_fill = ACCENT_BLUE;
    v.widgets.active.fg_stroke = Stroke::new(1.5, TOOLBAR_BTN_FG);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_BLUE);

    v.widgets.hovered.bg_fill = WIDGET_BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_BLUE);

    v.widgets.inactive.bg_fill = WIDGET_BG;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_COLOR);

    v.widgets.noninteractive.bg_fill = PANEL_BG;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_COLOR);

    v.selection.bg_fill = ACCENT_BLUE_MUTED;
    v.selection.stroke = Stroke::new(1.0, ACCENT_BLUE);

    v
}
