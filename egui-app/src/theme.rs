use egui::{Color32, Stroke, Visuals};

pub const TOOLBAR_BG: Color32 = Color32::from_rgb(26, 35, 50);
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(220, 230, 245);
pub const PANEL_BG: Color32 = Color32::from_rgb(225, 233, 248);
pub const CENTRAL_BG: Color32 = Color32::WHITE;
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(37, 99, 235);
#[allow(dead_code)]
pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(29, 78, 216);
pub const ACCENT_BLUE_MUTED: Color32 = Color32::from_rgb(219, 234, 254);
pub const BORDER_COLOR: Color32 = Color32::from_rgb(203, 213, 225);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 41, 59);
#[allow(dead_code)]
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 116, 139);
pub const CELL_TOOLBAR_BG: Color32 = Color32::from_rgb(232, 239, 251);
pub const WIDGET_BG: Color32 = Color32::from_rgb(235, 241, 252);
pub const WIDGET_BG_HOVER: Color32 = Color32::from_rgb(220, 230, 247);

pub fn tunny_light_visuals() -> Visuals {
    let mut v = Visuals::light();

    v.panel_fill = PANEL_BG;
    v.window_fill = CENTRAL_BG;
    v.window_stroke = Stroke::new(1.0, BORDER_COLOR);
    v.override_text_color = Some(TEXT_PRIMARY);
    v.extreme_bg_color = Color32::WHITE;

    v.widgets.active.bg_fill = ACCENT_BLUE;
    v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
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
