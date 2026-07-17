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

    // pareto_2d/3d require &mut AppState, so handle them first
    if matches!(chart_id, ChartId::ParetoScatter2D) {
        // Split borrow with surrogate_opt: mutably borrow pareto_2d and surrogate_opt at
        // the same time.
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
            use crate::theme::color_compute::{comparison_color_at, rgba_to_color32};
            use crate::ui::widgets::optimization_history::OptHistoryComparison;
            // Collect same-named objective value columns from the comparison Studies,
            // based on the currently selected objective name. Cached by selected
            // objective, comparison set, color, and Study identity to avoid a to_vec
            // clone every frame (M-11).
            let sel_idx = widgets
                .opt_history
                .obj_idx
                .min(obj_names.len().saturating_sub(1));
            let sel_name = obj_names.get(sel_idx).cloned();
            let key = crate::ui::widget_states::ComparisonSeriesKey {
                base_df: std::sync::Arc::as_ptr(&ctx.view.df) as usize,
                sel_name: sel_name.clone(),
                comps: app_state
                    .comparison_studies
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        (
                            std::sync::Arc::as_ptr(&s.view.df) as usize,
                            app_state
                                .comparison_colors
                                .get(i)
                                .copied()
                                .unwrap_or_else(|| comparison_color_at(i)),
                        )
                    })
                    .collect(),
            };
            let base_name = ctx.meta.name.clone();

            let crate::ui::widget_states::WidgetStates {
                ref mut render_cache,
                ref mut opt_history,
                ..
            } = *widgets;
            let comparisons =
                render_cache.opt_history_comparisons(key, || match sel_name.as_deref() {
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
                                .unwrap_or_else(|| comparison_color_at(i));
                            Some(OptHistoryComparison {
                                name: study.meta.name.clone(),
                                color: rgba_to_color32(color),
                                values,
                                is_minimize,
                            })
                        })
                        .collect(),
                    None => Vec::new(),
                });
            opt_history.show_with_comparisons(
                ui,
                &ctx.view,
                obj_names,
                directions,
                param_names,
                &base_name,
                comparisons,
                &app_state.artifact_map,
            );
        }
        ChartId::ConvergenceIndicators => {
            use crate::theme::color_compute::{comparison_color_at, rgba_to_color32};
            use crate::ui::widgets::convergence::ConvergenceSeries;
            // Cloning history/objective_names and rebuilding the comparison series only
            // happens when the selection or data changes (avoids cloning every frame;
            // item low). Data identity is detected via the Vec's data pointer + length.
            let key = crate::ui::widget_states::ConvergenceSyncKey {
                base_df: std::sync::Arc::as_ptr(&ctx.view.df) as usize,
                history: app_state
                    .convergence_history
                    .as_ref()
                    .map(|h| (h.values.as_ptr() as usize, h.values.len())),
                indicator: app_state.convergence_indicator,
                ref_override: app_state
                    .hv_ref_point_override
                    .as_ref()
                    .map(|v| (v.as_ptr() as usize, v.len())),
                comparisons: app_state
                    .comparison_studies
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        (
                            std::sync::Arc::as_ptr(&s.view.df) as usize,
                            app_state
                                .comparison_convergence_histories
                                .get(i)
                                .map(|h| (h.values.as_ptr() as usize, h.values.len())),
                            app_state
                                .comparison_colors
                                .get(i)
                                .copied()
                                .unwrap_or_else(|| comparison_color_at(i)),
                        )
                    })
                    .collect(),
            };
            if widgets.render_cache.convergence_sync.as_ref() != Some(&key) {
                widgets.render_cache.convergence_sync = Some(key);
                widgets.convergence.history = app_state.convergence_history.clone();
                widgets.convergence.base_name = ctx.meta.name.clone();
                widgets.convergence.objective_names = obj_names.clone();
                widgets.convergence.ref_point_override = app_state.hv_ref_point_override.clone();
                // Pass the currently selected convergence indicator to the widget.
                widgets.convergence.indicator = app_state.convergence_indicator;
                // Pass the comparison Studies' indicator history as colored series and
                // overlay them on the same graph.
                widgets.convergence.comparisons = app_state
                    .comparison_studies
                    .iter()
                    .enumerate()
                    .filter_map(|(i, study)| {
                        let history = app_state.comparison_convergence_histories.get(i)?.clone();
                        let color = app_state
                            .comparison_colors
                            .get(i)
                            .copied()
                            .unwrap_or_else(|| comparison_color_at(i));
                        Some(ConvergenceSeries {
                            name: study.meta.name.clone(),
                            color: rgba_to_color32(color),
                            history,
                        })
                    })
                    .collect();
            }
            widgets.convergence.show(
                ui,
                &ctx.view,
                param_names,
                obj_names,
                &app_state.artifact_map,
            );
            // Reflect an indicator change request into app_state and trigger
            // recomputation.
            if let Some(new_ind) = widgets.convergence.pending_indicator.take() {
                if new_ind != app_state.convergence_indicator {
                    app_state.convergence_indicator = new_ind;
                    app_state.convergence_history = None;
                }
            }
            // Reflect a reference point change request into app_state and let it
            // recompute. Skip recomputation if the value hasn't actually changed
            // (absorbs repeated DragValue commits).
            if let Some(change) = widgets.convergence.pending_ref_point.take() {
                use crate::ui::widgets::convergence::RefPointChange;
                let new_override = match change {
                    RefPointChange::Auto => None,
                    RefPointChange::Manual(v) => Some(v),
                };
                if app_state.hv_ref_point_override != new_override {
                    app_state.hv_ref_point_override = new_override;
                    app_state.convergence_history = None;
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
                &app_state.artifact_map,
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
                param_names,
                obj_names,
                &cmap,
                &app_state.selected_colormap,
                &app_state.artifact_map,
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
        ChartId::ObservedContour => {
            // The numeric filter on axis candidates is done on the widget side (the
            // modal display needs every variable name).
            crate::ui::widgets::observed_contour::show(
                ui,
                &mut widgets.observed_contour,
                param_names,
                obj_names,
                cmap,
                &ctx.view,
                &app_state.artifact_map,
                ctx.view.feasibility().has_constraints(),
            );
        }
        ChartId::SurrogateOpt => {
            let trial_count = ctx.trial_count();
            // Exclude categorical columns (ones that can't be numericized) from the
            // optimization target.
            let numeric_params = crate::ui::chart::poll_chart::numeric_param_names(ctx);
            // Cloning the objective column's to_vec and rebuilding observed_feasible only
            // happens when Study identity or the objective name the result references
            // changes (avoids a full clone every frame. M-11).
            let obj_history_name = widgets
                .surrogate_opt
                .result
                .as_ref()
                .map(|r| r.objective_name.clone());
            let multi_obj_names = widgets
                .surrogate_opt
                .multi_result
                .as_ref()
                .map(|r| r.objective_names.clone());
            let key = crate::ui::widget_states::SurrogateObservedKey {
                df: std::sync::Arc::as_ptr(&ctx.view.df) as usize,
                obj_history_name: obj_history_name.clone(),
                multi_obj_names: multi_obj_names.clone(),
            };

            let crate::ui::widget_states::WidgetStates {
                ref mut render_cache,
                ref mut surrogate_opt,
                ..
            } = *widgets;
            let entry = render_cache.surrogate_observed(key, || {
                // Get the objective column the current result references (None if there's
                // no result).
                let obj_history = obj_history_name
                    .as_ref()
                    .and_then(|name| ctx.view.numeric_column(name))
                    .map(|col| col.to_vec());
                // Observed points to overlay on the multi-objective front scatter plot.
                // Pass the full trial values of each objective, arranged in the result's
                // objective order, along with Pareto rank and feasibility, and classify
                // them the same way as ParetoScatter.
                let observed_cols: Option<Vec<Vec<f64>>> = multi_obj_names.as_ref().map(|names| {
                    names
                        .iter()
                        .map(|name| {
                            ctx.view
                                .numeric_column(name)
                                .map(|c| c.to_vec())
                                .unwrap_or_default()
                        })
                        .collect()
                });
                let observed_feasible: Vec<bool> = if observed_cols.is_some() {
                    let feas = ctx.view.feasibility();
                    (0..ctx.view.row_count())
                        .map(|i| feas.is_feasible(i))
                        .collect()
                } else {
                    Vec::new()
                };
                (obj_history, observed_cols, observed_feasible)
            });
            let observed = entry.observed_cols.as_ref().map(|cols| {
                crate::ui::widgets::surrogate_opt::ObservedData {
                    objective_cols: cols,
                    pareto_rank: &ctx.view.pareto_rank,
                    feasible: &entry.observed_feasible,
                }
            });
            let constraint_col_names = ctx.view.df.constraint_col_names().to_vec();
            crate::ui::widgets::surrogate_opt::show(
                ui,
                surrogate_opt,
                &numeric_params,
                obj_names,
                trial_count,
                entry.obj_history.as_deref(),
                observed.as_ref(),
                &constraint_col_names,
            );
        }
        ChartId::Robustness => {
            crate::ui::widgets::robustness::show(
                ui,
                &mut widgets.robustness,
                &ctx.view,
                obj_names,
                directions,
                ctx.trial_count(),
                &app_state.pinned_trials,
            );
        }
        ChartId::Histogram => {
            widgets
                .histogram
                .show(ui, &ctx.view, param_names, obj_names, &ctx.meta.name);
        }
        ChartId::BoxPlot => {
            widgets
                .box_plot
                .show(ui, &ctx.view, param_names, obj_names, &ctx.meta.name);
        }
        ChartId::CorrelationMatrix => {
            widgets
                .correlation_matrix
                .show(ui, &ctx.view, param_names, obj_names, &ctx.meta.name);
        }
        ChartId::RadarComparison => {
            widgets.radar_comparison.show(
                ui,
                &ctx.view,
                param_names,
                obj_names,
                directions,
                &app_state.pinned_trials,
            );
        }
        ChartId::ComparisonTable => {
            widgets.comparison_table.show(
                ui,
                &ctx.view,
                param_names,
                obj_names,
                directions,
                &app_state.pinned_trials,
            );
        }
        ChartId::PcaBiplot => {
            let study_name = ctx.meta.name.clone();
            widgets
                .pca_biplot
                .show(ui, &ctx.view, obj_names, &cmap, &study_name);
        }
        ChartId::SomMap => {
            widgets
                .som_map
                .show(ui, &ctx.view, param_names, obj_names, &ctx.meta.name, &cmap);
        }
        ChartId::Dendrogram => {
            widgets
                .dendrogram
                .show(ui, &ctx.view, param_names, obj_names, &ctx.meta.name);
        }
        ChartId::ResponseSurface3D => {
            // Exclude categorical columns (ones that can't be numericized) from the
            // response surface target (the same filtering as Robustness / SurrogateOpt).
            let numeric_params = crate::ui::chart::poll_chart::numeric_param_names(ctx);
            widgets.response_surface.show(
                ui,
                &ctx.view,
                &numeric_params,
                obj_names,
                directions,
                ctx.trial_count(),
                &app_state.pinned_trials,
                &cmap,
                &app_state.artifact_map,
            );
        }
        ChartId::SurrogateCompare => {
            // Exclude categorical columns (ones that can't be numericized) from the
            // comparison target (the same filtering as Robustness / SurrogateOpt /
            // ResponseSurface3D).
            let numeric_params = crate::ui::chart::poll_chart::numeric_param_names(ctx);
            crate::ui::widgets::compare::show(
                ui,
                &mut widgets.surrogate_compare,
                obj_names,
                &numeric_params,
                ctx.trial_count(),
            );
        }
        ChartId::IntermediateValues => {
            let extras = tunny_core::dataframe::active_extras_snapshot();
            widgets.intermediate_values.show(ui, extras.as_deref());
        }
        ChartId::Timeline => {
            let extras = tunny_core::dataframe::active_extras_snapshot();
            widgets.timeline.show(ui, extras.as_deref());
        }
        ChartId::EdfPlot => {
            use crate::theme::color_compute::{comparison_color_at, rgba_to_color32};
            use crate::ui::widgets::edf_plot::EdfComparison;
            // Collect same-named objective value columns from the comparison Studies,
            // based on the currently selected objective name (the same technique/cache
            // as OptimizationHistory. M-11).
            let sel_idx = widgets
                .edf_plot
                .obj_idx
                .min(obj_names.len().saturating_sub(1));
            let sel_name = obj_names.get(sel_idx).cloned();
            let key = crate::ui::widget_states::ComparisonSeriesKey {
                base_df: std::sync::Arc::as_ptr(&ctx.view.df) as usize,
                sel_name: sel_name.clone(),
                comps: app_state
                    .comparison_studies
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        (
                            std::sync::Arc::as_ptr(&s.view.df) as usize,
                            app_state
                                .comparison_colors
                                .get(i)
                                .copied()
                                .unwrap_or_else(|| comparison_color_at(i)),
                        )
                    })
                    .collect(),
            };
            let base_name = ctx.meta.name.clone();

            let crate::ui::widget_states::WidgetStates {
                ref mut render_cache,
                ref mut edf_plot,
                ..
            } = *widgets;
            let comparisons = render_cache.edf_comparisons(key, || match sel_name.as_deref() {
                Some(sel_name) => app_state
                    .comparison_studies
                    .iter()
                    .enumerate()
                    .filter_map(|(i, study)| {
                        if !study.view.objective_names().iter().any(|n| n == sel_name) {
                            return None;
                        }
                        let values = study.view.numeric_column(sel_name)?.to_vec();
                        let color = app_state
                            .comparison_colors
                            .get(i)
                            .copied()
                            .unwrap_or_else(|| comparison_color_at(i));
                        Some(EdfComparison {
                            name: study.meta.name.clone(),
                            color: rgba_to_color32(color),
                            values,
                        })
                    })
                    .collect(),
                None => Vec::new(),
            });
            edf_plot.show(ui, &ctx.view, obj_names, &base_name, comparisons);
        }
        ChartId::RankPlot => {
            widgets.rank_plot.show(
                ui,
                &ctx.view,
                param_names,
                obj_names,
                directions,
                &cmap,
                &app_state.artifact_map,
            );
        }
    }
}
