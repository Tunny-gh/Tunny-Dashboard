// 最適化履歴
mod history;
pub use history::convergence;
pub use history::edf_plot;
pub use history::intermediate_values;
pub use history::optimization_history;
pub use history::timeline;

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

// サロゲート最適化（応答曲面作成＋曲面上の最適化）
mod surrogate;
pub use surrogate::response_surface;
pub use surrogate::robustness;
pub use surrogate::surrogate_opt;

// 散布図・クラスタ探索
mod scatter;
pub use scatter::cluster_scatter;
pub use scatter::cluster_scatter_3d;
pub use scatter::dendrogram;
pub use scatter::observed_contour;
pub use scatter::parallel_coords;
pub use scatter::pca_biplot;
pub use scatter::rank_plot;
pub use scatter::scatter_3d;
pub use scatter::scatter_matrix;
pub use scatter::som_map;

// 意思決定分析 (MCDM)
mod decision;
pub use decision::comparison_table;
pub use decision::mcdm_chart;
pub use decision::mcdm_scatter_chart;
pub use decision::mcdm_scatter_chart_3d;
pub use decision::radar_comparison;

// 統計ウィジェット (ヒストグラム・箱ひげ図・相関行列)
mod stats;
pub use stats::box_plot;
pub use stats::correlation_matrix;
pub use stats::histogram;

// 共通 UI 部品
mod common;
pub use common::artifact_gallery;
pub use common::cluster_table;
pub use common::convergence_card;
pub use common::csv_import_modal;
pub use common::license_modal;
pub use common::trial_detail_modal;
pub use common::trial_table;
