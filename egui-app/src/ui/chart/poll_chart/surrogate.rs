use std::sync::mpsc;

use crate::state::analysis_job::spawn as spawn_analysis;
use crate::state::analysis_job::spawn_with_models as spawn_model;
use crate::state::analysis_job::DeliverySlot as AnalysisKind;
use crate::state::app_state::AppState;
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;
use tdashboard_analysis::preparation::{FitObjectives, Models, Preparation};

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
    if surrogate_stage_suggest(app_state, widgets, tx) {
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
    // Clear the previous training and optimization results before starting the fit.
    widgets.surrogate_opt.fitting = true;
    widgets.surrogate_opt.trained = None;
    widgets.surrogate_opt.result = None;
    widgets.surrogate_opt.error_message = None;

    // Shared progress/cancellation handle (shared between the UI and the training thread).
    let progress = tunny_core::surrogate_opt::FitProgress::new();
    widgets.surrogate_opt.fit_progress = Some(progress.clone());

    let tx = tx.clone();
    spawn_analysis(
        app_state,
        tx,
        AnalysisKind::Fit,
        (
            "single",
            fit_req.objective.clone(),
            if fit_req.auto_select {
                "Auto".into()
            } else {
                format!("{:?}", fit_req.model)
            },
            fit_req.auto_select,
            fit_req.use_constraints,
        ),
        Preparation::Fit {
            objectives: FitObjectives::Name(fit_req.objective.clone()),
            constraints: fit_req.use_constraints,
        },
        move |prepared| {
            let data = prepared.data().fit();
            let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
                x_matrix: data.x.clone(),
                y: data.ys[0].clone(),
                param_names: data.names.clone(),
                objective_name: data.objectives[0].clone(),
                model: fit_req.model,
                auto_select: fit_req.auto_select,
                constraints: data.constraints.clone(),
                priority_rows: vec![],
                param_bounds: Some(data.bounds.clone()),
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
        },
    );
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
    // Clear the previous multi-objective results before starting the fit.
    widgets.surrogate_opt.fitting = true;
    widgets.surrogate_opt.multi_trained = None;
    widgets.surrogate_opt.multi_result = None;
    widgets.surrogate_opt.error_message = None;

    // Shared progress/cancellation handle (shared between the UI and the training thread).
    let progress = tunny_core::surrogate_opt::FitProgress::new();
    widgets.surrogate_opt.fit_progress = Some(progress.clone());

    let tx = tx.clone();
    spawn_analysis(
        app_state,
        tx,
        AnalysisKind::MultiFit,
        ("multi", format!("{:?}", multi_fit_req.model)),
        Preparation::Fit {
            objectives: FitObjectives::All,
            constraints: false,
        },
        move |prepared| {
            let data = prepared.data().fit();
            // Train all objectives with Pareto-front concentration
            // (concentrates inducing points on non-dominated trials when N exceeds the GP inducing-point cap).
            match tunny_core::surrogate_opt::fit_multi_surrogates_tracked(
                &data.x,
                &data.ys,
                &data.names,
                &data.objectives,
                multi_fit_req.model,
                &data.minimize,
                Some(&data.bounds),
                &progress,
            ) {
                Ok(trained_vec) => {
                    AppMessage::SurrogateMultiFitDone(std::sync::Arc::new(trained_vec))
                }
                Err(e) => {
                    if progress.is_cancelled() {
                        AppMessage::SurrogateMultiFitCancelled
                    } else {
                        AppMessage::SurrogateMultiFitFailed(e)
                    }
                }
            }
        },
    );
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
        crate::state::analysis_job::supersede(app_state, AnalysisKind::Optimize);
        widgets.surrogate_opt.optimizing = false;
        widgets.surrogate_opt.error_message =
            Some("No trained model available. Run Fit & Validate first.".to_string());
        return true;
    };
    let obj_name = trained.objective_name.clone();

    // Response-surface slices have been removed, so none are generated.
    let slice_params: Option<(usize, usize)> = None;

    widgets.surrogate_opt.optimizing = true;
    let tx = tx.clone();
    spawn_model(
        app_state,
        tx,
        AnalysisKind::Optimize,
        (
            obj_name.clone(),
            format!("{:?}", opt_req.optimizer),
            slice_params,
        ),
        Models::Single(trained.clone()),
        move |prepared| {
            let trained = prepared.models().single();
            let minimize = prepared.model_directions()[0];
            use crate::state::messages::SurrogateOptUiResult;
            let param_names_owned = trained.param_names.clone();
            let spec = tunny_core::surrogate_opt::SurrogateOptimizeSpec {
                minimize,
                optimizer: opt_req.optimizer,
                slice_params,
                n_grid: tunny_core::surrogate_opt::DEFAULT_SLICE_GRID,
            };
            let constraint_names = trained.constraint_names.clone();
            let r = tunny_core::surrogate_opt::optimize_on_trained(trained, &spec);
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
        },
    );
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
        crate::state::analysis_job::supersede(app_state, AnalysisKind::MultiOptimize);
        widgets.surrogate_opt.optimizing = false;
        widgets.surrogate_opt.error_message =
            Some("No trained multi-objective model. Run Fit & Validate first.".to_string());
        return true;
    };
    // Response-surface slices have been removed, so none are generated.
    let slice_params: Option<(usize, usize)> = None;

    widgets.surrogate_opt.optimizing = true;
    let tx = tx.clone();
    spawn_model(
        app_state,
        tx,
        AnalysisKind::MultiOptimize,
        ("multi", slice_params),
        Models::Multi(multi_trained.clone()),
        move |prepared| {
            let multi_trained = prepared.models().multi();
            let objective_names_owned = multi_trained
                .iter()
                .map(|m| m.objective_name.clone())
                .collect();
            use crate::state::messages::SurrogateMultiOptUiResult;
            let refs: Vec<&tunny_core::surrogate_opt::TrainedSurrogate> =
                multi_trained.iter().collect();
            let spec = tunny_core::surrogate_opt::SurrogateMultiOptimizeSpec {
                minimize: prepared.model_directions().to_vec(),
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
        },
    );
    true
}

