use crate::app::TunnyApp;
use crate::state::app_state::AppState;
use crate::theme::TOOLBAR_BTN_FG;
use crate::ui::help::help_types::HelpLanguage;

/// Duration of the left/right panel open/close animation (seconds)
pub const PANEL_ANIM_TIME: f32 = 0.20;

/// The panel opens when the mouse comes within this many pixels
const HOVER_TRIGGER_PX: f32 = 20.0;

/// Width of the indicator strip drawn at the left/right edges
const EDGE_STRIP_W: f32 = 18.0;
/// Corner rounding radius of the indicator
const EDGE_STRIP_ROUNDING: f32 = 4.0;
/// Vertical size of the indicator (shown as a short tab in the center)
const EDGE_STRIP_H: f32 = 60.0;

/// Draws the TunnyApp layout (Toolbar + OverlayPanels + CentralPanel)
pub fn show_layout(app: &mut TunnyApp, ui: &mut egui::Ui) {
    use crate::ui::{
        left_panel::show_left_panel,
        main_canvas::show_main_canvas,
        right_panel::show_right_panel,
        toolbar::{show_colormap_selector, show_toolbar},
    };

    // In egui 0.35, Panel-family APIs require &mut Ui, so keep a clone around
    // for Context-based calls (Area, etc.).
    let ctx = ui.ctx().clone();
    let tx = app.sender();

    // ─── Toolbar ───────────────────────────────────────────────────
    egui::Panel::top("toolbar")
        .min_size(32.0)
        .frame(
            egui::Frame::default()
                .fill(crate::theme::TOOLBAR_BG())
                .inner_margin(egui::Margin::symmetric(8, 4)),
        )
        .show(ui, |ui| {
            ui.visuals_mut().override_text_color = Some(crate::theme::TOOLBAR_TEXT());
            {
                let vis = ui.visuals_mut();
                vis.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                vis.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                vis.widgets.inactive.fg_stroke =
                    egui::Stroke::new(1.0, crate::theme::TOOLBAR_TEXT());
                vis.widgets.hovered.bg_fill = crate::theme::TOOLBAR_BTN_HOVER();
                vis.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TOOLBAR_BTN_FG());
                vis.widgets.active.bg_fill = crate::theme::TOOLBAR_BTN_ACTIVE();
                vis.widgets.active.bg_stroke = egui::Stroke::NONE;
                vis.widgets.active.fg_stroke = egui::Stroke::new(1.5, TOOLBAR_BTN_FG());
            }
            let toolbar_actions = show_toolbar(
                ui,
                &app.app_state,
                app.is_loading,
                app.load_error.as_deref(),
            );
            app.apply_toolbar_actions(toolbar_actions);

            // Toolbar row 2: colormap on the left (always shown), Help Language on the right
            ui.horizontal(|ui| {
                show_colormap_selector(ui, &mut app.app_state, &mut app.widget_states);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    show_theme_toggle(ui, &mut app.app_state);
                    show_language_menu(ui, &mut app.app_state);
                    if ui.button("📄 Licenses").clicked() {
                        app.widget_states.license_modal.open = true;
                    }
                });
            });
        });

    // Effective area below the toolbar (reference for placing overlay panels)
    // egui 0.35: Context::available_rect() was removed. After Panel::show, the
    // parent Ui's available_rect_before_wrap() returns the area shrunk by the toolbar.
    let panel_area = ui.available_rect_before_wrap();

    // ─── Hover detection ───────────────────────────────────────────────────
    // Keep panels' open/closed state during a drag to avoid interrupting DnD.
    let is_using_pointer = ctx.egui_is_using_pointer();

    if !is_using_pointer {
        let mouse = ctx.input(|i| i.pointer.hover_pos());
        match mouse {
            Some(m) if m.y >= panel_area.top() => {
                // Left panel: opens when the cursor comes within HOVER_TRIGGER_PX.
                // Closes once it goes past panel width + HOVER_TRIGGER_PX.
                // However, don't change open/closed state while the cursor is over
                // the minimap's interaction zone in the bottom-left. This prevents
                // merely approaching the corner minimap from opening the left panel
                // and getting in the way of interacting with it.
                let minimap_fp = crate::ui::canvas::minimap::minimap_footprint();
                let minimap_zone = egui::Rect::from_min_max(
                    egui::pos2(panel_area.left(), panel_area.bottom() - minimap_fp.y),
                    egui::pos2(panel_area.left() + minimap_fp.x, panel_area.bottom()),
                );
                if !minimap_zone.contains(m) {
                    let left_close_x = app.layout.left_panel_width + HOVER_TRIGGER_PX;
                    if m.x < HOVER_TRIGGER_PX {
                        app.layout.left_panel_open = true;
                    } else if m.x > left_close_x {
                        app.layout.left_panel_open = false;
                    }
                }

                // Right panel: opens when the cursor comes within HOVER_TRIGGER_PX
                // of the right edge.
                // However, don't change open/closed state while the cursor is over
                // the fit button's interaction zone in the bottom-right. This prevents
                // merely approaching the corner button from opening the panel and
                // getting in the way of interacting with it.
                // The button is hidden once the panel is open, but the fit action
                // is not expected to be used in that state anyway.
                let fit_fp = crate::ui::canvas::minimap::fit_button_footprint();
                let fit_zone = egui::Rect::from_min_max(
                    egui::pos2(
                        panel_area.right() - fit_fp.x,
                        panel_area.bottom() - fit_fp.y,
                    ),
                    egui::pos2(panel_area.right(), panel_area.bottom()),
                );
                if !fit_zone.contains(m) {
                    // Base the close boundary on the actually rendered left edge from
                    // the last frame, not the configured width. This avoids closing
                    // the panel just from hovering over a tile even when the icon
                    // tiles were rendered wider than the configured width and shifted
                    // left.
                    let panel_left = app
                        .layout
                        .right_panel
                        .last_rendered_left_x
                        .unwrap_or(panel_area.right() - app.layout.right_panel.width);
                    let right_close_x = panel_left - HOVER_TRIGGER_PX;
                    if m.x > panel_area.right() - HOVER_TRIGGER_PX {
                        app.layout.right_panel.is_open = true;
                    } else if m.x < right_close_x {
                        app.layout.right_panel.is_open = false;
                    }
                }
            }
            None => {
                // Close when the mouse leaves the window
                app.layout.left_panel_open = false;
                app.layout.right_panel.is_open = false;
            }
            _ => {} // Don't change state while the mouse is over the toolbar
        }
    }

    // ─── Animation values ──────────────────────────────────────────────
    let left_t = ctx.animate_bool_with_time(
        egui::Id::new("left_panel_anim"),
        app.layout.left_panel_open,
        PANEL_ANIM_TIME,
    );
    let right_t = ctx.animate_bool_with_time(
        egui::Id::new("right_panel_anim"),
        app.layout.right_panel.is_open,
        PANEL_ANIM_TIME,
    );

    // Keep repainting while animating
    if (0.005..0.995).contains(&left_t) || (0.005..0.995).contains(&right_t) {
        ctx.request_repaint();
    }

    // Don't draw the edge strips while capturing (avoid them showing up in the
    // chart image). Read the flag before the closure mutably borrows app.
    let app_state_screenshot_requested = app.widget_states.capture.screenshot_requested;

    // ─── Central panel (always full width) ──────────────────────────────────────
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(crate::theme::CENTRAL_BG()))
        .show(ui, |ui| {
            // Draw the grid first, then overlay the indicators on top
            show_main_canvas(
                ui,
                &mut app.app_state,
                &mut app.layout,
                &mut app.widget_states,
                &mut app.canvas_widgets,
                &tx,
            );

            // Indicators at the left/right edges (drawn on top of the grid so they're visible)
            let painter = ui.painter();
            let mouse_pos = ctx.input(|i| i.pointer.hover_pos());

            // Brighter the closer the mouse is to the edge (proximity feedback)
            let left_hover_factor = mouse_pos
                .filter(|m| m.y >= panel_area.top())
                .map(|m| {
                    let dist = m.x.max(0.0);
                    (1.0 - (dist / HOVER_TRIGGER_PX).min(1.0)) * 0.5
                })
                .unwrap_or(0.0);
            let right_hover_factor = mouse_pos
                .filter(|m| m.y >= panel_area.top())
                .map(|m| {
                    let dist = (panel_area.right() - m.x).max(0.0);
                    (1.0 - (dist / HOVER_TRIGGER_PX).min(1.0)) * 0.5
                })
                .unwrap_or(0.0);

            // Base color: ACCENT_BLUE, TOOLBAR_BTN_ACTIVE on hover
            let base_color = crate::theme::ACCENT_BLUE();
            let hover_color = crate::theme::TOOLBAR_BTN_ACTIVE();

            let lerp_color = |a: egui::Color32, b: egui::Color32, t: f32| -> egui::Color32 {
                egui::Color32::from_rgba_unmultiplied(
                    (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
                    (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
                    (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
                    (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
                )
            };

            // Don't let the edge strips (the › ‹ indicators) show up in PNG/image
            // captures. Since screenshots capture the whole screen and crop by the
            // chart rect, excluding them means not drawing the foreground strip
            // that overlaps the chart.
            let capturing = app_state_screenshot_requested;

            // Only show the strip while the panel is closed (fades with the slide-out)
            if left_t < 0.995 && !capturing {
                let alpha = ((1.0 - left_t) * 255.0) as u8;
                let color = lerp_color(base_color, hover_color, left_hover_factor);
                let color =
                    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

                let strip_center_y = panel_area.center().y;
                let left_strip = egui::Rect::from_center_size(
                    egui::pos2(EDGE_STRIP_W / 2.0, strip_center_y),
                    egui::vec2(EDGE_STRIP_W, EDGE_STRIP_H),
                );
                painter.rect_filled(left_strip, EDGE_STRIP_ROUNDING, color);
                // Arrow icon
                painter.text(
                    left_strip.center(),
                    egui::Align2::CENTER_CENTER,
                    "›",
                    egui::FontId::proportional(16.0),
                    egui::Color32::WHITE,
                );
            }
            if right_t < 0.995 && !capturing {
                let alpha = ((1.0 - right_t) * 255.0) as u8;
                let color = lerp_color(base_color, hover_color, right_hover_factor);
                let color =
                    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

                let strip_center_y = panel_area.center().y;
                let right_strip = egui::Rect::from_center_size(
                    egui::pos2(panel_area.right() - EDGE_STRIP_W / 2.0, strip_center_y),
                    egui::vec2(EDGE_STRIP_W, EDGE_STRIP_H),
                );
                painter.rect_filled(right_strip, EDGE_STRIP_ROUNDING, color);
                // Arrow icon
                painter.text(
                    right_strip.center(),
                    egui::Align2::CENTER_CENTER,
                    "‹",
                    egui::FontId::proportional(16.0),
                    egui::Color32::WHITE,
                );
            }
        });

    // ─── Left panel overlay ────────────────────────────────────────
    if left_t > 0.005 {
        let panel_w = app.layout.left_panel_width;
        // Slide in from the left edge: t=0 fully off-screen left, t=1 at x=0
        let left_x = panel_w * (left_t - 1.0);

        egui::Area::new(egui::Id::new("left_panel_overlay"))
            .fixed_pos(egui::pos2(left_x, panel_area.top()))
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                let frame = egui::Frame::default()
                    .fill(crate::theme::PANEL_BG())
                    .inner_margin(egui::Margin::same(8))
                    .shadow(egui::Shadow {
                        offset: [4, 0],
                        blur: 12,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(50),
                    });
                frame.show(ui, |ui| {
                    let inner_w = (panel_w - 16.0).max(0.0);
                    let inner_h = (panel_area.height() - 16.0).max(0.0);
                    ui.set_min_size(egui::vec2(inner_w, inner_h));
                    show_left_panel(ui, &mut app.app_state, &mut app.layout);
                });
            });
    }

    // ─── Right panel overlay ────────────────────────────────────────
    if right_t > 0.005 {
        let panel_w = app.layout.right_panel.width;
        // Slide in from the right edge: t=0 at the screen's right edge, t=1 flush against it
        let right_x = panel_area.right() - panel_w * right_t;

        let area_resp = egui::Area::new(egui::Id::new("right_panel_overlay"))
            .fixed_pos(egui::pos2(right_x, panel_area.top()))
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                let frame = egui::Frame::default()
                    .fill(crate::theme::PANEL_BG())
                    .inner_margin(egui::Margin::same(8))
                    .shadow(egui::Shadow {
                        offset: [-4, 0],
                        blur: 12,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(50),
                    });
                frame.show(ui, |ui| {
                    let inner_w = (panel_w - 16.0).max(0.0);
                    let inner_h = (panel_area.height() - 16.0).max(0.0);
                    ui.set_min_size(egui::vec2(inner_w, inner_h));
                    show_right_panel(ui, &app.app_state);
                });
            });
        // Record the actual left edge so next frame's hover-close check can use it,
        // even if egui's constrain shifted it left because the icon tiles were
        // rendered wider than the configured width.
        app.layout.right_panel.last_rendered_left_x = Some(area_resp.response.rect.left());
    } else {
        // Discard the measured value while the panel is closed, so the next time
        // it opens it falls back to the configured width.
        app.layout.right_panel.last_rendered_left_x = None;
    }

    // ─── Maximized modal (overlaid above everything else) ──────────────────────
    crate::ui::chart_cell::show_maximized_modal(
        &ctx,
        &mut app.app_state,
        &mut app.widget_states,
        &tx,
    );
}

