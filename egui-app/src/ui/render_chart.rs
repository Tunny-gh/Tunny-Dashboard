use crate::state::app_state::{filter_rows_for_display, AppState, Direction};
use crate::state::layout_state::ChartId;
use crate::theme::colormap_name::colormap_from_name;
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::ahp_chart::AhpDataContext;

pub(crate) fn render_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
) {
    if app_state.current_study.is_none() {
        return;
    }

    // pareto_2d/3d は &mut AppState を要求するため先に処理する
    if matches!(chart_id, ChartId::ParetoScatter2D) {
        widgets.pareto_2d.show(ui, app_state);
        return;
    }
    if matches!(chart_id, ChartId::ParetoScatter3D) {
        widgets.pareto_3d.show(ui, app_state);
        return;
    }

    let ctx = app_state.current_study.as_ref().unwrap();
    let trial_rows = &ctx.trial_rows();
    let obj_names = &ctx.meta.objective_names;
    let param_names = &ctx.meta.param_names;
    let directions = &ctx.meta.directions;
    let cmap = colormap_from_name(&app_state.selected_colormap);

    match chart_id {
        ChartId::ParetoScatter2D | ChartId::ParetoScatter3D => unreachable!(),
        ChartId::OptimizationHistory => {
            widgets
                .opt_history
                .show(ui, trial_rows, obj_names, directions);
        }
        ChartId::HvHistory => {
            widgets.hv_history.hv_history = app_state.hv_history.clone();
            widgets.hv_history.show(ui);
        }
        ChartId::ImportanceChart => {
            let imp_key = (
                widgets.importance.metric.cache_id(),
                widgets.importance.objective_index,
            );
            let current_sensitivity = app_state.importance_cache.get(&imp_key);
            let current_sobol = app_state
                .sobol_cache
                .get(&widgets.importance.objective_index);
            widgets
                .importance
                .show(ui, current_sensitivity, current_sobol, obj_names);
        }
        ChartId::PdpChart => {
            // TASK-2237: pass selected ∪ pinned rows to PDP observed overlay
            let pdp_rows: Vec<&crate::state::app_state::TrialRow> =
                filter_rows_for_display(trial_rows, &app_state.selected_indices, &app_state.pinned_trials);
            let pdp_rows_owned: Vec<crate::state::app_state::TrialRow> =
                pdp_rows.into_iter().cloned().collect();
            widgets
                .pdp_chart
                .show(ui, param_names, obj_names, &pdp_rows_owned);
        }
        ChartId::PdpChart2D => {
            widgets.pdp_2d.show(ui, param_names, obj_names, cmap);
        }
        ChartId::ParallelCoordinates => {
            widgets.parallel_coords.show(
                ui,
                &ctx.view,
                param_names,
                obj_names,
                &widgets.chart_colors,
            );
            if let Some(sel) = widgets.parallel_coords.pending_selection.take() {
                app_state.selected_indices = sel;
            }
        }
        ChartId::ScatterMatrix => {
            widgets.scatter_matrix.show(
                ui,
                &ctx.view,
                param_names,
                obj_names,
                &widgets.chart_colors,
            );
        }
        ChartId::SensitivityHeatmap => {
            widgets.sensitivity_heatmap.show(ui);
        }
        ChartId::ClusterScatter => {
            widgets.cluster_scatter.show(
                ui,
                trial_rows,
                app_state.cluster_result.as_ref(),
                param_names,
                obj_names,
                &cmap,
            );
        }
        ChartId::McdmRankChart => {
            widgets
                .mcdm_chart
                .show(ui, obj_names, &app_state.mcdm_result, trial_rows);
        }
        ChartId::McdmScatterChart => {
            widgets
                .scatter_chart
                .show(ui, &app_state.mcdm_result, trial_rows, obj_names);
        }
        ChartId::McdmTable => {
            widgets
                .mcdm_table
                .show(ui, &app_state.mcdm_result, trial_rows, obj_names);
        }
        ChartId::AhpRankChart => {
            let objectives: Vec<f64> = trial_rows
                .iter()
                .flat_map(|r| r.objectives.iter().copied())
                .collect();
            let n_trials = trial_rows.len();
            let n_objectives = obj_names.len();
            let is_minimize: Vec<bool> = directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();
            let ahp_ctx = AhpDataContext {
                values: &objectives,
                n_trials,
                n_objectives,
                is_minimize: &is_minimize,
            };
            widgets
                .ahp_chart
                .show_rank_chart(ui, obj_names, &app_state.ahp_result, &ahp_ctx);
        }
        ChartId::AhpTable => {
            widgets
                .ahp_chart
                .show_table(ui, obj_names, trial_rows, &app_state.ahp_result);
        }
        ChartId::SliceChart => {
            widgets
                .slice_chart
                .show(ui, trial_rows, param_names, obj_names, directions);
        }
        ChartId::SurfacePlot => {
            let trial_count = ctx.trial_rows().len();
            crate::ui::widgets::surface_plot::show(
                ui,
                &mut widgets.surface_plot,
                param_names,
                obj_names,
                cmap,
                trial_count,
            );
        }
    }
}
