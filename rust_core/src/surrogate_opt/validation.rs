//! Holdout + k-fold CV validation for surrogate models.

use crate::pdp::utils::r_squared;
use rayon::prelude::*;

use super::models::{fit_surrogate, SurrogateModelKind};
use super::progress::FitProgress;
use crate::math::rng::SeededRng;

/// Result of a single CV fold (an intermediate form for aggregating in fold
/// order after parallel evaluation).
struct FoldOutcome {
    /// OOF (actual, predicted) pairs (this fold's validation points, in
    /// `cv_val_indices` order).
    oof: Vec<(f64, f64)>,
    /// Whether each OOF point belongs to the Pareto front (rank 0) trial.
    is_front: Vec<bool>,
    /// This fold's validation RMSE (computed even for degenerate folds).
    rmse: f64,
    /// This fold's validation R² (`None` for degenerate folds with < 2 points
    /// or zero variance).
    r2: Option<f64>,
}

/// Validation report for a surrogate model via holdout + k-fold CV.
#[derive(Debug, Clone)]
pub struct SurrogateValidationReport {
    pub n_samples: usize,
    /// Number of training samples in the holdout split (80% of the total).
    pub n_train: usize,
    /// Number of test samples in the holdout split (20% of the total).
    pub n_test: usize,
    /// Training R² of the final model trained on all data (original units).
    pub train_r2: f64,
    /// 80:20 holdout: R² on the remaining 20%, trained on the other 80%.
    pub holdout_r2: f64,
    /// RMSE on the same test data (original units).
    pub holdout_rmse: f64,
    /// Number of CV folds (can be fewer than 5 when data is scarce).
    pub cv_folds: usize,
    /// Mean and standard deviation of per-fold validation R² (population std).
    pub cv_r2_mean: f64,
    pub cv_r2_std: f64,
    /// Mean and standard deviation of per-fold validation RMSE.
    pub cv_rmse_mean: f64,
    pub cv_rmse_std: f64,
    /// Out-of-fold (actual, predicted) pairs (original units, for predicted
    /// vs. actual plots).
    pub oof_pairs: Vec<(f64, f64)>,
    /// Same order as `oof_pairs`; whether each point belongs to the Pareto
    /// front (multi-objective rank 0) trial. Non-empty only for
    /// multi-objective fits (empty for single-objective fits or validation
    /// under Auto selection). Used to color-code fit quality near the front
    /// in scatter plots.
    pub oof_is_front: Vec<bool>,
    /// OOF R² computed using only Pareto front points (None if fewer than 2
    /// front points or zero variance).
    pub front_r2: Option<f64>,
    /// OOF RMSE computed using only Pareto front points (None if there are no
    /// front points).
    pub front_rmse: Option<f64>,
}

#[cfg(test)]
impl SurrogateValidationReport {
    /// A placeholder validation report for tests, to attach to the analytic
    /// mock surrogate. No validation is performed (since the surface is
    /// known), so the R² fields are set to 1.0 to represent a perfect fit.
    pub(crate) fn placeholder() -> Self {
        SurrogateValidationReport {
            n_samples: 0,
            n_train: 0,
            n_test: 0,
            train_r2: 1.0,
            holdout_r2: 1.0,
            holdout_rmse: 0.0,
            cv_folds: 0,
            cv_r2_mean: 1.0,
            cv_r2_std: 0.0,
            cv_rmse_mean: 0.0,
            cv_rmse_std: 0.0,
            oof_pairs: vec![],
            oof_is_front: vec![],
            front_r2: None,
            front_rmse: None,
        }
    }
}

/// Computes RMSE (original units).
fn rmse(actual: &[f64], pred: &[f64]) -> f64 {
    let n = actual.len();
    if n == 0 {
        return 0.0;
    }
    let mse = actual
        .iter()
        .zip(pred.iter())
        .map(|(&a, &p)| (a - p).powi(2))
        .sum::<f64>()
        / n as f64;
    mse.sqrt()
}

/// Computes the population standard deviation.
fn population_std(values: &[f64]) -> f64 {
    let n = values.len();
    if n == 0 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let var = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    var.sqrt()
}

/// Runs holdout + k-fold CV for the given model kind and returns a validation
/// report.
///
/// - Shuffling is done deterministically with a ChaCha8 RNG seeded by `seed`.
/// - Holdout: n_test = max(1, round(n × 0.2)) points are used for the test set.
/// - k-fold CV: k = min(5, n). Fold assignment is round-robin after shuffling.
///   Degenerate folds (< 2 points or zero variance) are excluded from the R²
///   mean/std but are still included in the OOF pairs and RMSE.
///
/// `train_r2` is returned as 0.0 here, since the caller
/// (`fit_surrogate_with_validation`) overwrites it with the value from the
/// full-data model.
#[cfg(test)]
pub(crate) fn validate_surrogate(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
) -> Result<SurrogateValidationReport, String> {
    validate_surrogate_tracked(kind, x_matrix, y, seed, &FitProgress::default())
}

