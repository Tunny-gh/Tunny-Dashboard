use crate::app::TunnyApp;

/// パネルサイズの定数
pub const LEFT_WIDTH_MIN: f32 = 120.0;
pub const LEFT_WIDTH_MAX: f32 = 600.0;
pub const RIGHT_WIDTH_MIN: f32 = 100.0;
pub const RIGHT_WIDTH_MAX: f32 = 400.0;

/// 左パネル幅のクランプ
pub fn clamp_left_width(left_width: f32) -> f32 {
    left_width.clamp(LEFT_WIDTH_MIN, LEFT_WIDTH_MAX)
}

/// TunnyApp のレイアウトを描画する（Toolbar + LeftPanel + RightPanel + CentralPanel）
pub fn show_layout(app: &mut TunnyApp, ctx: &egui::Context) {
    use crate::ui::{
        left_panel::show_left_panel,
        main_canvas::show_main_canvas,
        right_panel::show_right_panel,
        toolbar::show_toolbar,
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

    // 右パネル（ウィジェット一覧・ハンバーガーメニュー）
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(app.layout.right_panel.width)
        .width_range(RIGHT_WIDTH_MIN..=RIGHT_WIDTH_MAX)
        .show(ctx, |ui| {
            show_right_panel(ui, &app.app_state, &mut app.layout);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        show_main_canvas(ui, &mut app.app_state, &mut app.layout, &mut app.widget_states);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_left_width_clamps_min() {
        assert_eq!(clamp_left_width(0.0), LEFT_WIDTH_MIN);
    }

    #[test]
    fn clamp_left_width_clamps_max() {
        assert_eq!(clamp_left_width(9999.0), LEFT_WIDTH_MAX);
    }

    #[test]
    fn clamp_left_width_passes_valid() {
        assert_eq!(clamp_left_width(240.0), 240.0);
    }

    #[test]
    fn width_constants_are_valid() {
        assert!(LEFT_WIDTH_MIN < LEFT_WIDTH_MAX);
        assert!(RIGHT_WIDTH_MIN < RIGHT_WIDTH_MAX);
    }
}
