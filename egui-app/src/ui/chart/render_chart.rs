use crate::state::app_state::{AppState, Direction};
use crate::state::layout_state::ChartId;
use crate::theme::colormap_name::colormap_from_name;
use crate::ui::widget_states::WidgetStates;

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
        // surrogate_opt との分割借用: pareto_2d と surrogate_opt を同時に可変借用する。
        let crate::ui::widget_states::WidgetStates {
            ref mut pareto_2d,
            ref surrogate_opt,
            ..
        } = *widgets;
        pareto_2d.show(ui, app_state, surrogate_opt.multi_result.as_ref());
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
    if matches!(chart_id, ChartId::ArtifactGallery) {
        widgets.artifact_gallery.show(ui, app_state);
        return;
    }

    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let param_names = &ctx.meta.param_names;
    let directions = &ctx.meta.directions;
    let cmap = colormap_from_name(&app_state.selected_colormap);

    match chart_id {
        ChartId::ParetoScatter2D
        | ChartId::ParetoScatter3D
        | ChartId::ClusterScatter3D
        | ChartId::ArtifactGallery => {
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
                &base_name,
                &comparisons,
            );
        }
        ChartId::HvHistory => {
            use crate::theme::color_compute::rgba_to_color32;
            use crate::ui::widgets::hv_history::HvSeries;
            widgets.hv_history.hv_history = app_state.hv_history.clone();
            widgets.hv_history.base_name = ctx.meta.name.clone();
            widgets.hv_history.objective_names = obj_names.clone();
            widgets.hv_history.ref_point_override = app_state.hv_ref_point_override.clone();
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
            // 参照点の変更要求を app_state へ反映し、HV を再計算させる。
            // 値が変わらない場合は再計算しない（DragValue の確定連発を吸収）。
            if let Some(change) = widgets.hv_history.pending_ref_point.take() {
                use crate::ui::widgets::hv_history::RefPointChange;
                let new_override = match change {
                    RefPointChange::Auto => None,
                    RefPointChange::Manual(v) => Some(v),
                };
                if app_state.hv_ref_point_override != new_override {
                    app_state.hv_ref_point_override = new_override;
                    app_state.hv_history = None;
                }
            }
        }
        ChartId::ImportanceChart => {
            let imp_key = (
                widgets.importance.metric.cache_id(),
                widgets.importance.objective_index,
                widgets.importance.feasible_only,
            );
            let current_sensitivity = app_state.importance_cache.get(&imp_key);
            let current_sobol = app_state.sobol_cache.get(&(
                widgets.importance.objective_index,
                widgets.importance.feasible_only,
            ));
            widgets.importance.show(
                ui,
                current_sensitivity,
                current_sobol,
                obj_names,
                ctx.view.feasibility().has_constraints(),
            );
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
            widgets.pdp_2d.show(
                ui,
                param_names,
                obj_names,
                cmap,
                &ctx.view,
                &app_state.selected_indices,
                &app_state.pinned_trials,
            );
        }
        ChartId::ParallelCoordinates => {
            widgets
                .parallel_coords
                .show(ui, &ctx.view, param_names, obj_names, &cmap);
            if let Some(sel) = widgets.parallel_coords.pending_selection.take() {
                app_state.selected_indices = sel;
            }
        }
        ChartId::ScatterMatrix => {
            widgets
                .scatter_matrix
                .show(ui, &ctx.view, param_names, obj_names, &cmap);
        }
        ChartId::SensitivityHeatmap => {
            let key = (
                widgets.sensitivity_heatmap.metric.cache_id(),
                widgets.sensitivity_heatmap.feasible_only,
            );
            let matrix = app_state.sensitivity_heatmap_cache.get(&key);
            widgets
                .sensitivity_heatmap
                .show(ui, matrix, ctx.view.feasibility().has_constraints());
        }
        ChartId::ClusterScatter => {
            let key = widgets.cluster_scatter.cache_key();
            widgets.cluster_scatter.show(
                ui,
                &ctx.view,
                app_state.cluster_cache.get(&key),
                param_names,
                obj_names,
                &cmap,
                &app_state.artifact_map,
                &app_state.selected_indices,
            );
        }
        ChartId::McdmRankChart => {
            let key = widgets.mcdm_chart.controls.cache_key();
            widgets
                .mcdm_chart
                .show(ui, obj_names, app_state.mcdm_cache.get(&key));
        }
        ChartId::McdmScatterChart => {
            let key = widgets.scatter_chart.controls.cache_key();
            widgets.scatter_chart.show(
                ui,
                app_state.mcdm_cache.get(&key),
                &ctx.view,
                param_names,
                obj_names,
                &cmap,
                &app_state.selected_colormap,
                &app_state.artifact_map,
                &app_state.selected_indices,
            );
        }
        ChartId::McdmScatterChart3D => {
            let key = widgets.mcdm_scatter_3d.controls.cache_key();
            widgets.mcdm_scatter_3d.show(
                ui,
                app_state.mcdm_cache.get(&key),
                &ctx.view,
                obj_names,
                &cmap,
                &app_state.selected_colormap,
            );
        }
        ChartId::SliceChart => {
            widgets.slice_chart.show(
                ui,
                &ctx.view,
                param_names,
                obj_names,
                directions,
                &app_state.artifact_map,
            );
        }
        ChartId::ResponseSurfacePlot => {
            let trial_count = ctx.trial_count();
            // カテゴリカル列（数値化できない列）はサロゲートの入力にしない。
            let numeric_params: Vec<String> = param_names
                .iter()
                .filter(|p| ctx.view.numeric_column(p).is_some())
                .cloned()
                .collect();
            let minimize_for_obj: Vec<bool> = directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();
            crate::ui::widgets::response_surface::show(
                ui,
                &mut widgets.response_surface,
                &widgets.surrogate_cache,
                &numeric_params,
                obj_names,
                &minimize_for_obj,
                cmap,
                trial_count,
                ctx.view.feasibility().has_constraints(),
            );
        }
        ChartId::SurrogateOpt => {
            let trial_count = ctx.trial_count();
            // カテゴリカル列（数値化できない列）は最適化対象から除外する。
            let numeric_params: Vec<String> = param_names
                .iter()
                .filter(|p| ctx.view.numeric_column(p).is_some())
                .cloned()
                .collect();
            // 現在の結果が参照する目的列を取得する（結果が無い場合は None）。
            let obj_history: Option<Vec<f64>> = widgets
                .surrogate_opt
                .result
                .as_ref()
                .and_then(|r| ctx.view.numeric_column(&r.objective_name))
                .map(|col| col.to_vec());
            let constraint_col_names = ctx.view.df.constraint_col_names().to_vec();
            crate::ui::widgets::surrogate_opt::show(
                ui,
                &mut widgets.surrogate_opt,
                &numeric_params,
                obj_names,
                cmap,
                trial_count,
                obj_history.as_deref(),
                &constraint_col_names,
            );
        }
    }
}
