use crate::state::app_state::{AppState, Direction, GpuBufferData, StudyContext, TrialRow, TrialState};
use crate::state::messages::{AppMessage, DownsampleKey};
use crate::state::results::{HvHistory, McdmResult};
use crate::ui::widget_states::WidgetStates;
use std::collections::HashMap;

/// バックグラウンドタスクからのメッセージを処理するハンドラー
pub struct MessageHandler;

impl MessageHandler {
    /// 単一メッセージを処理し、AppState と WidgetStates を更新する
    pub fn handle(
        msg: AppMessage,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
        load_error: &mut Option<String>,
    ) {
        match msg {
            AppMessage::JournalParsed { studies, path } => {
                app_state.all_studies = studies;
                app_state.journal_path = Some(path);
                *is_loading = false;
            }
            AppMessage::StudySelected {
                meta,
                trial_rows,
                gpu_data,
                pareto_indices,
            } => {
                app_state.clear();
                app_state.current_study = Some(StudyContext {
                    meta,
                    trial_rows,
                    gpu_data,
                    pareto_indices,
                });
                widget_states.hv_history.computing = false;
                widget_states.ahp_chart = Default::default();
                widget_states.cluster_scatter = Default::default();
                *is_loading = false;
                widget_states.update_chart_colors(app_state);
            }
            AppMessage::SensitivityDone { key, result } => {
                app_state.importance_cache.insert(key, result);
                widget_states.importance.computing = false;
            }
            AppMessage::SobolDone { obj_idx, result } => {
                app_state.sobol_cache.insert(obj_idx, result);
                widget_states.importance.computing = false;
            }
            AppMessage::ClusteringDone(result) => {
                Self::handle_clustering_done(result, app_state, widget_states);
            }
            AppMessage::ClusterFailed(err) => {
                Self::handle_cluster_failed(err, app_state, widget_states);
            }
            AppMessage::TopsisDone(result) => {
                app_state.topsis_result = Some(result);
            }
            AppMessage::McdmDone(result) => {
                match &result {
                    McdmResult::Topsis(r) => {
                        widget_states.mcdm_chart.cached_topsis = Some(r.clone());
                    }
                    McdmResult::Vikor(r) => {
                        widget_states.mcdm_chart.cached_vikor = Some(r.clone());
                    }
                    McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
                        widget_states.mcdm_chart.cached_promethee = Some(r.clone());
                    }
                }
                app_state.mcdm_result = Some(result);
                widget_states.mcdm_chart.computing = false;
                widget_states.update_chart_colors(app_state);
            }
            AppMessage::EntropyDone(result) => {
                widget_states.mcdm_chart.weights = result.weights.clone();
                widget_states.mcdm_chart.entropy_result = Some(result);
                widget_states.mcdm_chart.pending_entropy = false;
                widget_states.mcdm_chart.computing = false;
            }
            AppMessage::AhpDone(result) => {
                widget_states.ahp_chart.computing = false;
                app_state.ahp_result = Some(result);
            }
            AppMessage::DownsampleDone { key, indices } => match key {
                DownsampleKey::Scatter => app_state.downsample_cache.scatter = Some(indices),
                DownsampleKey::Pcp => app_state.downsample_cache.pcp = Some(indices),
                DownsampleKey::Thumbnail => app_state.downsample_cache.thumbnail = Some(indices),
                DownsampleKey::Hover => app_state.downsample_cache.hover = Some(indices),
            },
            AppMessage::HvHistoryDone {
                trial_ids,
                hv_values,
                sample_step,
            } => {
                app_state.hv_history = Some(HvHistory {
                    trial_ids,
                    hv_values,
                    sample_step,
                });
                widget_states.hv_history.computing = false;
            }
            AppMessage::Pdp2dDone(result) => {
                widget_states.pdp_2d.result = Some(result);
                widget_states.pdp_2d.computing = false;
            }
            AppMessage::Error(e) => {
                *load_error = Some(e);
                *is_loading = false;
            }
            AppMessage::SensitivityError(_e) => {
                widget_states.importance.computing = false;
            }
            AppMessage::LiveUpdateDone {
                new_trial_rows,
                updated_study_counts,
            } => {
                Self::handle_live_update_done(new_trial_rows, updated_study_counts, app_state);
            }
            AppMessage::LiveUpdateError(msg) => {
                app_state.live_update.poller_active = false;
                *load_error = Some(msg);
            }
            AppMessage::LiveUpdateMaybeComplete => {
                app_state.live_update.showing_completion_hint = true;
            }
            AppMessage::PdpDone {
                param,
                objective,
                model_type,
                result,
            } => {
                // キャッシュに挿入してから result を設定
                if let crate::state::messages::PdpResult::OneDim(ref r1d) = result {
                    widget_states.pdp_chart.insert_cache(
                        &param,
                        &objective,
                        &model_type,
                        r1d.clone(),
                    );
                }
                widget_states.pdp_chart.result = Some(result);
                widget_states.pdp_chart.computing = false;
            }

            // TASK-2112: 新規バリアント（TASK-2114 で詳細実装予定）
            AppMessage::TradeoffDone { sorted_indices } => {
                app_state.tradeoff_sorted_indices = Some(sorted_indices);
            }
            AppMessage::ComparisonStudyLoaded {
                study_idx: _,
                context,
            } => {
                app_state.comparison_studies.push(*context);
            }
            AppMessage::ArtifactsDirScanned {
                trial_artifacts,
                artifacts_dir,
            } => {
                app_state.artifact_map = trial_artifacts;
                app_state.artifacts_dir = Some(artifacts_dir);
            }
            AppMessage::HtmlReportDone { .. } => {
                // TASK-2117/2123 で実装
            }
            // TASK-1506: MCDM散布図メッセージ（散布図は同期計算のため現在は使用しない）
            AppMessage::McdmScatterComputed { .. } => {
                // 散布図ウィジェットの show() 内で同期計算済みのためno-op
            }
            AppMessage::McdmScatterComputeFailed(err) => {
                // エラーをログ出力（デバッグ用）
                #[cfg(debug_assertions)]
                eprintln!("McdmScatter compute failed: {}", err);
            }
        }
    }

    /// REQ-001: Trade-off Navigator — Chebyshev スコアを非同期計算して TradeoffDone を送信する
    pub fn trigger_tradeoff_computation(
        weights: Vec<f64>,
        is_minimize: Vec<bool>,
        tx: std::sync::mpsc::SyncSender<AppMessage>,
    ) {
        crate::app::spawn_task(tx, move || {
            let sorted_indices = tunny_core::multi_objective::pareto::score_tradeoff_navigator(
                &weights,
                &is_minimize,
            );
            AppMessage::TradeoffDone { sorted_indices }
        });
    }

    fn handle_live_update_done(
        new_core_rows: Vec<tunny_core::io::journal::live_update::TrialRow>,
        updated_study_counts: Vec<(u32, usize)>,
        app_state: &mut AppState,
    ) {
        if let Some(study) = &mut app_state.current_study {
            let base_number = study.trial_rows.len() as u32;
            for (i, core_row) in new_core_rows.iter().enumerate() {
                let app_row = TrialRow {
                    trial_id: core_row.trial_id,
                    trial_number: base_number + i as u32,
                    params: core_row.params.clone(),
                    objectives: core_row.objectives.clone(),
                    pareto_rank: 0,
                    cluster_id: None,
                    state: TrialState::Complete,
                    user_attrs: HashMap::new(),
                };
                study.trial_rows.push(app_row);
            }

            // Recompute Pareto ranks via nd_sort
            let is_minimize: Vec<bool> = study
                .meta
                .directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();
            let objectives: Vec<Vec<f64>> = study
                .trial_rows
                .iter()
                .map(|r| r.objectives.clone())
                .collect();
            let ranks = tunny_core::pareto::nd_sort(&objectives, &is_minimize);

            let mut pareto_indices = Vec::new();
            for (i, row) in study.trial_rows.iter_mut().enumerate() {
                let rank = ranks.get(i).copied().unwrap_or(0);
                row.pareto_rank = rank;
                if rank == 0 {
                    pareto_indices.push(i as u32);
                }
            }
            study.pareto_indices = pareto_indices;

            study.gpu_data = Self::build_gpu_data_from_rows(&study.trial_rows, &ranks);
        }

        // Update all_studies completed_trials
        for (study_id, new_count) in updated_study_counts {
            if let Some(meta) = app_state
                .all_studies
                .iter_mut()
                .find(|m| m.study_id == study_id)
            {
                meta.completed_trials = new_count;
            }
        }

    }

    fn build_gpu_data_from_rows(rows: &[TrialRow], ranks: &[u32]) -> GpuBufferData {
        let n = rows.len();
        let max_rank = ranks.iter().max().copied().unwrap_or(0);

        let n_obj = rows.first().map(|r| r.objectives.len()).unwrap_or(0);
        let x_scale = if n > 1 { (n - 1) as f32 } else { 1.0 };

        let mut positions = vec![0.0f32; n * 2];
        let mut positions3d = vec![0.0f32; n * 3];
        for (i, row) in rows.iter().enumerate() {
            match n_obj {
                0 => {}
                1 => {
                    positions[i * 2] = i as f32 / x_scale;
                    positions[i * 2 + 1] = row.objectives[0] as f32;
                    positions3d[i * 3] = i as f32 / x_scale;
                    positions3d[i * 3 + 1] = row.objectives[0] as f32;
                }
                _ => {
                    positions[i * 2] = row.objectives[0] as f32;
                    positions[i * 2 + 1] = row.objectives[1] as f32;
                    positions3d[i * 3] = row.objectives[0] as f32;
                    positions3d[i * 3 + 1] = row.objectives[1] as f32;
                    if n_obj >= 3 {
                        positions3d[i * 3 + 2] =
                            row.objectives.get(2).copied().unwrap_or(0.0) as f32;
                    }
                }
            }
        }

        let mut colors = Vec::with_capacity(n * 4);
        for i in 0..n {
            let rank = ranks.get(i).copied().unwrap_or(max_rank);
            let t = 1.0 - (rank as f32 / (max_rank + 1) as f32);
            colors.push(t);
            colors.push(0.5 + t * 0.5);
            colors.push(1.0 - t);
            colors.push(0.8f32);
        }

        GpuBufferData {
            positions,
            positions3d,
            colors,
            sizes: vec![1.0f32; n],
            trial_count: n as u32,
        }
    }

    fn handle_clustering_done(
        result: crate::state::results::ClusterResult,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
    ) {
        let trial_count = app_state
            .current_study
            .as_ref()
            .map(|c| c.trial_rows.len())
            .unwrap_or(0);
        if result.labels.len() == trial_count {
            app_state.cluster_result = Some(result);
            widget_states.cluster_scatter.clear_runtime_state();
        } else {
            app_state.cluster_result = None;
            widget_states
                .cluster_scatter
                .set_error(crate::state::messages::cluster_ui_error(
                    "Cluster result is inconsistent. Please run again.",
                    Some(format!(
                        "validation: labels_len({}) != trial_count({})",
                        result.labels.len(),
                        trial_count
                    )),
                    true,
                ));
        }
    }

    fn handle_cluster_failed(
        err: crate::state::messages::ClusterUiError,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
    ) {
        app_state.cluster_result = None;
        widget_states.cluster_scatter.set_error(err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{Direction, GpuBufferData, StudyMeta, TrialRow, TrialState};

    fn make_study_message(trial_count: usize) -> AppMessage {
        AppMessage::StudySelected {
            meta: StudyMeta {
                study_id: 1,
                name: "s".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: trial_count,
                total_trials: trial_count,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                user_attr_names: vec![],
                has_constraints: false,
            },
            trial_rows: (0..trial_count)
                .map(|i| TrialRow {
                    trial_id: i as u32,
                    trial_number: i as u32,
                    params: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                    objectives: vec![i as f64],
                    pareto_rank: 0,
                    cluster_id: None,
                    state: TrialState::Complete,
                    user_attrs: std::collections::HashMap::new(),
                })
                .collect(),
            gpu_data: GpuBufferData {
                positions: vec![],
                positions3d: vec![],
                colors: vec![],
                sizes: vec![],
                trial_count: trial_count as u32,
            },
            pareto_indices: vec![],
        }
    }

    #[test]
    fn clustering_done_updates_state_when_lengths_match() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        widgets.cluster_scatter.computing = true;
        MessageHandler::handle(
            AppMessage::ClusteringDone(crate::state::results::ClusterResult {
                labels: vec![0, 1, 0],
                n_clusters: 2,
            }),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.cluster_result.is_some());
        assert!(!widgets.cluster_scatter.computing);
        assert!(widgets.cluster_scatter.last_error.is_none());
    }

    #[test]
    fn clustering_done_rejects_mismatched_label_length() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        MessageHandler::handle(
            AppMessage::ClusteringDone(crate::state::results::ClusterResult {
                labels: vec![0, 1],
                n_clusters: 2,
            }),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.cluster_result.is_none());
        assert!(widgets.cluster_scatter.last_error.is_some());
    }

    fn make_core_trial_row(
        trial_id: u32,
        study_id: u32,
        objectives: Vec<f64>,
    ) -> tunny_core::io::journal::live_update::TrialRow {
        tunny_core::io::journal::live_update::TrialRow {
            trial_id,
            trial_number: trial_id,
            params: std::collections::HashMap::new(),
            param_categories: std::collections::HashMap::new(),
            objectives,
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
            study_id,
        }
    }

    #[test]
    fn live_update_done_appends_trial_rows() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_rows.len(), 3);

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![
                    make_core_trial_row(3, 1, vec![1.0]),
                    make_core_trial_row(4, 1, vec![2.0]),
                ],
                updated_study_counts: vec![(1, 5)],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.current_study.as_ref().unwrap().trial_rows.len(), 5);
    }

    #[test]
    fn live_update_done_updates_all_studies_counts() {
        let mut app_state = AppState::new();
        app_state.all_studies = vec![crate::state::app_state::StudyMeta {
            study_id: 1,
            name: "s".to_string(),
            directions: vec![],
            completed_trials: 100,
            total_trials: 100,
            param_names: vec![],
            objective_names: vec![],
            user_attr_names: vec![],
            has_constraints: false,
        }];
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![],
                updated_study_counts: vec![(1, 105)],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.all_studies[0].completed_trials, 105);
    }

    #[test]
    fn live_update_done_preserves_filter_ranges() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        app_state.filter_ranges.insert("x".to_string(), (0.0, 1.0));
        app_state.selected_indices = vec![0, 1];

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![make_core_trial_row(3, 1, vec![1.0])],
                updated_study_counts: vec![],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.filter_ranges.contains_key("x"));
        assert_eq!(app_state.selected_indices, vec![0, 1]);
    }

    #[test]
    fn live_update_error_sets_poller_inactive() {
        let mut app_state = AppState::new();
        app_state.live_update.poller_active = true;
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateError("test error".to_string()),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(!app_state.live_update.poller_active);
        assert!(load_error.is_some());
    }

    #[test]
    fn live_update_maybe_complete_sets_hint() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateMaybeComplete,
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.live_update.showing_completion_hint);
    }

    #[test]
    fn study_selected_resets_cluster_widget_runtime_state() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        widgets.cluster_scatter.computing = true;
        widgets.cluster_scatter.pending_compute =
            Some(crate::ui::widgets::cluster_scatter::ClusterComputeRequest {
                k: 3,
                target_space: crate::ui::widgets::cluster_scatter::ClusterSpace::Objective,
                k_mode: crate::ui::widgets::cluster_scatter::KSelectionMode::Manual,
                init_strategy:
                    crate::ui::widgets::cluster_scatter::KMeansInitStrategy::KMeansPlusPlus,
            });

        MessageHandler::handle(
            make_study_message(4),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(!widgets.cluster_scatter.computing);
        assert!(widgets.cluster_scatter.pending_compute.is_none());
        assert!(widgets.cluster_scatter.last_error.is_none());
    }
}
