use super::*;

impl MessageHandler {
    /// Publish the analysis-owned table/front and explicit all-state display input.
    pub(super) fn publish_journal(
        key: tdashboard_study::StudyKey,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
    ) {
        let Some(journal) = app_state
            .journal_studies
            .as_mut()
            .filter(|j| j.source == key.source)
        else {
            return;
        };
        let comparison_ready = journal.has_comparison(key)
            && journal
                .managers
                .get(&key.stored_id)
                .and_then(|m| m.load_state())
                .is_some_and(|s| s.phase == tdashboard_study::LoadPhase::Complete);
        let selected = journal.selected == Some(key.stored_id);
        let Some(input) = journal
            .managers
            .get(&key.stored_id)
            .and_then(|m| m.acquire())
        else {
            return;
        };
        let projection = journal.analysis.borrow_mut().project(input.clone());
        let complete = input.load_state().phase == tdashboard_study::LoadPhase::Complete;
        let meta = crate::io::journal::convert_projection_meta(projection.meta.clone());
        let front = if complete {
            let front = journal
                .analysis
                .borrow_mut()
                .pareto(input.clone())
                .expect("Complete input is Pareto-ready");
            Some(front)
        } else {
            None
        };
        let history = journal.analysis.borrow_mut().history(input.clone()).ok();
        let view = StudyView::from_projection(projection, input.clone(), front);
        #[cfg(test)]
        let pareto_indices = view
            .front
            .as_ref()
            .map(|f| f.pareto_indices.clone())
            .unwrap_or_default();
        let context = StudyContext {
            meta: meta.clone(),
            view,
            #[cfg(test)]
            pareto_indices,
        };
        let comparison_context = comparison_ready.then(|| context.clone());
        if let Some(existing) = app_state
            .all_studies
            .iter_mut()
            .find(|m| m.study_id == key.stored_id)
        {
            *existing = meta;
        }
        if selected {
            let stamp = input.stamp();
            let changed = app_state.journal_display != Some(stamp);
            let first = app_state.journal_display.is_none_or(|old| {
                old.input.study != stamp.input.study
                    || old.input.generation != stamp.input.generation
            });
            if first {
                app_state.clear();
                widget_states.cluster_scatter = Default::default();
                widget_states.cluster_scatter_3d.clear_runtime_state();
                widget_states.trial_table.cluster.clear_runtime_state();
                widget_states.reset_infeasible_flags();
            }
            if changed {
                app_state.clear_analysis_results();
                widget_states.invalidate_analysis_input();
            }
            app_state.journal_display = Some(stamp);
            app_state.current_study = Some(context);
            app_state.study_streaming = !complete;
            app_state.best_trial_history = history;
            app_state.convergence_history = None;
            app_state.comparison_convergence_histories.clear();
            widget_states.convergence.computing = false;
        }
        // Primary invalidation above must not clear histories just appended by a
        // shared acquisition's comparison handoff.
        if let Some(context) = comparison_context {
            Self::complete_journal_comparison(key, Some(context), app_state);
        }
        if selected {
            if let Some(path) = app_state
                .journal_path
                .clone()
                .filter(|p| crate::io::flat_csv::is_csv_path(p))
            {
                let images: Vec<_> = input
                    .data()
                    .trials()
                    .iter()
                    .flat_map(|t| {
                        t.artifacts
                            .iter()
                            .map(|a| (t.key.stored_id, a.reference.clone()))
                    })
                    .collect();
                let root = app_state.explicit_artifact_root.clone().unwrap_or_else(|| {
                    path.parent()
                        .unwrap_or(std::path::Path::new("."))
                        .to_path_buf()
                });
                app_state.artifact_map =
                    tdashboard_io::artifacts::resolve_csv_images(&root, &images);
                app_state.artifacts_dir = Some(root);
                app_state.csv_images = Some(images);
                app_state.artifact_revision = app_state.artifact_revision.wrapping_add(1);
            }
        }
    }
    pub(super) fn complete_journal_comparison(
        key: tdashboard_study::StudyKey,
        context: Option<StudyContext>,
        app_state: &mut AppState,
    ) {
        let Some(journal) = app_state
            .journal_studies
            .as_mut()
            .filter(|j| j.source == key.source)
        else {
            return;
        };
        for context in journal.complete_comparison(key, context) {
            app_state.comparison_studies.push(context);
            let idx = app_state.comparison_colors.len();
            app_state
                .comparison_colors
                .push(crate::theme::color_compute::comparison_color_at(idx));
            app_state
                .comparison_convergence_histories
                .push(ConvergenceHistory {
                    trial_ids: vec![],
                    values: vec![],
                    sample_step: 1,
                    ref_point: vec![],
                });
            app_state.convergence_history = None;
        }
    }
    /// Builds a single-objective Study's best-so-far history (trial_number,
    /// cumulative best) and stores it in `app_state.best_trial_history`. Left
    /// as None for multi-objective Studies (the convergence card is hidden,
    /// and the HV history instead handles the multi-objective trend).
    #[cfg(test)]
    pub(super) fn refresh_best_trial_history(app_state: &mut AppState) {
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
        app_state.best_trial_history =
            Some(tdashboard_analysis::convergence::build_best_trial_history(
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
    #[cfg(test)]
    pub(super) fn handle_study_chunk(
        study_id: u32,
        meta: crate::state::app_state::StudyMeta,
        new_rows: Vec<CoreTrialRow>,
        param_names: Vec<String>,
        objective_names: Vec<String>,
        user_attr_numeric_names: Vec<String>,
        user_attr_string_names: Vec<String>,
        max_constraints: usize,
        has_constraints: bool,
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
        app_state.study_streaming = !is_final;
        // Every chunk can change historical feasibility, including metadata-only
        // schema growth. Compute convergence only against the final snapshot.
        app_state.convergence_history = None;
        app_state.comparison_convergence_histories.clear();
        widget_states.convergence.computing = false;
        new_df.append_trials(
            &new_rows,
            &param_names,
            &objective_names,
            &user_attr_numeric_names,
            &user_attr_string_names,
            max_constraints,
        );
        if has_constraints && !new_df.feasibility().has_constraints() {
            new_df.mark_constrained();
        }
        let arc = std::sync::Arc::new(new_df);

        // Fixture Pareto uses only the supplied table and final-batch state.
        let (ranks, pareto_indices) = if is_final {
            let is_minimize: Vec<bool> = meta
                .directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();
            let pareto = tdashboard_analysis::pareto::compute_pareto_ranks(&arc, &is_minimize);
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
}
