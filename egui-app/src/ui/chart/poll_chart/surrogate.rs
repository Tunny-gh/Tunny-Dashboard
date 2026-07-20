use std::sync::mpsc;

use crate::state::app_state::{AppState, Direction};
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;

use super::compute::{
    build_numeric_fit_xy, build_numeric_fit_xy_multi, collect_constraints, minimize_flags,
    numeric_param_names,
};

/// Processes each stage of surrogate optimization (SurrogateOpt) in priority order.
/// In the order fit -> multi-objective fit -> optimize -> multi-objective optimize -> suggest candidates ->
/// multi-objective suggest candidates, only the first stage with a pending request is executed
/// (same behavior as the original else-if chain).
pub(super) fn poll_surrogate_opt(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if surrogate_stage_fit(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_multi_fit(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_optimize(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_multi_optimize(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_suggest(widgets, tx) {
        return;
    }
    surrogate_stage_multi_suggest(app_state, widgets, tx);
}

/// SurrogateOpt: single-objective fit stage. Returns true if a pending request was consumed.
fn surrogate_stage_fit(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(fit_req) = widgets.surrogate_opt.pending_fit.take() else {
        return false;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let Some((numeric_params, x_matrix, y, param_bounds, kept_rows)) =
        build_numeric_fit_xy(ctx, &fit_req.objective)
    else {
        widgets.surrogate_opt.error_message = Some("No numeric parameters available".to_string());
        return true;
    };

    // Clear the previous training and optimization results before starting the fit.
    widgets.surrogate_opt.fitting = true;
    widgets.surrogate_opt.trained = None;
    widgets.surrogate_opt.result = None;
    widgets.surrogate_opt.error_message = None;

    // Extract constraint columns (when use_constraints is set and constraint columns exist). Filtered
    // by kept_rows to stay aligned with the rows excluded by the non-finite filter.
    let constraints = if fit_req.use_constraints {
        collect_constraints(ctx, &kept_rows)
    } else {
        vec![]
    };

    // Shared progress/cancellation handle (shared between the UI and the training thread).
    let progress = tunny_core::surrogate_opt::FitProgress::new();
    widgets.surrogate_opt.fit_progress = Some(progress.clone());

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix,
            y,
            param_names: numeric_params,
            objective_name: fit_req.objective,
            model: fit_req.model,
            auto_select: fit_req.auto_select,
            constraints,
            priority_rows: vec![],
            param_bounds: Some(param_bounds),
        };
        match tunny_core::surrogate_opt::fit_surrogate_with_validation_tracked(
            &fit_core_req,
            &progress,
        ) {
            Ok(t) => AppMessage::SurrogateFitDone(std::sync::Arc::new(t)),
            Err(e) => {
                // Don't show an error for failures caused by cancellation.
                if progress.is_cancelled() {
                    AppMessage::SurrogateFitCancelled
                } else {
                    AppMessage::SurrogateFitFailed(e)
                }
            }
        }
    });
    true
}

/// SurrogateOpt: multi-objective fit stage (trains all objectives). Returns true if a pending request was consumed.
fn surrogate_stage_multi_fit(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(multi_fit_req) = widgets.surrogate_opt.pending_multi_fit.take() else {
        return false;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;
    let objective_names: Vec<String> = obj_names.to_vec();
    let Some((numeric_params, x_matrix, objective_values, param_bounds, _kept_rows)) =
        build_numeric_fit_xy_multi(ctx, obj_names)
    else {
        widgets.surrogate_opt.error_message = Some("No numeric parameters available".to_string());
        return true;
    };
    // Resolve the per-objective minimize flag from directions (same approach as the multi-objective optimization path).
    let minimize_flags = minimize_flags(directions, obj_names.len());

    // Clear the previous multi-objective results before starting the fit.
    widgets.surrogate_opt.fitting = true;
    widgets.surrogate_opt.multi_trained = None;
    widgets.surrogate_opt.multi_result = None;
    widgets.surrogate_opt.error_message = None;

    // Shared progress/cancellation handle (shared between the UI and the training thread).
    let progress = tunny_core::surrogate_opt::FitProgress::new();
    widgets.surrogate_opt.fit_progress = Some(progress.clone());

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        // Train all objectives with Pareto-front concentration
        // (concentrates inducing points on non-dominated trials when N exceeds the GP inducing-point cap).
        match tunny_core::surrogate_opt::fit_multi_surrogates_tracked(
            &x_matrix,
            &objective_values,
            &numeric_params,
            &objective_names,
            multi_fit_req.model,
            &minimize_flags,
            Some(&param_bounds),
            &progress,
        ) {
            Ok(trained_vec) => AppMessage::SurrogateMultiFitDone(std::sync::Arc::new(trained_vec)),
            Err(e) => {
                if progress.is_cancelled() {
                    AppMessage::SurrogateMultiFitCancelled
                } else {
                    AppMessage::SurrogateMultiFitFailed(e)
                }
            }
        }
    });
    true
}

/// SurrogateOpt: single-objective optimization stage. Returns true if a pending request was consumed.
fn surrogate_stage_optimize(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(opt_req) = widgets.surrogate_opt.pending_optimize.take() else {
        return false;
    };
    // The optimization stage requires a trained model.
    let Some(trained) = widgets.surrogate_opt.trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained model available. Run Fit & Validate first.".to_string());
        return true;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    let obj_name = trained.objective_name.clone();
    let obj_idx = obj_names.iter().position(|o| *o == obj_name);
    let minimize = obj_idx
        .and_then(|i| directions.get(i))
        .map(|d| matches!(d, Direction::Minimize))
        .unwrap_or(true);

    // Response-surface slices have been removed, so none are generated.
    let slice_params: Option<(usize, usize)> = None;

    widgets.surrogate_opt.optimizing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateOptUiResult;
        let param_names_owned = trained.param_names.clone();
        let spec = tunny_core::surrogate_opt::SurrogateOptimizeSpec {
            minimize,
            optimizer: opt_req.optimizer,
            slice_params,
            n_grid: tunny_core::surrogate_opt::DEFAULT_SLICE_GRID,
        };
        let constraint_names = trained.constraint_names.clone();
        let r = tunny_core::surrogate_opt::optimize_on_trained(&trained, &spec);
        let predicted_constraints: Vec<(String, f64)> = constraint_names
            .into_iter()
            .zip(r.predicted_constraints)
            .collect();
        AppMessage::SurrogateOptDone(SurrogateOptUiResult {
            best_params: param_names_owned.into_iter().zip(r.best_params).collect(),
            best_value: r.best_value,
            predicted_std: r.predicted_std,
            r_squared: r.r_squared,
            objective_name: obj_name,
            minimize,
            best_observed_value: r.best_observed_value,
            predicted_constraints,
            feasibility_probability: r.feasibility_probability,
        })
    });
    true
}

