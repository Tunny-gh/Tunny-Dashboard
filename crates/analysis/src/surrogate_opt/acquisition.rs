//! Next-candidate proposal from a surrogate model via acquisition functions.
//!
//! Uses the posterior mean and variance of a Gaussian Process (GP) surrogate to
//! propose a candidate point equivalent to one step of Bayesian optimization.
//! Batch candidates are generated with the Constant Liar strategy.
//!
//! All computation is done in normalized space [0,1]^d with a z-scored objective;
//! results are converted back to original units before being returned.

use super::feasibility::feasibility_probability;
use super::models::{fit_constraint_surrogate, fit_surrogate, FittedSurrogate, SurrogateModelKind};
use super::optimizers::minimize_scalar_fn;
use super::{best_observed_index, TrainedSurrogate};

/// Acquisition function kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AcquisitionKind {
    /// Expected Improvement.
    ExpectedImprovement,
    /// Lower Confidence Bound.
    LowerConfidenceBound,
}

/// EI exploration offset (z-score units).
const XI: f64 = 0.01;
/// LCB exploration coefficient κ.
const KAPPA: f64 = 2.0;
/// Penalty weight for constrained LCB (z-score units).
const CONSTRAINT_LCB_PENALTY: f64 = 10.0;

/// A single candidate point proposed by optimizing the acquisition function.
/// Parameter and predicted values are in original units.
#[derive(Debug, Clone)]
pub struct SuggestedCandidate {
    /// Parameter values (original units, same order as `param_names`).
    pub params: Vec<f64>,
    /// Surrogate-predicted value (original units).
    pub predicted_value: f64,
    /// Predicted standard deviation (Some only for GP-family models; already
    /// scaled to original units).
    pub predicted_std: Option<f64>,
    /// Acquisition score (maximization direction; higher values are more promising).
    pub acq_score: f64,
    /// Predicted constraint values (original units, same order as
    /// `constraint_names`). Empty when there are no constraints.
    pub predicted_constraints: Vec<f64>,
    /// Feasibility probability (0.0 to 1.0). `None` when there are no constraints.
    pub feasibility_probability: Option<f64>,
}

