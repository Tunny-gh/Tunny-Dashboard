use crate::app::TunnyApp;
use crate::theme::TOOLBAR_BTN_FG;

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
        left_panel::show_left_panel, main_canvas::show_main_canvas, right_panel::show_right_panel,
        toolbar::show_toolbar,
    };

    let tx = app.sender();

    egui::TopBottomPanel::top("toolbar")
        .min_height(32.0)
        .frame(
            egui::Frame::default()
                .fill(crate::theme::TOOLBAR_BG)
                .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
        )
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(crate::theme::TOOLBAR_TEXT);
            {
                let vis = ui.visuals_mut();
                vis.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                vis.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                vis.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, crate::theme::TOOLBAR_TEXT);
                vis.widgets.hovered.bg_fill = crate::theme::TOOLBAR_BTN_HOVER;
                vis.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TOOLBAR_BTN_FG);
                vis.widgets.active.bg_fill = crate::theme::TOOLBAR_BTN_ACTIVE;
                vis.widgets.active.bg_stroke = egui::Stroke::NONE;
                vis.widgets.active.fg_stroke = egui::Stroke::new(1.5, TOOLBAR_BTN_FG);
            }
            let toolbar_actions = show_toolbar(
                ui,
                &app.app_state,
                &app.layout,
                app.is_loading,
                app.load_error.as_deref(),
            );
            app.apply_toolbar_actions(toolbar_actions);
        });

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(app.layout.left_panel_width)
        .width_range(LEFT_WIDTH_MIN..=LEFT_WIDTH_MAX)
        .frame(
            egui::Frame::default()
                .fill(crate::theme::PANEL_BG)
                .inner_margin(egui::Margin::same(8.0)),
        )
        .show(ctx, |ui| {
            show_left_panel(
                ui,
                &mut app.app_state,
                &mut app.widget_states,
                &mut app.layout,
                &tx,
            );
        });

    // 右パネル（ウィジェット一覧・ハンバーガーメニュー）
    // 開閉で異なる ID を使い分けることで egui の幅キャッシュを分離する。
    // "right_panel" ID は開いた状態専用 → ユーザーがリサイズした幅がキャッシュされ、
    // 再オープン時にその幅が自動復元される。
    // "right_panel_closed" ID は閉じた状態専用 → 常に固定幅 (CLOSED_WIDTH) で表示。
    const CLOSED_WIDTH: f32 = 48.0;

    let panel_frame = egui::Frame::default()
        .fill(crate::theme::PANEL_BG)
        .inner_margin(egui::Margin::same(8.0));

    if app.layout.right_panel.is_open {
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(app.layout.right_panel.width)
            .width_range(RIGHT_WIDTH_MIN..=RIGHT_WIDTH_MAX)
            .frame(panel_frame)
            .show(ctx, |ui| {
                show_right_panel(ui, &app.app_state, &mut app.layout);
            });
    } else {
        egui::SidePanel::right("right_panel_closed")
            .resizable(false)
            .default_width(CLOSED_WIDTH)
            .width_range(CLOSED_WIDTH..=CLOSED_WIDTH)
            .frame(panel_frame)
            .show(ctx, |ui| {
                show_right_panel(ui, &app.app_state, &mut app.layout);
            });
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        show_main_canvas(
            ui,
            &mut app.app_state,
            &mut app.layout,
            &mut app.widget_states,
            &tx,
        );
    });

    crate::ui::help::help_modal::show_help_modal(ctx, &mut app.widget_states.help_modal);
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
