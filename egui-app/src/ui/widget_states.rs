use crate::ui::widgets::{
    cluster_scatter::ClusterScatter, hv_history::HvHistoryChart,
    importance_chart::ImportanceChart, optimization_history::OptimizationHistoryChart,
    parallel_coords::ParallelCoordsChart, pareto_2d::ParetoScatter2D, pdp_2d::PdpChart2DState,
    pdp_chart::PdpChart, scatter_matrix::ScatterMatrix,
    sensitivity_heatmap::SensitivityHeatmap,
};

/// 各チャートウィジェットの UI 状態をまとめて保持する
/// AppState（データ）とは分離した純粋な UI 状態
#[derive(Default)]
pub struct WidgetStates {
    pub pareto_2d: ParetoScatter2D,
    pub opt_history: OptimizationHistoryChart,
    pub hv_history: HvHistoryChart,
    pub importance: ImportanceChart,
    pub pdp_chart: PdpChart,
    pub pdp_2d: PdpChart2DState,
    pub parallel_coords: ParallelCoordsChart,
    pub scatter_matrix: ScatterMatrix,
    pub sensitivity_heatmap: SensitivityHeatmap,
    pub cluster_scatter: ClusterScatter,
}
