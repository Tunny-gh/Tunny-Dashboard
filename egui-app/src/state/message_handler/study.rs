use super::*;

impl MessageHandler {
    /// Builds a single-objective Study's best-so-far history (trial_number,
    /// cumulative best) and stores it in `app_state.best_trial_history`. Left
    /// as None for multi-objective Studies (the convergence card is hidden,
    /// and the HV history instead handles the multi-objective trend).
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
    pub(super) fn handle_study_chunk(
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
}
