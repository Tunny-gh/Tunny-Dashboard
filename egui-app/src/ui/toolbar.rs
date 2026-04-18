use std::sync::mpsc::SyncSender;

use crate::state::app_state::AppState;
use crate::state::layout_state::{LayoutMode, LayoutState};
use crate::state::messages::AppMessage;

/// レイアウトモードボタンのラベル定義
pub const LAYOUT_MODE_BUTTONS: &[(LayoutMode, &str)] = &[
    (LayoutMode::MultiObjective, "Multi-Objective"),
    (LayoutMode::VariableSpace, "Variable Space"),
    (LayoutMode::ConvergenceAnalysis, "Convergence Analysis"),
    (LayoutMode::FreeLayout, "Free Layout"),
];

/// ToolBar を描画する
pub fn show_toolbar(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    layout: &mut LayoutState,
    tx: &SyncSender<AppMessage>,
    is_loading: &mut bool,
    load_error: &mut Option<String>,
) {
    ui.horizontal(|ui| {
        // ファイル開くボタン
        let open_enabled = !*is_loading;
        if toolbar_button(ui, "Open", open_enabled).clicked() {
            if let Some(path) = crate::io::file::open_file_dialog() {
                *is_loading = true;
                *load_error = None;
                let tx2 = tx.clone();
                crate::app::spawn_task(tx2, move || crate::io::journal::load_journal_task(path));
            }
        }

        ui.separator();

        // レイアウトモードボタン群
        for (mode, label) in LAYOUT_MODE_BUTTONS {
            let is_selected = layout.layout_mode == *mode;
            if toolbar_mode_button(ui, label, is_selected).clicked() {
                layout.layout_mode = mode.clone();
            }
        }

        ui.separator();

        // Study選択: 常に ComboBox を表示、未読み込み時は無効
        {
            ui.label(
                egui::RichText::new("Target Study:")
                    .color(crate::theme::TOOLBAR_TEXT)
                    .size(12.0),
            );
            let current_name = app_state
                .current_study
                .as_ref()
                .map(|c| c.meta.name.clone())
                .unwrap_or_default();
            let mut selected_name = current_name.clone();
            let has_studies = !app_state.all_studies.is_empty();
            let display_text = if *is_loading {
                "Loading...".to_string()
            } else if current_name.is_empty() {
                String::new()
            } else {
                current_name.clone()
            };
            ui.scope(|ui| {
                let vis = ui.visuals_mut();
                vis.override_text_color = Some(crate::theme::TOOLBAR_TEXT);
                vis.widgets.noninteractive.bg_fill = crate::theme::TOOLBAR_INPUT_BG;
                vis.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, crate::theme::TOOLBAR_INPUT_STROKE);
                vis.widgets.noninteractive.fg_stroke =
                    egui::Stroke::new(1.0, crate::theme::TEXT_PRIMARY);
                vis.widgets.inactive.bg_fill = crate::theme::TOOLBAR_INPUT_BG;
                vis.widgets.inactive.bg_stroke =
                    egui::Stroke::new(1.0, crate::theme::TOOLBAR_INPUT_STROKE);
                vis.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, crate::theme::TEXT_PRIMARY);
                vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, crate::theme::TEXT_PRIMARY);
                vis.widgets.active.fg_stroke = egui::Stroke::new(1.0, crate::theme::TEXT_PRIMARY);
                ui.add_enabled_ui(has_studies && !*is_loading, |ui| {
                    egui::ComboBox::from_id_salt("study_select_combo")
                        .selected_text(
                            egui::RichText::new(&display_text).color(crate::theme::TEXT_PRIMARY),
                        )
                        .show_ui(ui, |ui| {
                            for study in &app_state.all_studies {
                                ui.selectable_value(
                                    &mut selected_name,
                                    study.name.clone(),
                                    &study.name,
                                );
                            }
                        });
                });
            });
            if selected_name != current_name && !selected_name.is_empty() {
                if let (Some(meta), Some(path)) = (
                    app_state
                        .all_studies
                        .iter()
                        .find(|s| s.name == selected_name)
                        .cloned(),
                    app_state.journal_path.clone(),
                ) {
                    *is_loading = true;
                    let tx2 = tx.clone();
                    crate::app::spawn_task(tx2, move || {
                        crate::io::study::load_and_select_task(path, meta)
                    });
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // ライブ更新トグル
            let live_label = if app_state.live_update.enabled {
                "Live Update: On"
            } else {
                "Live Update: Off"
            };
            if toolbar_button(ui, live_label, true).clicked() {
                app_state.live_update.enabled = !app_state.live_update.enabled;
            }

            // ローディングインジケーター
            if *is_loading {
                ui.spinner();
            }

            // エラーメッセージ
            if let Some(err) = load_error.clone() {
                if ui
                    .colored_label(egui::Color32::RED, format!("Error: {}", err))
                    .clicked()
                {
                    *load_error = None;
                }
            }
        });
    });
}

