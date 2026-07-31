use crate::state::app_state::{AppState, Direction, StudyContext, StudyView};
use crate::state::messages::AppMessage;
use crate::state::results::ConvergenceHistory;
use crate::ui::widget_states::WidgetStates;
use tunny_core::dataframe::{DataFrame, TrialRow as CoreTrialRow};

/// Handler that processes messages from background tasks.
pub struct MessageHandler;

impl MessageHandler {
    /// Processes a single message and updates AppState and WidgetStates.
    pub fn handle(
        msg: AppMessage,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
        load_error: &mut Option<String>,
    ) {
        match msg {
            AppMessage::JournalParsed { studies, path } => {
                app_state.all_studies = studies;
                app_state.journal_path = Some(path);
                *is_loading = false;
            }
            AppMessage::StudySelected {
                meta,
                study_id,
                pareto_rank,
                pareto_indices,
            } => {
                app_state.clear();
                match tunny_core::dataframe::snapshot(study_id) {
                    Some(df) => {
                        let view = StudyView::new(df, pareto_rank);
                        // Sync the all_studies entry with Phase 2's complete meta
                        // (in Phase 1, completed_trials etc. are 0)
                        if let Some(existing) = app_state
                            .all_studies
                            .iter_mut()
                            .find(|s| s.study_id == meta.study_id)
                        {
                            *existing = meta.clone();
                        }
                        app_state.current_study = Some(StudyContext {
                            meta,
                            view,
                            pareto_indices,
                        });
                        Self::refresh_best_trial_history(app_state);
                    }
                    None => {
                        *load_error =
                            Some(format!("study_id {} not found in shared store", study_id));
                        *is_loading = false;
                        return;
                    }
                }
                widget_states.convergence.computing = false;
                widget_states.cluster_scatter = Default::default();
                widget_states.reset_infeasible_flags();
                *is_loading = false;
            }
            AppMessage::StudyChunkLoaded {
                study_id,
                meta,
                new_rows,
                param_names,
                objective_names,
                user_attr_numeric_names,
                user_attr_string_names,
                max_constraints,
                is_first,
                is_final,
            } => {
                Self::handle_study_chunk(
                    study_id,
                    meta,
                    new_rows,
                    param_names,
                    objective_names,
                    user_attr_numeric_names,
                    user_attr_string_names,
                    max_constraints,
                    is_first,
                    is_final,
                    app_state,
                    widget_states,
                    is_loading,
                );
            }
            AppMessage::SensitivityDone { key, result } => {
                app_state.importance_cache.insert(key, result);
                widget_states.importance.computing = false;
            }
            AppMessage::SensitivityHeatmapDone {
                metric,
                feasible_only,
                result,
            } => {
                app_state
                    .sensitivity_heatmap_cache
                    .insert((metric.cache_id(), feasible_only), result);
                widget_states.sensitivity_heatmap.computing = false;
            }
            AppMessage::SobolDone { key, result } => {
                app_state.sobol_cache.insert(key, result);
                widget_states.importance.computing = false;
            }
            AppMessage::ClusteringDone {
                source,
                key,
                result,
            } => {
                Self::handle_clustering_done(source, key, result, app_state, widget_states);
            }
            AppMessage::ClusterFailed { source, err } => {
                Self::handle_cluster_failed(source, err, widget_states);
            }
            AppMessage::McdmDone {
                source,
                key,
                result,
            } => {
                // Cache per settings key, shared with other charts using the same settings.
                app_state.mcdm_cache.insert(key, result.clone());
                // Keep the most recently computed result as the basis for the McdmScore color mode.
                app_state.mcdm_result = Some(result);
                // Only clear the execution state of the chart that started the computation.
                Self::mcdm_controls_mut(source, widget_states).computing = false;
            }
            AppMessage::McdmFailed { source, message } => {
                let controls = Self::mcdm_controls_mut(source, widget_states);
                controls.computing = false;
                controls.pending_entropy = false;
                *load_error = Some(message);
            }
            AppMessage::EntropyDone { source, result } => {
                let controls = Self::mcdm_controls_mut(source, widget_states);
                controls.weights = result.weights.clone();
                controls.entropy_result = Some(result);
                controls.pending_entropy = false;
                controls.computing = false;
            }
            AppMessage::IndicatorHistoryDone {
                indicator,
                base,
                comparisons,
            } => {
                app_state.convergence_indicator = indicator;
                app_state.convergence_history = Some(base);
                app_state.comparison_convergence_histories = comparisons;
                widget_states.convergence.computing = false;
            }
            AppMessage::Pdp2dDone(result) => {
                widget_states.pdp_2d.result = Some(result);
                widget_states.pdp_2d.computing = false;
            }
            AppMessage::Error(e) => {
                // Failures during report export also reuse the generic Error.
                // If the user still has the dialog open, clear the generating
                // flag and also display it inside the modal.
                if let Some(dialog) = app_state.report_dialog.as_mut() {
                    if dialog.generating {
                        dialog.generating = false;
                        dialog.error = Some(e.clone());
                    }
                }
                *load_error = Some(e);
                *is_loading = false;
            }
            AppMessage::CsvExportDone => {
                // Does nothing on success since there's no user notification for it
                // (export success/failure is only shown via load_error on failure).
            }
            AppMessage::CsvExportFailed(err) => {
                *load_error = Some(err);
            }
            AppMessage::ReportExportDone { paths, overwrote } => {
                if let Some(dialog) = app_state.report_dialog.as_mut() {
                    dialog.generating = false;
                    dialog.error = None;
                    dialog.success_paths = Some(paths);
                    dialog.overwrote_paths = overwrote;
                }
            }
            AppMessage::SensitivityError(_e) => {
                widget_states.importance.computing = false;
            }
            AppMessage::TaskPanicked(detail) => {
                // A worker thread panicked. The widget responsible cannot be
                // identified (spawn_task has no way to know which computation
                // panicked), so surface the error to the user while only
                // clearing the loading flag.
                *load_error = Some(format!(
                    "A background task terminated unexpectedly: {detail}"
                ));
                *is_loading = false;
            }
            AppMessage::PdpDone {
                param,
                objective,
                model_type,
                feasible_only,
                result,
            } => {
                // Insert into the cache before setting result
                widget_states.pdp_chart.insert_cache(
                    &param,
                    &objective,
                    &model_type,
                    feasible_only,
                    result.clone(),
                );
                widget_states.pdp_chart.result = Some(result);
                widget_states.pdp_chart.computing = false;
            }

            AppMessage::ComparisonStudyLoaded { context } => {
                // Keep the 3 parallel Vecs (studies / colors / hv_histories) aligned in the same order.
                let idx = app_state.comparison_studies.len();
                app_state.comparison_studies.push(*context);
                app_state
                    .comparison_colors
                    .push(crate::theme::color_compute::comparison_color_at(idx));
                // Add a placeholder to keep the parallel Vec indices aligned.
                // The actual metric values are overwritten the next time
                // poll_chart batch-recomputes base + all comparisons.
                app_state
                    .comparison_convergence_histories
                    .push(ConvergenceHistory {
                        trial_ids: Vec::new(),
                        values: Vec::new(),
                        sample_step: 1,
                        ref_point: Vec::new(),
                    });
                // Set the baseline Study's metric to None to trigger a unified recomputation.
                app_state.convergence_history = None;
            }
            AppMessage::ArtifactsDirScanned {
                trial_artifacts,
                artifacts_dir,
            } => {
                app_state.artifact_map = trial_artifacts;
                app_state.artifacts_dir = Some(artifacts_dir);
            }
            AppMessage::ComparisonStudyLoadFailed(err) => {
                *load_error = Some(err);
            }
            AppMessage::ObservedContourDone(result) => {
                widget_states.observed_contour.result = Some(result);
                widget_states.observed_contour.computing = false;
                widget_states.observed_contour.error_message = None;
            }
            AppMessage::ObservedContourFailed(err) => {
                widget_states.observed_contour.error_message = Some(err);
                widget_states.observed_contour.computing = false;
            }
            AppMessage::SurrogateFitDone(trained) => {
                widget_states.surrogate_opt.trained = Some(trained);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateFitFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateFitCancelled => {
                // The user cancelled. Just revert the state without showing an error.
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateOptDone(result) => {
                widget_states.surrogate_opt.result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::SurrogateMultiFitDone(trained) => {
                widget_states.surrogate_opt.multi_trained = Some(trained);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateMultiFitFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateMultiFitCancelled => {
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateMultiOptDone(result) => {
                widget_states.surrogate_opt.multi_result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::SurrogateMultiOptFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::SurrogateSuggestDone(result) => {
                widget_states.surrogate_opt.suggest_result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.suggesting = false;
            }
            AppMessage::SurrogateSuggestFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.suggesting = false;
            }
            AppMessage::SurrogateMultiSuggestDone(result) => {
                widget_states.surrogate_opt.multi_suggest_result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.multi_suggesting = false;
            }
            AppMessage::SurrogateMultiSuggestFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.multi_suggesting = false;
            }
            AppMessage::RobustnessFitDone(trained) => {
                widget_states.robustness.trained = Some(trained);
                widget_states.robustness.fit_error = None;
                widget_states.robustness.fitting = false;
            }
            AppMessage::RobustnessFitFailed(err) => {
                widget_states.robustness.fit_error = Some(err);
                widget_states.robustness.fitting = false;
            }
            AppMessage::ResponseSurfaceFitDone(trained) => {
                widget_states.response_surface.trained = Some(trained);
                widget_states.response_surface.fit_error = None;
                widget_states.response_surface.fitting = false;
            }
            AppMessage::ResponseSurfaceFitFailed(err) => {
                widget_states.response_surface.fit_error = Some(err);
                widget_states.response_surface.fitting = false;
            }
            AppMessage::SurrogateCompareDone(result) => {
                widget_states.surrogate_compare.result = Some(result);
                widget_states.surrogate_compare.error = None;
                widget_states.surrogate_compare.computing = false;
            }
            AppMessage::SurrogateCompareFailed(err) => {
                widget_states.surrogate_compare.error = Some(err);
                widget_states.surrogate_compare.computing = false;
            }
            AppMessage::GhOptFinished { result } => {
                if let Some(run) = app_state.gh_opt_run.as_mut() {
                    run.finished = Some(match result {
                        Ok(summary) => Ok(Self::format_gh_summary(&summary)),
                        Err(e) => Err(e),
                    });
                }
            }
            AppMessage::ProcessOptFinished { result } => {
                // Reuses the same run-overlay state (`gh_opt_run`) as the .ghx run.
                if let Some(run) = app_state.gh_opt_run.as_mut() {
                    run.finished = Some(match result {
                        Ok(summary) => Ok(format!(
                            "Done: {} trials succeeded / {} failed{}",
                            summary.completed,
                            summary.failed,
                            if summary.cancelled {
                                " (cancelled)"
                            } else {
                                ""
                            }
                        )),
                        Err(e) => Err(e),
                    });
                }
            }
        }
    }
}

mod clustering;
mod gh;
mod study;

#[cfg(test)]
mod tests;
