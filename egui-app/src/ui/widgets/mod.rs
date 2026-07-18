// Optimization history
mod history;
pub use history::convergence;
pub use history::edf_plot;
pub use history::intermediate_values;
pub use history::optimization_history;
pub use history::timeline;

// Pareto analysis
mod pareto;
pub use pareto::pareto_2d;
pub use pareto::pareto_3d;

// Importance / sensitivity analysis
mod importance;
pub use importance::importance_chart;
pub use importance::sensitivity_heatmap;

// Partial dependence plots / model visualization
mod pdp;
pub use pdp::pdp_2d;
pub use pdp::pdp_chart;
pub use pdp::slice_chart;

// Surrogate optimization (build response surface + optimize on the surface)
mod surrogate;
pub use surrogate::anchor;
pub use surrogate::compare;
pub use surrogate::response_surface;
pub use surrogate::robustness;
pub use surrogate::surrogate_opt;

// Scatter plots / cluster exploration
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

// Decision analysis (MCDM)
mod decision;
pub use decision::comparison_table;
pub use decision::mcdm_chart;
pub use decision::mcdm_scatter_chart;
pub use decision::mcdm_scatter_chart_3d;
pub use decision::radar_comparison;

// Statistics widgets (histogram, box plot, correlation matrix)
mod stats;
pub use stats::box_plot;
pub use stats::correlation_matrix;
pub use stats::histogram;

// Common UI components
mod common;
pub use common::artifact_gallery;
pub use common::cluster_table;
pub use common::convergence_card;
pub use common::csv_import_modal;
pub use common::ghx_opt_modal;
pub use common::license_modal;
pub use common::process_def_modal;
pub use common::process_opt_modal;
pub use common::rdb_url_modal;
pub use common::report_modal;
pub use common::trial_detail_modal;
pub use common::trial_table;
// Re-exported so the min/max aggregation helper from the state layer
// (`state::types::StudyContext::param_range`) can be reused (D-9).
pub(crate) use common::range_math;
