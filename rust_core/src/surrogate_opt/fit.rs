//! Fitting a surrogate model with validation: input checks, large-data
//! subsampling, cross-validation, and the final full-data fit. This core is
//! shared by the single- and multi-objective entry points.

use super::model_selection::{model_display_name, select_best_model_tracked};
use super::models;
use super::progress::FitProgress;
use super::types::{ConstraintData, SurrogateFitRequest, TrainedSurrogate};
use super::validation;
use super::{AUTO_CANDIDATES, MAX_TRAIN_FOR_FIT, MIN_TRIALS_FOR_SURROGATE_OPT};
use crate::math::rng::SeededRng;

/// Performs common input validation (returns (n, n_dims) on success).
pub(crate) fn validate_inputs(x_matrix: &[Vec<f64>], y: &[f64]) -> Result<(usize, usize), String> {
    let n = y.len();
    let n_dims = x_matrix.first().map(|r| r.len()).unwrap_or(0);

    if n < MIN_TRIALS_FOR_SURROGATE_OPT {
        return Err(format!(
            "At least {} trials required (current: {})",
            MIN_TRIALS_FOR_SURROGATE_OPT, n
        ));
    }
    if x_matrix.len() != n {
        return Err("x_matrix and y length mismatch".to_string());
    }
    if n_dims == 0 {
        return Err("No numeric parameters available".to_string());
    }
    if x_matrix.iter().any(|row| row.len() != n_dims) {
        return Err("x_matrix rows have inconsistent dimensions".to_string());
    }
    if x_matrix
        .iter()
        .flatten()
        .chain(y.iter())
        .any(|v| !v.is_finite())
    {
        return Err("Input contains non-finite values".to_string());
    }
    Ok((n, n_dims))
}

/// Returns a new Vec containing only the rows specified by `idx`.
pub(crate) fn take_rows<T: Clone>(rows: &[T], idx: &[usize]) -> Vec<T> {
    idx.iter().map(|&i| rows[i].clone()).collect()
}

