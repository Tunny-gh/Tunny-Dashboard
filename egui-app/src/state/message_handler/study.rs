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
}
