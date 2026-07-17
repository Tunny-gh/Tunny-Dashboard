//! The surrogate optimization widget.
//!
//! Trains a response surface (surrogate model) from the sampled results (the trial
//! set), then runs optimization on that surface and shows the estimated optimum.
//! The computation is done in the background by `tunny_core::surrogate_opt` (see
//! poll_chart.rs).
//!
//! 2-stage flow:
//!   1. Fit & Validate: shows validation metrics via holdout + 5-fold CV.
//!   2. Run Optimization: runs optimization on the trained model.
//!
//! Layout:
//!   Full-width preamble: early return for no numeric parameters / insufficient
//!       trial count.
//!   Left column (Fit & Validate): Objective + Model combo -> Fit & Validate button
//!       -> fitting spinner -> validation metrics grid + quality assessment + OOF
//!       scatter plot.
//!   Right column (Optimization): Optimizer / Surface X / Y combo -> Run
//!       Optimization button -> optimizing spinner -> result.
//!
//! Module layout (split by responsibility; the externally visible public API is kept
//! in this mod):
//!   - `labels`: pure helpers for labels, choices, and quality assessment.
//!   - `fit`: the left column (fit, validate, OOF plot).
//!   - `optimize`: the right column (optimize column, result summary, history plot).
//!   - `front_view`: 2D/3D scatter plot of the predicted Pareto front.
//!   - `tables`: tabular rendering of the estimated optimum and front points.
//!   - `suggest`: result table for candidate suggestions (EI/LCB, EHVI).

mod fit;
mod front_view;
mod labels;
mod optimize;
mod suggest;
mod tables;

use crate::ui::widget_states::SurrogateOptState;
use tunny_core::surrogate_opt::{TrainedSurrogate, MIN_TRIALS_FOR_SURROGATE_OPT};

use fit::{render_fit_column, render_fit_column_multi};
use optimize::{render_optimize_column, render_optimize_column_multi};

// Re-export to keep the same `surrogate_opt::model_label` path as before the split.
// Referenced by other widgets (robustness / compare / response_surface) and CSV export.
pub(crate) use labels::model_label;

/// Observed (existing trial) data overlaid on the multi-objective front scatter plot.
/// All arrays are aligned in trial row order, and `objective_cols` is ordered by
/// `multi_result`'s objective order.
/// Used to classify observed points into Pareto front / dominated / infeasible, the
/// same way as ParetoScatter.
pub struct ObservedData<'a> {
    /// All trial observed values per objective (in `multi_result.objective_names` order).
    pub objective_cols: &'a [Vec<f64>],
    /// Each trial's Pareto rank (0 = observed front).
    pub pareto_rank: &'a [u32],
    /// Whether each trial is feasible.
    pub feasible: &'a [bool],
}

/// `param_names` contains numeric parameters only (categorical columns are not
/// optimization targets).
/// `obj_history` is all values (in trial order) of the objective column referenced
/// by the current result. For plotting.
/// `observed` is the observed points overlaid on the multi-objective front scatter
/// plot (None when there's no result).
/// `constraint_col_names` is the constraint column names (non-empty only for
/// constrained Studies).
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    obj_names: &[String],
    trial_count: usize,
    obj_history: Option<&[f64]>,
    observed: Option<&ObservedData>,
    constraint_col_names: &[String],
) {
    // ── Full-width preamble: no numeric parameters ──────────────────
    if param_names.is_empty() {
        ui.label("No numeric parameters available for surrogate optimization.");
        return;
    }

    // ── Full-width preamble: insufficient trial count ───────────────
    if trial_count < MIN_TRIALS_FOR_SURROGATE_OPT {
        ui.colored_label(
            egui::Color32::RED,
            format!(
                "At least {} trials required (current: {})",
                MIN_TRIALS_FOR_SURROGATE_OPT, trial_count
            ),
        );
        return;
    }

    // ── Multi-objective mode toggle checkbox (shown only when there are 2+ objectives) ──
    if obj_names.len() >= 2 {
        let prev_multi = state.multi_objective;
        ui.checkbox(
            &mut state.multi_objective,
            "Multi-objective (all objectives)",
        );
        if state.multi_objective != prev_multi {
            // Clear the error when switching modes.
            state.error_message = None;
        }
    }

    let busy = state.fitting
        || state.optimizing
        || state.suggesting
        || state.multi_suggesting
        || state.pending_fit.is_some()
        || state.pending_optimize.is_some()
        || state.pending_multi_fit.is_some()
        || state.pending_multi_optimize.is_some()
        || state.pending_suggest.is_some()
        || state.pending_multi_suggest.is_some();

    let has_matching_trained = state
        .trained
        .as_deref()
        .map(|t| trained_matches(t, state, obj_names))
        .unwrap_or(false);

    let has_matching_multi_trained = state
        .multi_trained
        .as_deref()
        .map(|v| multi_trained_matches(v, state, obj_names))
        .unwrap_or(false);

    // ── 2-column layout ──────────────────────────────────────────────
    // Same idiom as trial_detail_modal: horizontal_top + allocate_ui_with_layout to
    // split into equal-width columns.
    let available_w = ui.available_width();
    let col_w = (available_w / 2.0).max(200.0);

    // Error display (shown full-width for either fit or optimize failure).
    if let Some(ref err) = state.error_message.clone() {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }

    ui.horizontal_top(|ui| {
        // ── Left column: Fit & Validate ───────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if state.multi_objective {
                    render_fit_column_multi(ui, state, obj_names, busy);
                } else {
                    render_fit_column(ui, state, obj_names, busy, constraint_col_names);
                }
            },
        );

        ui.separator();

        // ── Right column: Optimization ────────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if state.multi_objective {
                    render_optimize_column_multi(
                        ui,
                        state,
                        busy,
                        has_matching_multi_trained,
                        observed,
                    );
                } else {
                    render_optimize_column(ui, state, busy, has_matching_trained, obj_history);
                }
            },
        );
    });
}

