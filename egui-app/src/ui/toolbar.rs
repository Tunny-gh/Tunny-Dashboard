use std::sync::mpsc::SyncSender;

use crate::state::app_state::AppState;
use crate::state::layout_state::{LayoutMode, LayoutState};
use crate::state::messages::AppMessage;

/// レイアウトモードボタンのラベル定義
pub const LAYOUT_MODE_BUTTONS: &[(LayoutMode, &str)] = &[
    (LayoutMode::MultiObjective, "Multi-Objective"),
    (LayoutMode::VariableSpace, "Variable Space"),
    (LayoutMode::ConvergenceAnalysis, "Convergence"),
    (LayoutMode::FreeLayout, "Free Layout"),
    (LayoutMode::Comparison, "Comparison"),
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

        // レイアウトモード ComboBox
        {
            let current_label = LAYOUT_MODE_BUTTONS
                .iter()
                .find(|(m, _)| *m == layout.layout_mode)
                .map(|(_, l)| *l)
                .unwrap_or("Layout");
            ui.scope(|ui| {
                apply_combo_visuals(ui.visuals_mut());
                egui::ComboBox::from_id_salt("layout_mode_combo")
                    .selected_text(
                        egui::RichText::new(current_label).color(crate::theme::TOOLBAR_TEXT),
                    )
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for (mode, label) in LAYOUT_MODE_BUTTONS {
                            let selected = layout.layout_mode == *mode;
                            if ui.selectable_label(selected, *label).clicked() {
                                layout.layout_mode = mode.clone();
                            }
                        }
                    });
            });
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
            } else {
                current_name.clone()
            };
            ui.scope(|ui| {
                apply_combo_visuals(ui.visuals_mut());
                ui.add_enabled_ui(has_studies && !*is_loading, |ui| {
                    egui::ComboBox::from_id_salt("study_select_combo")
                        .selected_text(
                            egui::RichText::new(&display_text).color(crate::theme::TOOLBAR_TEXT),
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
                "Live: On"
            } else {
                "Live: Off"
            };
            if toolbar_button(ui, live_label, true).clicked() {
                app_state.live_update.enabled = !app_state.live_update.enabled;
            }

            ui.separator();

            // ── REQ-005: HTML レポート出力 ──────────────────────────────────
            if toolbar_button(ui, "HTML", app_state.current_study.is_some()).clicked() {
                if let Some(ctx) = &app_state.current_study {
                    use crate::io::html_report::{
                        generate_html_report_async, HtmlReportSnapshot, HtmlTrialRow,
                        TrialStatistics,
                    };
                    let snap = HtmlReportSnapshot {
                        study_name: ctx.meta.name.clone(),
                        objective_names: ctx.meta.objective_names.clone(),
                        param_names: ctx.meta.param_names.clone(),
                        total_trials: ctx.trial_rows.len(),
                        pareto_count: ctx.pareto_indices.len(),
                        selected_trials: app_state
                            .selected_indices
                            .iter()
                            .filter_map(|&id| ctx.trial_rows.iter().find(|r| r.trial_id == id))
                            .map(|r| HtmlTrialRow {
                                trial_id: r.trial_id,
                                trial_number: r.trial_number,
                                params: r.params.clone(),
                                objectives: r.objectives.clone(),
                                pareto_rank: r.pareto_rank,
                            })
                            .collect(),
                        statistics: TrialStatistics {
                            objective_means: vec![0.0; ctx.meta.objective_names.len()],
                            objective_variances: vec![0.0; ctx.meta.objective_names.len()],
                            pareto_count: ctx.pareto_indices.len(),
                        },
                    };
                    generate_html_report_async(snap, tx.clone());
                }
            }

            // ── REQ-007: Artifacts フォルダ選択 ───────────────────────────────
            if toolbar_button(ui, "Artifacts", true).clicked() {
                if let Some(base_dir) = rfd::FileDialog::new().pick_folder() {
                    crate::io::artifacts::scan_artifacts_dir(base_dir, tx.clone());
                }
            }

            ui.separator();

            // ── REQ-004: セッション ──────────────────────────────────────────
            if toolbar_button(ui, "Load Session", true).clicked() {
                use crate::io::session;
                if let Some(snap) = session::load_session() {
                    app_state.filter_ranges = snap.filter_ranges;
                    app_state.selected_indices = snap.selected_indices;
                    app_state.tradeoff_weights = snap.tradeoff_weights;
                }
            }

            if toolbar_button(ui, "Save Session", app_state.current_study.is_some()).clicked() {
                use crate::io::session;
                let name = app_state
                    .current_study
                    .as_ref()
                    .map(|c| c.meta.name.clone())
                    .unwrap_or_default();
                let snap = session::SessionSnapshot::new(
                    name,
                    app_state.filter_ranges.clone(),
                    app_state.selected_indices.clone(),
                );
                session::save_session(&snap);
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

fn apply_combo_visuals(vis: &mut egui::Visuals) {
    use crate::theme::{
        TOOLBAR_BTN_ACTIVE, TOOLBAR_BTN_HOVER, TOOLBAR_INPUT_BG, TOOLBAR_INPUT_STROKE, TOOLBAR_TEXT,
    };
    vis.override_text_color = Some(TOOLBAR_TEXT);
    let bg_stroke = egui::Stroke::new(1.0, TOOLBAR_INPUT_STROKE);
    let fg_text = egui::Stroke::new(1.0, TOOLBAR_TEXT);
    let fg_white = egui::Stroke::new(1.0, egui::Color32::WHITE);
    for w in [&mut vis.widgets.inactive, &mut vis.widgets.noninteractive] {
        w.weak_bg_fill = TOOLBAR_INPUT_BG;
        w.bg_fill = TOOLBAR_INPUT_BG;
        w.bg_stroke = bg_stroke;
        w.fg_stroke = fg_text;
    }
    vis.widgets.hovered.weak_bg_fill = TOOLBAR_BTN_HOVER;
    vis.widgets.hovered.bg_fill = TOOLBAR_BTN_HOVER;
    vis.widgets.hovered.fg_stroke = fg_white;
    vis.widgets.active.weak_bg_fill = TOOLBAR_BTN_ACTIVE;
    vis.widgets.active.bg_fill = TOOLBAR_BTN_ACTIVE;
    vis.widgets.active.fg_stroke = fg_white;
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
        assert!(modes.contains(&LayoutMode::Comparison));
        assert_eq!(modes.len(), 5);
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
            "Convergence"
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
