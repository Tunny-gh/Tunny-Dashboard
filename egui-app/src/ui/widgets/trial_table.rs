use crate::state::app_state::{AppState, StudyContext, TrialRow};

/// トライアル一覧テーブルウィジェット。
/// 旧 BottomPanel の描画ロジックを PanelItem として独立させたもの。
/// グリッドキャンバスの任意のセルに D&D で配置できる。
#[derive(Default)]
pub struct TrialTableWidget;

impl TrialTableWidget {
    /// テーブルを描画する
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        if app_state.current_study.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Open a journal file");
            });
            return;
        }

        let study_ctx = app_state.current_study.as_ref().unwrap();
        let display_rows = get_display_rows(study_ctx, &app_state.selected_indices);
        let highlighted = app_state.highlighted_trial;

        let param_names = study_ctx.meta.param_names.clone();
        let obj_names = study_ctx.meta.objective_names.clone();

        use egui_extras::{Column, TableBuilder};

        let mut clicked_trial: Option<u32> = None;

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(60.0))
            .column(Column::remainder())
            .column(Column::remainder())
            .column(Column::auto().at_least(80.0))
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
            })
            .body(|body| {
                body.rows(18.0, display_rows.len(), |mut row| {
                    let trial = &display_rows[row.index()];
                    let is_highlighted = highlighted == Some(trial.trial_id);
                    let bg_color = if is_highlighted {
                        Some(egui::Color32::from_rgb(80, 120, 180))
                    } else {
                        None
                    };

                    row.col(|ui| {
                        let res = ui.selectable_label(is_highlighted, trial.trial_id.to_string());
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
                });
            });

        if let Some(trial_id) = clicked_trial {
            app_state.set_highlight(trial_id);
        }
    }
}

/// 表示対象の TrialRow を返す。
/// selected_indices が空なら全件、そうでなければ trial_id でフィルタリングする。
pub fn get_display_rows<'a>(
    study_ctx: &'a StudyContext,
    selected_indices: &[u32],
) -> Vec<&'a TrialRow> {
    if selected_indices.is_empty() {
        study_ctx.trial_rows.iter().collect()
    } else {
        let id_set: std::collections::HashSet<u32> =
            selected_indices.iter().copied().collect();
        study_ctx
            .trial_rows
            .iter()
            .filter(|r| id_set.contains(&r.trial_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{
        Direction, GpuBufferData, StudyContext, StudyMeta, TrialRow, TrialState,
    };
    use std::collections::HashMap;

    fn make_study_ctx(n: usize) -> StudyContext {
        let trial_rows: Vec<TrialRow> = (0..n as u32)
            .map(|i| TrialRow {
                trial_id: i,
                params: HashMap::new(),
                objectives: vec![i as f64],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            })
            .collect();
        StudyContext {
            meta: StudyMeta {
                study_id: 0,
                name: "test".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: n,
                total_trials: n,
                param_names: vec![],
                objective_names: vec!["y".to_string()],
                user_attr_names: vec![],
                has_constraints: false,
            },
            trial_rows,
            gpu_data: GpuBufferData {
                positions: vec![],
                positions3d: vec![],
                colors: vec![],
                sizes: vec![],
                trial_count: n as u32,
            },
            pareto_indices: vec![],
        }
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
        let rows = get_display_rows(&ctx, &[99]);
        assert_eq!(rows.len(), 0);
    }
}
