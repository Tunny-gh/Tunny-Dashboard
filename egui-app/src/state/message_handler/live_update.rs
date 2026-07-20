use super::*;

impl MessageHandler {
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
    pub(super) fn handle_sqlite_live_reload_done(
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

    pub(super) fn handle_live_update_done(
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
}
