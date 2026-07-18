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
            AppMessage::PollerReady { .. } => {
                // Poller startup is intercepted and handled by app.rs
                // (poll_messages), which holds the tx/poller, so this never
                // reaches MessageHandler (unreachable here).
            }
            AppMessage::LiveUpdateDone {
                new_trial_rows,
                updated_study_counts,
                extras_events,
            } => {
                Self::handle_live_update_done(
                    new_trial_rows,
                    updated_study_counts,
                    extras_events,
                    app_state,
                );
            }
            AppMessage::LiveUpdateError(msg) => {
                app_state.live_update.poller_active = false;
                *load_error = Some(msg);
            }
            AppMessage::LiveUpdateMaybeComplete => {
                app_state.live_update.showing_completion_hint = true;
            }
            AppMessage::SqliteLiveChanged { .. } => {
                // The actual reload has to go through a worker thread, so no
                // state is changed here. The caller (app.rs) detects this
                // message via `sqlite_reload_study_id` and issues
                // `dispatch_reload_sqlite_study`. The reload result arrives as
                // `SqliteLiveReloadDone`.
            }
            AppMessage::SqliteLiveReloadDone { study_id, meta } => {
                Self::handle_sqlite_live_reload_done(study_id, meta, app_state);
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

    /// Builds a single-objective Study's best-so-far history (trial_number,
    /// cumulative best) and stores it in `app_state.best_trial_history`. Left
    /// as None for multi-objective Studies (the convergence card is hidden,
    /// and the HV history instead handles the multi-objective trend).
    fn refresh_best_trial_history(app_state: &mut AppState) {
        let Some(ctx) = app_state.current_study.as_ref() else {
            app_state.best_trial_history = None;
            return;
        };
        if ctx.meta.directions.len() != 1 || ctx.meta.objective_names.len() != 1 {
            app_state.best_trial_history = None;
            return;
        }
        let Some(values) = ctx.view.numeric_column(&ctx.meta.objective_names[0]) else {
            app_state.best_trial_history = None;
            return;
        };
        let is_minimize = matches!(ctx.meta.directions[0], Direction::Minimize);
        let n = ctx.view.row_count();
        let trial_numbers: Vec<u32> = (0..n)
            .map(|i| ctx.view.df.get_trial_number(i).unwrap_or(i as u32))
            .collect();
        app_state.best_trial_history = Some(tunny_core::convergence::build_best_trial_history(
            &trial_numbers,
            values,
            is_minimize,
        ));
    }

    /// Applies one batch of the streaming load when a Study is selected.
    ///
    /// - First batch (`is_first`): clears existing state and creates a new StudyContext.
    /// - Subsequent batches: clones the existing DataFrame's columns and
    ///   appends new rows via `append_trials`. Rebuilding into row-oriented
    ///   form (the old core_rows_from_df approach) was removed because it
    ///   involved O(loaded row count) HashMap/String allocation per chunk,
    ///   making the whole load O(n^2).
    /// - Since Pareto is expensive (multi-objective nd_sort is O(N^2)), it is
    ///   **not computed during streaming**; it's computed once, definitively,
    ///   on the `is_final` batch (rank 0 is shown while loading).
    #[allow(clippy::too_many_arguments)]
    fn handle_study_chunk(
        study_id: u32,
        meta: crate::state::app_state::StudyMeta,
        new_rows: Vec<CoreTrialRow>,
        param_names: Vec<String>,
        objective_names: Vec<String>,
        user_attr_numeric_names: Vec<String>,
        user_attr_string_names: Vec<String>,
        max_constraints: usize,
        is_first: bool,
        is_final: bool,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
    ) {
        // The first batch resets existing state as a Study switch.
        // Subsequent batches: column clone (equivalent to memcpy) + in-place
        // append. An increase in constraint column count is absorbed by
        // append_trials, which takes the max with the existing column count.
        let start_fresh = is_first || app_state.current_study.is_none();
        let mut new_df = if start_fresh {
            app_state.clear();
            DataFrame::empty()
        } else {
            app_state
                .current_study
                .as_ref()
                .map(|s| (*s.view.df).clone())
                .unwrap_or_else(DataFrame::empty)
        };
        new_df.append_trials(
            &new_rows,
            &param_names,
            &objective_names,
            &user_attr_numeric_names,
            &user_attr_string_names,
            max_constraints,
        );
        let arc = std::sync::Arc::new(new_df);
        tunny_core::dataframe::swap_snapshot(study_id, arc.clone());

        // Pareto is only finalized on the last batch. Activated via select_study since it reads the active DataFrame.
        let (ranks, pareto_indices) = if is_final {
            let _ = tunny_core::dataframe::select_study(study_id);
            let is_minimize: Vec<bool> = meta
                .directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();
            let pareto = tunny_core::pareto::compute_pareto_ranks(&is_minimize);
            (pareto.ranks, pareto.pareto_indices)
        } else {
            (Vec::new(), Vec::new())
        };

        let view = StudyView::new(arc, ranks);
        if let Some(study) = &mut app_state.current_study {
            study.meta = meta.clone();
            study.view = view;
            study.pareto_indices = pareto_indices;
        } else {
            app_state.current_study = Some(StudyContext {
                meta: meta.clone(),
                view,
                pareto_indices,
            });
        }

        // Sync the all_studies entry with Phase 2's cumulative meta.
        if let Some(existing) = app_state
            .all_studies
            .iter_mut()
            .find(|s| s.study_id == study_id)
        {
            *existing = meta;
        }

        if start_fresh {
            // Activate early so downstream features can reference the active DataFrame.
            let _ = tunny_core::dataframe::select_study(study_id);
            widget_states.convergence.computing = false;
            widget_states.cluster_scatter = Default::default();
            widget_states.cluster_scatter_3d.clear_runtime_state();
            widget_states.trial_table.cluster.clear_runtime_state();
            app_state.cluster_cache.clear();
            app_state.mcdm_cache.clear();
            app_state.mcdm_result = None;
            widget_states.reset_infeasible_flags();
        }

        if is_final {
            Self::refresh_best_trial_history(app_state);
            *is_loading = false;
        }
    }

    /// Returns the study_id that needs to be reloaded, given an `AppMessage::SqliteLiveChanged`.
    /// Since SQLite mutates trial state in place, journal-style diff
    /// application can't be used; upon detecting a fingerprint change, the
    /// target study must be re-parsed entirely. That re-parse has to go
    /// through a worker thread (`MessageHandler::handle` has no tx), so the
    /// caller (app.rs) extracts the target study_id via this function and
    /// issues `dispatch_reload_sqlite_study` (RDB live update also reuses
    /// `SqliteLiveChanged`, so when storage_kind is Rdb, app.rs routes it to
    /// `dispatch_reload_rdb_study` instead).
    pub fn sqlite_reload_study_id(msg: &AppMessage) -> Option<u32> {
        match msg {
            AppMessage::SqliteLiveChanged { study_id } => Some(*study_id),
            _ => None,
        }
    }

    /// The processing body for `AppMessage::SqliteLiveReloadDone`.
    ///
    /// The worker thread (`crate::io::sqlite::reload_single_study_task`) has
    /// already replaced the shared store via `swap_snapshot` /
    /// `store_extras_for`, so this only does:
    /// - Activation (`select_study`)
    /// - Recomputing Pareto ranks (same computation as a normal sqlite study selection)
    /// - Replacing the StudyView
    /// - Rebuilding the convergence history / discarding row-dependent caches
    ///   (identical to `handle_live_update_done`)
    ///
    /// Unlike journal, this replaces the entire DataFrame rather than
    /// appending new rows, but the post-swap merge processing (cache discard,
    /// history recomputation, study count update) is made exactly the same as
    /// `handle_live_update_done`.
    fn handle_sqlite_live_reload_done(
        study_id: u32,
        meta: crate::state::app_state::StudyMeta,
        app_state: &mut AppState,
    ) {
        if let Some(study) = &mut app_state.current_study {
            if study.meta.study_id == study_id {
                if let Some(df) = tunny_core::dataframe::snapshot(study_id) {
                    // Adopt the df already swapped in by the worker, refresh
                    // meta, then rebuild Pareto + StudyView (shared with handle_live_update_done).
                    study.meta = meta.clone();
                    let is_minimize: Vec<bool> = meta
                        .directions
                        .iter()
                        .map(|d| matches!(d, Direction::Minimize))
                        .collect();
                    Self::rebuild_active_view(study, df, &is_minimize);
                }
            }
        }

        // Rebuild the convergence history / row-dependent caches since trial
        // count and best value may have changed (same as handle_live_update_done).
        Self::invalidate_row_dependent_state(app_state);

        if let Some(existing) = app_state
            .all_studies
            .iter_mut()
            .find(|m| m.study_id == study_id)
        {
            *existing = meta;
        }
    }

    /// Activates the already-swapped `arc`, recomputes Pareto, and rebuilds
    /// `study`'s StudyView (D-7: shared processing for live update / sqlite,rdb reload).
    /// Before calling, `study.meta` must be up to date and `arc` must already
    /// be swapped into the shared store.
    fn rebuild_active_view(
        study: &mut StudyContext,
        arc: std::sync::Arc<DataFrame>,
        is_minimize: &[bool],
    ) {
        let _ = tunny_core::dataframe::select_study(study.meta.study_id);
        let pareto = tunny_core::pareto::compute_pareto_ranks(is_minimize);
        study.view = StudyView::new(arc, pareto.ranks);
        study.pareto_indices = pareto.pareto_indices;
    }

    /// Shared post-processing after a live update / reload changes the trial
    /// count (D-7): recomputes the best-trial history and discards
    /// row-dependent caches (cluster / mcdm).
    fn invalidate_row_dependent_state(app_state: &mut AppState) {
        Self::refresh_best_trial_history(app_state);
        app_state.cluster_cache.clear();
        app_state.mcdm_cache.clear();
        app_state.mcdm_result = None;
    }

    /// Appends keys not yet in `dst` (and not `reserved`) in sorted order, so
    /// column creation order is deterministic regardless of HashMap iteration.
    fn extend_new_keys<'k>(
        dst: &mut Vec<String>,
        keys: impl Iterator<Item = &'k String>,
        reserved: &dyn Fn(&String) -> bool,
    ) {
        let new_keys: std::collections::BTreeSet<&String> =
            keys.filter(|k| !dst.contains(*k) && !reserved(k)).collect();
        dst.extend(new_keys.into_iter().cloned());
    }

    fn handle_live_update_done(
        new_core_rows: Vec<tunny_core::io::journal::live_update::TrialRow>,
        updated_study_counts: Vec<(u32, usize)>,
        extras_events: tunny_core::io::journal::live_update::ExtrasDiff,
        app_state: &mut AppState,
    ) {
        // Whether the DataFrame was rebuilt due to newly completed trial rows.
        // An update that is only intermediate values / state changes (extras)
        // doesn't change the columns, so skip the full clone + O(N^2) Pareto
        // recomputation + row-dependent cache discard (M-7).
        let mut df_rebuilt = false;
        if let Some(study) = &mut app_state.current_study {
            let study_id = study.meta.study_id;

            // Merge the live diff into the supplementary info (extras) for all trials (all states).
            Self::merge_extras_diff(study_id, &extras_events);

            // Append only the new trials from the live diff to a clone of the
            // existing DataFrame's columns (does not rebuild all rows into row-oriented form).
            let added_rows: Vec<CoreTrialRow> = new_core_rows
                .iter()
                .map(|core_row| CoreTrialRow {
                    trial_id: core_row.trial_id,
                    trial_number: core_row.trial_number,
                    param_display: core_row.params.clone(),
                    param_category_label: core_row.param_categories.clone(),
                    objective_values: core_row.objectives.clone(),
                    user_attrs_numeric: core_row.user_attrs_numeric.clone(),
                    user_attrs_string: core_row.user_attrs_string.clone(),
                    constraint_values: core_row.constraint_values.clone(),
                })
                .collect();

            // An update with no new trial rows (e.g. intermediate-value
            // reports while RUNNING) doesn't change the columns. extras was
            // already merged above, so stop here to avoid the heavy recomputation (M-7).
            if !added_rows.is_empty() {
                let param_names = study.meta.param_names.clone();
                let obj_names = study.meta.objective_names.clone();
                // User attrs (like constraints below) may first appear in live
                // rows; append_trials backfills existing rows when passed names
                // beyond the current columns. Keys whose name is already used
                // by a non-user-attr column must NOT be adopted: append_trials'
                // same-name pending queues would cross-contaminate the two
                // columns on later batches.
                let mut un = study.view.df.user_attr_numeric_col_names().to_vec();
                let mut us = study.view.df.user_attr_string_col_names().to_vec();
                let df = &study.view.df;
                let reserved = |name: &String| {
                    df.param_col_names().contains(name)
                        || df.objective_col_names().contains(name)
                        || df.constraint_col_names().contains(name)
                        || name == "is_feasible"
                        || name == "constraint_sum"
                };
                Self::extend_new_keys(
                    &mut un,
                    added_rows.iter().flat_map(|r| r.user_attrs_numeric.keys()),
                    &reserved,
                );
                Self::extend_new_keys(
                    &mut us,
                    added_rows.iter().flat_map(|r| r.user_attrs_string.keys()),
                    &reserved,
                );
                // Constraints may first appear in live rows (e.g. a .ghx run on
                // a fresh journal); append_trials itself takes the max with the
                // existing column count (the count never shrinks).
                let incoming_c = added_rows
                    .iter()
                    .map(|r| r.constraint_values.len())
                    .max()
                    .unwrap_or(0);
                let mut new_df = (*study.view.df).clone();
                new_df.append_trials(&added_rows, &param_names, &obj_names, &un, &us, incoming_c);

                let is_minimize: Vec<bool> = study
                    .meta
                    .directions
                    .iter()
                    .map(|d| matches!(d, Direction::Minimize))
                    .collect();

                // First swap in and activate the shared store, then compute
                // Pareto from a DataFrame with aligned columns (same approach
                // as handle_study_chunk). Passing all_rows directly to
                // nd_sort would panic on an out-of-range slice if the live
                // diff includes rows with a different objective count.
                // from_trials / compute_pareto_ranks always align the shape
                // by filling missing objectives with NaN.
                let arc = std::sync::Arc::new(new_df);
                tunny_core::dataframe::swap_snapshot(study_id, arc.clone());
                Self::rebuild_active_view(study, arc, &is_minimize);
                df_rebuilt = true;
            }
        }
        // Only rebuild the convergence history / row-dependent caches when the trial count or best value changed (M-7).
        if df_rebuilt {
            Self::invalidate_row_dependent_state(app_state);
        }

        // Update all_studies completed_trials
        for (study_id, new_count) in updated_study_counts {
            if let Some(meta) = app_state
                .all_studies
                .iter_mut()
                .find(|m| m.study_id == study_id)
            {
                meta.completed_trials = new_count;
            }
        }
    }

    /// Merges the live diff's [`ExtrasDiff`] into the target study's
    /// [`StudyExtras`] and atomically swaps it into the shared store.
    /// Maintains ascending trial_id order.
    ///
    /// - new_trials: adds [`TrialExtra`] entries with state=Running (existing trial_ids are left as-is).
    /// - intermediate_values: appends (step, value) to the corresponding trial (generates a placeholder if unknown).
    /// - state_changes: updates state and datetime_complete (generates a placeholder if unknown).
    fn merge_extras_diff(study_id: u32, diff: &tunny_core::io::journal::live_update::ExtrasDiff) {
        use std::collections::HashMap;
        use tunny_core::extras::{StudyExtras, TrialExtra, TrialState};

        if diff.new_trials.is_empty()
            && diff.intermediate_values.is_empty()
            && diff.state_changes.is_empty()
        {
            return;
        }

        // Make a mutable copy based on the current snapshot (empty if absent).
        let mut extras: StudyExtras = tunny_core::dataframe::extras_snapshot(study_id)
            .map(|arc| (*arc).clone())
            .unwrap_or_default();

        let mut index_of: HashMap<u32, usize> = extras
            .trials
            .iter()
            .enumerate()
            .map(|(i, t)| (t.trial_id, i))
            .collect();

        // Returns the index corresponding to trial_id. If absent, generates a
        // Running placeholder. (When trial_number is unknown, trial_id is
        // used as a provisional value — the same fallback as live_update.)
        fn ensure_trial(
            extras: &mut StudyExtras,
            index_of: &mut HashMap<u32, usize>,
            trial_id: u32,
            trial_number: u32,
            datetime_start: Option<f64>,
        ) -> usize {
            if let Some(&idx) = index_of.get(&trial_id) {
                return idx;
            }
            let idx = extras.trials.len();
            extras.trials.push(TrialExtra {
                trial_id,
                trial_number,
                state: TrialState::Running,
                datetime_start,
                datetime_complete: None,
                intermediate_values: Vec::new(),
            });
            index_of.insert(trial_id, idx);
            idx
        }

        for &(trial_id, _study, trial_number, datetime_start) in &diff.new_trials {
            let idx = ensure_trial(
                &mut extras,
                &mut index_of,
                trial_id,
                trial_number,
                datetime_start,
            );
            // For an existing trial, only fill in datetime_start.
            if extras.trials[idx].datetime_start.is_none() {
                extras.trials[idx].datetime_start = datetime_start;
            }
        }

        for &(trial_id, step, value) in &diff.intermediate_values {
            let idx = ensure_trial(&mut extras, &mut index_of, trial_id, trial_id, None);
            extras.trials[idx].intermediate_values.push((step, value));
        }

        for &(trial_id, state, datetime_complete) in &diff.state_changes {
            let idx = ensure_trial(&mut extras, &mut index_of, trial_id, trial_id, None);
            extras.trials[idx].state = TrialState::from_journal(state);
            if datetime_complete.is_some() {
                extras.trials[idx].datetime_complete = datetime_complete;
            }
        }

        // Maintain ascending trial_id order, and sort each trial's intermediate values by ascending step.
        extras.trials.sort_by_key(|t| t.trial_id);
        for trial in &mut extras.trials {
            trial.intermediate_values.sort_by_key(|(step, _)| *step);
        }

        tunny_core::dataframe::swap_extras(study_id, std::sync::Arc::new(extras));
    }

    fn handle_clustering_done(
        source: crate::state::messages::ClusterChartSource,
        key: crate::ui::widgets::cluster_scatter::ClusterCacheKey,
        result: crate::state::results::ClusterResult,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
    ) {
        let trial_count = app_state
            .current_study
            .as_ref()
            .map(|c| c.trial_count())
            .unwrap_or(0);
        if result.labels.len() == trial_count {
            // Cache the result per settings key, shared with other charts using the same settings.
            app_state.cluster_cache.insert(key, result);
            // Clear the spinner / pending state of the chart that started the run.
            Self::clear_cluster_runtime(source, widget_states);
        } else {
            let err = crate::state::messages::cluster_ui_error(
                "Cluster result is inconsistent. Please run again.",
                Some(format!(
                    "validation: labels_len({}) != trial_count({})",
                    result.labels.len(),
                    trial_count
                )),
                true,
            );
            Self::set_cluster_error(source, err, widget_states);
        }
    }

    fn handle_cluster_failed(
        source: crate::state::messages::ClusterChartSource,
        err: crate::state::messages::ClusterUiError,
        widget_states: &mut WidgetStates,
    ) {
        Self::set_cluster_error(source, err, widget_states);
    }

    /// Clears the execution state of the widget that started clustering.
    fn clear_cluster_runtime(
        source: crate::state::messages::ClusterChartSource,
        widget_states: &mut WidgetStates,
    ) {
        use crate::state::messages::ClusterChartSource;
        match source {
            ClusterChartSource::Scatter2D => widget_states.cluster_scatter.clear_runtime_state(),
            ClusterChartSource::Scatter3D => widget_states.cluster_scatter_3d.clear_runtime_state(),
            ClusterChartSource::Table => widget_states.trial_table.cluster.clear_runtime_state(),
            ClusterChartSource::ArtifactGallery => {
                widget_states.artifact_gallery.clear_cluster_runtime()
            }
        }
    }

    /// Returns a mutable reference to the controls of the chart that started the MCDM computation.
    fn mcdm_controls_mut(
        source: crate::state::messages::McdmChartSource,
        widget_states: &mut WidgetStates,
    ) -> &mut crate::ui::widgets::mcdm_chart::McdmControls {
        use crate::state::messages::McdmChartSource;
        match source {
            McdmChartSource::Rank => &mut widget_states.mcdm_chart.controls,
            McdmChartSource::Scatter2D => &mut widget_states.scatter_chart.controls,
            McdmChartSource::Scatter3D => &mut widget_states.mcdm_scatter_3d.controls,
            McdmChartSource::Table => &mut widget_states.trial_table.mcdm.controls,
            McdmChartSource::ArtifactGallery => &mut widget_states.artifact_gallery.mcdm,
        }
    }

    /// Sets an error on the widget that started clustering.
    fn set_cluster_error(
        source: crate::state::messages::ClusterChartSource,
        err: crate::state::messages::ClusterUiError,
        widget_states: &mut WidgetStates,
    ) {
        use crate::state::messages::ClusterChartSource;
        match source {
            ClusterChartSource::Scatter2D => widget_states.cluster_scatter.set_error(err),
            ClusterChartSource::Scatter3D => widget_states.cluster_scatter_3d.set_error(err),
            ClusterChartSource::Table => widget_states.trial_table.cluster.set_error(err),
            ClusterChartSource::ArtifactGallery => {
                widget_states.artifact_gallery.set_cluster_error(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{Direction, StudyMeta};

    /// For tests: stores a DataFrame into the shared store (thread_local in
    /// test builds) and returns a new StudySelected payload (study_id + pareto_rank).
    fn make_study_message(trial_count: usize) -> AppMessage {
        let core_rows: Vec<CoreTrialRow> = (0..trial_count)
            .map(|i| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["y".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::store_dataframes(vec![df]);

        AppMessage::StudySelected {
            meta: StudyMeta {
                study_id: 0,
                name: "s".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: trial_count,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                param_bounds: Default::default(),
            },
            study_id: 0,
            pareto_rank: vec![0; trial_count],
            pareto_indices: vec![],
        }
    }

    /// Guard for serializing tests that use the shared store (a process-global
    /// in production builds). Since tunny-desktop's tests link tunny-core
    /// normally, the store is shared across all tests. Tests using
    /// store_dataframes + snapshot are serialized with this guard to prevent races.
    fn test_store_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// For tests: builds a StudySelected for a single-objective Study, with
    /// arbitrary objective values / direction (for verifying best_trial_history wiring).
    fn make_study_message_single_objective(values: &[f64], direction: Direction) -> AppMessage {
        let trial_count = values.len();
        let core_rows: Vec<CoreTrialRow> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![v],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["y".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::store_dataframes(vec![df]);

        AppMessage::StudySelected {
            meta: StudyMeta {
                study_id: 0,
                name: "s".to_string(),
                directions: vec![direction],
                completed_trials: trial_count,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                param_bounds: Default::default(),
            },
            study_id: 0,
            pareto_rank: vec![0; trial_count],
            pareto_indices: vec![],
        }
    }

    /// For tests: builds a StudySelected for a 2-objective Study (for
    /// verifying best_trial_history stays None for multi-objective).
    fn make_study_message_multi_objective(trial_count: usize) -> AppMessage {
        let core_rows: Vec<CoreTrialRow> = (0..trial_count)
            .map(|i| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64, (trial_count - i) as f64],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["y1".to_string(), "y2".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::store_dataframes(vec![df]);

        AppMessage::StudySelected {
            meta: StudyMeta {
                study_id: 0,
                name: "s".to_string(),
                directions: vec![Direction::Minimize, Direction::Minimize],
                completed_trials: trial_count,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y1".to_string(), "y2".to_string()],
                param_bounds: Default::default(),
            },
            study_id: 0,
            pareto_rank: vec![0; trial_count],
            pareto_indices: vec![],
        }
    }

    #[test]
    fn best_trial_history_set_for_single_objective_minimize() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message_single_objective(&[3.0, 1.0, 2.0], Direction::Minimize),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(
            app_state.best_trial_history,
            Some(vec![(0, 3.0), (1, 1.0), (2, 1.0)])
        );
    }

    #[test]
    fn best_trial_history_set_for_single_objective_maximize() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message_single_objective(&[1.0, 3.0, 2.0], Direction::Maximize),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(
            app_state.best_trial_history,
            Some(vec![(0, 1.0), (1, 3.0), (2, 3.0)])
        );
    }

    #[test]
    fn best_trial_history_none_for_multi_objective() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message_multi_objective(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.best_trial_history.is_none());
    }

    #[test]
    fn clustering_done_updates_state_when_lengths_match() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        widgets.cluster_scatter.computing = true;
        let key = widgets.cluster_scatter.cache_key();
        MessageHandler::handle(
            AppMessage::ClusteringDone {
                source: crate::state::messages::ClusterChartSource::Scatter2D,
                key: key.clone(),
                result: crate::state::results::ClusterResult {
                    labels: vec![0, 1, 0],
                    n_clusters: 2,
                },
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.cluster_cache.contains_key(&key));
        assert!(!widgets.cluster_scatter.computing);
        assert!(widgets.cluster_scatter.last_error.is_none());
    }

    #[test]
    fn clustering_done_rejects_mismatched_label_length() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let key = widgets.cluster_scatter.cache_key();
        MessageHandler::handle(
            AppMessage::ClusteringDone {
                source: crate::state::messages::ClusterChartSource::Scatter2D,
                key: key.clone(),
                result: crate::state::results::ClusterResult {
                    labels: vec![0, 1],
                    n_clusters: 2,
                },
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.cluster_cache.is_empty());
        assert!(widgets.cluster_scatter.last_error.is_some());
    }

    fn make_core_trial_row(
        trial_id: u32,
        study_id: u32,
        objectives: Vec<f64>,
    ) -> tunny_core::io::journal::live_update::TrialRow {
        tunny_core::io::journal::live_update::TrialRow {
            trial_id,
            trial_number: trial_id,
            params: std::collections::HashMap::new(),
            param_categories: std::collections::HashMap::new(),
            objectives,
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
            study_id,
        }
    }

    fn make_chunk_row(trial_id: u32, x: f64, obj: f64) -> CoreTrialRow {
        CoreTrialRow {
            trial_id,
            trial_number: trial_id,
            param_display: std::collections::HashMap::from([("x".to_string(), x)]),
            param_category_label: std::collections::HashMap::new(),
            objective_values: vec![obj],
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
        }
    }

    fn chunk_message(rows: Vec<CoreTrialRow>, is_first: bool, is_final: bool) -> AppMessage {
        AppMessage::StudyChunkLoaded {
            study_id: 0,
            meta: StudyMeta {
                study_id: 0,
                name: "s".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 0,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                param_bounds: Default::default(),
            },
            new_rows: rows,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_numeric_names: vec![],
            user_attr_string_names: vec![],
            max_constraints: 0,
            is_first,
            is_final,
        }
    }

    #[test]
    fn study_chunks_accumulate_rows_across_batches() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = true;
        let mut load_error = None;

        // 1st batch: establishes study, still loading.
        MessageHandler::handle(
            chunk_message(
                vec![make_chunk_row(0, 0.1, 1.0), make_chunk_row(1, 0.2, 2.0)],
                true,
                false,
            ),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 2);
        assert!(is_loading, "still loading mid-stream");

        // 2nd (final) batch: appends and finalizes.
        MessageHandler::handle(
            chunk_message(vec![make_chunk_row(2, 0.3, 3.0)], false, true),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 3);
        assert!(!is_loading, "loading cleared on final batch");

        // The column data has been merged
        let xs = app_state
            .current_study
            .as_ref()
            .unwrap()
            .view
            .numeric_column("x")
            .unwrap()
            .to_vec();
        assert_eq!(xs, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn live_update_done_appends_trial_rows() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 3);

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![
                    make_core_trial_row(3, 1, vec![1.0]),
                    make_core_trial_row(4, 1, vec![2.0]),
                ],
                updated_study_counts: vec![(1, 5)],
                extras_events: Default::default(),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 5);
    }

    /// Regression: even when the live diff includes a row with a different
    /// objective count (empty objectives), the multi-objective Pareto
    /// computation must not panic on an out-of-range slice.
    /// (Reproduces the case where a Trial that straddles the next
    /// create/complete boundary produces an empty-objectives row.)
    #[test]
    fn live_update_done_handles_ragged_objectives_without_panic() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        // Build a 2-objective study.
        let core_rows: Vec<CoreTrialRow> = (0..3)
            .map(|i| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64, (i as f64) * 2.0],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["o1".to_string(), "o2".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::store_dataframes(vec![df]);
        MessageHandler::handle(
            AppMessage::StudySelected {
                meta: StudyMeta {
                    study_id: 0,
                    name: "s".to_string(),
                    directions: vec![Direction::Minimize, Direction::Minimize],
                    completed_trials: 3,
                    param_names: vec!["x".to_string()],
                    objective_names: vec!["o1".to_string(), "o2".to_string()],
                    param_bounds: Default::default(),
                },
                study_id: 0,
                pareto_rank: vec![0; 3],
                pareto_indices: vec![],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        // Send a mix of 1 complete row + 1 garbage row with empty objectives (the old implementation panicked here).
        let mut empty_obj_row = make_core_trial_row(4, 0, vec![]);
        empty_obj_row.objectives = vec![];
        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![make_core_trial_row(3, 0, vec![1.0, 2.0]), empty_obj_row],
                updated_study_counts: vec![],
                extras_events: Default::default(),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        // Verifies it doesn't panic and results in 5 rows.
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 5);
    }

    #[test]
    fn live_update_done_updates_all_studies_counts() {
        let mut app_state = AppState::new();
        app_state.all_studies = vec![crate::state::app_state::StudyMeta {
            study_id: 1,
            name: "s".to_string(),
            directions: vec![],
            completed_trials: 100,
            param_names: vec![],
            objective_names: vec![],
            param_bounds: Default::default(),
        }];
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![],
                updated_study_counts: vec![(1, 105)],
                extras_events: Default::default(),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.all_studies[0].completed_trials, 105);
    }

    #[test]
    fn live_update_done_preserves_filter_ranges() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        app_state.filter_ranges.insert("x".to_string(), (0.0, 1.0));
        app_state.selected_indices = vec![0, 1];

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![make_core_trial_row(3, 1, vec![1.0])],
                updated_study_counts: vec![],
                extras_events: Default::default(),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.filter_ranges.contains_key("x"));
        assert_eq!(app_state.selected_indices, vec![0, 1]);
    }

    #[test]
    fn live_update_error_sets_poller_inactive() {
        let mut app_state = AppState::new();
        app_state.live_update.poller_active = true;
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateError("test error".to_string()),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(!app_state.live_update.poller_active);
        assert!(load_error.is_some());
    }

    #[test]
    fn live_update_maybe_complete_sets_hint() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateMaybeComplete,
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.live_update.showing_completion_hint);
    }

    // ── SQLite live update: SqliteLiveChanged / SqliteLiveReloadDone ──────

    #[test]
    fn sqlite_live_changed_reports_reload_study_id() {
        // SqliteLiveChanged is just a signal message that carries the study_id needing a reload.
        // The actual reload dispatch is done by app.rs (which holds tx) using this function's return value.
        let msg = AppMessage::SqliteLiveChanged { study_id: 7 };
        assert_eq!(MessageHandler::sqlite_reload_study_id(&msg), Some(7));
    }

    #[test]
    fn sqlite_reload_study_id_is_none_for_other_messages() {
        let msg = AppMessage::LiveUpdateMaybeComplete;
        assert_eq!(MessageHandler::sqlite_reload_study_id(&msg), None);
    }

    #[test]
    fn sqlite_live_changed_handle_does_not_mutate_state() {
        // handle() itself does not mutate state (dispatch is app.rs's responsibility).
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::SqliteLiveChanged { study_id: 0 },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.current_study.is_none());
        assert!(load_error.is_none());
    }

    #[test]
    fn sqlite_live_reload_done_rebuilds_view_and_clears_caches() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        // Initial selection: study_id=0 with 3 trials.
        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        app_state.all_studies = vec![StudyMeta {
            study_id: 0,
            name: "s".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 3,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        }];
        // Simulate the cache having something in it (should be discarded by reload).
        app_state.mcdm_result = Some(crate::state::app_state::McdmResult::Topsis(
            crate::state::app_state::TopsisResult {
                scores: vec![0.5],
                ranked_indices: vec![0],
                duration_ms: 1.0,
            },
        ));

        // As the worker thread would do, first reflect the reload result (8
        // trials) into the shared store, then send SqliteLiveReloadDone.
        let reloaded_rows: Vec<CoreTrialRow> = (0..8)
            .map(|i| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let reloaded_df = DataFrame::from_trials(
            &reloaded_rows,
            &["x".to_string()],
            &["y".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::swap_snapshot(0, std::sync::Arc::new(reloaded_df));

        MessageHandler::handle(
            AppMessage::SqliteLiveReloadDone {
                study_id: 0,
                meta: StudyMeta {
                    study_id: 0,
                    name: "s".to_string(),
                    directions: vec![Direction::Minimize],
                    completed_trials: 8,
                    param_names: vec!["x".to_string()],
                    objective_names: vec!["y".to_string()],
                    param_bounds: Default::default(),
                },
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let study = app_state.current_study.as_ref().unwrap();
        assert_eq!(study.trial_count(), 8, "view must reflect the reloaded df");
        assert!(
            !study.pareto_indices.is_empty(),
            "pareto ranks must be recomputed"
        );
        assert_eq!(study.meta.completed_trials, 8);
        assert!(
            app_state.mcdm_result.is_none(),
            "row-count-dependent caches must be cleared"
        );
        assert_eq!(app_state.all_studies[0].completed_trials, 8);
    }

    #[test]
    fn study_selected_resets_cluster_widget_runtime_state() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        widgets.cluster_scatter.computing = true;
        widgets.cluster_scatter.pending_compute =
            Some(crate::ui::widgets::cluster_scatter::ClusterComputeRequest {
                k: 3,
                target_space: crate::ui::widgets::cluster_scatter::ClusterSpace::Objective,
                k_mode: crate::ui::widgets::cluster_scatter::KSelectionMode::Manual,
                init_strategy:
                    crate::ui::widgets::cluster_scatter::KMeansInitStrategy::KMeansPlusPlus,
                elbow_max_k: 10,
            });

        MessageHandler::handle(
            make_study_message(4),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(!widgets.cluster_scatter.computing);
        assert!(widgets.cluster_scatter.pending_compute.is_none());
        assert!(widgets.cluster_scatter.last_error.is_none());
    }

    // ── TASK-2230: comparison load message tests ─────────────────

    #[test]
    fn comparison_load_message_updates_state_entrypoint() {
        use crate::state::app_state::StudyContext;
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        let context = StudyContext::from_rows_for_test(
            StudyMeta {
                study_id: 99,
                name: "compare".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 0,
                param_names: vec![],
                objective_names: vec![],
                param_bounds: Default::default(),
            },
            vec![],
        );

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoaded {
                context: Box::new(context),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.comparison_studies.len(), 1);
        assert_eq!(app_state.comparison_studies[0].meta.study_id, 99);
        // Verifies the parallel Vecs stay the same length
        assert_eq!(app_state.comparison_colors.len(), 1);
        assert_eq!(app_state.comparison_convergence_histories.len(), 1);
    }

    #[test]
    fn comparison_load_failed_message_sets_load_error() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoadFailed("file not found".to_string()),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(load_error.as_deref(), Some("file not found"));
    }

    // ── R4: report export done/failed messages ────────────────────

    #[test]
    fn report_export_done_stores_paths_and_clears_generating() {
        use crate::ui::widgets::report_modal::ReportDialogState;

        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;
        app_state.report_dialog = Some(ReportDialogState {
            generating: true,
            ..Default::default()
        });

        let paths = vec![
            std::path::PathBuf::from("/tmp/report_s.html"),
            std::path::PathBuf::from("/tmp/report_s.json"),
        ];
        MessageHandler::handle(
            AppMessage::ReportExportDone {
                paths: paths.clone(),
                overwrote: vec![std::path::PathBuf::from("/tmp/report_s.json")],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let dialog = app_state.report_dialog.as_ref().expect("dialog remains");
        assert!(!dialog.generating);
        assert!(dialog.error.is_none());
        assert_eq!(dialog.success_paths.as_deref(), Some(paths.as_slice()));
        assert_eq!(
            dialog.overwrote_paths,
            vec![std::path::PathBuf::from("/tmp/report_s.json")]
        );
        assert!(load_error.is_none());
    }

    #[test]
    fn report_export_done_without_dialog_is_noop() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;

        MessageHandler::handle(
            AppMessage::ReportExportDone {
                paths: vec![],
                overwrote: vec![],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.report_dialog.is_none());
        assert!(load_error.is_none());
    }

    #[test]
    fn error_during_report_generation_surfaces_in_dialog() {
        use crate::ui::widgets::report_modal::ReportDialogState;

        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;
        app_state.report_dialog = Some(ReportDialogState {
            generating: true,
            ..Default::default()
        });

        MessageHandler::handle(
            AppMessage::Error("disk full".to_string()),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let dialog = app_state.report_dialog.as_ref().expect("dialog remains");
        assert!(!dialog.generating);
        assert_eq!(dialog.error.as_deref(), Some("disk full"));
        assert_eq!(load_error.as_deref(), Some("disk full"));
    }

    // ── .ghx D&D -> optimization run: GhOptFinished ────────────────

    fn make_gh_opt_run_state() -> crate::state::app_state::GhOptRunState {
        crate::state::app_state::GhOptRunState {
            progress: tunny_core::surrogate_opt::FitProgress::new(),
            journal_path: std::path::PathBuf::from("/tmp/model_optuna.log"),
            study_name: "model-000001".to_string(),
            finished: None,
        }
    }

    #[test]
    fn gh_opt_finished_ok_formats_success_message() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;
        app_state.gh_opt_run = Some(make_gh_opt_run_state());

        MessageHandler::handle(
            AppMessage::GhOptFinished {
                result: Ok(tunny_core::gh::GhRunSummary {
                    study_id: 0,
                    completed: 48,
                    failed: 2,
                    cancelled: false,
                }),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let run = app_state.gh_opt_run.as_ref().expect("run state remains");
        assert_eq!(
            run.finished.as_ref(),
            Some(&Ok("Done: 48 trials succeeded / 2 failed".to_string()))
        );
    }

    #[test]
    fn gh_opt_finished_ok_cancelled_appends_hint() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;
        app_state.gh_opt_run = Some(make_gh_opt_run_state());

        MessageHandler::handle(
            AppMessage::GhOptFinished {
                result: Ok(tunny_core::gh::GhRunSummary {
                    study_id: 0,
                    completed: 10,
                    failed: 0,
                    cancelled: true,
                }),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let run = app_state.gh_opt_run.as_ref().expect("run state remains");
        assert_eq!(
            run.finished.as_ref(),
            Some(&Ok(
                "Done: 10 trials succeeded / 0 failed (cancelled)".to_string()
            ))
        );
    }

    #[test]
    fn gh_opt_finished_err_sets_error_string() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;
        app_state.gh_opt_run = Some(make_gh_opt_run_state());

        MessageHandler::handle(
            AppMessage::GhOptFinished {
                result: Err("journal write failed".to_string()),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let run = app_state.gh_opt_run.as_ref().expect("run state remains");
        assert_eq!(
            run.finished.as_ref(),
            Some(&Err("journal write failed".to_string()))
        );
    }

    #[test]
    fn gh_opt_finished_without_run_state_is_noop() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;

        MessageHandler::handle(
            AppMessage::GhOptFinished {
                result: Err("no run".to_string()),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.gh_opt_run.is_none());
        assert!(load_error.is_none());
    }
}
