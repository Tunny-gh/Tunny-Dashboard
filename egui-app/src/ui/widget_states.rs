use crate::ui::widgets::{
    ahp_chart::AhpChart, cluster_scatter::ClusterScatter, hv_history::HvHistoryChart,
    importance_chart::ImportanceChart, mcdm_chart::McdmRankChart, mcdm_chart::McdmTable,
    mcdm_scatter_chart::McdmScatterChart, optimization_history::OptimizationHistoryChart,
    parallel_coords::ParallelCoordsChart, pareto_2d::ParetoScatter2D, pareto_3d::Pareto3dChart,
    pdp_2d::PdpChart2DState, pdp_chart::PdpChart, scatter_matrix::ScatterMatrix,
    sensitivity_heatmap::SensitivityHeatmap, slice_chart::SliceChart,
};
use crate::{state::app_state::AppState, theme::color_compute::compute_chart_colors};

/// Bottom Panel のタブ種別
#[derive(Default, PartialEq, Clone)]
pub enum BottomTab {
    #[default]
    Trials,
    BestHistory,
}

/// 各チャートウィジェットの UI 状態をまとめて保持する
/// AppState（データ）とは分離した純粋な UI 状態
#[derive(Default)]
pub struct WidgetStates {
    pub pareto_2d: ParetoScatter2D,
    pub pareto_3d: Pareto3dChart,
    pub opt_history: OptimizationHistoryChart,
    pub hv_history: HvHistoryChart,
    pub importance: ImportanceChart,
    pub pdp_chart: PdpChart,
    pub pdp_2d: PdpChart2DState,
    pub parallel_coords: ParallelCoordsChart,
    pub scatter_matrix: ScatterMatrix,
    pub sensitivity_heatmap: SensitivityHeatmap,
    pub cluster_scatter: ClusterScatter,
    pub mcdm_chart: McdmRankChart,
    pub mcdm_table: McdmTable,
    pub ahp_chart: AhpChart,
    pub slice_chart: SliceChart,
    // TASK-1504: MCDM 散布図ウィジェット
    pub scatter_chart: McdmScatterChart,
    // TASK-2121: Artifacts modal state
    pub artifact_modal_open: bool,
    pub artifact_modal_trial_id: Option<u32>,
    // TASK-2123: Bottom panel tab
    pub bottom_tab: BottomTab,
    /// チャート描画用の色キャッシュ（UI専用）
    pub chart_colors: Vec<egui::Color32>,
}

impl WidgetStates {
    pub fn update_chart_colors(&mut self, app_state: &AppState) {
        if let Some(ctx) = &app_state.current_study {
            let color_mode = app_state.color_mode.clone();
            let colormap_name = app_state.selected_colormap.clone();
            let trial_rows = &ctx.trial_rows;
            let objective_names = &ctx.meta.objective_names;
            let mcdm_scores = app_state.mcdm_result.as_ref().map(|r| r.primary_scores());
            self.chart_colors = compute_chart_colors(
                &color_mode,
                &colormap_name,
                trial_rows,
                objective_names,
                mcdm_scores,
            );
        } else {
            self.chart_colors.clear();
        }
    }
}
