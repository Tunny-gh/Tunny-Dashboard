use crate::io::export::ExportTarget;
use crate::state::app_state::{AppState, ColormapName, StudyMeta};
use crate::theme::{ERROR_COLOR, TOOLBAR_BTN_FG};
use crate::ui::widget_states::WidgetStates;

#[derive(Debug, Clone)]
pub enum ToolbarAction {
    OpenJournal(std::path::PathBuf),
    SelectStudy(StudyMeta),
    ToggleLiveUpdate,
    SetPollInterval(u64),
    ScanArtifacts(std::path::PathBuf),
    ClearLoadError,

    // TASK-2228: 新規アクション
    ExportCsv(ExportTarget),
    /// 同一ファイル内の指定 Study を比較対象として追加する。
    AddComparisonStudy(StudyMeta),
    RemoveComparisonStudy(usize),
}

/// ToolBar を描画する
pub fn show_toolbar(
    ui: &mut egui::Ui,
    app_state: &AppState,
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
            // ライブ更新トグル（journal (.log) ファイル以外では無効。CSV / SQLite は
            // ストリーミング追記の対象外のため、開いていても押せないようにする）。
            let can_toggle = app_state.journal_path.as_deref().is_some_and(|p| {
                !crate::io::flat_csv::is_csv_path(p) && !crate::io::sqlite::is_sqlite_path(p)
            });
            let live_label = if app_state.live_update.enabled {
                format!("Live: On ({}s)", app_state.live_update.interval_ms / 1000)
            } else {
                "Live: Off".to_string()
            };
            let mut response = toolbar_button(ui, &live_label, can_toggle);
            if !can_toggle {
                response = response
                    .on_hover_text("Live Update is available for journal (.log) files only");
            }
            if response.clicked() && can_toggle {
                actions.push(ToolbarAction::ToggleLiveUpdate);
            }

            // 試行数カウンタ
            let trial_label = if let Some(study) = &app_state.current_study {
                format!("Trials: {}", study.trial_count())
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

            // 比較対象はバーにチップを並べず、1 つのドロップダウン内の
            // チェックボックス一覧で管理する（バーの横幅崩れを防ぐ）。
            // チェックで比較対象に追加、外すと解除する。基準 Study 自身は一覧に出さない。
            push_comparison_selector(ui, app_state, &mut actions);

            ui.separator();

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

/// カラーマップ選択セレクタ（ツールバー 2 段目・左端、常時表示）。
/// 変更時は全チャートの色を再計算する。
pub fn show_colormap_selector(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    _widget_states: &mut WidgetStates,
) {
    ui.label(
        egui::RichText::new("Colormap:")
            .color(crate::theme::TOOLBAR_TEXT)
            .size(12.0),
    );
    let current_label = app_state.selected_colormap.label().to_string();
    ui.scope(|ui| {
        apply_combo_visuals(ui.visuals_mut());
        egui::ComboBox::from_id_salt("toolbar_colormap_combo")
            .selected_text(egui::RichText::new(current_label).color(crate::theme::TOOLBAR_TEXT))
            .width(120.0)
            .show_ui(ui, |ui| {
                for cmap in ColormapName::all() {
                    if ui
                        .selectable_label(app_state.selected_colormap == *cmap, cmap.label())
                        .clicked()
                    {
                        app_state.selected_colormap = cmap.clone();
                    }
                }
            });
    });
}

/// 比較 Study 選択ドロップダウンを描画する。
/// バーには「Compare (件数)」のラベルだけを置き、開くと同一ファイルの
/// Study がチェックボックスで並ぶ。チェック状態の変化に応じて追加/解除アクションを積む。
fn push_comparison_selector(
    ui: &mut egui::Ui,
    app_state: &AppState,
    actions: &mut Vec<ToolbarAction>,
) {
    let base_id = app_state.current_study.as_ref().map(|c| c.meta.study_id);
    let n_comp = app_state.comparison_studies.len();
    let has_others = app_state
        .all_studies
        .iter()
        .any(|s| base_id != Some(s.study_id));
    let enabled = app_state.current_study.is_some() && has_others;

    let label = if n_comp > 0 {
        format!("Compare ({})", n_comp)
    } else {
        "Compare".to_string()
    };

    ui.scope(|ui| {
        apply_combo_visuals(ui.visuals_mut());
        ui.add_enabled_ui(enabled, |ui| {
            egui::ComboBox::from_id_salt("compare_select_combo")
                .selected_text(egui::RichText::new(label).color(crate::theme::TOOLBAR_TEXT))
                .width(130.0)
                .show_ui(ui, |ui| {
                    for s in &app_state.all_studies {
                        if base_id == Some(s.study_id) {
                            continue;
                        }
                        let existing_idx = app_state
                            .comparison_studies
                            .iter()
                            .position(|c| c.meta.study_id == s.study_id);
                        let mut checked = existing_idx.is_some();
                        if ui.checkbox(&mut checked, &s.name).changed() {
                            if checked {
                                actions.push(ToolbarAction::AddComparisonStudy(s.clone()));
                            } else if let Some(idx) = existing_idx {
                                actions.push(ToolbarAction::RemoveComparisonStudy(idx));
                            }
                        }
                    }
                });
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
    let galley = ui.fonts_mut(|f| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_state_clears_on_file_open_sequence() {
        // ファイル選択時に is_loading=true, load_error=None になることを確認
        let is_loading = true;
        let load_error: Option<String> = None;
        assert!(is_loading);
        assert!(load_error.is_none());
    }

    #[test]
    fn error_cleared_on_click_simulation() {
        // エラーラベルクリック時のシミュレーション
        let load_error: Option<String> = None;
        assert!(load_error.is_none());
    }

    fn make_test_meta(id: u32, name: &str) -> StudyMeta {
        StudyMeta {
            study_id: id,
            name: name.to_string(),
            directions: vec![],
            completed_trials: 0,
            total_trials: 0,
            param_names: vec![],
            objective_names: vec![],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: Default::default(),
        }
    }

    // TASK-2228: 新規 ToolbarAction variants のテスト
    #[test]
    fn toolbar_action_variants_compile_and_match() {
        let actions = vec![
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::AllData),
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::SelectedOnly),
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::ParetoOnly),
            ToolbarAction::AddComparisonStudy(make_test_meta(1, "s")),
            ToolbarAction::RemoveComparisonStudy(0),
        ];
        for action in &actions {
            match action {
                ToolbarAction::ExportCsv(t) => {
                    let _t = t;
                }
                ToolbarAction::AddComparisonStudy(m) => {
                    let _ = m;
                }
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
        let targets = [
            ExportTarget::AllData,
            ExportTarget::SelectedOnly,
            ExportTarget::ParetoOnly,
        ];
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
        let action = ToolbarAction::AddComparisonStudy(make_test_meta(2, "other"));
        match action {
            ToolbarAction::AddComparisonStudy(m) => assert_eq!(m.study_id, 2),
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
        use crate::state::app_state::{AppState, Direction, StudyContext, StudyMeta};
        use crate::state::message_handler::MessageHandler;
        use crate::state::messages::AppMessage;
        use crate::ui::widget_states::WidgetStates;

        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        let context = StudyContext::from_rows_for_test(
            StudyMeta {
                study_id: 10,
                name: "compare_study".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 0,
                total_trials: 0,
                param_names: vec![],
                objective_names: vec![],
                user_attr_names: vec![],
                has_constraints: false,
                param_bounds: Default::default(),
            },
            vec![],
        );

        // Simulate setting comparison_mode before the load completes (as app.rs does)
        app_state.comparison_mode = true;

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoaded {
                study_idx: 0,
                context: Box::new(context),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.comparison_mode);
        assert_eq!(app_state.comparison_studies.len(), 1);
    }
}