/// Returns the ascending indices to subsample large training data down to `cap`
/// points. Returns `None` (no subsampling needed) when `N <= cap`.
///
/// Strategy: always keep the elites (regions important for optimization), and
/// fill the remaining budget from the non-elites at random (fixed seed). Since
/// Optuna trials cluster in good regions, random fill covers the space coarsely
/// while preserving that density distribution (space-filling is not used because
/// it would flatten the density and dilute the good regions). Elites take up to
/// half the budget (`cap/2`):
/// - Single-objective: both tails of the objective value (best/worst, 1/4 each).
///   `fit` is agnostic to the optimization direction, so keeping both tails
///   preserves the optimum side regardless of minimize/maximize.
/// - Multi-objective: ascending non-domination rank. Starting from rank 0,
///   expand to rank 1, 2, ... if `cap/2` is not yet reached (`nd_sort` returns
///   all rank 0 in the single-objective case, so it isn't used on that path).
pub(crate) fn subsample_indices(
    objective_cols: &[&[f64]],
    minimize: &[bool],
    cap: usize,
    seed: u64,
) -> Option<Vec<usize>> {
    let n = objective_cols.first().map_or(0, |c| c.len());
    if n <= cap {
        return None;
    }
    let elite_target = (cap / 2).min(n);
    let mut is_elite = vec![false; n];

    if objective_cols.len() <= 1 {
        // Single-objective: sort ascending by value and make both tails elite
        // (agnostic to optimization direction).
        let col = objective_cols[0];
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            col[a]
                .partial_cmp(&col[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let low = elite_target / 2;
        let high = elite_target - low;
        for &i in order.iter().take(low) {
            is_elite[i] = true;
        }
        for &i in order.iter().rev().take(high) {
            is_elite[i] = true;
        }
    } else {
        // Multi-objective: take the first elite_target points in ascending
        // non-domination rank order (expanding rank 0 -> 1 -> 2 -> ...).
        let rows: Vec<Vec<f64>> = (0..n)
            .map(|i| objective_cols.iter().map(|c| c[i]).collect())
            .collect();
        let ranks = crate::multi_objective::pareto::nd_sort(&rows, minimize);
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&i| ranks[i]); // Stable sort: index order within the same rank.
        for &i in order.iter().take(elite_target) {
            is_elite[i] = true;
        }
    }

    let mut chosen: Vec<usize> = (0..n).filter(|&i| is_elite[i]).collect();
    let mut rest: Vec<usize> = (0..n).filter(|&i| !is_elite[i]).collect();
    let mut rng = SeededRng::from_seed(seed);
    rng.shuffle(&mut rest);
    let fill = cap.saturating_sub(chosen.len());
    chosen.extend(rest.into_iter().take(fill));
    chosen.sort_unstable();
    Some(chosen)
}

/// Returns a subsampled request when the single-objective fit request is too
/// large (`None` when `N <= cap`). Constraint values and priority rows are
/// subsampled consistently using the same indices.
pub(crate) fn subsample_fit_request(req: &SurrogateFitRequest) -> Option<SurrogateFitRequest> {
    let idx = subsample_indices(&[&req.y], &[], MAX_TRAIN_FOR_FIT, 42)?;

    // Mapping from old index to new position (for remapping priority rows).
    let mut remap = vec![usize::MAX; req.y.len()];
    for (new_pos, &old) in idx.iter().enumerate() {
        remap[old] = new_pos;
    }
    let priority_rows = req
        .priority_rows
        .iter()
        .filter_map(|&o| {
            let p = remap.get(o).copied().unwrap_or(usize::MAX);
            (p != usize::MAX).then_some(p)
        })
        .collect();

    Some(SurrogateFitRequest {
        x_matrix: take_rows(&req.x_matrix, &idx),
        y: take_rows(&req.y, &idx),
        param_names: req.param_names.clone(),
        objective_name: req.objective_name.clone(),
        model: req.model,
        auto_select: req.auto_select,
        constraints: req
            .constraints
            .iter()
            .map(|c| ConstraintData {
                name: c.name.clone(),
                values: take_rows(&c.values, &idx),
            })
            .collect(),
        priority_rows,
        param_bounds: req.param_bounds.clone(),
    })
}

/// Estimates the number of model fits planned for training (the progress bar
/// denominator). Kept in sync with how many times [`fit_validated_inner`] calls
/// `inc_done`: for auto selection, per-candidate validation (1 holdout + k CV)
/// times the number of candidates, plus the main validation (1 + k), plus 1 for
/// the final model, plus the number of constraints.
pub(crate) fn estimate_fit_count(req: &SurrogateFitRequest) -> usize {
    let k = req.y.len().min(5);
    let validate = 1 + k;
    let auto = if req.auto_select {
        AUTO_CANDIDATES.len() * validate
    } else {
        0
    };
    auto + validate + 1 + req.constraints.len()
}

/// Fits a surrogate and returns the result validated by holdout + k-fold CV.
///
/// Uses validation seed 42. Constraint models are fit on all data without CV.
pub fn fit_surrogate_with_validation(
    req: &SurrogateFitRequest,
) -> Result<TrainedSurrogate, String> {
    fit_surrogate_with_validation_tracked(req, &FitProgress::default())
}

/// Same as [`fit_surrogate_with_validation`], but supports progress reporting
/// and cancellation via `progress` (used by background training from the UI).
pub fn fit_surrogate_with_validation_tracked(
    req: &SurrogateFitRequest,
    progress: &FitProgress,
) -> Result<TrainedSurrogate, String> {
    validate_inputs(&req.x_matrix, &req.y)?;

    // Subsample large data before fitting (validation fits the same model
    // multiple times, so cost scales nearly linearly with N). The subsampled
    // set is used for everything downstream (CV, final model, constraints), so
    // the validation score and the actually-deployed model see the same data.
    let subsampled = subsample_fit_request(req);
    let req = subsampled.as_ref().unwrap_or(req);

    progress.set_total(estimate_fit_count(req));
    fit_validated_inner(req, progress, "")
}

/// The core of validation + full-data fitting (assumes the caller has already
/// done input validation and subsampling).
///
/// Updates `progress` at each model-fit boundary and returns `Err` early if
/// cancellation is requested. `stage_prefix` is the prefix for the stage label
/// (used to identify the objective in the multi-objective case).
pub(crate) fn fit_validated_inner(
    req: &SurrogateFitRequest,
    progress: &FitProgress,
    stage_prefix: &str,
) -> Result<TrainedSurrogate, String> {
    // For auto selection, cross-validate AUTO_CANDIDATES to decide the best
    // model. All subsequent fitting, validation, and constraint models use the
    // concrete chosen model kind (this stays consistent automatically since
    // SurrogateModelKind has no "Auto" variant).
    let (model_kind, model_selection) = if req.auto_select {
        let report = select_best_model_tracked(&req.x_matrix, &req.y, 42, progress, stage_prefix)?;
        let chosen = report.chosen;
        (chosen, Some(report))
    } else {
        (req.model, None)
    };

    // Run CV and holdout validation.
    progress.set_stage(format!(
        "{stage_prefix}Cross-validating {}",
        model_display_name(model_kind)
    ));
    let mut report = validation::validate_surrogate_tracked_front(
        model_kind,
        &req.x_matrix,
        &req.y,
        42,
        &req.priority_rows,
        progress,
    )?;

    // Fit the final model on all data. If priority rows (e.g. the Pareto front)
    // are given, concentrate the GP's inducing points there. The CV/holdout
    // validation side keeps uniform inducing points since it estimates
    // generalization performance (validate_surrogate does not accept priority).
    progress.check()?;
    progress.set_stage(format!("{stage_prefix}Fitting final model"));
    let surrogate = models::fit_surrogate_with_priority_bounds(
        model_kind,
        &req.x_matrix,
        &req.y,
        &req.priority_rows,
        req.param_bounds.as_deref(),
    )?;
    progress.inc_done();

    // Set the full-data training R² from the final model.
    report.train_r2 = surrogate.r_squared;

    // Parameter importance from the ARD length scales (Some only for GP, same
    // order as param_names).
    let param_importance = surrogate.param_importance();

    // Fit a surrogate for each constraint (no CV, all data).
    let mut constraint_names = Vec::with_capacity(req.constraints.len());
    let mut constraint_models = Vec::with_capacity(req.constraints.len());
    let mut constraint_values: Vec<Vec<f64>> = Vec::with_capacity(req.x_matrix.len());
    for _ in 0..req.x_matrix.len() {
        constraint_values.push(Vec::with_capacity(req.constraints.len()));
    }

    for cd in &req.constraints {
        // Constraint models are fit with the same model kind as the objective.
        // For GP-family models, the posterior variance gives a smooth
        // feasibility probability P(c <= 0) (enabling search that accounts for
        // uncertainty near the constraint boundary); Ridge / LightGBM fall back
        // to a hard indicator (see feasibility::single_prob).
        // Under auto selection, the constraint model also uses the same
        // "chosen" kind as the objective model.
        progress.check()?;
        progress.set_stage(format!("{stage_prefix}Fitting constraint '{}'", cd.name));
        let cm = models::fit_constraint_surrogate_bounds(
            model_kind,
            &req.x_matrix,
            &cd.values,
            req.param_bounds.as_deref(),
        )
        .map_err(|e| format!("Constraint '{}' fit failed: {}", cd.name, e))?;
        progress.inc_done();
        constraint_names.push(cd.name.clone());
        constraint_models.push(cm);
        for (i, &v) in cd.values.iter().enumerate() {
            if let Some(row) = constraint_values.get_mut(i) {
                row.push(v);
            }
        }
    }

    Ok(TrainedSurrogate {
        surrogate,
        model_kind,
        param_names: req.param_names.clone(),
        objective_name: req.objective_name.clone(),
        x_matrix: req.x_matrix.clone(),
        y: req.y.clone(),
        validation: report,
        param_importance,
        constraint_names,
        constraint_models,
        constraint_values,
        model_selection,
    })
}
