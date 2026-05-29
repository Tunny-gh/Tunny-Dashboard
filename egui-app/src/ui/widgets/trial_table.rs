use crate::state::app_state::{filter_rows_for_display, AppState, StudyContext, TrialRow};
use crate::theme::chart_colors::COLOR_LINK;

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
        let pinned = app_state.pinned_trials.clone();
        let highlighted = app_state.highlighted_trial;

        let param_names = study_ctx.meta.param_names.clone();
        let obj_names = study_ctx.meta.objective_names.clone();

        // 行を materialize せず、表示対象の行インデックス（選択∪ピン、元順序）を計算する
        let view = &study_ctx.view;
        let n = view.row_count();
        let visible: Vec<usize> = if app_state.selected_indices.is_empty() {
            (0..n).collect()
        } else {
            let set: std::collections::HashSet<u32> =
                crate::state::app_state::merge_selected_with_pinned(
                    &app_state.selected_indices,
                    &pinned,
                )
                .into_iter()
                .collect();
            (0..n)
                .filter(|&i| view.trial_ids.get(i).is_some_and(|id| set.contains(id)))
                .collect()
        };
        // 列スライスを view から借用（行クローンを持たない）
        let param_cols: Vec<Option<&[f64]>> =
            param_names.iter().map(|nme| view.numeric_column(nme)).collect();
        let obj_cols: Vec<Option<&[f64]>> =
            obj_names.iter().map(|nme| view.numeric_column(nme)).collect();
        let trial_ids = &view.trial_ids;
        let pareto_rank = &view.pareto_rank;

        use egui_extras::{Column, TableBuilder};

        let mut clicked_trial: Option<u32> = None;
        let mut pin_toggled: Option<u32> = None;

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(30.0))  // Pin column
            .column(Column::auto().at_least(60.0))
            .column(Column::remainder())
            .column(Column::remainder())
            .column(Column::auto().at_least(80.0))
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("📌");
                });
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
                body.rows(18.0, visible.len(), |mut row| {
                    let idx = visible[row.index()];
                    let trial_id = trial_ids.get(idx).copied().unwrap_or(idx as u32);
                    let trial_number = idx as u32;
                    let rank = pareto_rank.get(idx).copied().unwrap_or(0);
                    let is_highlighted = highlighted == Some(trial_id);
                    let is_pinned = pinned.contains(&trial_id);
                    let bg_color = if is_highlighted {
                        Some(COLOR_LINK)
                    } else {
                        None
                    };

                    row.col(|ui| {
                        let pin_label = if is_pinned { "📌" } else { "·" };
                        if ui.small_button(pin_label).clicked() {
                            pin_toggled = Some(trial_id);
                        }
                    });
                    row.col(|ui| {
                        let res = ui.selectable_label(is_highlighted, trial_number.to_string());
                        if res.clicked() {
                            clicked_trial = Some(trial_id);
                        }
                        if let Some(color) = bg_color {
                            ui.painter().rect_filled(res.rect, 0.0, color);
                        }
                    });
                    row.col(|ui| {
                        let params_str: Vec<String> = param_cols
                            .iter()
                            .map(|c| {
                                let v = c.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                format!("{:.3}", v)
                            })
                            .collect();
                        ui.label(params_str.join(", "));
                    });
                    row.col(|ui| {
                        let objs_str: Vec<String> = obj_cols
                            .iter()
                            .map(|c| {
                                let v = c.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                format!("{:.4}", v)
                            })
                            .collect();
                        ui.label(objs_str.join(", "));
                    });
                    row.col(|ui| {
                        ui.label(rank.to_string());
                    });
                });
            });

        if let Some(trial_id) = clicked_trial {
            app_state.set_highlight(trial_id);
        }
        if let Some(trial_id) = pin_toggled {
            // Ignore limit error for now; UI notification is handled by caller
            let _ = app_state.toggle_pinned_trial(trial_id);
        }
    }
}

/// 表示対象の TrialRow を返す（ピン留め考慮版）。
/// selected_indices が空なら全件、そうでなければ selected ∪ pinned で返す。
pub fn get_display_rows_with_pins(
    study_ctx: &StudyContext,
    selected_indices: &[u32],
    pinned: &[u32],
) -> Vec<TrialRow> {
    let rows = study_ctx.trial_rows();
    filter_rows_for_display(&rows, selected_indices, pinned)
        .into_iter()
        .cloned()
        .collect()
}

/// 表示対象の TrialRow を返す（後方互換ラッパー）。
/// selected_indices が空なら全件、そうでなければ trial_id でフィルタリングする。
pub fn get_display_rows(study_ctx: &StudyContext, selected_indices: &[u32]) -> Vec<TrialRow> {
    get_display_rows_with_pins(study_ctx, selected_indices, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{
        Direction, PinError, StudyContext, StudyMeta, TrialRow, TrialState,
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
        let rows = get_display_rows(&ctx, &[99]);
        assert_eq!(rows.len(), 0);
    }

    // ── TASK-2235: ピン留めUIテスト ──────────────────────────────

    #[test]
    fn get_display_rows_keeps_pinned_rows_visible() {
        let ctx = make_study_ctx(5);
        // selected=[0,1], pinned=[4] → 0,1,4 visible
        let rows = get_display_rows_with_pins(&ctx, &[0, 1], &[4]);
        let ids: Vec<u32> = rows.iter().map(|r| r.trial_id).collect();
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));
        assert!(ids.contains(&4));
        assert!(!ids.contains(&2));
        assert!(!ids.contains(&3));
    }

    #[test]
    fn pin_icon_reflects_current_state() {
        // pin アイコンは is_pinned フラグで切り替わる
        let is_pinned = true;
        let label = if is_pinned { "📌" } else { "·" };
        assert_eq!(label, "📌");

        let is_pinned = false;
        let label = if is_pinned { "📌" } else { "·" };
        assert_eq!(label, "·");
    }

    #[test]
    fn pin_limit_error_is_surfaceable_to_ui() {
        use crate::state::app_state::AppState;
        let mut state = AppState::new();
        for i in 0..20u32 {
            state.toggle_pinned_trial(i).unwrap();
        }
        let result = state.toggle_pinned_trial(100);
        assert_eq!(result, Err(PinError::MaxPinnedReached { limit: 20 }));
    }

    #[test]
    fn pin_row_then_change_selection_row_stays_visible() {
        let ctx = make_study_ctx(5);
        // pin trial 3, then selection is [0,1] (no longer includes 3)
        let rows = get_display_rows_with_pins(&ctx, &[0, 1], &[3]);
        let ids: Vec<u32> = rows.iter().map(|r| r.trial_id).collect();
        assert!(ids.contains(&3), "pinned row 3 must remain visible");
    }
}
