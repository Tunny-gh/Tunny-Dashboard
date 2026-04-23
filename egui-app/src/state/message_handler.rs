use crate::state::app_state::{AppState, StudyContext};
use crate::state::messages::{AppMessage, DownsampleKey};
use crate::state::results::HvHistory;
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
                app_state.cluster_result = Some(result);
            }
            AppMessage::TopsisDone(result) => {
                app_state.topsis_result = Some(result);
            }
            AppMessage::McdmDone(result) => {
                app_state.mcdm_result = Some(result);
                widget_states.mcdm_chart.computing = false;
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
            AppMessage::LiveUpdateDone { .. } | AppMessage::PdpDone { .. } => {
                // TODO: 今後のタスクで実装
            }
        }
    }
}
