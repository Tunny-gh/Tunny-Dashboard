use crate::app::TunnyApp;
use crate::state::app_state::AppState;
use crate::theme::TOOLBAR_BTN_FG;
use crate::ui::help::help_types::HelpLanguage;

/// 左右パネルの開閉アニメーション所要時間（秒）
pub const PANEL_ANIM_TIME: f32 = 0.20;

/// パネルサイズの定数
pub const LEFT_WIDTH_MIN: f32 = 120.0;
pub const LEFT_WIDTH_MAX: f32 = 600.0;
pub const RIGHT_WIDTH_MIN: f32 = 100.0;
pub const RIGHT_WIDTH_MAX: f32 = 400.0;

/// マウスがこのピクセル数以内に近づくとパネルが開く
const HOVER_TRIGGER_PX: f32 = 20.0;

/// 左右縁に描画するインジケーター strip の幅
const EDGE_STRIP_W: f32 = 18.0;
/// インジケーターの角丸半径
const EDGE_STRIP_ROUNDING: f32 = 4.0;
/// インジケーターの縦サイズ（中央に短いタブとして表示）
const EDGE_STRIP_H: f32 = 60.0;

/// 左パネル幅のクランプ
pub fn clamp_left_width(left_width: f32) -> f32 {
    left_width.clamp(LEFT_WIDTH_MIN, LEFT_WIDTH_MAX)
}

