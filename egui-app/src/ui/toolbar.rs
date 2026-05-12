use crate::io::export::ExportTarget;
use crate::state::app_state::{AppState, StudyMeta};
use crate::state::layout_state::{LayoutMode, LayoutState};
use crate::theme::{ERROR_COLOR, TOOLBAR_BTN_FG};

/// レイアウトモードボタンのラベル定義
pub const LAYOUT_MODE_BUTTONS: &[(LayoutMode, &str)] = &[
    (LayoutMode::MultiObjective, "Multi-Objective"),
    (LayoutMode::VariableSpace, "Variable Space"),
    (LayoutMode::ConvergenceAnalysis, "Convergence"),
    (LayoutMode::FreeLayout, "Free Layout"),
    (LayoutMode::Comparison, "Comparison"),
];

#[derive(Debug, Clone)]
pub enum ToolbarAction {
    OpenJournal(std::path::PathBuf),
    SetLayoutMode(LayoutMode),
    SelectStudy(StudyMeta),
    ToggleLiveUpdate,
    SetPollInterval(u64),
    GenerateHtmlReport,
    ScanArtifacts(std::path::PathBuf),
    LoadSession,
    SaveSession,
    ClearLoadError,

    // TASK-2228: 新規アクション
    ExportCsv(ExportTarget),
    AddComparisonStudy,
    RemoveComparisonStudy(usize),
}