/// Determines whether the trained model matches the current UI selection (objective, model kind).
pub(super) fn trained_matches(
    trained: &TrainedSurrogate,
    state: &SurrogateOptState,
    obj_names: &[String],
) -> bool {
    let selected_obj = obj_names
        .get(state.selected_objective)
        .map(|s| s.as_str())
        .unwrap_or("");
    if trained.objective_name != selected_obj {
        return false;
    }
    // In Auto mode, core picks the concrete model kind, so the match is determined
    // by "was it trained with Auto (model_selection is Some)" rather than model_kind.
    if state.auto_select {
        trained.model_selection.is_some()
    } else {
        trained.model_selection.is_none() && trained.model_kind == state.model
    }
}

/// Determines whether the multi-objective trained model set matches the current UI
/// selection (model, objective set).
pub(crate) fn multi_trained_matches(
    trained: &[TrainedSurrogate],
    state: &SurrogateOptState,
    obj_names: &[String],
) -> bool {
    if trained.len() != obj_names.len() {
        return false;
    }
    let trained_obj_names: Vec<&str> = trained.iter().map(|t| t.objective_name.as_str()).collect();
    let expected_obj_names: Vec<&str> = obj_names.iter().map(|s| s.as_str()).collect();
    if trained_obj_names != expected_obj_names {
        return false;
    }
    trained.iter().all(|t| t.model_kind == state.model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tunny_core::surrogate_opt::SurrogateModelKind;

    #[test]
    fn optimize_click_requires_matching_trained() {
        let state = SurrogateOptState::default();
        let obj_names = ["obj0".to_string()];
        // has_matching_trained is false since trained is None.
        let has_matching = state
            .trained
            .as_deref()
            .map(|t| trained_matches(t, &state, &obj_names))
            .unwrap_or(false);
        assert!(!has_matching);
    }

    // ── unit tests for multi_trained_matches ──────────────────────────

    fn make_dummy_trained(
        obj_name: &str,
        model: SurrogateModelKind,
    ) -> tunny_core::surrogate_opt::TrainedSurrogate {
        // Fit a TrainedSurrogate with only the minimum required fields filled in.
        let xs: Vec<Vec<f64>> = (0..12)
            .map(|i| vec![i as f64 / 12.0, (i as f64 / 12.0) * 0.5])
            .collect();
        let ys: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix: xs,
            y: ys,
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_name: obj_name.to_string(),
            model,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
        };
        tunny_core::surrogate_opt::fit_surrogate_with_validation(&req)
            .expect("dummy fit should succeed")
    }

    #[test]
    fn multi_trained_matches_correct() {
        let obj_names = vec!["f0".to_string(), "f1".to_string()];
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_model() {
        let obj_names = vec!["f0".to_string(), "f1".to_string()];
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        // Case where the model changed to Ridge
        let state = SurrogateOptState {
            model: SurrogateModelKind::Ridge,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_objectives() {
        let obj_names = vec!["f0".to_string(), "f2".to_string()]; // f2 is different
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_length() {
        let obj_names = vec!["f0".to_string()]; // objective count is 1
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn adopt_compute_state_propagates_multi_trained() {
        use std::sync::Arc;

        let trained_vec = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let arc = Arc::new(trained_vec);

        let global = SurrogateOptState {
            fitting: false,
            optimizing: false,
            multi_trained: Some(arc.clone()),
            ..Default::default()
        };

        let mut item = SurrogateOptState {
            fitting: true,
            ..Default::default()
        };
        item.adopt_compute_state(&global);

        assert!(!item.fitting);
        assert!(item.multi_trained.is_some());
        assert_eq!(item.multi_trained.as_ref().unwrap().len(), 2);
    }
}
