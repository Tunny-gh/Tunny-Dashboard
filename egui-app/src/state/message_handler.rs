#[cfg(test)]
use crate::state::app_state::Direction;
use crate::state::app_state::{AppState, StudyContext, StudyView};
use crate::state::messages::AppMessage;
use crate::state::results::ConvergenceHistory;
use crate::ui::widget_states::WidgetStates;
#[cfg(test)]
use tunny_core::dataframe::{DataFrame, TrialRow as CoreTrialRow};

/// Handler that processes messages from background tasks.
pub struct MessageHandler;

impl MessageHandler {
    fn fixture_delivery_only() {
        #[cfg(not(test))]
        unreachable!("Shared results must arrive through the stamped handoff");
    }
    /// Processes a single message and updates AppState and WidgetStates.
    pub fn handle(
        msg: AppMessage,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
        load_error: &mut Option<String>,
    ) {
        match msg {
            AppMessage::AnalysisUnavailable(slot) => slot.clear_busy(widget_states),
            AppMessage::AnalysisCompleted { slot, completion } => {
                for message in crate::state::analysis_job::adopt_all(
                    slot,
                    completion,
                    app_state,
                    widget_states,
                ) {
                    Self::handle(message, app_state, widget_states, is_loading, load_error);
                }
            }
            AppMessage::JournalBatch {
                header,
                batch,
                reply,
            } => {
                let result = app_state
                    .journal_studies
                    .as_mut()
                    .filter(|j| j.source == header.ticket.study().source)
                    .and_then(|j| j.managers.get_mut(&header.ticket.study().stored_id))
                    .ok_or_else(|| "Superseded Journal source".to_string())
                    .and_then(|m| m.apply(header, batch).map_err(|e| e.to_string()));
                if result.is_ok() {
                    Self::publish_journal(header.ticket.study(), app_state, widget_states);
                }
                let _ = reply.send(result);
            }
            AppMessage::JournalFinished { header, result } => {
                let selected = app_state
                    .journal_studies
                    .as_ref()
                    .is_some_and(|j| j.selected == Some(header.ticket.study().stored_id));
                let Some(manager) = app_state
                    .journal_studies
                    .as_mut()
                    .filter(|j| j.source == header.ticket.study().source)
                    .and_then(|j| j.managers.get_mut(&header.ticket.study().stored_id))
                else {
                    return;
                };
                if manager.header(header.ticket).is_err() {
                    return;
                }
                let succeeded = match result {
                    Ok(()) => {
                        if let Err(e) = manager.finish(header.ticket, header.sequence) {
                            let _ = manager.fail(header.ticket);
                            *load_error = Some(e.to_string());
                            false
                        } else {
                            true
                        }
                    }
                    Err(e) => {
                        let _ = manager.fail(header.ticket);
                        *load_error = Some(e);
                        false
                    }
                };
                if !succeeded {
                    Self::complete_journal_comparison(header.ticket.study(), None, app_state);
                }
                Self::publish_journal(header.ticket.study(), app_state, widget_states);
                if selected {
                    *is_loading = false;
                }
            }
            AppMessage::JournalActivate { key } => {
                if !app_state
                    .journal_studies
                    .as_ref()
                    .filter(|j| j.source == key.source)
                    .and_then(|j| j.managers.get(&key.stored_id))
                    .and_then(|m| m.load_state())
                    .is_some_and(|s| s.phase == tdashboard_study::LoadPhase::Complete)
                {
                    return;
                }
                Self::publish_journal(key, app_state, widget_states);
                if app_state
                    .journal_studies
                    .as_ref()
                    .is_some_and(|j| j.source == key.source && j.selected == Some(key.stored_id))
                {
                    *is_loading = false;
                }
            }
            AppMessage::JournalScanned { .. } => {
                unreachable!("Journal scan is handled by the entry adapter")
            }
            AppMessage::JournalParsed { studies, path } => {
                app_state.all_studies = studies;
                app_state.journal_path = Some(path);
                *is_loading = false;
            }
            #[cfg(test)]
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
            #[cfg(test)]
            AppMessage::StudyChunkLoaded {
                study_id,
                meta,
                new_rows,
                param_names,
                objective_names,
                user_attr_numeric_names,
                user_attr_string_names,
                max_constraints,
                has_constraints,
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
                    has_constraints,
                    is_first,
                    is_final,
                    app_state,
                    widget_states,
                    is_loading,
                );
            }
            AppMessage::SharedAnalysis { slot, result } => {
                use crate::state::analysis_job::DeliverySlot;
                use crate::state::results::{ClusterResult, McdmResult};
                use tdashboard_analysis::shared::SharedResult;
                let Some(view) = app_state.current_study.as_ref().map(|s| &s.view) else {
                    return;
                };
                if view
                    .input()
                    .is_none_or(|i| result.key().analysis.inputs != [i.stamp()])
                {
                    return;
                }
                match slot {
                    DeliverySlot::Cluster(source) => {
                        if let Ok((key, shared)) = SharedResult::<ClusterResult>::keyed::<
                            crate::ui::widgets::cluster_scatter::ClusterCacheKey,
                        >(result)
                        {
                            if view.trial_keys().is_some_and(|rows| rows != shared.rows()) {
                                return;
                            }
                            app_state.cluster_cache.select(key, shared);
                            Self::clear_cluster_runtime(source, widget_states);
                        }
                    }
                    DeliverySlot::Mcdm(source) => {
                        if let Ok((key, shared)) = SharedResult::<McdmResult>::keyed::<
                            crate::ui::widgets::mcdm_chart::McdmCacheKey,
                        >(result)
                        {
                            if view.trial_keys().is_some_and(|rows| rows != shared.rows()) {
                                return;
                            }
                            app_state.mcdm_cache.select(key, shared.clone());
                            app_state.mcdm_result = Some(shared);
                            Self::mcdm_controls_mut(source, widget_states).computing = false;
                        }
                    }
                    DeliverySlot::Importance => {
                        if app_state.importance_cache.adopt(result.clone()).is_err() {
                            let _ = app_state.sobol_cache.adopt(result);
                        }
                        widget_states.importance.computing = false;
                    }
                    DeliverySlot::Heatmap => {
                        let _ = app_state.sensitivity_heatmap_cache.adopt(result);
                        widget_states.sensitivity_heatmap.computing = false;
                    }
                    _ => unreachable!("Only shared widget results use this delivery"),
                }
            }
            AppMessage::ConsumerDelivery { .. } => {
                unreachable!("Consumer delivery must be routed by the entry")
            }
            AppMessage::SensitivityDone { key, result } => {
                Self::fixture_delivery_only();
                #[cfg(test)]
                app_state
                    .importance_cache
                    .select(key, crate::state::analysis_job::fixture_result(result));
                #[cfg(not(test))]
                let _ = (key, result);
                widget_states.importance.computing = false;
            }
            AppMessage::SensitivityHeatmapDone {
                metric,
                feasible_only,
                result,
            } => {
                Self::fixture_delivery_only();
                #[cfg(test)]
                app_state.sensitivity_heatmap_cache.select(
                    (metric.cache_id(), feasible_only),
                    crate::state::analysis_job::fixture_result(result),
                );
                #[cfg(not(test))]
                let _ = (metric, feasible_only, result);
                widget_states.sensitivity_heatmap.computing = false;
            }
            AppMessage::SobolDone { key, result } => {
                Self::fixture_delivery_only();
                #[cfg(test)]
                app_state
                    .sobol_cache
                    .select(key, crate::state::analysis_job::fixture_result(result));
                #[cfg(not(test))]
                let _ = (key, result);
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
                Self::fixture_delivery_only();
                #[cfg(test)]
                {
                    let shared = crate::state::analysis_job::fixture_result(result);
                    app_state.mcdm_cache.select(key, shared.clone());
                    app_state.mcdm_result = Some(shared);
                }
                #[cfg(not(test))]
                let _ = (key, result);
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
                #[cfg(test)]
                source_df,
                indicator,
                base,
                comparisons,
            } => {
                #[cfg(test)]
                if app_state.study_streaming
                    || !app_state.current_study.as_ref().is_some_and(|study| {
                        match source_df.provenance() {
                            Some(stamp) => study.view.df.provenance() == Some(stamp),
                            None => std::sync::Arc::ptr_eq(&study.view.df, &source_df),
                        }
                    })
                {
                    // Do not clear the running flag of a newer snapshot's job.
                    return;
                }
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

            #[cfg(test)]
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
                ..
            } => {
                app_state.artifact_map = trial_artifacts;
                app_state.artifact_revision = app_state.artifact_revision.wrapping_add(1);
                app_state.artifacts_dir = Some(artifacts_dir);
            }
            #[cfg(test)]
            AppMessage::CsvArtifacts {
                source_path,
                source_generation,
                images,
            } => {
                if app_state.source_generation != source_generation
                    || app_state.journal_path.as_ref() != Some(&source_path)
                {
                    return;
                }
                let root = app_state.explicit_artifact_root.clone().unwrap_or_else(|| {
                    source_path
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .to_path_buf()
                });
                app_state.artifact_map = crate::io::flat_csv::build_artifact_map(&root, &images);
                app_state.artifact_revision = app_state.artifact_revision.wrapping_add(1);
                app_state.artifacts_dir = Some(root);
                app_state.csv_images = Some(images);
            }
            #[cfg(test)]
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
