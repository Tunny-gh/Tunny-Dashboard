use crate::ui::widgets::{
    cluster_scatter::ClusterScatter, hv_history::HvHistoryChart, importance_chart::ImportanceChart,
    mcdm_chart::McdmRankChart, mcdm_chart::McdmTable,
    optimization_history::OptimizationHistoryChart, parallel_coords::ParallelCoordsChart,
    pareto_2d::ParetoScatter2D, pareto_3d::Pareto3dChart, pdp_2d::PdpChart2DState,
    pdp_chart::PdpChart, scatter_matrix::ScatterMatrix, sensitivity_heatmap::SensitivityHeatmap,
};

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
    // TASK-2121: Artifacts modal state
    pub artifact_modal_open: bool,
    pub artifact_modal_trial_id: Option<u32>,
    // TASK-2123: Bottom panel tab
    pub bottom_tab: BottomTab,
}
