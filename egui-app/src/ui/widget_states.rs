use crate::ui::widgets::{
    hv_history::HvHistoryChart, importance_chart::ImportanceChart,
    optimization_history::OptimizationHistoryChart, pareto_2d::ParetoScatter2D,
    pdp_2d::PdpChart2DState, pdp_chart::PdpChart,
};

/// 各チャートウィジェットの UI 状態をまとめて保持する
/// AppState（データ）とは分離した純粋な UI 状態
pub struct WidgetStates {
    pub pareto_2d: ParetoScatter2D,
    pub opt_history: OptimizationHistoryChart,
    pub hv_history: HvHistoryChart,
    pub importance: ImportanceChart,
    pub pdp_chart: PdpChart,
    pub pdp_2d: PdpChart2DState,
}

impl Default for WidgetStates {
    fn default() -> Self {
        Self {
            pareto_2d: ParetoScatter2D::default(),
            opt_history: OptimizationHistoryChart::default(),
            hv_history: HvHistoryChart::default(),
            importance: ImportanceChart::default(),
            pdp_chart: PdpChart::default(),
            pdp_2d: PdpChart2DState::default(),
        }
    }
}