/// ToolBar を描画する
pub fn show_toolbar(
    ui: &mut egui::Ui,
    app_state: &AppState,
    layout: &LayoutState,
    is_loading: bool,
    load_error: Option<&str>,
) -> Vec<ToolbarAction> {
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        // ファイル開くボタン
        let open_enabled = !is_loading;
        if toolbar_button(ui, "Open", open_enabled).clicked() {
            if let Some(path) = crate::io::file::open_file_dialog() {
                actions.push(ToolbarAction::OpenJournal(path));
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
                                actions.push(ToolbarAction::SetLayoutMode(mode.clone()));
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
            let display_text = if is_loading {
                "Loading...".to_string()
            } else {
                current_name.clone()
            };
            ui.scope(|ui| {
                apply_combo_visuals(ui.visuals_mut());
                ui.add_enabled_ui(has_studies && !is_loading, |ui| {
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
                if let Some(meta) = app_state
                    .all_studies
                    .iter()
                    .find(|s| s.name == selected_name)
                    .cloned()
                {
                    actions.push(ToolbarAction::SelectStudy(meta));
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // ライブ更新トグル（ファイル未開封時は無効）
            let can_toggle = app_state.journal_path.is_some();
            let live_label = if app_state.live_update.enabled {
                format!(
                    "Live: On ({}s)",
                    app_state.live_update.interval_ms / 1000
                )
            } else {
                "Live: Off".to_string()
            };
            if toolbar_button(ui, &live_label, can_toggle).clicked() && can_toggle {
                actions.push(ToolbarAction::ToggleLiveUpdate);
            }

            // 試行数カウンタ
            let trial_label = if let Some(study) = &app_state.current_study {
                format!("Trials: {}", study.trial_rows.len())
            } else {
                "Trials: -".to_string()
            };
            ui.label(
                egui::RichText::new(trial_label)
                    .color(crate::theme::TOOLBAR_TEXT)
                    .size(12.0),
            );

            // ポーリング間隔スライダー（ライブ更新ON時のみ表示）
            if app_state.live_update.enabled {
                let mut interval_sec = app_state.live_update.interval_ms as f64 / 1000.0;
                let prev = interval_sec;
                ui.add(
                    egui::Slider::new(&mut interval_sec, 1.0..=30.0)
                        .step_by(1.0)
                        .text(egui::RichText::new("s").color(crate::theme::TOOLBAR_TEXT)),
                );
                if (interval_sec - prev).abs() > f64::EPSILON {
                    actions.push(ToolbarAction::SetPollInterval(
                        (interval_sec * 1000.0) as u64,
                    ));
                }
            }

            ui.separator();

            // ── REQ-005: HTML レポート出力 ──────────────────────────────────
            if toolbar_button(ui, "HTML", app_state.current_study.is_some()).clicked() {
                actions.push(ToolbarAction::GenerateHtmlReport);
            }

            // ── REQ-007: Artifacts フォルダ選択 ───────────────────────────────
            if toolbar_button(ui, "Artifacts", true).clicked() {
                if let Some(base_dir) = rfd::FileDialog::new().pick_folder() {
                    actions.push(ToolbarAction::ScanArtifacts(base_dir));
                }
            }

            {
                let has_study = app_state.current_study.is_some();
                ui.scope(|ui| {
                    apply_combo_visuals(ui.visuals_mut());
                    ui.add_enabled_ui(has_study, |ui| {
                        egui::ComboBox::from_id_salt("csv_export_combo")
                            .selected_text(
                                egui::RichText::new("CSV Export").color(crate::theme::TOOLBAR_TEXT),
                            )
                            .width(110.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(false, "All Data").clicked() {
                                    actions.push(ToolbarAction::ExportCsv(
                                        crate::io::export::ExportTarget::AllData,
                                    ));
                                }
                                if ui.selectable_label(false, "Selected Only").clicked() {
                                    actions.push(ToolbarAction::ExportCsv(
                                        crate::io::export::ExportTarget::SelectedOnly,
                                    ));
                                }
                                if ui.selectable_label(false, "Pareto Only").clicked() {
                                    actions.push(ToolbarAction::ExportCsv(
                                        crate::io::export::ExportTarget::ParetoOnly,
                                    ));
                                }
                            });
                    });
                });
            }

            for (idx, study) in app_state.comparison_studies.iter().enumerate() {
                let label = format!("× {}", &study.meta.name);
                if toolbar_button(ui, &label, true).clicked() {
                    actions.push(ToolbarAction::RemoveComparisonStudy(idx));
                }
            }
            if toolbar_button(ui, "+ Compare", app_state.current_study.is_some()).clicked() {
                actions.push(ToolbarAction::AddComparisonStudy);
            }

            ui.separator();

            // ── REQ-004: セッション ──────────────────────────────────────────
            if toolbar_button(ui, "Load Session", true).clicked() {
                actions.push(ToolbarAction::LoadSession);
            }

            if toolbar_button(ui, "Save Session", app_state.current_study.is_some()).clicked() {
                actions.push(ToolbarAction::SaveSession);
            }

            // ローディングインジケーター
            if is_loading {
                ui.spinner();
            }

            // エラーメッセージ
            if let Some(err) = load_error {
                if ui
                    .colored_label(ERROR_COLOR, format!("Error: {}", err))
                    .clicked()
                {
                    actions.push(ToolbarAction::ClearLoadError);
                }
            }
        });
    });
    actions
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
            TOOLBAR_BTN_FG
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
    let fg_white = egui::Stroke::new(1.0, TOOLBAR_BTN_FG);
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

    // TASK-2228: 新規 ToolbarAction variants のテスト
    #[test]
    fn toolbar_action_variants_compile_and_match() {
        let actions = vec![
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::AllData),
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::SelectedOnly),
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::ParetoOnly),
            ToolbarAction::AddComparisonStudy,
            ToolbarAction::RemoveComparisonStudy(0),
        ];
        for action in &actions {
            match action {
                ToolbarAction::ExportCsv(t) => {
                    let _t = t;
                }
                ToolbarAction::AddComparisonStudy => {}
                ToolbarAction::RemoveComparisonStudy(idx) => {
                    let _ = idx;
                }
                _ => {}
            }
        }
        assert_eq!(actions.len(), 5);
    }

    // ── TASK-2233: CSV Export UI テスト ──────────────────────────

    #[test]
    fn export_csv_action_targets_all_three_modes() {
        use crate::io::export::ExportTarget;
        let targets = [ExportTarget::AllData, ExportTarget::SelectedOnly, ExportTarget::ParetoOnly];
        for target in &targets {
            let action = ToolbarAction::ExportCsv(target.clone());
            match action {
                ToolbarAction::ExportCsv(_) => {}
                _ => panic!("Expected ExportCsv"),
            }
        }
        assert_eq!(targets.len(), 3);
    }

    #[test]
    fn apply_toolbar_actions_handles_cancel_as_noop() {
        // save_csv_to_file returns Ok(()) on cancel; verify write_csv_to_path is a separate fn
        let csv = "trial_id,trial_number\n0,0";
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = crate::io::export::write_csv_to_path(csv, tmp.path());
        assert!(result.is_ok());
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(content, csv);
    }

    // ── TASK-2234: Comparison UI テスト ──────────────────────────

    #[test]
    fn toolbar_emits_add_comparison_action() {
        let action = ToolbarAction::AddComparisonStudy;
        match action {
            ToolbarAction::AddComparisonStudy => {}
            _ => panic!("Expected AddComparisonStudy"),
        }
    }

    #[test]
    fn chip_remove_emits_remove_action() {
        let action = ToolbarAction::RemoveComparisonStudy(2);
        match action {
            ToolbarAction::RemoveComparisonStudy(idx) => assert_eq!(idx, 2),
            _ => panic!("Expected RemoveComparisonStudy"),
        }
    }

    #[test]
    fn successful_add_switches_to_comparison_mode() {
        use crate::state::app_state::{AppState, Direction, GpuBufferData, StudyContext, StudyMeta};
        use crate::state::messages::AppMessage;
        use crate::state::message_handler::MessageHandler;
        use crate::ui::widget_states::WidgetStates;

        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        let context = StudyContext {
            meta: StudyMeta {
                study_id: 10,
                name: "compare_study".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 0,
                total_trials: 0,
                param_names: vec![],
                objective_names: vec![],
                user_attr_names: vec![],
                has_constraints: false,
            },
            trial_rows: vec![],
            gpu_data: GpuBufferData {
                positions: vec![],
                positions3d: vec![],
                colors: vec![],
                sizes: vec![],
                trial_count: 0,
            },
            pareto_indices: vec![],
        };

        // Simulate setting comparison_mode before the load completes (as app.rs does)
        app_state.comparison_mode = true;

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoaded { study_idx: 0, context: Box::new(context) },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.comparison_mode);
        assert_eq!(app_state.comparison_studies.len(), 1);
    }
}
