use crate::state::app_state::{AppState, StudyContext, TrialRow};
use crate::theme::chart_colors::COLOR_LINK;
use crate::ui::widget_states::{BottomTab, WidgetStates};

/// trial の artifacts ファイルリストから最初のファイルタイプに基づくアイコンを返す
fn artifact_icon(files: &[std::path::PathBuf]) -> &'static str {
    if files.is_empty() {
        return "";
    }
    use crate::io::artifacts::ArtifactFileType;
    match ArtifactFileType::from_path(&files[0]) {
        ArtifactFileType::Image => "[IMG]",
        ArtifactFileType::Csv => "[CSV]",
        ArtifactFileType::Other => "[FILE]",
    }
}

/// BottomPanel を描画する
pub fn show_bottom_panel(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widget_states: &mut WidgetStates,
) {
    if app_state.current_study.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label("Open a journal file");
        });
        return;
    }

    let obj_count = app_state
        .current_study
        .as_ref()
        .map(|s| s.meta.objective_names.len())
        .unwrap_or(0);
    let obj_names_top: Vec<String> = app_state
        .current_study
        .as_ref()
        .map(|s| s.meta.objective_names.clone())
        .unwrap_or_default();

    // REQ-008-E: タブ切り替え（単目的の場合は Best 遷移タブを追加）
    ui.horizontal(|ui| {
        ui.selectable_value(&mut widget_states.bottom_tab, BottomTab::Trials, "Trials");
        if obj_count == 1 {
            ui.selectable_value(
                &mut widget_states.bottom_tab,
                BottomTab::BestHistory,
                "Best History",
            );
        }
    });
    ui.separator();

    // 多目的に切り替わった時に BestHistory タブが残らないようリセット
    if obj_count != 1 && widget_states.bottom_tab == BottomTab::BestHistory {
        widget_states.bottom_tab = BottomTab::Trials;
    }

    match widget_states.bottom_tab {
        BottomTab::BestHistory => {
            show_best_history_table(ui, app_state, &obj_names_top);
            return;
        }
        BottomTab::Trials => {}
    }

    let study_ctx = app_state.current_study.as_ref().unwrap();
    let display_rows = get_display_rows(study_ctx, &app_state.selected_indices);
    let highlighted = app_state.highlighted_trial;

    // ヘッダーカラム名を先に取得
    let param_names = study_ctx.meta.param_names.clone();
    let obj_names = study_ctx.meta.objective_names.clone();
    let show_artifacts_col = !app_state.artifact_map.is_empty();

    use egui_extras::{Column, TableBuilder};

    let mut clicked_trial: Option<u32> = None;
    let mut artifact_clicked: Option<u32> = None;

    let mut builder = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .column(Column::auto().at_least(60.0)) // trial_id
        .column(Column::remainder()) // パラメータ
        .column(Column::remainder()) // 目的値
        .column(Column::auto().at_least(80.0)); // Pareto ランク
    if show_artifacts_col {
        builder = builder.column(Column::auto().at_least(80.0)); // Artifacts
    }
    builder
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Trial ID");
            });
            header.col(|ui| {
                ui.strong(format!("Parameters ({})", param_names.len()));
            });
            header.col(|ui| {
                ui.strong(format!("Objectives ({})", obj_names.len()));
            });
            header.col(|ui| {
                ui.strong("Pareto Rank");
            });
            if show_artifacts_col {
                header.col(|ui| {
                    ui.strong("Artifacts");
                });
            }
        })
        .body(|body| {
            body.rows(18.0, display_rows.len(), |mut row| {
                let trial = &display_rows[row.index()];
                let is_highlighted = highlighted == Some(trial.trial_id);
                let bg_color = if is_highlighted {
                    Some(COLOR_LINK)
                } else {
                    None
                };

                row.col(|ui| {
                    let res = ui.selectable_label(is_highlighted, trial.trial_number.to_string());
                    if res.clicked() {
                        clicked_trial = Some(trial.trial_id);
                    }
                    if let Some(color) = bg_color {
                        ui.painter().rect_filled(res.rect, 0.0, color);
                    }
                });
                row.col(|ui| {
                    let params_str: Vec<String> = param_names
                        .iter()
                        .map(|n| {
                            let v = trial.params.get(n).copied().unwrap_or(0.0);
                            format!("{:.3}", v)
                        })
                        .collect();
                    ui.label(params_str.join(", "));
                });
                row.col(|ui| {
                    let objs_str: Vec<String> = trial
                        .objectives
                        .iter()
                        .map(|v| format!("{:.4}", v))
                        .collect();
                    ui.label(objs_str.join(", "));
                });
                row.col(|ui| {
                    ui.label(trial.pareto_rank.to_string());
                });
                if show_artifacts_col {
                    row.col(|ui| {
                        if let Some(files) = app_state.artifact_map.get(&trial.trial_id) {
                            let icon = artifact_icon(files);
                            if !icon.is_empty() && ui.button(icon).clicked() {
                                artifact_clicked = Some(trial.trial_id);
                            }
                        }
                    });
                }
            });
        });

    if let Some(trial_id) = clicked_trial {
        app_state.set_highlight(trial_id);
    }
    if let Some(trial_id) = artifact_clicked {
        widget_states.artifact_modal_trial_id = Some(trial_id);
        widget_states.artifact_modal_open = true;
    }
}

