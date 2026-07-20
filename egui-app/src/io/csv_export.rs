use crate::state::app_state::{AppState, McdmResult, StudyContext};
use crate::state::layout_state::ChartId;
use crate::state::results::ClusterResult;
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::trial_table::TrialTableMode;

mod clustering;
mod distribution;
mod history;
mod importance;
mod mcdm;
mod surrogate;
mod trial;

use clustering::*;
use distribution::*;
use history::*;
use importance::*;
use mcdm::*;
use surrogate::*;
use trial::*;

/// Resolves the cluster result from the cache using the chart-specific clustering
/// settings key. 2D / 3D / Table each have independent settings, so the export target
/// is also looked up with each one's own key.
pub(super) fn cluster_result_for_chart<'a>(
    chart_id: &ChartId,
    app_state: &'a AppState,
    widgets: &WidgetStates,
) -> Option<&'a ClusterResult> {
    let key = match chart_id {
        ChartId::ClusterScatter => widgets.cluster_scatter.cache_key(),
        ChartId::ClusterScatter3D => widgets.cluster_scatter_3d.cache_key(),
        _ => return None,
    };
    app_state.cluster_cache.get(&key)
}

/// Resolves the result from the cache using the chart-specific MCDM settings key.
pub(super) fn mcdm_result_for_chart<'a>(
    chart_id: &ChartId,
    app_state: &'a AppState,
    widgets: &WidgetStates,
) -> Option<&'a McdmResult> {
    let key = match chart_id {
        ChartId::McdmRankChart => widgets.mcdm_chart.controls.cache_key(),
        ChartId::McdmScatterChart => widgets.scatter_chart.controls.cache_key(),
        ChartId::McdmScatterChart3D => widgets.mcdm_scatter_3d.controls.cache_key(),
        _ => return None,
    };
    app_state.mcdm_cache.get(&key)
}

/// Consolidates the boilerplate guard used at the start of many `build_*_csv` functions
/// (getting current_study and ensuring there's at least 1 trial). Returns `None` if
/// either no study is selected or the trial count is 0.
pub(super) fn require_study(app_state: &AppState) -> Option<&StudyContext> {
    let study = app_state.current_study.as_ref()?;
    (study.trial_count() > 0).then_some(study)
}

pub fn build_chart_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    // Use has_csv_data (the button-enable check) as the single source of truth for
    // whether data exists, to prevent the two from diverging. has_csv_data is
    // lightweight, so calling it at the start of every export is fine.
    if !has_csv_data(chart_id, app_state, widgets) {
        return None;
    }
    match chart_id {
        ChartId::OptimizationHistory => build_optimization_history_csv(app_state, widgets),
        ChartId::ConvergenceIndicators => build_convergence_csv(app_state),
        ChartId::ImportanceChart => build_importance_csv(app_state, widgets),
        ChartId::PdpChart => build_pdp_csv(app_state, widgets),
        ChartId::PdpChart2D => build_pdp_2d_csv(app_state, widgets),
        ChartId::ParallelCoordinates => build_trial_based_csv(app_state),
        ChartId::ScatterMatrix => build_trial_based_csv(app_state),
        ChartId::ClusterScatter => build_cluster_csv(chart_id, app_state, widgets),
        ChartId::SensitivityHeatmap => build_sensitivity_csv(app_state, widgets),
        ChartId::ParetoScatter2D => build_pareto_csv(app_state),
        ChartId::ParetoScatter3D => build_pareto_csv(app_state),
        ChartId::McdmRankChart => mcdm_result_for_chart(chart_id, app_state, widgets)
            .and_then(|r| build_mcdm_rank_csv(r, app_state)),
        ChartId::McdmScatterChart => mcdm_result_for_chart(chart_id, app_state, widgets)
            .and_then(|r| build_mcdm_scatter_csv(r, app_state)),
        ChartId::SliceChart => build_slice_csv(app_state, widgets),
        ChartId::ObservedContour => build_observed_contour_csv(widgets),
        ChartId::SurrogateOpt => build_surrogate_opt_csv(widgets),
        ChartId::Robustness => build_robustness_csv(widgets),
        ChartId::ClusterScatter3D => build_cluster_csv(chart_id, app_state, widgets),
        ChartId::McdmScatterChart3D => mcdm_result_for_chart(chart_id, app_state, widgets)
            .and_then(|r| build_mcdm_scatter_csv(r, app_state)),
        ChartId::Histogram => build_histogram_csv(app_state, widgets),
        ChartId::BoxPlot => build_box_plot_csv(app_state, widgets),
        ChartId::CorrelationMatrix => build_correlation_matrix_csv(app_state, widgets),
        ChartId::ArtifactGallery => None,
        ChartId::RadarComparison => build_radar_comparison_csv(app_state, widgets),
        ChartId::ComparisonTable => build_comparison_table_csv(app_state, widgets),
        ChartId::PcaBiplot => build_pca_biplot_csv(widgets),
        ChartId::SomMap => build_som_csv(app_state, widgets),
        ChartId::Dendrogram => build_dendrogram_csv(widgets),
        ChartId::ResponseSurface3D => build_response_surface_csv(widgets),
        ChartId::IntermediateValues => build_intermediate_values_csv(),
        ChartId::Timeline => build_timeline_csv(),
        ChartId::EdfPlot => build_edf_csv(app_state, widgets),
        ChartId::RankPlot => build_rank_plot_csv(app_state, widgets),
        ChartId::SurrogateCompare => build_surrogate_compare_csv(widgets),
    }
}

