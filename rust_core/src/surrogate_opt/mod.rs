//! Fitting a response surface (surrogate model) and optimizing over that surface.
//!
//! Fits a surrogate model from sampling results (a set of trials), then runs
//! optimization inside the normalized [0,1]^d box to return the estimated optimum.
//! Extend the model and optimization methods by adding variants to
//! [`SurrogateModelKind`] / [`OptimizerKind`] respectively.

mod acquisition;
mod ard;
mod ehvi;
pub(crate) mod feasibility;
mod models;
// Exposed within the crate because the gh runner (crate::gh::runner) repurposes nsga2
// for real objective-function evaluation.
pub(crate) mod optimizers;
pub(crate) mod progress;
mod robustness;
pub(crate) mod validation;

pub use acquisition::{suggest_candidates, AcquisitionKind, SuggestedCandidate};
pub use ard::{compute_ard_importance_from_df, ArdImportanceResult};
pub use ehvi::{suggest_candidates_multi, MultiSuggestedCandidate};
pub use models::SurrogateModelKind;
pub use optimizers::OptimizerKind;
pub use progress::{FitProgress, FitProgressSnapshot};
pub use robustness::{robustness_analysis, NoiseDistribution, RobustnessResult, RobustnessSpec};
pub use validation::SurrogateValidationReport;

use crate::math::grid::linspace;
use crate::math::rng::SeededRng;
use progress::FIT_CANCELLED;
use validation::validate_surrogate_tracked;

/// Minimum number of trials required to fit a surrogate.
pub const MIN_TRIALS_FOR_SURROGATE_OPT: usize = 10;

/// Upper bound on the number of trials used for fitting. Above this, the data is
/// subsampled by keeping an elite band plus random fill.
///
/// GP-FITC compresses the information into M=100 inducing points, so subsampling
/// N down to roughly this size barely degrades the response surface quality.
/// Meanwhile cost scales nearly linearly with N (validation fits the same model
/// 7 times), so this cuts wait time substantially for large studies.
pub const MAX_TRAIN_FOR_FIT: usize = 2000;

/// Candidate models for automatic model selection (Auto). The candidate with the
/// highest CV R² is chosen.
///
/// The order is arranged "simplest / lowest-cost model first" (Ridge -> GP-FITC ->
/// GP-VFE -> LightGBM). On a tie, the candidate earlier in this order is preferred
/// (tie-break).
///
/// GpMoe is excluded from the candidates:
///   - It searches the cluster count via CV, so its per-candidate validation cost
///     is far higher than the others.
///   - On smooth/linear data it degenerates to a single GP (clusters collapse),
///     giving Auto poor cost-effectiveness. MoE is meant to be selected manually
///     when the response is known to be discontinuous or multimodal.
pub const AUTO_CANDIDATES: [SurrogateModelKind; 4] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::Lgbm,
];

/// Result of automatic model selection (Auto). Holds the chosen model and the
/// per-candidate CV R².
#[derive(Debug, Clone)]
pub struct ModelSelectionReport {
    /// The selected model kind (highest CV R²; ties prefer the earlier entry in
    /// AUTO_CANDIDATES).
    pub chosen: SurrogateModelKind,
    /// Per-candidate (model kind, score = cv_r2_mean), in the same order as
    /// `AUTO_CANDIDATES`. A candidate whose fit/validation fails is recorded as
    /// f64::NEG_INFINITY and excluded from selection.
    pub scores: Vec<(SurrogateModelKind, f64)>,
}

/// Cross-validates `AUTO_CANDIDATES` and selects the model with the highest CV R².
///
/// Runs [`validate_surrogate`] for each candidate and uses `cv_r2_mean` as its
/// score. Candidates whose score differs by less than 1e-3 are treated as "tied",
/// preferring the candidate earlier in `AUTO_CANDIDATES` (simpler / lower cost).
/// Returns `Err` only if every candidate fails.
pub fn select_best_model(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
) -> Result<ModelSelectionReport, String> {
    select_best_model_tracked(x_matrix, y, seed, &FitProgress::default(), "")
}