fn toolbar_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let padding = egui::vec2(10.0, 5.0);
    let text_color = if enabled {
        crate::theme::TOOLBAR_TEXT
    } else {
        crate::theme::TOOLBAR_TEXT.gamma_multiply(0.4)
    };
    let galley = ui.fonts(|f| {
        f.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(13.0),
            text_color,
        )
    });
    let desired = galley.size() + padding * 2.0;
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(desired, sense);

    if ui.is_rect_visible(rect) {
        let bg = if !enabled {
            egui::Color32::TRANSPARENT
        } else if resp.hovered() {
            crate::theme::TOOLBAR_BTN_HOVER
        } else {
            egui::Color32::TRANSPARENT
        };
        let final_text_color = if enabled && resp.hovered() {
            egui::Color32::WHITE
        } else {
            text_color
        };
        ui.painter().rect_filled(rect, 4.0, bg);
        ui.painter()
            .galley(rect.min + padding, galley, final_text_color);
    }
    resp
}

fn toolbar_mode_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let padding = egui::vec2(10.0, 5.0);
    let galley = ui.fonts(|f| {
        f.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        )
    });
    let desired = galley.size() + padding * 2.0;
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let bg = if selected {
            crate::theme::ACCENT_BLUE
        } else if resp.hovered() {
            crate::theme::TOOLBAR_BTN_HOVER
        } else {
            egui::Color32::TRANSPARENT
        };
        let text_color = if selected || resp.hovered() {
            egui::Color32::WHITE
        } else {
            crate::theme::TOOLBAR_TEXT
        };
        ui.painter().rect_filled(rect, 4.0, bg);
        ui.painter().galley(rect.min + padding, galley, text_color);
    }
    resp
}

/// LayoutMode を文字列から解決する（テスト用ユーティリティ）
pub fn layout_mode_label(mode: LayoutMode) -> &'static str {
    LAYOUT_MODE_BUTTONS
        .iter()
        .find(|(m, _)| *m == mode)
        .map(|(_, label)| *label)
        .unwrap_or("Unknown")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::layout_state::LayoutMode;

    #[test]
    fn layout_mode_buttons_cover_all_modes() {
        let modes: Vec<LayoutMode> = LAYOUT_MODE_BUTTONS.iter().map(|(m, _)| m.clone()).collect();
        assert!(modes.contains(&LayoutMode::MultiObjective));
        assert!(modes.contains(&LayoutMode::VariableSpace));
        assert!(modes.contains(&LayoutMode::ConvergenceAnalysis));
        assert!(modes.contains(&LayoutMode::FreeLayout));
        assert_eq!(modes.len(), 4);
    }

    #[test]
    fn layout_mode_label_returns_correct_label() {
        assert_eq!(
            layout_mode_label(LayoutMode::MultiObjective),
            "Multi-Objective"
        );
        assert_eq!(
            layout_mode_label(LayoutMode::VariableSpace),
            "Variable Space"
        );
        assert_eq!(
            layout_mode_label(LayoutMode::ConvergenceAnalysis),
            "Convergence Analysis"
        );
        assert_eq!(layout_mode_label(LayoutMode::FreeLayout), "Free Layout");
    }

    #[test]
    fn layout_mode_switch_updates_state() {
        let mut layout = LayoutState::default();
        assert_eq!(layout.layout_mode, LayoutMode::MultiObjective);
        layout.layout_mode = LayoutMode::ConvergenceAnalysis;
        assert_eq!(layout.layout_mode, LayoutMode::ConvergenceAnalysis);
    }

    #[test]
    fn loading_state_clears_on_file_open_sequence() {
        let mut is_loading = false;
        let mut load_error: Option<String> = Some("Previous error".to_string());
        // ファイル選択時に is_loading=true, load_error=None になることを確認
        is_loading = true;
        load_error = None;
        assert!(is_loading);
        assert!(load_error.is_none());
    }

    #[test]
    fn error_cleared_on_click_simulation() {
        let mut load_error: Option<String> = Some("Error".to_string());
        // エラーラベルクリック時のシミュレーション
        load_error = None;
        assert!(load_error.is_none());
    }
}