/// CDF Φ(z) of the standard normal distribution.
///
/// Uses the erf approximation from Abramowitz & Stegun formula 7.1.26.
/// Error is |ε| < 1.5 × 10⁻⁷.
pub(crate) fn normal_cdf(z: f64) -> f64 {
    // erf(x) ≈ 1 - (a1·t + a2·t² + a3·t³ + a4·t⁴ + a5·t⁵)·exp(-x²)  (t = 1/(1+0.3275911·x))
    // Φ(z) = 0.5 · (1 + erf(z / √2))
    let x = z / std::f64::consts::SQRT_2;
    let sign = if x < 0.0 { -1.0f64 } else { 1.0f64 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t
        * (0.254829592
            + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let erf_abs = 1.0 - poly * (-x * x).exp();
    0.5 * (1.0 + sign * erf_abs)
}

/// PDF φ(z) of the standard normal distribution.
fn normal_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Computes EI (Expected Improvement) in normalized space. Maximization direction.
///
/// `f_best`: best z-score value in the training data (the minimum if minimizing,
/// or the minimum of the sign-flipped values if maximizing).
/// `mu`: surrogate posterior mean (z-score).
/// `sigma`: posterior standard deviation (z-score).
/// `minimize`: true for a minimization problem.
fn ei_norm(f_best: f64, mu: f64, sigma: f64) -> f64 {
    // Treated as minimization (f_best already has the minimize/maximize sign conversion applied).
    if sigma < 1e-12 {
        return (f_best - mu - XI).max(0.0);
    }
    let i = f_best - mu - XI;
    let z = i / sigma;
    i * normal_cdf(z) + sigma * normal_pdf(z)
}

/// Computes LCB in normalized space. Minimization score (lower is more promising).
///
/// `mu` / `sigma` are in z-score units.
fn lcb_norm(mu: f64, sigma: f64) -> f64 {
    mu - KAPPA * sigma
}

/// Gets the incumbent (best value in z-score space).
///
/// If constraints are present, only feasible trials are considered (if every
/// trial is infeasible, computed over the full set instead).
/// For minimization this is the z-score minimum; for maximization, the
/// sign-flipped z-score maximum (= −max). Internally everything is always
/// treated as a "minimization" problem, so maximize uses the negative sign.
fn incumbent(
    surrogate: &FittedSurrogate,
    y: &[f64],
    minimize: bool,
    constraint_values: &[Vec<f64>],
) -> f64 {
    // y is in original units, so convert it to z-score.
    let y_norm: Vec<f64> = y
        .iter()
        .map(|&v| (v - surrogate.y_mean) / surrogate.y_std)
        .collect();

    // Indices of feasible trials: all constraint values ≤ 0.
    let feasible_indices: Vec<usize> = if constraint_values.is_empty() {
        (0..y_norm.len()).collect()
    } else {
        (0..y_norm.len())
            .filter(|&i| {
                constraint_values
                    .get(i)
                    .is_none_or(|cv| cv.iter().all(|&c| c <= 0.0))
            })
            .collect()
    };

    // Select from feasible trials if any exist, otherwise from the full set.
    let indices = if feasible_indices.is_empty() {
        (0..y_norm.len()).collect::<Vec<_>>()
    } else {
        feasible_indices
    };

    if minimize {
        indices
            .iter()
            .map(|&i| y_norm[i])
            .fold(f64::INFINITY, f64::min)
    } else {
        // maximize → sign-flip and treat as a minimization problem.
        indices
            .iter()
            .map(|&i| -y_norm[i])
            .fold(f64::INFINITY, f64::min)
    }
}

/// Evaluates the acquisition cost (minimization direction) at a single point.
///
/// - Constrained EI: `-EI(x) · P_feas(x)`
/// - Constrained LCB: `LCB(x) + CONSTRAINT_LCB_PENALTY · (1 − P_feas(x))`
///
/// If `c_models` is empty, P_feas is treated as 1 (no constraints).
fn acquisition_cost(
    surrogate: &FittedSurrogate,
    c_models: &[FittedSurrogate],
    acquisition: AcquisitionKind,
    minimize: bool,
    f_best: f64,
    x_norm: &[f64],
) -> f64 {
    let mu = if minimize {
        surrogate.predict_norm(x_norm)
    } else {
        -surrogate.predict_norm(x_norm)
    };
    let sigma = surrogate
        .predict_var_norm(x_norm)
        .map(|v| v.max(0.0).sqrt())
        .unwrap_or(0.0);
    let p = if c_models.is_empty() {
        1.0
    } else {
        feasibility_probability(c_models, x_norm)
    };
    match acquisition {
        AcquisitionKind::ExpectedImprovement => -ei_norm(f_best, mu, sigma) * p,
        AcquisitionKind::LowerConfidenceBound => {
            lcb_norm(mu, sigma) + CONSTRAINT_LCB_PENALTY * (1.0 - p)
        }
    }
}

/// Reconstructs the row-major constraint value matrix from `work_c` (columns
/// per constraint), including any Constant Liar appended rows.
fn constraint_rows(work_c: &[Vec<f64>], n_constraints: usize, n_rows: usize) -> Vec<Vec<f64>> {
    (0..n_rows)
        .map(|row| {
            (0..n_constraints)
                .map(|ci| {
                    work_c
                        .get(ci)
                        .and_then(|col| col.get(row))
                        .copied()
                        .unwrap_or(0.0)
                })
                .collect()
        })
        .collect()
}

/// Searches for and finalizes the candidate point (in normalized space) that
/// maximizes the acquisition function.
///
/// If it duplicates a previous candidate (normalized L2² < 1e-12), retries the
/// search exactly once from a random start seeded with `retry_seed`.
#[allow(clippy::too_many_arguments)]
fn locate_candidate_norm(
    surrogate: &FittedSurrogate,
    c_models: &[FittedSurrogate],
    acquisition: AcquisitionKind,
    minimize: bool,
    f_best: f64,
    start_norm: &[f64],
    prev_candidates: &[SuggestedCandidate],
    retry_seed: u64,
) -> Vec<f64> {
    let n_dims = start_norm.len();
    // Acquisition function (minimization direction).
    let eval_acq = |x_norm: &[f64]| -> f64 {
        acquisition_cost(surrogate, c_models, acquisition, minimize, f_best, x_norm)
    };

    let cand = minimize_scalar_fn(&eval_acq, n_dims, start_norm);

    // Duplicate guard: retry if L2 distance from a previous candidate is ≤ 1e-6.
    let is_dup = prev_candidates.iter().any(|prev| {
        let prev_norm = surrogate.to_norm_x(&prev.params);
        let dist2: f64 = cand
            .iter()
            .zip(prev_norm.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum();
        dist2 < 1e-12
    });
    if is_dup {
        // Retry: re-search from a random start with a different seed.
        let mut rng = crate::math::rng::SeededRng::from_seed(retry_seed);
        let alt_start: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
        minimize_scalar_fn(&eval_acq, n_dims, &alt_start)
    } else {
        cand
    }
}

/// Converts a finalized candidate point (normalized space) into a
/// [`SuggestedCandidate`] in original units.
fn describe_candidate(
    surrogate: &FittedSurrogate,
    c_models: &[FittedSurrogate],
    acquisition: AcquisitionKind,
    minimize: bool,
    f_best: f64,
    best_norm: &[f64],
) -> SuggestedCandidate {
    let params = surrogate.to_original_x(best_norm);
    let predicted_value = surrogate.to_original_y(surrogate.predict_norm(best_norm));
    let predicted_std = surrogate
        .predict_var_norm(best_norm)
        .map(|v| v.max(0.0).sqrt() * surrogate.y_std);

    // Acquisition score is in the maximization direction (sign-flipped minimization cost).
    let acq_score = -acquisition_cost(
        surrogate,
        c_models,
        acquisition,
        minimize,
        f_best,
        best_norm,
    );

    // Compute predicted constraint values and feasibility probability.
    let (predicted_constraints, p_feas) = if c_models.is_empty() {
        (vec![], None)
    } else {
        let preds: Vec<f64> = c_models
            .iter()
            .map(|cm| cm.to_original_y(cm.predict_norm(best_norm)))
            .collect();
        (preds, Some(feasibility_probability(c_models, best_norm)))
    };

    SuggestedCandidate {
        params,
        predicted_value,
        predicted_std,
        acq_score,
        predicted_constraints,
        feasibility_probability: p_feas,
    }
}

/// Refits the objective surrogate and all constraint surrogates on the Constant
/// Liar working data.
///
/// Returns `None` if any refit fails (the caller then stops the batch early and
/// returns the candidates gathered so far).
fn refit_constant_liar(
    model_kind: SurrogateModelKind,
    work_x: &[Vec<f64>],
    work_y: &[f64],
    work_c: &[Vec<f64>],
) -> Option<(FittedSurrogate, Vec<FittedSurrogate>)> {
    let surrogate = fit_surrogate(model_kind, work_x, work_y).ok()?;
    let mut c_models = Vec::with_capacity(work_c.len());
    for col in work_c {
        c_models.push(fit_constraint_surrogate(model_kind, work_x, col).ok()?);
    }
    Some((surrogate, c_models))
}

/// Proposes candidate points for the next trial from a trained surrogate.
///
/// - `trained`: validated training result (GP-family models only).
/// - `n_candidates`: number of candidates to propose (≥ 1).
/// - `acquisition`: acquisition function to use.
/// - `minimize`: true = minimization problem, false = maximization problem.
///
/// Batches (n > 1) use the Constant Liar strategy: after each candidate is
/// added, a "lie" observation (the best observed value) is appended and the GP
/// is refit before searching for the next candidate.
/// Constraint models are refit at the same time, appending the candidate's
/// predicted constraint mean as the lie value.
pub fn suggest_candidates(
    trained: &TrainedSurrogate,
    n_candidates: usize,
    acquisition: AcquisitionKind,
    minimize: bool,
) -> Result<Vec<SuggestedCandidate>, String> {
    if n_candidates == 0 {
        return Err("n_candidates must be ≥ 1".to_string());
    }

    // Check whether this is a GP-family model (posterior variance is required).
    let probe = trained
        .x_matrix
        .first()
        .map(|row| trained.surrogate.to_norm_x(row));
    let has_variance = probe
        .as_deref()
        .and_then(|xn| trained.surrogate.predict_var_norm(xn))
        .is_some();
    if !has_variance {
        return Err(
            "acquisition requires a Gaussian Process model (GP-FITC, GP-VFE, or GP-MOE)"
                .to_string(),
        );
    }

    let mut candidates: Vec<SuggestedCandidate> = Vec::with_capacity(n_candidates);

    // Working copy for Constant Liar.
    let mut work_x = trained.x_matrix.clone();
    let mut work_y = trained.y.clone();
    // Working copy of constraints for Constant Liar (one column per constraint).
    let mut work_c: Vec<Vec<f64>> = trained
        .constraint_models
        .iter()
        .enumerate()
        .map(|(ci, _)| {
            trained
                .constraint_values
                .iter()
                .map(|row| row.get(ci).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    // The Constant Liar "lie" is the current best observed value (minimum for
    // minimize, maximum for maximize).
    let lie_y = {
        let best_idx = best_observed_index(&trained.y, minimize);
        trained.y[best_idx]
    };

    // Vec that owns the surrogate used at each iteration.
    // The first element is a placeholder (i=0 uses trained.surrogate directly).
    // For i >= 1, refitted[i-1] is referenced instead.
    let mut refitted: Vec<FittedSurrogate> = Vec::new();
    // Refitted constraint models (constraint × iteration).
    // refitted_constraints[i-1][ci] is the surrogate for constraint ci used at iteration i.
    let mut refitted_constraints: Vec<Vec<FittedSurrogate>> = Vec::new();

    for i in 0..n_candidates {
        // Get references to the surrogate/constraint surrogates used for this iteration.
        let surrogate: &FittedSurrogate = if i == 0 {
            &trained.surrogate
        } else {
            &refitted[i - 1]
        };
        let c_models: &[FittedSurrogate] = if i == 0 {
            &trained.constraint_models
        } else {
            &refitted_constraints[i - 1]
        };

        // Reconstruct the constraint value matrix from work_y and work_c
        // (including Constant Liar appended rows).
        let work_constraint_values = constraint_rows(&work_c, c_models.len(), work_y.len());
        let f_best = incumbent(surrogate, &work_y, minimize, &work_constraint_values);

        // Starting point: current best observed point (normalized space).
        let best_idx = best_observed_index(&work_y, minimize);
        let start_norm = surrogate.to_norm_x(&work_x[best_idx]);

        // Optimize the acquisition function to finalize the candidate point.
        let best_norm = locate_candidate_norm(
            surrogate,
            c_models,
            acquisition,
            minimize,
            f_best,
            &start_norm,
            &candidates,
            42 + i as u64 + 1,
        );

        // Convert the candidate to original units and record it.
        let candidate = describe_candidate(
            surrogate,
            c_models,
            acquisition,
            minimize,
            f_best,
            &best_norm,
        );
        candidates.push(candidate);

        // Constant Liar: append to the working data and refit for the next candidate.
        if i + 1 < n_candidates {
            let last = &candidates[i];
            work_x.push(last.params.clone());
            work_y.push(lie_y);
            // Constraint lie value: use the candidate's predicted constraint mean.
            let lie_constraints = last.predicted_constraints.clone();
            for (ci, col) in work_c.iter_mut().enumerate() {
                col.push(lie_constraints.get(ci).copied().unwrap_or(0.0));
            }
            match refit_constant_liar(trained.model_kind, &work_x, &work_y, &work_c) {
                Some((new_surrogate, new_c_models)) => {
                    refitted.push(new_surrogate);
                    refitted_constraints.push(new_c_models);
                }
                // Refit failed → return the candidates gathered so far as Ok.
                None => return Ok(candidates),
            }
        }
    }

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::rng::SeededRng;
    use crate::surrogate_opt::models::SurrogateModelKind;
    use crate::surrogate_opt::{fit_surrogate_with_validation, SurrogateFitRequest};

    // ── Unit tests for normal_cdf ─────────────────────────────────

    #[test]
    fn normal_cdf_at_zero() {
        let v = normal_cdf(0.0);
        assert!((v - 0.5).abs() < 1e-4, "Φ(0) = 0.5, got {v}");
    }

    #[test]
    fn normal_cdf_at_1_96() {
        let v = normal_cdf(1.96);
        assert!((v - 0.975).abs() < 1e-4, "Φ(1.96) ≈ 0.975, got {v}");
    }

    #[test]
    fn normal_cdf_negative_1() {
        let v = normal_cdf(-1.0);
        assert!((v - 0.1587).abs() < 1e-4, "Φ(-1) ≈ 0.1587, got {v}");
    }

    // ── Property tests for EI ────────────────────────────────────────

    #[test]
    fn ei_zero_when_sigma_tiny_and_mu_above_fbest() {
        // σ → 0, μ > f_best → EI ≈ 0.
        let f_best = 0.0;
        let mu = 1.0; // μ > f_best, so no improvement
        let sigma = 1e-15;
        let ei = ei_norm(f_best, mu, sigma);
        assert!(
            ei < 1e-6,
            "EI should be near 0 when σ→0 and μ > f_best, got {ei}"
        );
    }

    #[test]
    fn ei_grows_with_sigma_at_fixed_mu() {
        // Larger σ gives larger EI.
        let f_best = 0.0;
        let mu = 0.0;
        let ei_small = ei_norm(f_best, mu, 0.01);
        let ei_large = ei_norm(f_best, mu, 1.0);
        assert!(
            ei_large > ei_small,
            "EI should grow with σ: ei(σ=0.01)={ei_small}, ei(σ=1.0)={ei_large}"
        );
    }

    // ── Candidate-proposal tests on an analytic mock (no refit: n=1) ─────────
    // Injects a known convex quadratic surface f(x,y) = (x−0.3)² + (y−0.7)² with
    // constant variance σ²=0.05. This verifies acquisition-function optimization
    // instantly and deterministically without a GP fit, and since the surface is
    // known, we can also confirm the proposed point lands near the true minimum.

    fn quad2(x: &[f64]) -> f64 {
        (x[0] - 0.3).powi(2) + (x[1] - 0.7).powi(2)
    }

    /// Analytic mock TrainedSurrogate for the quadratic surface. `with_variance=false`
    /// represents a model with no posterior variance (i.e., a non-GP model).
    fn analytic_quadratic_mock(with_variance: bool) -> TrainedSurrogate {
        let var: Option<crate::surrogate_opt::models::AnalyticFn> = if with_variance {
            Some(Box::new(|_x: &[f64]| 0.05))
        } else {
            None
        };
        let surrogate = FittedSurrogate::analytic(2, quad2, var);
        let x_matrix = vec![
            vec![0.2, 0.8],
            vec![0.9, 0.05],
            vec![0.5, 0.5],
            vec![0.1, 0.9],
        ];
        let y: Vec<f64> = x_matrix.iter().map(|r| quad2(r)).collect();
        TrainedSurrogate::analytic_mock(x_matrix, y, surrogate)
    }

    /// A trained result with an actually-fit GP-FITC for batch tests (batch tests
    /// involving Constant Liar refits can't be mocked, so a real fit is used).
    fn quadratic_trained_fitc(n: usize) -> TrainedSurrogate {
        let mut rng = SeededRng::from_seed(7);
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|_| vec![rng.next_f64(), rng.next_f64()])
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|r| quad2(r)).collect();
        fit_surrogate_with_validation(&SurrogateFitRequest {
            x_matrix,
            y,
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_name: "obj".to_string(),
            model: SurrogateModelKind::GpFitc,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
        })
        .expect("fit should succeed")
    }

    #[test]
    fn single_ei_candidate_targets_known_minimum_with_std() {
        let trained = analytic_quadratic_mock(true);
        let candidates =
            suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, true)
                .expect("suggest should succeed");
        assert_eq!(candidates.len(), 1);
        let c = &candidates[0];
        // Proposed parameters should be in [0,1] in original units.
        assert!(
            c.params.iter().all(|&v| (0.0..=1.0).contains(&v)),
            "params out of [0,1]: {:?}",
            c.params
        );
        // With constant variance, EI is maximized where the predicted mean is
        // minimal → targets the true minimum (0.3, 0.7).
        assert!(
            (c.params[0] - 0.3).abs() < 0.1 && (c.params[1] - 0.7).abs() < 0.1,
            "EI with constant σ should target the minimum (0.3, 0.7), got {:?}",
            c.params
        );
        // acq_score >= 0 (EI is non-negative).
        assert!(c.acq_score >= 0.0, "EI should be ≥ 0, got {}", c.acq_score);
        // Mock has posterior variance, so predicted_std is Some (√0.05).
        assert!(c.predicted_std.is_some(), "GP-like mock has predicted_std");
        assert!(
            (c.predicted_std.unwrap() - 0.05_f64.sqrt()).abs() < 1e-9,
            "std should equal √0.05, got {}",
            c.predicted_std.unwrap()
        );
    }

    #[test]
    fn batch_3_candidates_pairwise_diverse() {
        let trained = quadratic_trained_fitc(40);
        let candidates =
            suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
                .expect("batch suggest should succeed");
        assert_eq!(candidates.len(), 3);

        // Pairwise normalized L2 distance > 1e-4.
        let surrogate = &trained.surrogate;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let ni = surrogate.to_norm_x(&candidates[i].params);
                let nj = surrogate.to_norm_x(&candidates[j].params);
                let dist: f64 = ni
                    .iter()
                    .zip(nj.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(
                    dist > 1e-4,
                    "candidates {i} and {j} too close (dist={dist:.2e})"
                );
            }
        }
    }

    #[test]
    fn batch_3_deterministic_across_two_runs() {
        let trained = quadratic_trained_fitc(40);
        let c1 = suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
            .expect("first run");
        let c2 = suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
            .expect("second run");
        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            for (pa, pb) in a.params.iter().zip(b.params.iter()) {
                assert!(
                    (pa - pb).abs() < 1e-9,
                    "results differ between runs: {pa} vs {pb}"
                );
            }
        }
    }

    #[test]
    fn maximize_steers_away_from_minimum() {
        // With maximize=true (n=1, no refit), it should move toward the
        // quadratic's maximum (the farthest corner).
        let trained = analytic_quadratic_mock(true);
        let y_median = {
            let mut ys = trained.y.clone();
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys[ys.len() / 2]
        };
        let candidates =
            suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, false)
                .expect("maximize suggest");
        // Predicted value should be at or above the median (moving in the
        // maximization direction).
        assert!(
            candidates[0].predicted_value >= y_median - 0.1,
            "maximize should steer toward higher values, got {}",
            candidates[0].predicted_value
        );
    }

    #[test]
    fn non_gp_model_returns_error() {
        // A model without posterior variance (e.g., Ridge / LightGBM) can't use
        // the acquisition function and should return an error requiring GP.
        let trained = analytic_quadratic_mock(false);
        let err = suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, true)
            .unwrap_err();
        assert!(
            err.contains("Gaussian Process"),
            "expected GP error, got: {err}"
        );
    }
}
