use crate::state::app_state::{AppState, StudyContext};
use crate::state::messages::{AppMessage, DownsampleKey};
use crate::state::results::{HvHistory, McdmResult};
use crate::ui::widget_states::WidgetStates;

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
                app_state.update_chart_colors();
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
            AppMessage::LiveUpdateDone { .. } => {
                // TODO: 今後のタスクで実装
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