/// Same as [`select_best_model`] but supports progress reporting and cancellation.
///
/// `stage_prefix` is the prefix for the stage label (used to prepend
/// "Objective k/N: " in the multi-objective case). If a cancellation is
/// requested, returns [`FIT_CANCELLED`] rather than letting it look like an
/// ordinary candidate-validation failure.
fn select_best_model_tracked(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
    progress: &FitProgress,
    stage_prefix: &str,
) -> Result<ModelSelectionReport, String> {
    validate_inputs(x_matrix, y)?;

    let mut scores: Vec<(SurrogateModelKind, f64)> = Vec::with_capacity(AUTO_CANDIDATES.len());
    for (i, &kind) in AUTO_CANDIDATES.iter().enumerate() {
        progress.check()?;
        progress.set_stage(format!(
            "{stage_prefix}Evaluating candidate {} ({}/{})",
            model_display_name(kind),
            i + 1,
            AUTO_CANDIDATES.len()
        ));
        // A candidate whose fit/validation fails is recorded as NEG_INFINITY and
        // excluded from selection. However, a failure caused by cancellation is
        // propagated rather than swallowed.
        let score = match validate_surrogate_tracked(kind, x_matrix, y, seed, progress) {
            Ok(report) => report.cv_r2_mean,
            Err(_) if progress.is_cancelled() => return Err(FIT_CANCELLED.to_string()),
            Err(_) => f64::NEG_INFINITY,
        };
        scores.push((kind, score));
    }

    // Candidates whose CV R² differs by less than this value are treated as
    // "tied", preferring the candidate earlier in AUTO_CANDIDATES (simpler /
    // lower cost). On perfectly linear data both GP and Ridge fit almost
    // perfectly (R² ≈ 1), so this avoids picking the more complex GP over a
    // negligible difference.
    const TIE_TOLERANCE: f64 = 1e-3;

    // Select the candidate with the highest score. Scanning from the front of
    // AUTO_CANDIDATES and only accepting a strictly-better-than-tolerance score
    // means ties are left resolved in favor of the earlier (simpler) candidate.
    let mut chosen: Option<(SurrogateModelKind, f64)> = None;
    for &(kind, score) in &scores {
        if !score.is_finite() {
            continue;
        }
        match chosen {
            Some((_, best)) if score <= best + TIE_TOLERANCE => {}
            _ => chosen = Some((kind, score)),
        }
    }

    let chosen = chosen
        .map(|(kind, _)| kind)
        .ok_or_else(|| "All candidate models failed validation".to_string())?;

    Ok(ModelSelectionReport { chosen, scores })
}

/// Default resolution of the slice grid.
pub const DEFAULT_SLICE_GRID: usize = 20;

/// Input to surrogate optimization.
pub struct SurrogateOptRequest {
    /// Training data (row = trial, column = parameter), in original units.
    pub x_matrix: Vec<Vec<f64>>,
    /// Objective values (original units).
    pub y: Vec<f64>,
    /// Name of each parameter column (same order as `best_params` in the result).
    pub param_names: Vec<String>,
    /// Objective name (for display).
    pub objective_name: String,
    /// true = minimize, false = maximize.
    pub minimize: bool,
    /// Surrogate model to use.
    pub model: SurrogateModelKind,
    /// Optimizer to use.
    pub optimizer: OptimizerKind,
    /// Column indices of the two parameters for the response-surface slice
    /// through the optimum (for display).
    pub slice_params: Option<(usize, usize)>,
    /// Number of points along one side of the slice grid.
    pub n_grid: usize,
    /// Constraint data (empty = unconstrained).
    pub constraints: Vec<ConstraintData>,
}

/// Data for a single constraint passed to surrogate fitting.
///
/// Optuna's constraint convention: value ≤ 0 is feasible.
pub struct ConstraintData {
    /// Constraint name (for display/logging).
    pub name: String,
    /// Constraint value per trial (same row order as `x_matrix`).
    pub values: Vec<f64>,
}

/// Input to surrogate fitting + validation.
pub struct SurrogateFitRequest {
    pub x_matrix: Vec<Vec<f64>>,
    pub y: Vec<f64>,
    pub param_names: Vec<String>,
    pub objective_name: String,
    pub model: SurrogateModelKind,
    /// When true, ignores `model` and cross-validates `AUTO_CANDIDATES` to
    /// automatically select and fit the best model (the outcome is recorded in
    /// `TrainedSurrogate.model_selection`).
    pub auto_select: bool,
    /// Constraint data (empty = unconstrained). Each element is one constraint.
    pub constraints: Vec<ConstraintData>,
    /// Row indices (into `x_matrix`) to prioritize as inducing points. Empty =
    /// uniform (default). Used to concentrate the GP's inducing points on
    /// Pareto-front trials in the multi-objective case. Has no effect when N is
    /// at or below the GP's inducing-point cap (100), since Z = X uses every
    /// point anyway.
    pub priority_rows: Vec<usize>,
    /// Declared range (low, high) per parameter column (derived from the log;
    /// same order as `param_names`). When `Some(vec)`, each column is normalized
    /// using this range instead of the observed min/max, so the optimization
    /// search box (normalized space [0,1]^d) matches the true variable range.
    /// Falls back to the observed range for columns that are `None`, or when the
    /// whole field is `None`.
    pub param_bounds: Option<Vec<Option<(f64, f64)>>>,
}