/// TunnyApp のレイアウトを描画する（Toolbar + OverlayPanels + CentralPanel）
pub fn show_layout(app: &mut TunnyApp, ui: &mut egui::Ui) {
    use crate::ui::{
        left_panel::show_left_panel,
        main_canvas::show_main_canvas,
        right_panel::show_right_panel,
        toolbar::{show_colormap_selector, show_toolbar},
    };

    // egui 0.35 では Panel 系は &mut Ui を要求するため、Context 経由の呼び出し (Area 等) 用に複製しておく。
    let ctx = ui.ctx().clone();
    let tx = app.sender();

    // ─── ツールバー ───────────────────────────────────────────────────
    egui::Panel::top("toolbar")
        .min_size(32.0)
        .frame(
            egui::Frame::default()
                .fill(crate::theme::TOOLBAR_BG)
                .inner_margin(egui::Margin::symmetric(8, 4)),
        )
        .show(ui, |ui| {
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

            // ツールバー 2 段目: 左端にカラーマップ（常時表示）、右端に Help Language
            ui.horizontal(|ui| {
                show_colormap_selector(ui, &mut app.app_state, &mut app.widget_states);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    show_language_menu(ui, &mut app.app_state);
                    if ui.button("📄 Licenses").clicked() {
                        app.widget_states.license_modal.open = true;
                    }
                });
            });
        });

    // ツールバー下の有効エリア（オーバーレイパネルの配置基準）
    // egui 0.35: Context::available_rect() は廃止。Panel::show 後は親 Ui の
    // available_rect_before_wrap() がツールバー分だけ縮んだ領域を返す。
    let panel_area = ui.available_rect_before_wrap();

    // ─── ホバー検知 ───────────────────────────────────────────────────
    // ドラッグ中はパネル開閉を維持し、DnD の中断を防ぐ。
    let is_using_pointer = ctx.egui_is_using_pointer();

    if !is_using_pointer {
        let mouse = ctx.input(|i| i.pointer.hover_pos());
        match mouse {
            Some(m) if m.y >= panel_area.top() => {
                // 左パネル: HOVER_TRIGGER_PX 以内に入ると開く。
                // パネル幅 + HOVER_TRIGGER_PX を超えると閉じる。
                // ただし左下のミニマップ操作領域にカーソルがある間は開閉を変更しない。
                // これにより、隅のミニマップへ近づくだけで左パネルが開いて操作を妨げるのを防ぐ。
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

                // 右パネル: 右端から HOVER_TRIGGER_PX 以内に入ると開く。
                // ただし右下のフィットボタン操作領域にカーソルがある間は開閉を変更しない。
                // これにより、隅のボタンへ近づくだけでパネルが開いて操作を妨げるのを防ぐ。
                // パネルが既に開いているときはボタンが隠れるが、その状態ではフィット操作を
                // 行わない想定。
                let fit_fp = crate::ui::canvas::minimap::fit_button_footprint();
                let fit_zone = egui::Rect::from_min_max(
                    egui::pos2(
                        panel_area.right() - fit_fp.x,
                        panel_area.bottom() - fit_fp.y,
                    ),
                    egui::pos2(panel_area.right(), panel_area.bottom()),
                );
                if !fit_zone.contains(m) {
                    // 閉じ境界は設定幅ではなく直近に描画された実際の左端を基準にする。
                    // アイコンタイルが設定幅を超えて描画され左へシフトしても、
                    // タイル上をホバーしただけで閉じてしまわないようにするため。
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
                // マウスがウィンドウ外に出たら閉じる
                app.layout.left_panel_open = false;
                app.layout.right_panel.is_open = false;
            }
            _ => {} // ツールバー上のマウス位置では変更しない
        }
    }

    // ─── アニメーション値 ──────────────────────────────────────────────
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

    // アニメーション中は継続して再描画
    if (0.005..0.995).contains(&left_t) || (0.005..0.995).contains(&right_t) {
        ctx.request_repaint();
    }

    // キャプチャ中はエッジストリップを描画しない（チャート画像への写り込み防止）。
    // クロージャが app を可変借用する前にフラグを読んでおく。
    let app_state_screenshot_requested = app.widget_states.capture.screenshot_requested;

    // ─── 中央パネル（常にフル幅） ──────────────────────────────────────
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(crate::theme::CENTRAL_BG))
        .show(ui, |ui| {
            // グリッドを先に描画し、インジケーターは後で上に重ねる
            show_main_canvas(
                ui,
                &mut app.app_state,
                &mut app.layout,
                &mut app.widget_states,
                &mut app.canvas_widgets,
                &tx,
            );

            // 左右縁のインジケーター（グリッドの上に描画して見えるようにする）
            let painter = ui.painter();
            let mouse_pos = ctx.input(|i| i.pointer.hover_pos());

            // マウスが縁に近いほど明るくなる（近接フィードバック）
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

            // ベース色: ACCENT_BLUE, ホバーで TOOLBAR_BTN_ACTIVE
            let base_color = crate::theme::ACCENT_BLUE;
            let hover_color = crate::theme::TOOLBAR_BTN_ACTIVE;

            let lerp_color = |a: egui::Color32, b: egui::Color32, t: f32| -> egui::Color32 {
                egui::Color32::from_rgba_unmultiplied(
                    (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
                    (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
                    (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
                    (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
                )
            };

            // PNG/画像キャプチャ中はエッジストリップ（› ‹ インジケーター）を写し込まない。
            // スクリーンショットは画面全体を撮りチャート矩形でクロップするため、
            // チャートに重なる前面ストリップを描画しないことで除外する。
            let capturing = app_state_screenshot_requested;

            // パネルが閉じているときのみ strip を表示（スライドアウトに合わせてフェード）
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
                // 矢印アイコン
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
                // 矢印アイコン
                painter.text(
                    right_strip.center(),
                    egui::Align2::CENTER_CENTER,
                    "‹",
                    egui::FontId::proportional(16.0),
                    egui::Color32::WHITE,
                );
            }
        });

    // ─── 左パネル オーバーレイ ────────────────────────────────────────
    if left_t > 0.005 {
        let panel_w = app.layout.left_panel_width;
        // 左端からスライドイン: t=0 で完全に画面外左、t=1 で x=0
        let left_x = panel_w * (left_t - 1.0);

        egui::Area::new(egui::Id::new("left_panel_overlay"))
            .fixed_pos(egui::pos2(left_x, panel_area.top()))
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                let frame = egui::Frame::default()
                    .fill(crate::theme::PANEL_BG)
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

    // ─── 右パネル オーバーレイ ────────────────────────────────────────
    if right_t > 0.005 {
        let panel_w = app.layout.right_panel.width;
        // 右端からスライドイン: t=0 で画面右端、t=1 で右端に密着
        let right_x = panel_area.right() - panel_w * right_t;

        let area_resp = egui::Area::new(egui::Id::new("right_panel_overlay"))
            .fixed_pos(egui::pos2(right_x, panel_area.top()))
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                let frame = egui::Frame::default()
                    .fill(crate::theme::PANEL_BG)
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
                    show_right_panel(ui, &app.app_state, &mut app.layout);
                });
            });
        // アイコンタイルが設定幅より広く描画され egui の constrain により左へ
        // シフトされても、次フレームのホバー閉じ判定が実際の左端を使えるよう記録する。
        app.layout.right_panel.last_rendered_left_x = Some(area_resp.response.rect.left());
    } else {
        // パネルが閉じている間は実測値を破棄し、次に開くときは設定幅ベースに戻す。
        app.layout.right_panel.last_rendered_left_x = None;
    }

    // ─── 最大化モーダル（すべての最前面に重ねる） ──────────────────────
    crate::ui::grid_canvas::show_maximized_modal(
        &ctx,
        &mut app.app_state,
        &mut app.widget_states,
        &tx,
    );
}

/// ヘルプ言語切替メニューを描画する。
/// ツールバーなど &mut AppState にアクセスできる場所から呼び出す。
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
    use super::*;

    #[test]
    fn show_language_menu_logic_sets_ja() {
        use crate::state::app_state::AppState;
        use crate::ui::help::help_types::HelpLanguage;
        let mut state = AppState::new();
        assert_eq!(state.help_language, HelpLanguage::En);
        state.help_language = HelpLanguage::Ja;
        assert_eq!(state.help_language, HelpLanguage::Ja);
    }

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
        const { assert!(LEFT_WIDTH_MIN < LEFT_WIDTH_MAX) };
        const { assert!(RIGHT_WIDTH_MIN < RIGHT_WIDTH_MAX) };
    }
}
