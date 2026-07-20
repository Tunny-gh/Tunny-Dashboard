//! Automatic model selection (Auto): cross-validates the candidate models in
//! [`super::AUTO_CANDIDATES`] and picks the one with the highest CV R².

use super::progress::{FitProgress, FIT_CANCELLED};
use super::validation::validate_surrogate_tracked;
use super::{validate_inputs, SurrogateModelKind, AUTO_CANDIDATES};

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
pub(crate) fn select_best_model_tracked(
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