/// Same as [`validate_surrogate`], but updates `progress` at the boundary of
/// each model training step, returning `Err` early if cancellation is
/// requested. Calls [`FitProgress::inc_done`] once per training run (1 holdout
/// + k CV runs). The caller sets the stage label.
pub(crate) fn validate_surrogate_tracked(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
    progress: &FitProgress,
) -> Result<SurrogateValidationReport, String> {
    validate_surrogate_tracked_front(kind, x_matrix, y, seed, &[], progress)
}

/// Same as [`validate_surrogate_tracked`], but takes `front_rows` (Pareto
/// front = rank 0 row indices, indexing into `x_matrix`), records whether each
/// OOF point is on the front, and also computes R²/RMSE for front points
/// only. Used to show fit quality near the front for multi-objective fits.
pub(crate) fn validate_surrogate_tracked_front(
    kind: SurrogateModelKind,
    x_matrix: &[Vec<f64>],
    y: &[f64],
    seed: u64,
    front_rows: &[usize],
    progress: &FitProgress,
) -> Result<SurrogateValidationReport, String> {
    use std::collections::HashSet;
    let front_set: HashSet<usize> = front_rows.iter().copied().collect();
    let n = y.len();

    // Generate shuffled indices.
    let mut indices: Vec<usize> = (0..n).collect();
    let mut rng = SeededRng::from_seed(seed);
    rng.shuffle(&mut indices);

    // ---- Holdout ----
    let n_test = ((n as f64 * 0.2).round() as usize).max(1);
    let n_train = n - n_test;

    let train_indices: Vec<usize> = indices[..n_train].to_vec();
    let test_indices: Vec<usize> = indices[n_train..].to_vec();

    let train_x: Vec<Vec<f64>> = train_indices.iter().map(|&i| x_matrix[i].clone()).collect();
    let train_y: Vec<f64> = train_indices.iter().map(|&i| y[i]).collect();
    let test_x: Vec<Vec<f64>> = test_indices.iter().map(|&i| x_matrix[i].clone()).collect();
    let test_y: Vec<f64> = test_indices.iter().map(|&i| y[i]).collect();

    progress.check()?;
    let holdout_model = fit_surrogate(kind, &train_x, &train_y)
        .map_err(|e| format!("ホールドアウト訓練失敗: {e}"))?;
    progress.inc_done();

    let holdout_pred: Vec<f64> = test_x
        .iter()
        .map(|row| {
            let x_norm = holdout_model.to_norm_x(row);
            holdout_model.to_original_y(holdout_model.predict_norm(&x_norm))
        })
        .collect();

    let holdout_r2 = r_squared(&test_y, &holdout_pred);
    let holdout_rmse = rmse(&test_y, &holdout_pred);

    // ---- k-fold CV ----
    let k = n.min(5);

    // Assign shuffled indices to k folds via round-robin.
    let mut fold_indices: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (pos, &idx) in indices.iter().enumerate() {
        fold_indices[pos % k].push(idx);
    }

    let mut oof_pairs: Vec<(f64, f64)> = Vec::with_capacity(n);
    let mut oof_is_front: Vec<bool> = Vec::with_capacity(n);
    let mut cv_r2_values: Vec<f64> = Vec::with_capacity(k);
    let mut cv_rmse_values: Vec<f64> = Vec::with_capacity(k);

    // Each fold's training is independent and shares no RNG, so we parallelize
    // with rayon. Results are aggregated in fold order, so the OOF pair/score
    // order matches sequential execution (preserving reproducibility for a
    // fixed seed). `progress`'s inc_done is atomic and order-independent.
    let fold_outcomes: Vec<FoldOutcome> = (0..k)
        .into_par_iter()
        .map(|fold| -> Result<FoldOutcome, String> {
            // Use every fold except this one for training.
            let cv_train_indices: Vec<usize> = (0..k)
                .filter(|&f| f != fold)
                .flat_map(|f| fold_indices[f].iter().copied())
                .collect();
            let cv_val_indices: &[usize] = &fold_indices[fold];

            let cv_train_x: Vec<Vec<f64>> = cv_train_indices
                .iter()
                .map(|&i| x_matrix[i].clone())
                .collect();
            let cv_train_y: Vec<f64> = cv_train_indices.iter().map(|&i| y[i]).collect();
            let cv_val_x: Vec<Vec<f64>> = cv_val_indices
                .iter()
                .map(|&i| x_matrix[i].clone())
                .collect();
            let cv_val_y: Vec<f64> = cv_val_indices.iter().map(|&i| y[i]).collect();

            progress.check()?;
            let cv_model = fit_surrogate(kind, &cv_train_x, &cv_train_y)
                .map_err(|e| format!("CV fold {fold} 訓練失敗: {e}"))?;
            progress.inc_done();

            let cv_pred: Vec<f64> = cv_val_x
                .iter()
                .map(|row| {
                    let x_norm = cv_model.to_norm_x(row);
                    cv_model.to_original_y(cv_model.predict_norm(&x_norm))
                })
                .collect();

            // Collect OOF pairs (also record front membership by original row index).
            let mut oof = Vec::with_capacity(cv_val_indices.len());
            let mut is_front = Vec::with_capacity(cv_val_indices.len());
            for ((&idx, &actual), &predicted) in cv_val_indices
                .iter()
                .zip(cv_val_y.iter())
                .zip(cv_pred.iter())
            {
                oof.push((actual, predicted));
                is_front.push(front_set.contains(&idx));
            }

            // Fold RMSE (included even for degenerate folds).
            let rmse_fold = rmse(&cv_val_y, &cv_pred);

            // Exclude degenerate folds (< 2 points or zero variance) from R².
            let r2_fold = if cv_val_y.len() < 2 {
                None
            } else {
                let y_mean = cv_val_y.iter().sum::<f64>() / cv_val_y.len() as f64;
                let ss_tot: f64 = cv_val_y.iter().map(|&v| (v - y_mean).powi(2)).sum();
                if ss_tot < f64::EPSILON {
                    None
                } else {
                    Some(r_squared(&cv_val_y, &cv_pred))
                }
            };

            Ok(FoldOutcome {
                oof,
                is_front,
                rmse: rmse_fold,
                r2: r2_fold,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Aggregate in fold order (same ordering as sequential execution).
    for outcome in fold_outcomes {
        oof_pairs.extend(outcome.oof);
        oof_is_front.extend(outcome.is_front);
        cv_rmse_values.push(outcome.rmse);
        if let Some(r2) = outcome.r2 {
            cv_r2_values.push(r2);
        }
    }

    // Mean and standard deviation of CV R² (valid folds only).
    let cv_r2_mean = if cv_r2_values.is_empty() {
        0.0
    } else {
        cv_r2_values.iter().sum::<f64>() / cv_r2_values.len() as f64
    };
    let cv_r2_std = population_std(&cv_r2_values);

    // Mean and standard deviation of CV RMSE (all folds).
    let cv_rmse_mean = if cv_rmse_values.is_empty() {
        0.0
    } else {
        cv_rmse_values.iter().sum::<f64>() / cv_rmse_values.len() as f64
    };
    let cv_rmse_std = population_std(&cv_rmse_values);

    // OOF R²/RMSE using only Pareto front points (fit quality near the front).
    let front_actual: Vec<f64> = oof_pairs
        .iter()
        .zip(oof_is_front.iter())
        .filter(|(_, &f)| f)
        .map(|(&(a, _), _)| a)
        .collect();
    let front_pred: Vec<f64> = oof_pairs
        .iter()
        .zip(oof_is_front.iter())
        .filter(|(_, &f)| f)
        .map(|(&(_, p), _)| p)
        .collect();
    let front_rmse = if front_actual.is_empty() {
        None
    } else {
        Some(rmse(&front_actual, &front_pred))
    };
    let front_r2 = if front_actual.len() < 2 {
        None
    } else {
        let mean = front_actual.iter().sum::<f64>() / front_actual.len() as f64;
        let ss_tot: f64 = front_actual.iter().map(|&v| (v - mean).powi(2)).sum();
        if ss_tot < f64::EPSILON {
            None
        } else {
            Some(r_squared(&front_actual, &front_pred))
        }
    };

    Ok(SurrogateValidationReport {
        n_samples: n,
        n_train,
        n_test,
        train_r2: 0.0, // Overwritten by the caller with the full-data model's value.
        holdout_r2,
        holdout_rmse,
        cv_folds: k,
        cv_r2_mean,
        cv_r2_std,
        cv_rmse_mean,
        cv_rmse_std,
        oof_pairs,
        oof_is_front,
        front_r2,
        front_rmse,
    })
}
