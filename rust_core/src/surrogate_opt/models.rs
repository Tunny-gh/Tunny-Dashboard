//! Training and prediction wrapper for surrogate models.
//!
//! Wraps Ridge (`sensitivity::ridge`), three Gaussian process variants (FITC / VFE /
//! mixture of experts), and LightGBM behind a unified interface. Predictions are made
//! in normalized space (X: min-max [0,1], y: z-score); [`FittedSurrogate`] handles
//! conversion back to original units.

use std::sync::Mutex;

use crate::gaussian_process::{GpMethod, GpModel};
use crate::lgbm::{lgbm_predict, train_lgbm_rf, LgbmBooster, LgbmRfConfig};
use crate::pdp::utils::{normalize_x_minmax, normalize_y, r_squared};
use crate::sensitivity::compute_ridge_from_vecs;

/// Surrogate model kind used to build the response surface.
/// Add new models as variants here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SurrogateModelKind {
    /// Ridge regression (linear). Fast, but the surface is planar.
    Ridge,
    /// Sparse Gaussian process regression via the FITC (Fully Independent Training
    /// Conditional) approximation. Uses M = min(N, 100) inducing points; equivalent
    /// to exact GP when N ≤ 100.
    GpFitc,
    /// Sparse Gaussian process regression via the VFE (Variational Free Energy)
    /// approximation. Tends to estimate noise more conservatively than FITC.
    /// M = min(N, 100).
    GpVfe,
    /// Mixture of experts (smoothly recombines per-cluster FITC GPs).
    /// Suited to discontinuous, multimodal responses. Cluster count is
    /// auto-selected via cross-validation (up to 3).
    GpMoe,
    /// LightGBM (RandomForest mode). Handles nonlinear, non-smooth responses
    /// well, but predictions are piecewise constant, which pairs poorly with
    /// gradient-based methods (L-BFGS).
    Lgbm,
}

/// Closed-form closure type held by the analytic mock surrogate used in tests
/// (shared by mean and variance).
#[cfg(test)]
pub(crate) type AnalyticFn = Box<dyn Fn(&[f64]) -> f64 + Send + Sync>;

/// The trained model itself (predicts in normalized space).
pub(crate) enum FittedModel {
    /// Ridge coefficients for z-score standardized columns (same convention as
    /// `sensitivity::ridge`).
    Ridge {
        beta: Vec<f64>,
        col_mean: Vec<f64>,
        col_std: Vec<f64>,
        y_norm_mean: f64,
    },
    /// Gaussian process via the egobox-gp backend (shared by FITC / VFE / MoE).
    Gp(Box<GpModel>),
    /// LightGBM RandomForest booster.
    /// FittedSurrogate / TrainedSurrogate can be shared across threads via Arc,
    /// but LightGBM's predict is not thread-safe for the same handle, so we
    /// serialize access with a Mutex to satisfy Sync (`LgbmBooster` only
    /// implements Send).
    Lgbm(Mutex<LgbmBooster>),
    /// Test-only: an analytic mock that returns a known closed-form function.
    ///
    /// Lets us inject a response surface behind the same interface instead of
    /// fitting a GP. Because the surface is analytically known, code that
    /// consumes the surface (optimization, acquisition functions, feasibility,
    /// etc.) can be verified exactly rather than with loose tolerances. When
    /// `var` is Some, it behaves like a GP-family model (has posterior
    /// variance); when None, it represents a model without posterior variance,
    /// like Ridge / LightGBM.
    #[cfg(test)]
    Analytic {
        mean: AnalyticFn,
        var: Option<AnalyticFn>,
    },
}

/// A trained surrogate together with its normalization statistics.
pub(crate) struct FittedSurrogate {
    pub(crate) model: FittedModel,
    /// Per-column (min, range) (same as `normalize_x_minmax`).
    pub(crate) col_stats: Vec<(f64, f64)>,
    pub(crate) y_mean: f64,
    pub(crate) y_std: f64,
    /// Coefficient of determination on the training data (evaluated in
    /// original units).
    pub(crate) r_squared: f64,
}