/// A validated fit result, reused for optimization.
pub struct TrainedSurrogate {
    pub(crate) surrogate: models::FittedSurrogate,
    pub model_kind: SurrogateModelKind,
    pub param_names: Vec<String>,
    pub objective_name: String,
    /// Original data used for fitting (used as the optimization start point).
    pub(crate) x_matrix: Vec<Vec<f64>>,
    pub(crate) y: Vec<f64>,
    pub validation: SurrogateValidationReport,
    /// Relative parameter importance derived from the ARD length scales (same
    /// order as `param_names`, summing to 1.0).
    ///
    /// Some only for GP (single SGP: FITC / VFE). None for MoE / Ridge / LightGBM.
    /// The importance corresponds to the model's input dimensions (= columns of
    /// `x_matrix`), whose column order matches `param_names` (since
    /// `fit_surrogate` never reorders columns).
    pub param_importance: Option<Vec<f64>>,
    /// Constraint names (same order as `constraint_models`; empty = unconstrained).
    pub constraint_names: Vec<String>,
    /// Fitted surrogate per constraint (same order as `constraint_names`).
    pub(crate) constraint_models: Vec<models::FittedSurrogate>,
    /// Constraint value per trial (row = trial, column = constraint; same order
    /// as `constraint_names`). Used to compute the feasible incumbent.
    pub(crate) constraint_values: Vec<Vec<f64>>,
    /// History of automatic model selection (`auto_select = true`). None when
    /// the model was specified manually. `model_kind` holds the concrete model
    /// kind that was chosen.
    pub model_selection: Option<ModelSelectionReport>,
}

/// Configuration for the optimization stage (run against an already-fitted model).
pub struct SurrogateOptimizeSpec {
    pub minimize: bool,
    pub optimizer: OptimizerKind,
    pub slice_params: Option<(usize, usize)>,
    pub n_grid: usize,
}

/// A 2D slice of the response surface through the optimum (other dimensions
/// fixed at the optimum).
#[derive(Debug, Clone)]
pub struct SurfaceSlice {
    pub param_x_idx: usize,
    pub param_y_idx: usize,
    /// X-axis grid values (original units).
    pub x_values: Vec<f64>,
    /// Y-axis grid values (original units).
    pub y_values: Vec<f64>,
    /// Predicted value grid. `z_values[i][j] = f(x_values[i], y_values[j])`.
    pub z_values: Vec<Vec<f64>>,
    /// Grid of predicted standard deviations (original units, same shape as
    /// `z_values`). Some only for models with a posterior variance (GP family);
    /// None for Ridge / LightGBM.
    pub z_std: Option<Vec<Vec<f64>>>,
}

/// Result of surrogate optimization.
#[derive(Debug, Clone)]
pub struct SurrogateOptResult {
    /// Parameter values at the estimated optimum (original units, same order as
    /// `param_names`).
    pub best_params: Vec<f64>,
    /// Surrogate prediction at the estimated optimum (original units).
    pub best_value: f64,
    /// Predicted standard deviation (Gaussian-process models only; None for Ridge).
    pub predicted_std: Option<f64>,
    /// Coefficient of determination of the surrogate on the training data.
    pub r_squared: f64,
    /// Response-surface slice through the optimum (only when `slice_params` is
    /// given).
    pub slice: Option<SurfaceSlice>,
    /// Best value among the observed data (original units). The minimum when
    /// minimizing, the maximum when maximizing.
    pub best_observed_value: f64,
    /// Predicted value of each constraint surrogate at the estimated optimum
    /// (original units, same order as `constraint_names`). Empty when
    /// unconstrained (`constraint_names` is empty).
    pub predicted_constraints: Vec<f64>,
    /// Feasibility probability at the estimated optimum (0.0-1.0). None when
    /// unconstrained.
    pub feasibility_probability: Option<f64>,
}

