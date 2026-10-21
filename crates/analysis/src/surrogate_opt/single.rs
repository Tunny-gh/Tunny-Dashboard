//! Single-objective optimization against a fitted surrogate: the shared
//! `run_optimize` core, the "optimize an already-trained model" entry point,
//! and the "fit + optimize in one call" entry point.

use super::feasibility;
use super::models;
use super::optimizers::{self, OptimizerKind};
use super::slice::{best_observed_index, build_slice};
use super::types::{
    SurrogateOptRequest, SurrogateOptResult, SurrogateOptimizeSpec, TrainedSurrogate,
};
use super::validate_inputs;

/// Common logic that runs optimization against a fitted surrogate and returns
/// the result.
///
/// When `constraint_models` is non-empty, the search adds a constraint penalty
/// to the cost function.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_optimize(
    surrogate: &models::FittedSurrogate,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    minimize: bool,
    optimizer: OptimizerKind,
    slice_params: Option<(usize, usize)>,
    n_grid: usize,
    constraint_models: &[models::FittedSurrogate],
) -> SurrogateOptResult {
    let n_dims = x_matrix.first().map(|r| r.len()).unwrap_or(0);

    // Best observed point (used as the optimization start point).
    let best_observed_idx = best_observed_index(y, minimize);
    let start_norm = surrogate.to_norm_x(&x_matrix[best_observed_idx]);

    let t_best = optimizers::minimize_on_surrogate(
        surrogate,
        minimize,
        optimizer,
        &start_norm,
        constraint_models,
    );

    let best_value = surrogate.to_original_y(surrogate.predict_norm(&t_best));
    let predicted_std = surrogate
        .predict_var_norm(&t_best)
        .map(|v| v.max(0.0).sqrt() * surrogate.y_std);

    let slice = slice_params
        .and_then(|(px, py)| build_slice(surrogate, &t_best, px, py, n_grid.max(2), n_dims));

    let best_observed_value = y[best_observed_idx];

    // Compute the predicted constraint values and feasibility probability.
    let (predicted_constraints, feasibility_probability) = if constraint_models.is_empty() {
        (vec![], None)
    } else {
        let preds: Vec<f64> = constraint_models
            .iter()
            .map(|cm| cm.to_original_y(cm.predict_norm(&t_best)))
            .collect();
        let p_feas = feasibility::feasibility_probability(constraint_models, &t_best);
        (preds, Some(p_feas))
    };

    SurrogateOptResult {
        best_params: surrogate.to_original_x(&t_best),
        best_value,
        predicted_std,
        r_squared: surrogate.r_squared,
        slice,
        best_observed_value,
        predicted_constraints,
        feasibility_probability,
    }
}

/// Runs optimization against a fitted surrogate model.
pub fn optimize_on_trained(
    trained: &TrainedSurrogate,
    spec: &SurrogateOptimizeSpec,
) -> SurrogateOptResult {
    run_optimize(
        &trained.surrogate,
        &trained.x_matrix,
        &trained.y,
        spec.minimize,
        spec.optimizer,
        spec.slice_params,
        spec.n_grid,
        &trained.constraint_models,
    )
}

/// Fits a surrogate model and runs optimization over that surface.
///
/// Does not depend on a thread-local DataFrame, so it can be called from a
/// background thread.
pub fn run_surrogate_optimization(req: &SurrogateOptRequest) -> Result<SurrogateOptResult, String> {
    validate_inputs(&req.x_matrix, &req.y)?;

    let surrogate = models::fit_surrogate(req.model, &req.x_matrix, &req.y)?;

    // Fit constraint surrogates (empty vec when unconstrained). Uses the same
    // model kind as the objective: a smooth feasibility probability for
    // GP-family models, a hard indicator for Ridge / LightGBM.
    let constraint_models: Vec<models::FittedSurrogate> = req
        .constraints
        .iter()
        .map(|cd| {
            models::fit_constraint_surrogate(req.model, &req.x_matrix, &cd.values)
                .map_err(|e| format!("Constraint '{}' fit failed: {}", cd.name, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(run_optimize(
        &surrogate,
        &req.x_matrix,
        &req.y,
        req.minimize,
        req.optimizer,
        req.slice_params,
        req.n_grid,
        &constraint_models,
    ))
}
