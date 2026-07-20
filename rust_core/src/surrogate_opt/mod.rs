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
mod fit;
mod model_selection;
mod models;
mod multi;
// Exposed within the crate because the gh runner (crate::gh::runner) repurposes nsga2
// for real objective-function evaluation.
pub(crate) mod optimizers;
pub(crate) mod progress;
mod robustness;
mod single;
mod slice;
mod types;
pub(crate) mod validation;

pub use acquisition::{suggest_candidates, AcquisitionKind, SuggestedCandidate};
pub use ard::{compute_ard_importance_from_df, ArdImportanceResult};
pub use ehvi::{suggest_candidates_multi, MultiSuggestedCandidate};
pub use models::SurrogateModelKind;
pub use optimizers::OptimizerKind;
pub use progress::{FitProgress, FitProgressSnapshot};
pub use robustness::{robustness_analysis, NoiseDistribution, RobustnessResult, RobustnessSpec};
pub use validation::SurrogateValidationReport;

// ── model_selection.rs ──────────────────────────────────────────────────
pub use model_selection::{select_best_model, ModelSelectionReport};

// ── fit.rs ───────────────────────────────────────────────────────────────
pub use fit::{fit_surrogate_with_validation, fit_surrogate_with_validation_tracked};
pub(crate) use fit::{fit_validated_inner, subsample_indices, take_rows, validate_inputs};

// ── single.rs ────────────────────────────────────────────────────────────
pub use single::{optimize_on_trained, run_surrogate_optimization};

// ── slice.rs ─────────────────────────────────────────────────────────────
pub(crate) use slice::best_observed_index;
pub use slice::{line_slice_at, surface_slice_at, LineSlice};

// ── multi.rs ─────────────────────────────────────────────────────────────
pub use multi::{
    fit_multi_surrogates, fit_multi_surrogates_tracked, optimize_multi_on_trained,
    run_surrogate_multi_optimization,
};

// ── types.rs ─────────────────────────────────────────────────────────────
pub use types::{
    ConstraintData, ParetoFrontPoint, SurfaceSlice, SurrogateFitRequest, SurrogateMultiOptRequest,
    SurrogateMultiOptResult, SurrogateMultiOptimizeSpec, SurrogateOptRequest, SurrogateOptResult,
    SurrogateOptimizeSpec, TrainedSurrogate,
};

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

/// Default resolution of the slice grid.
pub const DEFAULT_SLICE_GRID: usize = 20;

#[cfg(test)]
mod tests;