// ============================================================
// TASK-2123: Best 解遷移テーブル
// ============================================================

/// best_trial_history から Best 値が更新されたエントリのみを抽出する
pub fn extract_best_entries(history: &[(u32, f64)]) -> Vec<(u32, f64)> {
    let mut entries = Vec::new();
    let mut last_best: Option<f64> = None;
    for &(trial_id, best_val) in history {
        let is_new_best = last_best.is_none_or(|prev| best_val < prev);
        if is_new_best {
            entries.push((trial_id, best_val));
            last_best = Some(best_val);
        }
    }
    entries
}

/// Best 解遷移テーブル（単目的 Study 専用）
fn show_best_history_table(ui: &mut egui::Ui, app_state: &AppState, objective_names: &[String]) {
    let history = match &app_state.best_trial_history {
        Some(h) if !h.is_empty() => h,
        _ => {
            ui.label("No best history data");
            return;
        }
    };

    let best_entries = extract_best_entries(history);

    let top_param_names: Vec<String> = app_state
        .current_study
        .as_ref()
        .map(|s| s.meta.param_names.iter().take(5).cloned().collect())
        .unwrap_or_default();

    let obj_name = objective_names
        .first()
        .map(|s| s.as_str())
        .unwrap_or("Objective");

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("best_history_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Trial ID");
                ui.strong(obj_name);
                ui.strong("Delta");
                for name in &top_param_names {
                    ui.strong(name);
                }
                ui.end_row();

                let mut prev_val: Option<f64> = None;
                for &(trial_id, best_val) in &best_entries {
                    let delta = prev_val.map(|p| best_val - p).unwrap_or(0.0);
                    prev_val = Some(best_val);

                    ui.label(format!("#{trial_id}"));
                    ui.label(format!("{:.6}", best_val));
                    ui.label(if delta == 0.0 {
                        "-".to_string()
                    } else {
                        format!("{:+.6}", delta)
                    });

                    // trial_id に対応する試行データから変数値を取得
                    let trial_opt = app_state
                        .current_study
                        .as_ref()
                        .and_then(|s| s.trial_rows().into_iter().find(|t| t.trial_id == trial_id));
                    if let Some(trial) = trial_opt {
                        for name in &top_param_names {
                            let val = trial
                                .params
                                .get(name)
                                .map(|v| format!("{:.4}", v))
                                .unwrap_or_else(|| "-".to_string());
                            ui.label(val);
                        }
                    } else {
                        for _ in &top_param_names {
                            ui.label("-");
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

/// 表示対象の TrialRow を返す。
/// selected_indices が空なら全件、そうでなければ trial_id でフィルタリングする。
pub fn get_display_rows(study_ctx: &StudyContext, selected_indices: &[u32]) -> Vec<TrialRow> {
    let rows = study_ctx.trial_rows();
    if selected_indices.is_empty() {
        rows
    } else {
        let id_set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
        rows.into_iter()
            .filter(|r| id_set.contains(&r.trial_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{
        Direction, StudyContext, StudyMeta, TrialRow, TrialState,
    };
    use std::collections::HashMap;

    fn make_study_ctx(n: usize) -> StudyContext {
        let trial_rows: Vec<TrialRow> = (0..n as u32)
            .map(|i| TrialRow {
                trial_id: i,
                trial_number: i,
                params: HashMap::new(),
                objectives: vec![i as f64],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            })
            .collect();
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: n,
            total_trials: n,
            param_names: vec![],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
        };
        StudyContext::from_rows_for_test(meta, trial_rows)
    }

    #[test]
    fn get_display_rows_empty_selected_returns_all() {
        let ctx = make_study_ctx(5);
        let rows = get_display_rows(&ctx, &[]);
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn get_display_rows_filters_by_trial_id() {
        let ctx = make_study_ctx(5);
        let rows = get_display_rows(&ctx, &[0, 2, 4]);
        assert_eq!(rows.len(), 3);
        let ids: Vec<u32> = rows.iter().map(|r| r.trial_id).collect();
        assert!(ids.contains(&0));
        assert!(ids.contains(&2));
        assert!(ids.contains(&4));
        assert!(!ids.contains(&1));
        assert!(!ids.contains(&3));
    }

    #[test]
    fn get_display_rows_nonexistent_id_excluded() {
        let ctx = make_study_ctx(3);
        // trial IDs are 0, 1, 2 — requesting ID 99 should return 0 rows
        let rows = get_display_rows(&ctx, &[99]);
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn set_highlight_updates_highlighted_trial() {
        let mut state = AppState::new();
        state.set_highlight(42);
        assert_eq!(state.highlighted_trial, Some(42));
    }

    #[test]
    fn set_highlight_overwrites_previous() {
        let mut state = AppState::new();
        state.set_highlight(1);
        state.set_highlight(5);
        assert_eq!(state.highlighted_trial, Some(5));
    }

    // TASK-2121 tests
    #[test]
    fn artifact_icon_image() {
        let files = vec![std::path::PathBuf::from("result.png")];
        let icon = artifact_icon(&files);
        assert!(!icon.is_empty());
    }

    #[test]
    fn artifact_icon_csv() {
        let files_img = vec![std::path::PathBuf::from("a.png")];
        let files_csv = vec![std::path::PathBuf::from("data.csv")];
        assert_ne!(artifact_icon(&files_img), artifact_icon(&files_csv));
    }

    #[test]
    fn artifact_icon_empty() {
        let icon = artifact_icon(&[]);
        assert_eq!(icon, "");
    }

    #[test]
    fn show_artifacts_col_false_when_empty() {
        let state = AppState::new();
        let show_artifacts_col = !state.artifact_map.is_empty();
        assert!(!show_artifacts_col);
    }

    // TASK-2123 tests
    #[test]
    fn extract_best_entries_basic() {
        let history = vec![(0u32, 1.0_f64), (1, 1.0), (2, 0.8), (3, 0.8), (4, 0.5)];
        let entries = extract_best_entries(&history);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], (0, 1.0));
        assert_eq!(entries[1], (2, 0.8));
        assert_eq!(entries[2], (4, 0.5));
    }

    #[test]
    fn extract_best_entries_empty() {
        let entries = extract_best_entries(&[]);
        assert!(entries.is_empty());
    }
}