impl FittedSurrogate {
    /// Prediction in normalized space (y in z-score units).
    pub(crate) fn predict_norm(&self, x_norm: &[f64]) -> f64 {
        match &self.model {
            FittedModel::Ridge {
                beta,
                col_mean,
                col_std,
                y_norm_mean,
            } => {
                let mut acc = *y_norm_mean;
                for (d, &b) in beta.iter().enumerate() {
                    acc += b * (x_norm[d] - col_mean[d]) / col_std[d];
                }
                acc
            }
            FittedModel::Gp(model) => model.predict_mean(x_norm),
            FittedModel::Lgbm(booster) => {
                // On a poisoned lock, use the inner value as-is to avoid a panic
                // cascade (safe because Booster's predict doesn't mutate internal
                // state).
                let booster = booster.lock().unwrap_or_else(|e| e.into_inner());
                lgbm_predict(&booster, &[x_norm.to_vec()])
                    .and_then(|preds| preds.first().copied())
                    .unwrap_or(0.0)
            }
            #[cfg(test)]
            FittedModel::Analytic { mean, .. } => mean(x_norm),
        }
    }

    /// Predicted variance in normalized space (only for models with posterior
    /// variance). All three Gaussian process variants (FITC / VFE / MoE) return
    /// Some.
    pub(crate) fn predict_var_norm(&self, x_norm: &[f64]) -> Option<f64> {
        match &self.model {
            FittedModel::Ridge { .. } | FittedModel::Lgbm(_) => None,
            FittedModel::Gp(model) => Some(model.predict_variance(x_norm)),
            #[cfg(test)]
            FittedModel::Analytic { var, .. } => var.as_ref().map(|f| f(x_norm)),
        }
    }

    /// Maps a point in original units into normalized space [0,1]^d.
    pub(crate) fn to_norm_x(&self, x_orig: &[f64]) -> Vec<f64> {
        x_orig
            .iter()
            .zip(self.col_stats.iter())
            .map(|(&v, &(min, range))| ((v - min) / range).clamp(0.0, 1.0))
            .collect()
    }

    /// Maps a point in normalized space back to original units.
    pub(crate) fn to_original_x(&self, x_norm: &[f64]) -> Vec<f64> {
        x_norm
            .iter()
            .zip(self.col_stats.iter())
            .map(|(&t, &(min, range))| min + t * range)
            .collect()
    }

    /// Converts a prediction in z-score units back to original units.
    pub(crate) fn to_original_y(&self, y_norm: f64) -> f64 {
        y_norm * self.y_std + self.y_mean
    }

    /// Relative parameter importance derived from ARD length scales (per input
    /// dimension, summing to 1.0).
    ///
    /// Only returns Some for GP (single SGP). Normalizes each dimension's θ_d
    /// by dividing by the sum of θ (per the egobox / SMT convention, larger θ_d
    /// means greater sensitivity to dimension d). Returns None if the sum is
    /// ≤ 0 or any value is non-finite. Returns None for MoE because θ isn't
    /// unique, and for Ridge / LightGBM because they have no ARD. The ordering
    /// matches the input column order used during training (i.e. the column
    /// order of `param_names` / `x_matrix`).
    pub(crate) fn param_importance(&self) -> Option<Vec<f64>> {
        let theta = match &self.model {
            FittedModel::Gp(model) => model.ard_theta()?,
            FittedModel::Ridge { .. } | FittedModel::Lgbm(_) => return None,
            #[cfg(test)]
            FittedModel::Analytic { .. } => return None,
        };
        if theta.is_empty() || theta.iter().any(|t| !t.is_finite()) {
            return None;
        }
        let sum: f64 = theta.iter().sum();
        // theta's finiteness was already checked above, so sum is finite too.
        // Can't normalize unless it's positive.
        if sum <= 0.0 {
            return None;
        }
        Some(theta.iter().map(|t| t / sum).collect())
    }
}

