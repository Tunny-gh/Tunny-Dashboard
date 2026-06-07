// 最適化履歴
mod history;
pub use history::hv_history;
pub use history::optimization_history;

// パレート分析
mod pareto;
pub use pareto::pareto_2d;
pub use pareto::pareto_3d;

// 重要度・感度分析
mod importance;
pub use importance::importance_chart;
pub use importance::sensitivity_heatmap;

// 部分依存プロット・モデル可視化
mod pdp;
pub use pdp::pdp_2d;
pub use pdp::pdp_chart;
pub use pdp::slice_chart;
pub use pdp::surface_plot;

// 散布図・クラスタ探索
mod scatter;
pub use scatter::cluster_scatter;
pub use scatter::cluster_scatter_3d;
pub use scatter::parallel_coords;
pub use scatter::scatter_3d;
pub use scatter::scatter_matrix;

// 意思決定分析 (MCDM / AHP)
mod decision;
pub use decision::ahp_chart;
pub use decision::mcdm_chart;
pub use decision::mcdm_scatter_chart;
pub use decision::mcdm_scatter_chart_3d;

// 共通 UI 部品
mod common;
pub use common::cluster_table;
pub use common::convergence_card;
pub use common::trial_table;