/// SurrogateOpt: single-objective candidate-suggestion stage. Returns true if a pending request was consumed.
fn surrogate_stage_suggest(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(suggest_req) = widgets.surrogate_opt.pending_suggest.take() else {
        return false;
    };
    // Candidate-suggestion stage: requires a trained GP surrogate.
    let Some(trained) = widgets.surrogate_opt.trained.clone() else {
        crate::state::analysis_job::supersede(app_state, AnalysisKind::Suggest);
        widgets.surrogate_opt.suggesting = false;
        widgets.surrogate_opt.error_message =
            Some("No trained model available. Run Fit & Validate first.".to_string());
        return true;
    };

    widgets.surrogate_opt.suggesting = true;
    widgets.surrogate_opt.error_message = None;
    let tx = tx.clone();
    spawn_model(
        app_state,
        tx,
        AnalysisKind::Suggest,
        (
            suggest_req.n_candidates,
            format!("{:?}", suggest_req.acquisition),
            suggest_req.minimize,
        ),
        Models::Single(trained.clone()),
        move |prepared| {
            let trained = prepared.models().single();
            let param_names = trained.param_names.clone();
            let objective_name = trained.objective_name.clone();
            use crate::state::messages::SurrogateSuggestUiResult;
            match tunny_core::surrogate_opt::suggest_candidates(
                trained,
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
        },
    );
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
        crate::state::analysis_job::supersede(app_state, AnalysisKind::MultiSuggest);
        widgets.surrogate_opt.multi_suggesting = false;
        widgets.surrogate_opt.error_message =
            Some("No trained multi-objective model. Run Fit & Validate first.".to_string());
        return true;
    };
    widgets.surrogate_opt.multi_suggesting = true;
    widgets.surrogate_opt.error_message = None;
    let tx = tx.clone();
    spawn_model(
        app_state,
        tx,
        AnalysisKind::MultiSuggest,
        multi_suggest_req.n_candidates,
        Models::Multi(multi_trained.clone()),
        move |prepared| {
            let multi_trained = prepared.models().multi();
            let param_names = multi_trained
                .first()
                .map(|m| m.param_names.clone())
                .unwrap_or_default();
            let objective_names = multi_trained
                .iter()
                .map(|m| m.objective_name.clone())
                .collect();
            use crate::state::messages::SurrogateMultiSuggestUiResult;
            match tunny_core::surrogate_opt::suggest_candidates_multi(
                multi_trained,
                prepared.model_directions(),
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
        },
    );
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
    widgets.robustness.trained = None;
    widgets.robustness.fit_error = None;

    let tx = tx.clone();
    spawn_analysis(
        app_state,
        tx,
        AnalysisKind::Robustness,
        (
            "robustness",
            format!("{:?}", fit_req.model),
            fit_req.objective_index,
        ),
        Preparation::Fit {
            objectives: FitObjectives::Index(fit_req.objective_index),
            constraints: true,
        },
        move |prepared| {
            let data = prepared.data().fit();
            let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
                x_matrix: data.x.clone(),
                y: data.ys[0].clone(),
                param_names: data.names.clone(),
                objective_name: data.objectives[0].clone(),
                model: fit_req.model,
                auto_select: false,
                constraints: data.constraints.clone(),
                priority_rows: vec![],
                param_bounds: Some(data.bounds.clone()),
            };
            match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
                Ok(t) => AppMessage::RobustnessFitDone(std::sync::Arc::new(t)),
                Err(e) => AppMessage::RobustnessFitFailed(e),
            }
        },
    );
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
    widgets.response_surface.trained = None;
    widgets.response_surface.fit_error = None;

    let tx = tx.clone();
    spawn_analysis(
        app_state,
        tx,
        AnalysisKind::Surface,
        (
            "surface",
            format!("{:?}", fit_req.model),
            fit_req.objective_index,
        ),
        Preparation::Fit {
            objectives: FitObjectives::Index(fit_req.objective_index),
            constraints: false,
        },
        move |prepared| {
            let data = prepared.data().fit();
            let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
                x_matrix: data.x.clone(),
                y: data.ys[0].clone(),
                param_names: data.names.clone(),
                objective_name: data.objectives[0].clone(),
                model: fit_req.model,
                auto_select: false,
                // Response-surface slice evaluation doesn't handle feasibility, so no constraints are passed.
                constraints: vec![],
                priority_rows: vec![],
                param_bounds: Some(data.bounds.clone()),
            };
            match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
                Ok(t) => AppMessage::ResponseSurfaceFitDone(std::sync::Arc::new(t)),
                Err(e) => AppMessage::ResponseSurfaceFitFailed(e),
            }
        },
    );
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
    let slice_param = req.slice_param;
    let kinds = crate::ui::widgets::compare::model_kinds(req.include_moe);

    widgets.surrogate_compare.error = None;

    let tx = tx.clone();
    spawn_analysis(
        app_state,
        tx,
        AnalysisKind::Compare,
        (req.objective_index, req.slice_param, req.include_moe),
        Preparation::Compare {
            objective: req.objective_index,
            slice: req.slice_param,
        },
        move |prepared| {
            let data = prepared.data().fit();
            let (x_matrix, y, numeric_params, param_bounds) =
                (&data.x, &data.ys[0], &data.names, &data.bounds);
            let objective_name = data.objectives[0].clone();
            let param_name = data.names[slice_param].clone();
            let anchor = data
                .anchor
                .as_ref()
                .expect("Compare preparation owns the anchor")
                .clone();
            let observed = data.observed.clone();
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
                        if let Some(slice) = tunny_core::surrogate_opt::line_slice_at(
                            &trained,
                            &anchor,
                            slice_param,
                            60,
                        ) {
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
        },
    );
}