/// SurrogateOpt: multi-objective optimization stage. Returns true if a pending request was consumed.
fn surrogate_stage_multi_optimize(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    if widgets
        .surrogate_opt
        .pending_multi_optimize
        .take()
        .is_none()
    {
        return false;
    }
    // Multi-objective optimization stage: requires a set of trained surrogates.
    let Some(multi_trained) = widgets.surrogate_opt.multi_trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained multi-objective model. Run Fit & Validate first.".to_string());
        return true;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    // Resolve the per-objective minimize flag from directions.
    let minimize_flags = minimize_flags(directions, obj_names.len());

    // Response-surface slices have been removed, so none are generated.
    let slice_params: Option<(usize, usize)> = None;

    let objective_names_owned = obj_names.to_vec();
    widgets.surrogate_opt.optimizing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateMultiOptUiResult;
        let refs: Vec<&tunny_core::surrogate_opt::TrainedSurrogate> =
            multi_trained.iter().collect();
        let spec = tunny_core::surrogate_opt::SurrogateMultiOptimizeSpec {
            minimize: minimize_flags,
            slice_params,
            n_grid: tunny_core::surrogate_opt::DEFAULT_SLICE_GRID,
        };
        match tunny_core::surrogate_opt::optimize_multi_on_trained(&refs, &spec) {
            Ok(r) => {
                let param_names = refs
                    .first()
                    .map(|t| t.param_names.clone())
                    .unwrap_or_default();
                AppMessage::SurrogateMultiOptDone(SurrogateMultiOptUiResult {
                    param_names,
                    objective_names: objective_names_owned,
                    front: r.front,
                    r_squared: r.r_squared,
                })
            }
            Err(e) => AppMessage::SurrogateMultiOptFailed(e),
        }
    });
    true
}

/// SurrogateOpt: single-objective candidate-suggestion stage. Returns true if a pending request was consumed.
fn surrogate_stage_suggest(widgets: &mut WidgetStates, tx: &mpsc::SyncSender<AppMessage>) -> bool {
    let Some(suggest_req) = widgets.surrogate_opt.pending_suggest.take() else {
        return false;
    };
    // Candidate-suggestion stage: requires a trained GP surrogate.
    let Some(trained) = widgets.surrogate_opt.trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained model available. Run Fit & Validate first.".to_string());
        return true;
    };

    let param_names = trained.param_names.clone();
    let objective_name = trained.objective_name.clone();
    widgets.surrogate_opt.suggesting = true;
    widgets.surrogate_opt.error_message = None;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateSuggestUiResult;
        match tunny_core::surrogate_opt::suggest_candidates(
            &trained,
            suggest_req.n_candidates,
            suggest_req.acquisition,
            suggest_req.minimize,
        ) {
            Ok(candidates) => AppMessage::SurrogateSuggestDone(SurrogateSuggestUiResult {
                candidates,
                param_names,
                objective_name,
            }),
            Err(e) => AppMessage::SurrogateSuggestFailed(e),
        }
    });
    true
}