/// Normalizes each column to [0,1]. Columns where `bounds[d] = Some((lo, hi))`
/// (finite, lo<hi) are normalized against that declared range; other columns
/// use the observed min/max. Using the declared range makes the optimization
/// search box (normalized space [0,1]^d) match the true variable range derived
/// from the log, allowing exploration outside the observed data (unobserved
/// but valid regions), while the clamp in `to_original_x` still keeps values
/// from leaving that range.
fn normalize_x_box(
    x_matrix: &[Vec<f64>],
    bounds: Option<&[Option<(f64, f64)>]>,
) -> (Vec<(f64, f64)>, Vec<Vec<f64>>) {
    let (observed_stats, _) = normalize_x_minmax(x_matrix);
    let col_stats: Vec<(f64, f64)> = observed_stats
        .iter()
        .enumerate()
        .map(
            |(d, &obs)| match bounds.and_then(|b| b.get(d)).copied().flatten() {
                Some((lo, hi)) if lo.is_finite() && hi.is_finite() && hi > lo => {
                    (lo, (hi - lo).max(f64::EPSILON))
                }
                _ => obs,
            },
        )
        .collect();
    let x_norm = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(d, &v)| {
                    let (min, range) = col_stats[d];
                    (v - min) / range
                })
                .collect()
        })
        .collect();
    (col_stats, x_norm)
}

/// Trains a surrogate with the given model kind.
pub(crate) fn fit_surrogate(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
) -> Result<FittedSurrogate, String> {
    // Delegates to the no-priority-rows, observed-range normalization (legacy behavior).
    fit_surrogate_with_priority(kind, x_matrix, y, &[])
}

/// Same as `fit_surrogate`, but for GP-family models, trains with `priority`
/// (row indices to prioritize as inducing points) concentrated on e.g. the
/// Pareto front. Only has an effect when N exceeds the GP's inducing point
/// cap. Ridge / LightGBM ignore `priority`.
pub(crate) fn fit_surrogate_with_priority(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    priority: &[usize],
) -> Result<FittedSurrogate, String> {
    fit_surrogate_with_priority_bounds(kind, x_matrix, y, priority, None)
}

/// Same as [`fit_surrogate_with_priority`], but lets you specify each column's
/// declared range via `bounds`. Columns with a range are normalized against it
/// (so the search box matches the true variable range); columns without one
/// fall back to the observed range.
pub(crate) fn fit_surrogate_with_priority_bounds(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    priority: &[usize],
    bounds: Option<&[Option<(f64, f64)>]>,
) -> Result<FittedSurrogate, String> {
    let (col_stats, x_norm) = normalize_x_box(x_matrix, bounds);
    let (y_mean, y_std, y_norm) = normalize_y(y);

    let model = match kind {
        SurrogateModelKind::Ridge => fit_ridge(&x_norm, &y_norm)?,
        SurrogateModelKind::GpFitc => FittedModel::Gp(Box::new(
            GpModel::fit_front_focused(&x_norm, &y_norm, GpMethod::Fitc, 100, 42, priority)
                .ok_or("GP-FITC training failed")?,
        )),
        SurrogateModelKind::GpVfe => FittedModel::Gp(Box::new(
            GpModel::fit_front_focused(&x_norm, &y_norm, GpMethod::Vfe, 100, 42, priority)
                .ok_or("GP-VFE training failed")?,
        )),
        SurrogateModelKind::GpMoe => FittedModel::Gp(Box::new(
            GpModel::fit_front_focused(&x_norm, &y_norm, GpMethod::Moe, 100, 42, priority)
                .ok_or("GP-MOE training failed")?,
        )),
        SurrogateModelKind::Lgbm => FittedModel::Lgbm(Mutex::new(
            train_lgbm_rf(&x_norm, &y_norm, &LgbmRfConfig::default())
                .ok_or("LightGBM training failed")?,
        )),
    };

    let mut surrogate = FittedSurrogate {
        model,
        col_stats,
        y_mean,
        y_std,
        r_squared: 0.0,
    };
    let y_pred: Vec<f64> = x_norm
        .iter()
        .map(|row| surrogate.to_original_y(surrogate.predict_norm(row)))
        .collect();
    surrogate.r_squared = r_squared(y, &y_pred);
    Ok(surrogate)
}