/// Performs common input validation (returns (n, n_dims) on success).
fn validate_inputs(x_matrix: &[Vec<f64>], y: &[f64]) -> Result<(usize, usize), String> {
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
fn take_rows<T: Clone>(rows: &[T], idx: &[usize]) -> Vec<T> {
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
fn subsample_indices(
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
fn subsample_fit_request(req: &SurrogateFitRequest) -> Option<SurrogateFitRequest> {
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

/// Common logic that runs optimization against a fitted surrogate and returns
/// the result.
///
/// When `constraint_models` is non-empty, the search adds a constraint penalty
/// to the cost function.
#[allow(clippy::too_many_arguments)]
fn run_optimize(
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

/// Display name of a surrogate model kind (for progress labels).
fn model_display_name(kind: SurrogateModelKind) -> &'static str {
    match kind {
        SurrogateModelKind::Ridge => "Ridge",
        SurrogateModelKind::GpFitc => "GP-FITC",
        SurrogateModelKind::GpVfe => "GP-VFE",
        SurrogateModelKind::GpMoe => "GP-MOE",
        SurrogateModelKind::Lgbm => "LightGBM",
    }
}

/// Estimates the number of model fits planned for training (the progress bar
/// denominator). Kept in sync with how many times [`fit_validated_inner`] calls
/// `inc_done`: for auto selection, per-candidate validation (1 holdout + k CV)
/// times the number of candidates, plus the main validation (1 + k), plus 1 for
/// the final model, plus the number of constraints.
fn estimate_fit_count(req: &SurrogateFitRequest) -> usize {
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
fn fit_validated_inner(
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

/// Evaluates a response-surface slice of a fitted surrogate (for the 3D
/// response-surface viewer).
///
/// Passes through `anchor_orig` (original units, same order as `param_names`)
/// and evaluates an `n_grid` x `n_grid` grid over the full declared range in
/// the `param_x_idx` x `param_y_idx` plane. Unlike a PDP, it does not marginalize
/// the other parameters — it returns a "raw cross-section" with them fixed at
/// the anchor point.
pub fn surface_slice_at(
    trained: &TrainedSurrogate,
    anchor_orig: &[f64],
    param_x_idx: usize,
    param_y_idx: usize,
    n_grid: usize,
) -> Option<SurfaceSlice> {
    let surrogate = &trained.surrogate;
    let n_dims = surrogate.col_stats.len();
    if anchor_orig.len() != n_dims {
        return None;
    }
    let anchor_norm = surrogate.to_norm_x(anchor_orig);
    build_slice(
        surrogate,
        &anchor_norm,
        param_x_idx,
        param_y_idx,
        n_grid.max(2),
        n_dims,
    )
}

/// Row index of the best observed value (minimum when minimizing, maximum when
/// maximizing).
fn best_observed_index(y: &[f64], minimize: bool) -> usize {
    let mut best = 0usize;
    for (i, &v) in y.iter().enumerate() {
        let better = if minimize { v < y[best] } else { v > y[best] };
        if better {
            best = i;
        }
    }
    best
}

/// Evaluates a 2D slice grid through the optimum `t_best` (normalized space)
/// using the surrogate.
fn build_slice(
    surrogate: &models::FittedSurrogate,
    t_best: &[f64],
    param_x_idx: usize,
    param_y_idx: usize,
    n_grid: usize,
    n_dims: usize,
) -> Option<SurfaceSlice> {
    if param_x_idx >= n_dims || param_y_idx >= n_dims || param_x_idx == param_y_idx {
        return None;
    }
    let (min_x, range_x) = surrogate.col_stats[param_x_idx];
    let (min_y, range_y) = surrogate.col_stats[param_y_idx];
    let x_values = linspace(min_x, min_x + range_x, n_grid);
    let y_values = linspace(min_y, min_y + range_y, n_grid);

    // Evaluate the mean (original units) at each grid point, plus the
    // original-unit standard deviation derived from the posterior variance
    // where available. z_std holds Some only when the model has a posterior
    // variance (GP family).
    let mut z_values: Vec<Vec<f64>> = Vec::with_capacity(x_values.len());
    let mut z_std_grid: Vec<Vec<f64>> = Vec::with_capacity(x_values.len());
    let mut has_std = true;
    for &vx in &x_values {
        let mut z_row = Vec::with_capacity(y_values.len());
        let mut std_row = Vec::with_capacity(y_values.len());
        for &vy in &y_values {
            let mut pt = t_best.to_vec();
            pt[param_x_idx] = (vx - min_x) / range_x;
            pt[param_y_idx] = (vy - min_y) / range_y;
            z_row.push(surrogate.to_original_y(surrogate.predict_norm(&pt)));
            match surrogate.predict_var_norm(&pt) {
                // Normalized-space variance -> original-unit standard deviation
                // (scaled by y_std).
                Some(var) => std_row.push(var.max(0.0).sqrt() * surrogate.y_std),
                None => has_std = false,
            }
        }
        z_values.push(z_row);
        z_std_grid.push(std_row);
    }
    let z_std = has_std.then_some(z_std_grid);

    Some(SurfaceSlice {
        param_x_idx,
        param_y_idx,
        x_values,
        y_values,
        z_values,
        z_std,
    })
}

/// A predicted slice along one parameter direction, through the anchor point
/// (for the surrogate comparison view).
#[derive(Debug, Clone)]
pub struct LineSlice {
    /// Column index of the parameter being sliced.
    pub param_idx: usize,
    /// Grid values (original units).
    pub x_values: Vec<f64>,
    /// Predicted values (original units).
    pub y_values: Vec<f64>,
    /// Predicted standard deviation (original units). Some only for models with
    /// a posterior variance (GP family).
    pub y_std: Option<Vec<f64>>,
}

/// Evaluates a 1D predicted slice through the anchor point (original units)
/// using the surrogate.
///
/// Fixes every dimension other than `param_idx` at the anchor value, and
/// evaluates `param_idx` over its declared range (falling back to the observed
/// range) at `n_grid` points (minimum 2). Returns `None` on a dimension
/// mismatch or out-of-range index.
pub fn line_slice_at(
    trained: &TrainedSurrogate,
    anchor_orig: &[f64],
    param_idx: usize,
    n_grid: usize,
) -> Option<LineSlice> {
    let surrogate = &trained.surrogate;
    let n_dims = surrogate.col_stats.len();
    if anchor_orig.len() != n_dims || param_idx >= n_dims {
        return None;
    }
    let anchor_norm = surrogate.to_norm_x(anchor_orig);
    let (min_x, range_x) = surrogate.col_stats[param_idx];
    let x_values = linspace(min_x, min_x + range_x, n_grid.max(2));

    let mut y_values = Vec::with_capacity(x_values.len());
    let mut std_values = Vec::with_capacity(x_values.len());
    let mut has_std = true;
    for &vx in &x_values {
        let mut pt = anchor_norm.clone();
        pt[param_idx] = (vx - min_x) / range_x;
        y_values.push(surrogate.to_original_y(surrogate.predict_norm(&pt)));
        match surrogate.predict_var_norm(&pt) {
            // Normalized-space variance -> original-unit standard deviation
            // (scaled by y_std).
            Some(var) => std_values.push(var.max(0.0).sqrt() * surrogate.y_std),
            None => has_std = false,
        }
    }

    Some(LineSlice {
        param_idx,
        x_values,
        y_values,
        y_std: has_std.then_some(std_values),
    })
}

/// Input to multi-objective surrogate optimization.
pub struct SurrogateMultiOptRequest {
    /// Training data (row = trial, column = parameter), in original units.
    pub x_matrix: Vec<Vec<f64>>,
    /// Value column per objective. `ys[k][i]` = value of objective k for trial i.
    pub ys: Vec<Vec<f64>>,
    /// Name of each parameter column.
    pub param_names: Vec<String>,
    /// Objective names, same order as `ys`.
    pub objective_names: Vec<String>,
    /// Per-objective true = minimize. Same length as `ys`.
    pub minimize: Vec<bool>,
    /// Surrogate model to use.
    pub model: SurrogateModelKind,
    /// Column indices of the two parameters for the response-surface slice
    /// (for display).
    pub slice_params: Option<(usize, usize)>,
    /// Number of points along one side of the slice grid.
    pub n_grid: usize,
}

/// A single point on the predicted Pareto front.
#[derive(Debug, Clone)]
pub struct ParetoFrontPoint {
    /// Parameter values (original units, same order as `param_names`).
    pub params: Vec<f64>,
    /// Surrogate-predicted value for each objective (original units, same order
    /// as `objective_names`).
    pub values: Vec<f64>,
}

/// Result of multi-objective surrogate optimization.
#[derive(Debug, Clone)]
pub struct SurrogateMultiOptResult {
    /// Predicted Pareto front, sorted ascending by the first objective's value.
    pub front: Vec<ParetoFrontPoint>,
    /// Training-data coefficient of determination per objective (same order as
    /// `objective_names`).
    pub r_squared: Vec<f64>,
    /// Response-surface slice per objective (only when `slice_params` is given,
    /// same order as `objective_names`; empty when unspecified/invalid).
    pub slices: Vec<SurfaceSlice>,
}

/// Configuration for the multi-objective optimization stage (run against a set
/// of already-fitted models).
pub struct SurrogateMultiOptimizeSpec {
    /// Per-objective true = minimize. Same length as `trained`.
    pub minimize: Vec<bool>,
    pub slice_params: Option<(usize, usize)>,
    pub n_grid: usize,
}

/// Common input to multi-objective optimization (for a single objective).
struct MultiObjectiveEntry<'a> {
    surrogate: &'a models::FittedSurrogate,
    /// Training data used to find the observed-best point (initial seed).
    x_matrix: &'a [Vec<f64>],
    y: &'a [f64],
}

/// Common logic that runs NSGA-II against a set of fitted surrogates and
/// post-processes the resulting front.
///
/// Assumes every entry's surrogate shares the same normalization transform
/// (col_stats), i.e. all were fit from an x_matrix over the same parameter
/// space.
fn run_multi_optimize(
    entries: &[MultiObjectiveEntry<'_>],
    minimize: &[bool],
    slice_params: Option<(usize, usize)>,
    n_grid: usize,
) -> SurrogateMultiOptResult {
    let n_obj = entries.len();
    let surrogates: Vec<&models::FittedSurrogate> = entries.iter().map(|e| e.surrogate).collect();
    let ref_surrogate = surrogates[0];
    let n_dims = ref_surrogate.col_stats.len();

    let r_squared: Vec<f64> = surrogates.iter().map(|s| s.r_squared).collect();

    // ── Initial seeds: normalize the observed-best point per objective ─────
    // col_stats is shared across all surrogates, so use the first surrogate's
    // to_norm_x.
    let seeds: Vec<Vec<f64>> = entries
        .iter()
        .zip(minimize.iter())
        .map(|(e, &min_k)| {
            let best_idx = best_observed_index(e.y, min_k);
            ref_surrogate.to_norm_x(&e.x_matrix[best_idx])
        })
        .collect();

    // ── Run NSGA-II ──────────────────────────────────────────────────
    let signs: Vec<f64> = minimize
        .iter()
        .map(|&m| if m { 1.0 } else { -1.0 })
        .collect();
    let raw_front = optimizers::multi_objective_nsga2(&surrogates, &signs, &seeds);

    // ── Post-process front points ───────────────────────────────────
    // Remove duplicate genomes (within 1e-9 across every dimension).
    let mut deduped: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
    'outer: for (genome, fitness) in raw_front {
        for (existing, _) in &deduped {
            if genome
                .iter()
                .zip(existing.iter())
                .all(|(a, b)| (a - b).abs() < 1e-9)
            {
                continue 'outer;
            }
        }
        deduped.push((genome, fitness));
    }

    // Clamp each point's genome to [0,1] and compute the surrogate-predicted
    // value (original units) for every objective.
    let mut front_points: Vec<ParetoFrontPoint> = deduped
        .into_iter()
        .map(|(genome, _)| {
            let clamped: Vec<f64> = genome.iter().map(|v| v.clamp(0.0, 1.0)).collect();
            let params = ref_surrogate.to_original_x(&clamped);
            let values: Vec<f64> = surrogates
                .iter()
                .map(|s| s.to_original_y(s.predict_norm(&clamped)))
                .collect();
            ParetoFrontPoint { params, values }
        })
        .collect();

    // Sort ascending by the first objective's value.
    front_points.sort_by(|a, b| {
        a.values[0]
            .partial_cmp(&b.values[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── Slices ───────────────────────────────────────────────────────
    let slices = if let Some((px, py)) = slice_params {
        // Balance point: the point closest to the ideal point in normalized
        // objective space. Ideal point = the sign-adjusted minimum of each
        // objective (NSGA-II's minimization direction).
        if front_points.is_empty() || px >= n_dims || py >= n_dims || px == py {
            Vec::new()
        } else {
            // Ideal and nadir points in the sign-adjusted (minimization) frame.
            let ideal: Vec<f64> = (0..n_obj)
                .map(|k| {
                    front_points
                        .iter()
                        .map(|p| signs[k] * p.values[k])
                        .fold(f64::INFINITY, f64::min)
                })
                .collect();
            let nadir: Vec<f64> = (0..n_obj)
                .map(|k| {
                    front_points
                        .iter()
                        .map(|p| signs[k] * p.values[k])
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .collect();
            // Per-objective range, used to normalize each objective to [0, 1]
            // before measuring distance. Without this the Euclidean distance is
            // dominated by whichever objective has the larger numeric magnitude
            // (e.g. one in [0, 1000] vs one in [0, 1]), so the chosen "balance"
            // point would not actually be balanced. Guard the degenerate
            // zero-range case.
            let ranges: Vec<f64> = (0..n_obj)
                .map(|k| (nadir[k] - ideal[k]).max(1e-12))
                .collect();

            // Find the normalized parameters of the balance point (the point
            // closest to the ideal point in normalized objective space).
            let ideal_dist = |p: &ParetoFrontPoint| -> f64 {
                (0..n_obj)
                    .map(|k| ((signs[k] * p.values[k] - ideal[k]) / ranges[k]).powi(2))
                    .sum()
            };
            let balance_norm = front_points
                .iter()
                .min_by(|a, b| {
                    ideal_dist(a)
                        .partial_cmp(&ideal_dist(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|p| ref_surrogate.to_norm_x(&p.params))
                .unwrap_or_else(|| vec![0.5; n_dims]);

            // Build a slice for each objective.
            surrogates
                .iter()
                .filter_map(|s| build_slice(s, &balance_norm, px, py, n_grid.max(2), n_dims))
                .collect()
        }
    } else {
        Vec::new()
    };

    SurrogateMultiOptResult {
        front: front_points,
        r_squared,
        slices,
    }
}

/// Estimates the Pareto front via NSGA-II against a set of validated fit results.
/// `trained[k]` is the surrogate for objective k. Every element must share the
/// same param_names and training-data dimensionality.
pub fn optimize_multi_on_trained(
    trained: &[&TrainedSurrogate],
    spec: &SurrogateMultiOptimizeSpec,
) -> Result<SurrogateMultiOptResult, String> {
    let n_obj = trained.len();
    if n_obj < 2 {
        return Err(format!(
            "At least 2 objectives required (current: {})",
            n_obj
        ));
    }
    if spec.minimize.len() != n_obj {
        return Err("trained and minimize length mismatch".to_string());
    }
    let first = trained[0];
    if trained.iter().any(|t| t.param_names != first.param_names) {
        return Err("trained surrogates have inconsistent param_names".to_string());
    }
    let n_dims = first.surrogate.col_stats.len();
    if trained
        .iter()
        .any(|t| t.surrogate.col_stats.len() != n_dims)
    {
        return Err("trained surrogates have inconsistent dimensions".to_string());
    }

    let entries: Vec<MultiObjectiveEntry<'_>> = trained
        .iter()
        .map(|t| MultiObjectiveEntry {
            surrogate: &t.surrogate,
            x_matrix: &t.x_matrix,
            y: &t.y,
        })
        .collect();

    Ok(run_multi_optimize(
        &entries,
        &spec.minimize,
        spec.slice_params,
        spec.n_grid,
    ))
}

/// Fits multi-objective surrogate models and estimates the Pareto front via
/// NSGA-II.
///
/// Does not depend on a thread-local DataFrame, so it can be called from a
/// background thread.
pub fn run_surrogate_multi_optimization(
    req: &SurrogateMultiOptRequest,
) -> Result<SurrogateMultiOptResult, String> {
    // ── Validation ───────────────────────────────────────────────────
    let n_obj = req.ys.len();
    if n_obj < 2 {
        return Err(format!(
            "At least 2 objectives required (current: {})",
            n_obj
        ));
    }
    if req.objective_names.len() != n_obj {
        return Err("ys and objective_names length mismatch".to_string());
    }
    if req.minimize.len() != n_obj {
        return Err("ys and minimize length mismatch".to_string());
    }

    let n = req.ys[0].len();
    if n < MIN_TRIALS_FOR_SURROGATE_OPT {
        return Err(format!(
            "At least {} trials required (current: {})",
            MIN_TRIALS_FOR_SURROGATE_OPT, n
        ));
    }
    for (k, yk) in req.ys.iter().enumerate() {
        if yk.len() != n {
            return Err(format!(
                "ys[{}] length {} does not match ys[0] length {}",
                k,
                yk.len(),
                n
            ));
        }
    }
    if req.x_matrix.len() != n {
        return Err("x_matrix and y length mismatch".to_string());
    }
    let n_dims = req.x_matrix.first().map(|r| r.len()).unwrap_or(0);
    if n_dims == 0 {
        return Err("No numeric parameters available".to_string());
    }
    if req.x_matrix.iter().any(|row| row.len() != n_dims) {
        return Err("x_matrix rows have inconsistent dimensions".to_string());
    }
    if req.x_matrix.iter().flatten().any(|v| !v.is_finite())
        || req.ys.iter().flatten().any(|v| !v.is_finite())
    {
        return Err("Input contains non-finite values".to_string());
    }

    // ── Fit a surrogate for each objective ──────────────────────────
    let surrogates: Vec<models::FittedSurrogate> = req
        .ys
        .iter()
        .map(|yk| models::fit_surrogate(req.model, &req.x_matrix, yk))
        .collect::<Result<Vec<_>, _>>()?;

    let entries: Vec<MultiObjectiveEntry<'_>> = surrogates
        .iter()
        .zip(req.ys.iter())
        .map(|(surrogate, yk)| MultiObjectiveEntry {
            surrogate,
            x_matrix: &req.x_matrix,
            y: yk,
        })
        .collect();

    Ok(run_multi_optimize(
        &entries,
        &req.minimize,
        req.slice_params,
        req.n_grid,
    ))
}

/// Fits a multi-objective surrogate for each objective (with Pareto-front
/// concentration).
///
/// `objective_values[k]` is the column for objective k (length N); `minimize[k]`
/// is its optimization direction. Reassembles all objectives into row vectors,
/// finds the non-dominated (rank == 0) trials via `nd_sort`, and prioritizes
/// them as inducing points for every GP (`SurrogateFitRequest.priority_rows`).
///
/// Front concentration only changes the model when N exceeds the GP's
/// inducing-point cap (100). For N <= 100 each GP uses Z = X (all points), so
/// the priority setting has no effect on the result.
pub fn fit_multi_surrogates(
    x_matrix: &[Vec<f64>],
    objective_values: &[Vec<f64>],
    param_names: &[String],
    objective_names: &[String],
    model: SurrogateModelKind,
    minimize: &[bool],
) -> Result<Vec<TrainedSurrogate>, String> {
    fit_multi_surrogates_tracked(
        x_matrix,
        objective_values,
        param_names,
        objective_names,
        model,
        minimize,
        None,
        &FitProgress::default(),
    )
}

/// Same as [`fit_multi_surrogates`], but supports progress reporting and
/// cancellation via `progress` (used by background training from the UI).
/// Progress is expressed as the total fit count across every objective, and
/// the label shows the objective name while fitting objective k.
#[allow(clippy::too_many_arguments)]
pub fn fit_multi_surrogates_tracked(
    x_matrix: &[Vec<f64>],
    objective_values: &[Vec<f64>],
    param_names: &[String],
    objective_names: &[String],
    model: SurrogateModelKind,
    minimize: &[bool],
    param_bounds: Option<&[Option<(f64, f64)>]>,
    progress: &FitProgress,
) -> Result<Vec<TrainedSurrogate>, String> {
    let n_obj = objective_values.len();
    if n_obj != objective_names.len() || n_obj != minimize.len() {
        return Err(
            "objective_values, objective_names and minimize must have equal length".to_string(),
        );
    }
    if n_obj == 0 {
        return Err("At least 1 objective required".to_string());
    }
    let n = x_matrix.len();
    for (k, col) in objective_values.iter().enumerate() {
        if col.len() != n {
            return Err(format!(
                "objective_values[{}] length {} does not match x_matrix rows {}",
                k,
                col.len(),
                n
            ));
        }
    }

    // Subsample large data into a single subset shared across all objectives
    // (using a different subset per objective would make the Pareto front
    // inconsistent). After subsampling, each objective's fit already has
    // N <= cap, so it isn't subsampled a second time. Priority rows (rank 0)
    // are also recomputed on the subsampled set.
    let obj_cols: Vec<&[f64]> = objective_values.iter().map(Vec::as_slice).collect();
    let subset = subsample_indices(&obj_cols, minimize, MAX_TRAIN_FOR_FIT, 42);
    let x_subset: Vec<Vec<f64>>;
    let obj_subset: Vec<Vec<f64>>;
    let (x_matrix, objective_values): (&[Vec<f64>], &[Vec<f64>]) = match &subset {
        Some(idx) => {
            x_subset = take_rows(x_matrix, idx);
            obj_subset = objective_values.iter().map(|c| take_rows(c, idx)).collect();
            (&x_subset, &obj_subset)
        }
        None => (x_matrix, objective_values),
    };
    let n = x_matrix.len();

    // Build the per-row objective vector rows[i][k] and make non-dominated
    // trials (rank == 0) the priority rows.
    let rows: Vec<Vec<f64>> = (0..n)
        .map(|i| objective_values.iter().map(|col| col[i]).collect())
        .collect();
    let ranks = crate::multi_objective::pareto::nd_sort(&rows, minimize);
    let priority: Vec<usize> = ranks
        .iter()
        .enumerate()
        .filter(|(_, &r)| r == 0)
        .map(|(i, _)| i)
        .collect();

    // Total progress: each objective fits (1 holdout + k CV) + 1 final model.
    // Each objective's req has auto_select=false and no constraints, so this
    // matches estimate_fit_count.
    let per_obj = (1 + n.min(5)) + 1;
    progress.set_total(n_obj * per_obj);

    let mut trained = Vec::with_capacity(n_obj);
    for k in 0..n_obj {
        let req = SurrogateFitRequest {
            x_matrix: x_matrix.to_vec(),
            y: objective_values[k].clone(),
            param_names: param_names.to_vec(),
            objective_name: objective_names[k].clone(),
            model,
            auto_select: false,
            constraints: vec![],
            priority_rows: priority.clone(),
            param_bounds: param_bounds.map(|b| b.to_vec()),
        };
        // The training data is already subsampled (N <= cap), so call the core
        // directly, skipping subsampling and set_total (each objective's
        // inc_done accumulates on the shared handle).
        let prefix = format!("Objective {}/{} ({}): ", k + 1, n_obj, objective_names[k]);
        let t = fit_validated_inner(&req, progress, &prefix).map_err(|e| {
            format!(
                "Fitting failed for objective '{}': {}",
                objective_names[k], e
            )
        })?;
        trained.push(t);
    }
    Ok(trained)
}

#[cfg(test)]
impl TrainedSurrogate {
    /// For tests: assembles a `TrainedSurrogate` from an analytic mock surrogate.
    ///
    /// An entry point for testing "surface-consuming" logic (optimization,
    /// slicing, multi-objective fronts, acquisition functions, feasibility)
    /// without ever running a GP fit. Pass a known surface built with
    /// [`models::FittedSurrogate::analytic`] as `surrogate`. `x_matrix` / `y`
    /// are used only to compute the optimization start point (observed best);
    /// the surface itself is defined entirely by `surrogate`.
    pub(crate) fn analytic_mock(
        x_matrix: Vec<Vec<f64>>,
        y: Vec<f64>,
        surrogate: models::FittedSurrogate,
    ) -> Self {
        let n_dims = surrogate.col_stats.len();
        TrainedSurrogate {
            surrogate,
            model_kind: SurrogateModelKind::GpFitc,
            param_names: (0..n_dims).map(|d| format!("x{d}")).collect(),
            objective_name: "obj".to_string(),
            x_matrix,
            y,
            validation: SurrogateValidationReport::placeholder(),
            param_importance: None,
            constraint_names: vec![],
            constraint_models: vec![],
            constraint_values: vec![],
            model_selection: None,
        }
    }

    /// Adds one constraint surrogate to an analytic mock (used together with
    /// [`analytic_mock`]). `values` is the constraint value per trial (same row
    /// order as `x_matrix`).
    pub(crate) fn with_analytic_constraint(
        mut self,
        name: &str,
        values: Vec<f64>,
        model: models::FittedSurrogate,
    ) -> Self {
        self.constraint_names.push(name.to_string());
        self.constraint_models.push(model);
        if self.constraint_values.len() != values.len() {
            self.constraint_values = values.iter().map(|&v| vec![v]).collect();
        } else {
            for (row, &v) in self.constraint_values.iter_mut().zip(values.iter()) {
                row.push(v);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests;