/// Builds the CSV for the unified trial table (`PanelItem::TrialTable`) according to the
/// current mode. All outputs the trial list, Cluster outputs cluster assignments, and
/// MCDM outputs the ranking.
pub fn build_trial_table_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    match widgets.trial_table.mode {
        TrialTableMode::All => build_trial_based_csv(app_state),
        TrialTableMode::Cluster => {
            let key = widgets.trial_table.cluster.cache_key();
            let cr = app_state.cluster_cache.get(&key)?;
            build_cluster_csv_from_result(cr, app_state)
        }
        TrialTableMode::Mcdm => {
            let key = widgets.trial_table.mcdm.controls.cache_key();
            let result = app_state.mcdm_cache.get(&key)?;
            build_mcdm_table_csv(result, app_state)
        }
    }
}

/// Determines whether the unified trial table has exportable data in the current mode.
pub fn has_trial_table_csv(app_state: &AppState, widgets: &WidgetStates) -> bool {
    match widgets.trial_table.mode {
        TrialTableMode::All => app_state
            .current_study
            .as_ref()
            .is_some_and(|s| s.trial_count() > 0),
        TrialTableMode::Cluster => {
            let key = widgets.trial_table.cluster.cache_key();
            app_state
                .current_study
                .as_ref()
                .zip(app_state.cluster_cache.get(&key))
                .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count())
        }
        TrialTableMode::Mcdm => {
            let key = widgets.trial_table.mcdm.controls.cache_key();
            app_state.current_study.is_some() && app_state.mcdm_cache.contains_key(&key)
        }
    }
}

/// Returns the CSV file name for the unified trial table according to the current mode.
pub fn trial_table_csv_filename(widgets: &WidgetStates) -> String {
    let name = match widgets.trial_table.mode {
        TrialTableMode::All => "trial_table",
        TrialTableMode::Cluster => "cluster_table",
        TrialTableMode::Mcdm => "mcdm_table",
    };
    format!("{}.csv", name)
}