/// Trains a constraint surrogate. Uses the same `kind` as the objective by
/// default, but falls back to Ridge if GP-family training fails.
///
/// For a perfectly linear, noise-free constraint (e.g. `c = 0.5 - x`), GP
/// hyperparameter optimization can degenerate (optimal lengthscale → ∞) and
/// training can fail. Falling back to Ridge for just that constraint (though
/// the feasibility probability becomes a hard indicator) lets the feature keep
/// working overall, while other constraints retain GP's smooth P(c ≤ 0).
pub(crate) fn fit_constraint_surrogate(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    values: &[f64],
) -> Result<FittedSurrogate, String> {
    fit_constraint_surrogate_bounds(kind, x_matrix, values, None)
}

/// Same as [`fit_constraint_surrogate`], but lets you specify each column's
/// declared range via `bounds`. Because the constraint surrogate is evaluated
/// in the same normalized space as the objective surrogate during
/// optimization, you must pass the same `bounds` as the objective to keep the
/// normalization box consistent.
pub(crate) fn fit_constraint_surrogate_bounds(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    values: &[f64],
    bounds: Option<&[Option<(f64, f64)>]>,
) -> Result<FittedSurrogate, String> {
    match fit_surrogate_with_priority_bounds(kind, x_matrix, values, &[], bounds) {
        Ok(m) => Ok(m),
        Err(e) if kind != SurrogateModelKind::Ridge => fit_surrogate_with_priority_bounds(
            SurrogateModelKind::Ridge,
            x_matrix,
            values,
            &[],
            bounds,
        )
        .map_err(|ridge_err| {
            format!("{kind:?} failed ({e}); Ridge fallback also failed ({ridge_err})")
        }),
        Err(e) => Err(e),
    }
}

fn fit_ridge(x_norm: &[Vec<f64>], y_norm: &[f64]) -> Result<FittedModel, String> {
    let ridge = compute_ridge_from_vecs(x_norm, y_norm, 1.0);
    if ridge.beta.is_empty() {
        return Err("Ridge training failed".to_string());
    }
    let n = y_norm.len() as f64;
    let n_dims = x_norm[0].len();
    let col_mean: Vec<f64> = (0..n_dims)
        .map(|d| x_norm.iter().map(|r| r[d]).sum::<f64>() / n)
        .collect();
    let col_std: Vec<f64> = (0..n_dims)
        .map(|d| {
            let var = x_norm
                .iter()
                .map(|r| (r[d] - col_mean[d]).powi(2))
                .sum::<f64>()
                / n;
            var.sqrt().max(f64::EPSILON)
        })
        .collect();
    let y_norm_mean = y_norm.iter().sum::<f64>() / n;
    Ok(FittedModel::Ridge {
        beta: ridge.beta,
        col_mean,
        col_std,
        y_norm_mean,
    })
}

#[cfg(test)]
impl FittedSurrogate {
    /// Builds an analytic mock surrogate for tests.
    ///
    /// Fixes normalization to the identity (`col_stats = (0, 1)`, `y_mean = 0`,
    /// `y_std = 1`), so normalized space [0,1]^d coincides with the original
    /// unit space. The `mean` / `var` closures' outputs are therefore directly
    /// the predicted mean and variance in original units, letting a known
    /// closed-form response surface be injected without a GP fit. When `var`
    /// is `Some`, it behaves like a GP-family model (has posterior variance);
    /// when `None`, it behaves like a model without posterior variance, as
    /// Ridge / LightGBM do.
    pub(crate) fn analytic(
        n_dims: usize,
        mean: impl Fn(&[f64]) -> f64 + Send + Sync + 'static,
        var: Option<AnalyticFn>,
    ) -> Self {
        FittedSurrogate {
            model: FittedModel::Analytic {
                mean: Box::new(mean),
                var,
            },
            col_stats: vec![(0.0, 1.0); n_dims],
            y_mean: 0.0,
            y_std: 1.0,
            r_squared: 1.0,
        }
    }
}
