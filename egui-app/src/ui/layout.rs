use crate::app::TunnyApp;

/// 4エリアレイアウトの定数
pub const LEFT_WIDTH_MIN: f32 = 120.0;
pub const LEFT_WIDTH_MAX: f32 = 600.0;
pub const BOTTOM_HEIGHT_MIN: f32 = 60.0;
pub const BOTTOM_HEIGHT_MAX: f32 = 600.0;

/// LayoutState のパネルサイズ設定を確認する
pub fn validate_panel_constraints(
    left_width: f32,
    bottom_height: f32,
) -> (f32, f32) {
    let clamped_left = left_width
        .max(LEFT_WIDTH_MIN)
        .min(LEFT_WIDTH_MAX);
    let clamped_bottom = bottom_height
        .max(BOTTOM_HEIGHT_MIN)
        .min(BOTTOM_HEIGHT_MAX);
    (clamped_left, clamped_bottom)
}

/// TunnyApp の4エリアレイアウトを描画する
pub fn show_layout(app: &mut TunnyApp, ctx: &egui::Context) {
    use crate::ui::{
        bottom_panel::show_bottom_panel, left_panel::show_left_panel,
        main_canvas::show_main_canvas, toolbar::show_toolbar,
    };

    let tx = app.sender();

    egui::TopBottomPanel::top("toolbar")
        .min_height(32.0)
        .show(ctx, |ui| {
            show_toolbar(
                ui,
                &mut app.app_state,
                &mut app.layout,
                &tx,
                &mut app.is_loading,
                &mut app.load_error,
            );
        });

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(app.layout.left_panel_width)
        .width_range(LEFT_WIDTH_MIN..=LEFT_WIDTH_MAX)
        .show(ctx, |ui| {
            show_left_panel(ui, &mut app.app_state, &mut app.layout);
        });

    egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .default_height(app.layout.bottom_panel_height)
        .height_range(BOTTOM_HEIGHT_MIN..=BOTTOM_HEIGHT_MAX)
        .show(ctx, |ui| {
            show_bottom_panel(ui, &mut app.app_state);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        show_main_canvas(ui, &mut app.app_state, &mut app.layout);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_panel_constraints_clamps_min() {
        let (left, bottom) = validate_panel_constraints(0.0, 0.0);
        assert_eq!(left, LEFT_WIDTH_MIN);
        assert_eq!(bottom, BOTTOM_HEIGHT_MIN);
    }

    #[test]
    fn validate_panel_constraints_clamps_max() {
        let (left, bottom) = validate_panel_constraints(9999.0, 9999.0);
        assert_eq!(left, LEFT_WIDTH_MAX);
        assert_eq!(bottom, BOTTOM_HEIGHT_MAX);
    }

    #[test]
    fn validate_panel_constraints_passes_valid_values() {
        let (left, bottom) = validate_panel_constraints(240.0, 200.0);
        assert_eq!(left, 240.0);
        assert_eq!(bottom, 200.0);
    }

    #[test]
    fn constants_are_valid() {
        assert!(LEFT_WIDTH_MIN < LEFT_WIDTH_MAX);
        assert!(BOTTOM_HEIGHT_MIN < BOTTOM_HEIGHT_MAX);
    }
}