pub fn has_csv_data(chart_id: &ChartId, app_state: &AppState, widgets: &WidgetStates) -> bool {
    match chart_id {
        ChartId::SurrogateOpt => {
            widgets.surrogate_opt.result.is_some() || widgets.surrogate_opt.multi_result.is_some()
        }
        ChartId::Robustness => widgets.robustness.cached_result().is_some(),
        ChartId::OptimizationHistory | ChartId::ParallelCoordinates | ChartId::ScatterMatrix => {
            app_state
                .current_study
                .as_ref()
                .is_some_and(|s| s.trial_count() > 0)
        }
        ChartId::ConvergenceIndicators => app_state.convergence_history.is_some(),
        ChartId::ImportanceChart => {
            if widgets.importance.computing {
                return false;
            }
            let obj_idx = widgets.importance.objective_index;
            let feasible_only = widgets.importance.feasible_only;
            if widgets.importance.metric.is_sobol() {
                app_state
                    .sobol_cache
                    .contains_key(&(obj_idx, feasible_only))
            } else {
                let key = (widgets.importance.metric.cache_id(), obj_idx, feasible_only);
                app_state.importance_cache.contains_key(&key)
            }
        }
        ChartId::PdpChart => widgets
            .pdp_chart
            .result
            .as_ref()
            .is_some_and(|d| !d.x_values.is_empty()),
        ChartId::PdpChart2D => widgets
            .pdp_2d
            .result
            .as_ref()
            .is_some_and(|r| !r.x_values.is_empty() && !r.y_values.is_empty()),
        ChartId::ClusterScatter => app_state
            .current_study
            .as_ref()
            .zip(cluster_result_for_chart(chart_id, app_state, widgets))
            .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count()),
        ChartId::SensitivityHeatmap => app_state
            .sensitivity_heatmap_cache
            .get(&(
                widgets.sensitivity_heatmap.metric.cache_id(),
                widgets.sensitivity_heatmap.feasible_only,
            ))
            .is_some_and(|m| m.is_well_formed()),
        ChartId::ParetoScatter2D | ChartId::ParetoScatter3D => app_state
            .current_study
            .as_ref()
            .is_some_and(|s| !s.pareto_indices.is_empty()),
        ChartId::McdmRankChart | ChartId::McdmScatterChart => {
            app_state.current_study.is_some()
                && mcdm_result_for_chart(chart_id, app_state, widgets).is_some()
        }
        ChartId::SliceChart => app_state.current_study.as_ref().is_some_and(|s| {
            s.trial_count() > 0
                && s.meta
                    .param_names
                    .get(widgets.slice_chart.selected_param_idx)
                    .is_some()
                && s.meta
                    .objective_names
                    .get(widgets.slice_chart.selected_obj_idx)
                    .is_some()
        }),
        ChartId::ObservedContour => widgets
            .observed_contour
            .result
            .as_ref()
            .is_some_and(|r| !r.surface.x_values.is_empty()),
        ChartId::ClusterScatter3D => app_state
            .current_study
            .as_ref()
            .zip(cluster_result_for_chart(chart_id, app_state, widgets))
            .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count()),
        ChartId::McdmScatterChart3D => {
            app_state.current_study.is_some()
                && mcdm_result_for_chart(chart_id, app_state, widgets).is_some()
        }
        ChartId::Histogram | ChartId::BoxPlot => app_state
            .current_study
            .as_ref()
            .is_some_and(|s| s.trial_count() > 0),
        ChartId::CorrelationMatrix => {
            app_state
                .current_study
                .as_ref()
                .is_some_and(|s| s.trial_count() > 0)
                && (widgets.correlation_matrix.include_params
                    || widgets.correlation_matrix.include_objectives)
        }
        ChartId::ArtifactGallery => false,
        ChartId::RadarComparison => app_state.current_study.as_ref().is_some_and(|s| {
            !app_state.pinned_trials.is_empty()
                && !crate::ui::widgets::radar_comparison::build_axes(
                    &s.view,
                    &s.meta.param_names,
                    &s.meta.objective_names,
                    widgets.radar_comparison.include_params,
                )
                .is_empty()
        }),
        ChartId::ComparisonTable => app_state.current_study.as_ref().is_some_and(|s| {
            !crate::ui::widgets::comparison_table::resolve_pinned_rows(
                &s.view,
                &app_state.pinned_trials,
            )
            .is_empty()
                && !crate::ui::widgets::comparison_table::build_rows(
                    &s.view,
                    &s.meta.param_names,
                    &s.meta.objective_names,
                    widgets.comparison_table.show_params,
                    widgets.comparison_table.show_user_attrs,
                )
                .is_empty()
        }),
        ChartId::PcaBiplot => widgets
            .pca_biplot
            .cached_result()
            .is_some_and(|r| !r.projections.is_empty()),
        ChartId::SomMap => app_state.current_study.as_ref().is_some_and(|s| {
            widgets
                .som_map
                .current_grid(&s.meta.param_names, &s.meta.objective_names)
                .is_some()
        }),
        ChartId::Dendrogram => widgets
            .dendrogram
            .leaf_assignments()
            .is_some_and(|a| !a.is_empty()),
        ChartId::ResponseSurface3D => widgets
            .response_surface
            .cached_slice()
            .is_some_and(|s| !s.x_values.is_empty() && !s.y_values.is_empty()),
        ChartId::IntermediateValues => {
            tunny_core::dataframe::active_extras_snapshot().is_some_and(|e| e.has_intermediate())
        }
        ChartId::Timeline => {
            tunny_core::dataframe::active_extras_snapshot().is_some_and(|e| e.has_datetimes())
        }
        ChartId::EdfPlot => app_state.current_study.as_ref().is_some_and(|s| {
            s.meta
                .objective_names
                .get(widgets.edf_plot.obj_idx)
                .is_some_and(|name| s.view.numeric_column(name).is_some_and(|c| !c.is_empty()))
        }),
        ChartId::RankPlot => app_state.current_study.as_ref().is_some_and(|s| {
            s.trial_count() > 0
                && s.meta
                    .param_names
                    .get(widgets.rank_plot.x_param_idx)
                    .is_some()
                && s.meta
                    .param_names
                    .get(widgets.rank_plot.y_param_idx)
                    .is_some()
                && s.meta
                    .objective_names
                    .get(widgets.rank_plot.obj_idx)
                    .is_some()
        }),
        ChartId::SurrogateCompare => widgets
            .surrogate_compare
            .result
            .as_ref()
            .is_some_and(|r| !r.rows.is_empty()),
    }
}

