//! Automatic model selection (Auto): cross-validates the candidate models in
//! [`super::AUTO_CANDIDATES`] and prefers the earliest within 0.01 of the best finite CV R².

use super::progress::{FitProgress, FIT_CANCELLED};
use super::validation::validate_surrogate_tracked;
use super::{validate_inputs, SurrogateModelKind, AUTO_CANDIDATES};

/// Result of automatic model selection (Auto). Holds the chosen model and the
/// per-candidate CV R².
#[derive(Debug, Clone)]
pub struct ModelSelectionReport {
    /// The earliest evaluated candidate within 0.01 of the highest finite CV R²
    /// in the evaluated set.
    pub chosen: SurrogateModelKind,
    /// Per-candidate (model kind, score = cv_r2_mean), in the same order as
    /// the evaluated candidate set. General Auto uses `AUTO_CANDIDATES`; adaptive
    /// EI/EHVI uses only GP-FITC and GP-VFE. A failed fit/validation is recorded as
    /// f64::NEG_INFINITY. All non-finite scores are excluded from selection.
    pub scores: Vec<(SurrogateModelKind, f64)>,
}

/// Cross-validates `AUTO_CANDIDATES` and selects the earliest near-best model.
///
/// Runs validation for each candidate and uses `cv_r2_mean` as its score.
/// First finds the maximum finite score, then selects the earliest candidate in
/// `AUTO_CANDIDATES` with an absolute R² gap <= 0.01 from that maximum. The order
/// is an explicit preference policy, not a universal cost ranking.
/// Returns `Err` for invalid inputs, cancellation, or no finite candidate score.
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
pub(crate) fn select_best_model_tracked(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
    progress: &FitProgress,
    stage_prefix: &str,
) -> Result<ModelSelectionReport, String> {
    select_model_candidates_tracked(x_matrix, y, seed, progress, stage_prefix, &AUTO_CANDIDATES)
}

/// Internal eligible-subset selection; eligibility is decided before validation
/// and final fitting, and the report contains only evaluated candidates.
pub(super) fn select_model_candidates_tracked(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
    progress: &FitProgress,
    stage_prefix: &str,
    candidates: &[SurrogateModelKind],
) -> Result<ModelSelectionReport, String> {
    validate_inputs(x_matrix, y)?;

    let mut scores: Vec<(SurrogateModelKind, f64)> = Vec::with_capacity(candidates.len());
    for (i, &kind) in candidates.iter().enumerate() {
        progress.check()?;
        progress.set_stage(format!(
            "{stage_prefix}Evaluating candidate {} ({}/{})",
            model_display_name(kind),
            i + 1,
            candidates.len()
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

    let chosen = choose_model(&scores)?;

    Ok(ModelSelectionReport { chosen, scores })
}

/// Scores must be in candidate preference order. Compare against the global
/// maximum, never against a previously preferred candidate's score.
pub(super) fn choose_model(
    scores: &[(SurrogateModelKind, f64)],
) -> Result<SurrogateModelKind, String> {
    const TIE_TOLERANCE: f64 = 0.01;
    let best = scores
        .iter()
        .map(|&(_, score)| score)
        .filter(|score| score.is_finite())
        .reduce(f64::max);
    best.and_then(|best| {
        scores
            .iter()
            .find(|&&(_, score)| score.is_finite() && best - score <= TIE_TOLERANCE)
            .map(|&(kind, _)| kind)
    })
    .ok_or_else(|| "All candidate models failed validation".to_string())
}

/// Display name of a surrogate model kind (for progress labels).
pub(crate) fn model_display_name(kind: SurrogateModelKind) -> &'static str {
    match kind {
        SurrogateModelKind::Ridge => "Ridge",
        SurrogateModelKind::GpFitc => "GP-FITC",
        SurrogateModelKind::GpVfe => "GP-VFE",
        SurrogateModelKind::GpMoe => "GP-MOE",
        SurrogateModelKind::Lgbm => "LightGBM",
    }
}