/// SurrogateOpt: multi-objective candidate-suggestion stage (EHVI). Returns true if a pending request was consumed.
fn surrogate_stage_multi_suggest(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(multi_suggest_req) = widgets.surrogate_opt.pending_multi_suggest.take() else {
        return false;
    };
    // Multi-objective candidate-suggestion stage (EHVI): requires a set of trained GP surrogates.
    let Some(multi_trained) = widgets.surrogate_opt.multi_trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained multi-objective model. Run Fit & Validate first.".to_string());
        return true;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    // Resolve the per-objective minimize flag from directions.
    let minimize_flags = minimize_flags(directions, obj_names.len());

    let param_names = multi_trained
        .first()
        .map(|t| t.param_names.clone())
        .unwrap_or_default();
    let objective_names = obj_names.to_vec();
    widgets.surrogate_opt.multi_suggesting = true;
    widgets.surrogate_opt.error_message = None;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateMultiSuggestUiResult;
        match tunny_core::surrogate_opt::suggest_candidates_multi(
            &multi_trained,
            &minimize_flags,
            multi_suggest_req.n_candidates,
        ) {
            Ok(candidates) => {
                AppMessage::SurrogateMultiSuggestDone(SurrogateMultiSuggestUiResult {
                    candidates,
                    param_names,
                    objective_names,
                })
            }
            Err(e) => AppMessage::SurrogateMultiSuggestFailed(e),
        }
    });
    true
}

/// Dispatches the fit stage of robustness analysis.
pub(super) fn poll_robustness(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(fit_req) = widgets.robustness.pending_fit.take() else {
        return;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    // Check for the existence of numeric parameters first (preserving the original validation order).
    if numeric_param_names(ctx).is_empty() {
        widgets.robustness.fit_error = Some("No numeric parameters available".to_string());
        widgets.robustness.fitting = false;
        return;
    }
    let Some(objective) = obj_names.get(fit_req.objective_index).cloned() else {
        widgets.robustness.fit_error = Some("Invalid objective selection".to_string());
        widgets.robustness.fitting = false;
        return;
    };
    let Some((numeric_params, x_matrix, y, param_bounds, kept_rows)) =
        build_numeric_fit_xy(ctx, &objective)
    else {
        widgets.robustness.fit_error = Some("No numeric parameters available".to_string());
        widgets.robustness.fitting = false;
        return;
    };

    // Robustness analysis also wants the constraint feasibility rate, so always pass constraints when present.
    let constraints = collect_constraints(ctx, &kept_rows);

    widgets.robustness.trained = None;
    widgets.robustness.fit_error = None;

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix,
            y,
            param_names: numeric_params,
            objective_name: objective,
            model: fit_req.model,
            auto_select: false,
            constraints,
            priority_rows: vec![],
            param_bounds: Some(param_bounds),
        };
        match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
            Ok(t) => AppMessage::RobustnessFitDone(std::sync::Arc::new(t)),
            Err(e) => AppMessage::RobustnessFitFailed(e),
        }
    });
}

/// Dispatches the fit stage of the 3D response surface.
pub(super) fn poll_response_surface(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(fit_req) = widgets.response_surface.pending_fit.take() else {
        return;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    // Check for the existence of numeric parameters first (preserving the original validation order).
    if numeric_param_names(ctx).is_empty() {
        widgets.response_surface.fit_error = Some("No numeric parameters available".to_string());
        widgets.response_surface.fitting = false;
        return;
    }
    let Some(objective) = obj_names.get(fit_req.objective_index).cloned() else {
        widgets.response_surface.fit_error = Some("Invalid objective selection".to_string());
        widgets.response_surface.fitting = false;
        return;
    };
    let Some((numeric_params, x_matrix, y, param_bounds, _kept_rows)) =
        build_numeric_fit_xy(ctx, &objective)
    else {
        widgets.response_surface.fit_error = Some("No numeric parameters available".to_string());
        widgets.response_surface.fitting = false;
        return;
    };

    widgets.response_surface.trained = None;
    widgets.response_surface.fit_error = None;

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix,
            y,
            param_names: numeric_params,
            objective_name: objective,
            model: fit_req.model,
            auto_select: false,
            // Response-surface slice evaluation doesn't handle feasibility, so no constraints are passed.
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: Some(param_bounds),
        };
        match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
            Ok(t) => AppMessage::ResponseSurfaceFitDone(std::sync::Arc::new(t)),
            Err(e) => AppMessage::ResponseSurfaceFitFailed(e),
        }
    });
}