pub fn csv_export_filename(chart_id: &ChartId) -> String {
    let name = match chart_id {
        ChartId::OptimizationHistory => "optimization_history",
        ChartId::ConvergenceIndicators => "convergence_indicators",
        ChartId::ImportanceChart => "importance_chart",
        ChartId::PdpChart => "pdp_chart",
        ChartId::PdpChart2D => "pdp_chart_2d",
        ChartId::ParallelCoordinates => "parallel_coordinates",
        ChartId::ScatterMatrix => "scatter_matrix",
        ChartId::ClusterScatter => "cluster_scatter",
        ChartId::SensitivityHeatmap => "sensitivity_heatmap",
        ChartId::ParetoScatter2D => "pareto_scatter_2d",
        ChartId::ParetoScatter3D => "pareto_scatter_3d",
        ChartId::McdmRankChart => "mcdm_rank_chart",
        ChartId::McdmScatterChart => "mcdm_scatter_chart",
        ChartId::SliceChart => "slice_chart",
        ChartId::ObservedContour => "observed_contour",
        ChartId::SurrogateOpt => "surrogate_optimizer",
        ChartId::Robustness => "robustness",
        ChartId::ClusterScatter3D => "cluster_scatter_3d",
        ChartId::McdmScatterChart3D => "mcdm_scatter_chart_3d",
        ChartId::Histogram => "histogram",
        ChartId::BoxPlot => "box_plot",
        ChartId::CorrelationMatrix => "correlation_matrix",
        ChartId::ArtifactGallery => "artifact_gallery",
        ChartId::RadarComparison => "radar_comparison",
        ChartId::ComparisonTable => "comparison_table",
        ChartId::PcaBiplot => "pca_biplot",
        ChartId::SomMap => "som_map",
        ChartId::Dendrogram => "dendrogram",
        ChartId::ResponseSurface3D => "response_surface_3d",
        ChartId::IntermediateValues => "intermediate_values",
        ChartId::Timeline => "timeline",
        ChartId::EdfPlot => "edf_plot",
        ChartId::RankPlot => "rank_plot",
        ChartId::SurrogateCompare => "surrogate_compare",
    };
    format!("{}.csv", name)
}

#[cfg(test)]
mod tests;
