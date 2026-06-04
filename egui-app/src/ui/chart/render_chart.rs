use crate::state::app_state::{AppState, Direction};
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
    if matches!(chart_id, ChartId::ClusterScatter3D) {
        widgets.cluster_scatter_3d.show(ui, app_state);
        return;
    }

    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let param_names = &ctx.meta.param_names;
    let directions = &ctx.meta.directions;
    let cmap = colormap_from_name(&app_state.selected_colormap);

    match chart_id {
        ChartId::ParetoScatter2D | ChartId::ParetoScatter3D | ChartId::ClusterScatter3D => {
            unreachable!()
        }
        ChartId::OptimizationHistory => {
            use crate::theme::color_compute::rgba_to_color32;
            use crate::ui::widgets::optimization_history::OptHistoryComparison;
            // 選択中の目的名を基準に、比較 Study から同名の目的値列を集める。
            let sel_idx = widgets
                .opt_history
                .obj_idx
                .min(obj_names.len().saturating_sub(1));
            let comparisons: Vec<OptHistoryComparison> = match obj_names.get(sel_idx) {
                Some(sel_name) => app_state
                    .comparison_studies
                    .iter()
                    .enumerate()
                    .filter_map(|(i, study)| {
                        let pos = study
                            .view
                            .objective_names()
                            .iter()
                            .position(|n| n == sel_name)?;
                        let values = study.view.numeric_column(sel_name)?.to_vec();
                        let is_minimize = study
                            .meta
                            .directions
                            .get(pos)
                            .map(|d| matches!(d, Direction::Minimize))
                            .unwrap_or(true);
                        let color = app_state
                            .comparison_colors
                            .get(i)
                            .copied()
                            .unwrap_or([66, 133, 244, 255]);
                        Some(OptHistoryComparison {
                            name: study.meta.name.clone(),
                            color: rgba_to_color32(color),
                            values,
                            is_minimize,
                        })
                    })
                    .collect(),
                None => Vec::new(),
            };
            let base_name = ctx.meta.name.clone();
            widgets.opt_history.show_with_comparisons(
                ui,
                &ctx.view,
                obj_names,
                directions,
                None,
                &base_name,
                &comparisons,
            );
        }
        ChartId::HvHistory => {
            use crate::theme::color_compute::rgba_to_color32;
            use crate::ui::widgets::hv_history::HvSeries;
            widgets.hv_history.hv_history = app_state.hv_history.clone();
            widgets.hv_history.base_name = ctx.meta.name.clone();
            // 比較 Study の HV 履歴を色付き系列として渡し、同一グラフに重ねる。
            widgets.hv_history.comparisons = app_state
                .comparison_studies
                .iter()
                .enumerate()
                .filter_map(|(i, study)| {
                    let hv = app_state.comparison_hv_histories.get(i)?.clone();
                    let color = app_state
                        .comparison_colors
                        .get(i)
                        .copied()
                        .unwrap_or([66, 133, 244, 255]);
                    Some(HvSeries {
                        name: study.meta.name.clone(),
                        color: rgba_to_color32(color),
                        history: hv,
                    })
                })
                .collect();
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
            widgets.pdp_chart.show(
                ui,
                param_names,
                obj_names,
                &ctx.view,
                &app_state.selected_indices,
                &app_state.pinned_trials,
            );
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
                &ctx.view,
                app_state.cluster_result.as_ref(),
                param_names,
                obj_names,
                &cmap,
            );
        }
        ChartId::McdmRankChart => {
            widgets
                .mcdm_chart
                .show(ui, obj_names, &app_state.mcdm_result);
        }
        ChartId::McdmScatterChart => {
            let top_n = widgets.mcdm_chart.top_n.value();
            widgets.scatter_chart.show(
                ui,
                &app_state.mcdm_result,
                &ctx.view,
                obj_names,
                &cmap,
                &app_state.selected_colormap,
                top_n,
            );
        }
        ChartId::McdmScatterChart3D => {
            let top_n = widgets.mcdm_chart.top_n.value();
            widgets.mcdm_scatter_3d.show(
                ui,
                &app_state.mcdm_result,
                &ctx.view,
                obj_names,
                &cmap,
                &app_state.selected_colormap,
                top_n,
            );
        }
        ChartId::McdmTable => {
            widgets
                .mcdm_table
                .show(ui, &app_state.mcdm_result, &ctx.view, obj_names);
        }
        ChartId::AhpRankChart => {
            let n_trials = ctx.trial_count();
            let n_objectives = obj_names.len();
            let obj_cols = ctx.view.numeric_columns(obj_names);
            let objectives: Vec<f64> = (0..n_trials)
                .flat_map(|i| {
                    obj_cols
                        .iter()
                        .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                })
                .collect();
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
                .show_table(ui, obj_names, &ctx.view, &app_state.ahp_result);
        }
        ChartId::SliceChart => {
            widgets
                .slice_chart
                .show(ui, &ctx.view, param_names, obj_names, directions);
        }
        ChartId::SurfacePlot => {
            let trial_count = ctx.trial_count();
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