/// Dispatches computation for Compare Surrogates (CV metric comparison across all model kinds + prediction slices).
pub(super) fn poll_surrogate_compare(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(req) = widgets.surrogate_compare.pending.take() else {
        return;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;
    // Check for the existence of numeric parameters first (preserving the original validation order).
    if numeric_param_names(ctx).is_empty() {
        widgets.surrogate_compare.error = Some("No numeric parameters available".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    }
    let Some(objective) = obj_names.get(req.objective_index).cloned() else {
        widgets.surrogate_compare.error = Some("Invalid objective selection".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };
    let Some((numeric_params, x_matrix, y, param_bounds, kept_rows)) =
        build_numeric_fit_xy(ctx, &objective)
    else {
        widgets.surrogate_compare.error = Some("No numeric parameters available".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };
    if req.slice_param >= numeric_params.len() {
        widgets.surrogate_compare.error = Some("Invalid slice parameter selection".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    }

    // Anchor: the observed best row for the selected objective (direction-aware). Since best_trial_row
    // returns the row index in the original df, map it to its position within kept_rows for the
    // non-finite-filtered x_matrix.
    let Some(best_row) =
        crate::ui::widgets::anchor::best_trial_row(&ctx.view, obj_names, directions, &objective)
    else {
        widgets.surrogate_compare.error = Some("Could not resolve an anchor point".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };
    let Some(anchor) = kept_rows
        .iter()
        .position(|&r| r == best_row)
        .and_then(|pos| x_matrix.get(pos))
        .cloned()
    else {
        widgets.surrogate_compare.error = Some("Could not resolve an anchor point".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };

    let slice_param = req.slice_param;
    let n = x_matrix.len();
    let observed: Vec<(f64, f64)> = (0..n)
        .filter_map(|i| {
            let xv = x_matrix.get(i)?.get(slice_param).copied()?;
            let yv = y.get(i).copied()?;
            (xv.is_finite() && yv.is_finite()).then_some((xv, yv))
        })
        .collect();

    let kinds = crate::ui::widgets::compare::model_kinds(req.include_moe);
    let param_name = numeric_params[slice_param].clone();
    let objective_name = objective.clone();

    widgets.surrogate_compare.error = None;

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::{SurrogateCompareRow, SurrogateCompareUiResult};

        let mut rows: Vec<SurrogateCompareRow> = Vec::with_capacity(kinds.len());
        let mut slices = Vec::new();

        for kind in kinds {
            let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
                x_matrix: x_matrix.clone(),
                y: y.clone(),
                param_names: numeric_params.clone(),
                objective_name: objective_name.clone(),
                model: kind,
                auto_select: false,
                // Constraints aren't handled since this is a simple comparison view (same reason as ResponseSurface3D).
                constraints: vec![],
                priority_rows: vec![],
                param_bounds: Some(param_bounds.clone()),
            };
            match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
                Ok(trained) => {
                    let v = &trained.validation;
                    rows.push(SurrogateCompareRow {
                        kind,
                        cv_r2_mean: v.cv_r2_mean,
                        cv_r2_std: v.cv_r2_std,
                        holdout_r2: v.holdout_r2,
                        holdout_rmse: v.holdout_rmse,
                        train_r2: v.train_r2,
                        error: None,
                    });
                    if let Some(slice) =
                        tunny_core::surrogate_opt::line_slice_at(&trained, &anchor, slice_param, 60)
                    {
                        slices.push((kind, slice));
                    }
                }
                Err(e) => {
                    rows.push(SurrogateCompareRow {
                        kind,
                        cv_r2_mean: 0.0,
                        cv_r2_std: 0.0,
                        holdout_r2: 0.0,
                        holdout_rmse: 0.0,
                        train_r2: 0.0,
                        error: Some(e),
                    });
                }
            }
        }

        // Only report Failed if all models failed (partial failures are shown as per-row errors).
        if rows.iter().all(|r| r.error.is_some()) {
            let combined = rows
                .iter()
                .filter_map(|r| r.error.clone())
                .collect::<Vec<_>>()
                .join("; ");
            AppMessage::SurrogateCompareFailed(format!("All models failed to fit: {combined}"))
        } else {
            AppMessage::SurrogateCompareDone(std::sync::Arc::new(SurrogateCompareUiResult {
                rows,
                slices,
                observed,
                param_name,
                objective_name,
                anchor,
            }))
        }
    });
}
