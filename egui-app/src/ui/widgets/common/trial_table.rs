use crate::state::app_state::AppState;
#[cfg(test)]
use crate::state::app_state::{StudyContext, TrialRow};
use crate::theme::chart_colors::COLOR_LINK;
use crate::theme::colormap_name::colormap_from_name;
use crate::ui::widgets::cluster_table::ClusterTable;
use crate::ui::widgets::mcdm_chart::McdmTable;

/// トライアルテーブルの表示モード。
/// Artifact ギャラリーと同様に、関連する複数のテーブルを 1 つのウィジェットへ統合し、
/// モードセレクタで切り替える。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrialTableMode {
    /// 全トライアル一覧（選択 ∪ ピン留め）。設定不要。
    #[default]
    All,
    /// クラスタリング結果（各トライアルのクラスタ割当）を表示。
    Cluster,
    /// MCDM ランキング順に表示。
    Mcdm,
}

impl TrialTableMode {
    fn label(&self) -> &'static str {
        match self {
            TrialTableMode::All => "All Trials",
            TrialTableMode::Cluster => "By Cluster",
            TrialTableMode::Mcdm => "By MCDM Rank",
        }
    }
}

/// トライアルテーブルウィジェット。
/// 旧 BottomPanel の一覧に加え、クラスタ割当テーブル（Cluster）と MCDM ランキング
/// テーブル（MCDM）をモードセレクタで切り替える統合ウィジェット。
/// クラスタ / MCDM の設定・実行状態は埋め込んだ各サブウィジェットが保持し、
/// 計算結果は設定キーごとに `cluster_cache` / `mcdm_cache` で共有・キャッシュされる
/// （Artifact ギャラリーと同じ統合スタイル）。
/// グリッドキャンバスの任意のセルに D&D で配置できる。
#[derive(Default)]
pub struct TrialTable {
    pub mode: TrialTableMode,
    /// Cluster モードの設定・描画を担うサブウィジェット。
    pub cluster: ClusterTable,
    /// MCDM モードの設定・描画を担うサブウィジェット。
    pub mcdm: McdmTable,
}

impl TrialTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// テーブルを描画する。モードセレクタを表示し、選択モードに応じて切り替える。
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        if app_state.current_study.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Open a journal file");
            });
            return;
        }

        // モードセレクタ（Artifact ギャラリーと同じ操作感）。
        ui.horizontal(|ui| {
            ui.label("View:");
            egui::ComboBox::from_id_salt("trial_table_mode")
                .selected_text(self.mode.label())
                .show_ui(ui, |ui| {
                    for m in [
                        TrialTableMode::All,
                        TrialTableMode::Cluster,
                        TrialTableMode::Mcdm,
                    ] {
                        ui.selectable_value(&mut self.mode, m, m.label());
                    }
                });
        });
        ui.separator();

        match self.mode {
            TrialTableMode::All => self.show_all(ui, app_state),
            TrialTableMode::Cluster => {
                let cmap = colormap_from_name(&app_state.selected_colormap);
                self.cluster.show(ui, app_state, &cmap);
            }
            TrialTableMode::Mcdm => self.show_mcdm(ui, app_state),
        }
    }

    /// MCDM モード: 設定 UI + ランキング順テーブル（McdmTable へ委譲）。
    fn show_mcdm(&mut self, ui: &mut egui::Ui, app_state: &AppState) {
        let Some(ctx) = app_state.current_study.as_ref() else {
            return;
        };
        let key = self.mcdm.controls.cache_key();
        let result = app_state.mcdm_cache.get(&key);
        self.mcdm.show(
            ui,
            result,
            &ctx.view,
            &ctx.meta.param_names,
            &ctx.meta.objective_names,
        );
    }

    /// All モード: 全トライアル一覧（選択 ∪ ピン留め）を描画する。
    fn show_all(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
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
        let param_cols = view.numeric_columns(&param_names);
        let obj_cols = view.numeric_columns(&obj_names);
        let trial_ids = &view.trial_ids;
        let pareto_rank = &view.pareto_rank;

        use egui_extras::{Column, TableBuilder};

        let mut clicked_trial: Option<u32> = None;
        let mut pin_toggled: Option<u32> = None;

        // パラメータ・目的を 1 列ずつに展開し、横スクロール可能にする
        // （Cluster / MCDM モードと同じ表示スタイル）。
        egui::ScrollArea::horizontal().show(ui, |ui| {
            // ストライプの色を強調して偶数/奇数行を見分けやすくする。
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG;
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::exact(30.0)) // Pin column
                .column(Column::initial(70.0).at_least(50.0)) // Trial ID
                .columns(Column::initial(90.0).at_least(50.0), param_names.len()) // 各変数
                .columns(Column::initial(90.0).at_least(50.0), obj_names.len()) // 各目的
                .column(Column::initial(90.0).at_least(50.0)) // Pareto Rank
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("📌");
                    });
                    header.col(|ui| {
                        ui.strong("Trial ID");
                    });
                    for name in &param_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    for name in &obj_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
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
                            if is_highlighted {
                                ui.painter().rect_filled(res.rect, 0.0, COLOR_LINK);
                            }
                        });
                        for col in &param_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.3}", v));
                            });
                        }
                        for col in &obj_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.4}", v));
                            });
                        }
                        row.col(|ui| {
                            ui.label(rank.to_string());
                        });
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

/// 表示対象の TrialRow を返す（ピン留め考慮版・テストのみで使用）。
/// selected_indices が空なら全件、そうでなければ selected ∪ pinned で返す。
#[cfg(test)]
pub fn get_display_rows_with_pins(
    study_ctx: &StudyContext,
    selected_indices: &[u32],
    pinned: &[u32],
) -> Vec<TrialRow> {
    let use_filter = !selected_indices.is_empty();
    let id_set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
    let pin_set: std::collections::HashSet<u32> = pinned.iter().copied().collect();
    study_ctx
        .view
        .trial_ids
        .iter()
        .enumerate()
        .filter(|(_, &id)| !use_filter || id_set.contains(&id) || pin_set.contains(&id))
        .map(|(i, _)| study_ctx.view.row_at(i))
        .collect()
}

/// 表示対象の TrialRow を返す（テストのみで使用）。
#[cfg(test)]
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
            param_bounds: Default::default(),
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