/// Draws the light/dark theme toggle button.
/// Actually applying `Visuals` is handled by `TunnyApp::logic`'s per-frame sync,
/// so here we just need to flip the `dark_mode` flag.
pub fn show_theme_toggle(ui: &mut egui::Ui, app_state: &mut AppState) {
    let (icon, tooltip) = if app_state.dark_mode {
        ("☀ Light", "ライトテーマに切り替え / Switch to light theme")
    } else {
        ("🌙 Dark", "ダークテーマに切り替え / Switch to dark theme")
    };
    if ui.button(icon).on_hover_text(tooltip).clicked() {
        app_state.dark_mode = !app_state.dark_mode;
    }
}

/// Draws the help language switch menu.
/// Call from a place with access to &mut AppState, such as the toolbar.
pub fn show_language_menu(ui: &mut egui::Ui, app_state: &mut AppState) {
    let current = app_state.help_language;
    ui.menu_button("🌐 Help Language", |ui| {
        if ui
            .selectable_label(current == HelpLanguage::En, "English")
            .clicked()
        {
            app_state.help_language = HelpLanguage::En;
        }
        if ui
            .selectable_label(current == HelpLanguage::Ja, "日本語")
            .clicked()
        {
            app_state.help_language = HelpLanguage::Ja;
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn show_language_menu_logic_sets_ja() {
        use crate::state::app_state::AppState;
        use crate::ui::help::help_types::HelpLanguage;
        let mut state = AppState::new();
        assert_eq!(state.help_language, HelpLanguage::En);
        state.help_language = HelpLanguage::Ja;
        assert_eq!(state.help_language, HelpLanguage::Ja);
    }
}
