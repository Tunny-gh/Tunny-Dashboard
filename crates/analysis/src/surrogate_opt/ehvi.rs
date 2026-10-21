//! Next-candidate proposal from a multi-objective surrogate using Expected
//! Hypervolume Improvement (EHVI).
//!
//! This is the multi-objective analog of the single-objective acquisition
//! function (`acquisition.rs`). Each objective has its own independent GP
//! surrogate; from their posterior means and variances we Monte Carlo
//! estimate the expected hypervolume improvement over the observed Pareto
//! front and maximize it.
//!
//! ## z-score minimization frame
//!
//! All EHVI computation is done in a z-score-normalized minimization frame.
//! For objective k, define the normalized objective
//! `g_k(x) = sign_k * predict_norm_k(x)`, where `sign_k = +1` (minimize) /
//! `-1` (maximize), so smaller g is always better. The posterior standard
//! deviation is `s_k(x) = sqrt(predict_var_norm_k(x))` (the sign does not
//! affect std).
//!
//! ## Determinism (common random numbers)
//!
//! For each call to `suggest_candidates_multi`, an `S × n_obj` standard
//! normal matrix is drawn **exactly once** from a fixed-seed RNG and reused
//! across all x evaluations (Common Random Numbers). This makes MC-EHVI a
//! deterministic, smooth function of x, so L-BFGS's numerical gradient
//! works correctly. It is not redrawn per evaluation.
//!
//! ## Reference point
//!
//! From the observed front P, per dimension:
//! `r_k = max_{p∈P} g_k(p) + REF_MARGIN` (nadir + margin, in z-score units).

use super::models::{fit_surrogate, FittedSurrogate};
use super::optimizers::minimize_scalar_fn;
use super::TrainedSurrogate;
use crate::math::rng::SeededRng;
use crate::multi_objective::pareto::{dominates_minimized, hypervolume_nd};

/// Margin above the nadir for the reference point (in z-score units).
const REF_MARGIN: f64 = 0.1;
/// Number of MC-EHVI samples (number of rows in the common random matrix).
const EHVI_SAMPLES: usize = 128;
/// Fixed seed for the common random matrix.
const EHVI_SEED: u64 = 42;

/// A single candidate point proposed by EHVI optimization. Parameter values
/// and predicted values are in original units.
#[derive(Debug, Clone)]
pub struct MultiSuggestedCandidate {
    /// Parameter values (original units, same order as `param_names`).
    pub params: Vec<f64>,
    /// Surrogate-predicted values per objective (original units, same order as `objective_names`).
    pub predicted_values: Vec<f64>,
    /// Predicted standard deviation per objective (`Some` only for GP-family models, original units).
    pub predicted_stds: Vec<Option<f64>>,
    /// EHVI score (maximization direction; larger is more promising).
    pub ehvi_score: f64,
}

/// Working context for EHVI computation (for a single iteration).
struct EhviContext<'a> {
    /// Surrogate per objective (provides `predict_norm` / `predict_var_norm`).
    surrogates: Vec<&'a FittedSurrogate>,
    /// Sign per objective (minimize = +1.0, maximize = -1.0).
    signs: Vec<f64>,
    /// Observed Pareto front P (z-score minimization frame, non-dominated points only).
    front: Vec<Vec<f64>>,
    /// Reference point r (z-score minimization frame).
    ref_point: Vec<f64>,
    /// HV(P) (precomputed, fixed for a given P).
    hv_p: f64,
    /// Fixed S×n_obj standard normal matrix (common random numbers).
    z_matrix: &'a [Vec<f64>],
}

impl EhviContext<'_> {
    /// Computes MC-EHVI at point x in normalized space (maximization direction, ≥ 0).
    fn ehvi(&self, x_norm: &[f64]) -> f64 {
        let n_obj = self.surrogates.len();
        // Precompute g_k(x), s_k(x).
        let g: Vec<f64> = (0..n_obj)
            .map(|k| self.signs[k] * self.surrogates[k].predict_norm(x_norm))
            .collect();
        let s: Vec<f64> = (0..n_obj)
            .map(|k| {
                self.surrogates[k]
                    .predict_var_norm(x_norm)
                    .map(|v| v.max(0.0).sqrt())
                    .unwrap_or(0.0)
            })
            .collect();

        let s_samples = self.z_matrix.len();
        if s_samples == 0 {
            return 0.0;
        }

        // For each sample, build v_s[k] = g_k + s_k * Z[s][k] and accumulate HV(P ∪ {v_s}) − HV(P).
        // The front is cloned only once outside the loop; only the trailing placeholder element
        // is overwritten per sample (zero allocation per sample).
        let mut acc = 0.0;
        let mut augmented: Vec<Vec<f64>> = Vec::with_capacity(self.front.len() + 1);
        augmented.extend_from_slice(&self.front);
        augmented.push(vec![0.0; n_obj]);
        let last = augmented.len() - 1;
        for z_row in self.z_matrix {
            let slot = &mut augmented[last];
            for k in 0..n_obj {
                slot[k] = g[k] + s[k] * z_row[k];
            }
            let hv_aug = hypervolume_nd(&augmented, &self.ref_point);
            let improvement = hv_aug - self.hv_p;
            if improvement > 0.0 {
                acc += improvement;
            }
        }
        acc / s_samples as f64
    }
}

/// Builds the Pareto front P in the z-score minimization frame from
/// observations (raw y per objective).
///
/// Converts each trial via `sign_k * (y_k - y_mean_k) / y_std_k` and, under
/// the minimization convention, keeps only non-dominated points (points that
/// are not dominated).
fn build_observed_front(
    surrogates: &[&FittedSurrogate],
    ys: &[&[f64]],
    signs: &[f64],
) -> Vec<Vec<f64>> {
    let n_obj = surrogates.len();
    let n = ys.first().map(|y| y.len()).unwrap_or(0);

    // Convert to the z-score minimization frame.
    let points: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n_obj)
                .map(|k| {
                    let s = surrogates[k];
                    let z = if s.y_std > 1e-12 {
                        (ys[k][i] - s.y_mean) / s.y_std
                    } else {
                        0.0
                    };
                    signs[k] * z
                })
                .collect()
        })
        .collect();

    // Extract non-dominated points (minimization: a is dominated by b ⟺ a ≥ b in every dimension and a > b in at least one).
    let mut front: Vec<Vec<f64>> = Vec::new();
    for p in &points {
        let dominated = points.iter().any(|q| dominates_minimized(q, p));
        if !dominated {
            // Remove duplicate points.
            let dup = front
                .iter()
                .any(|f| f.iter().zip(p.iter()).all(|(a, b)| (a - b).abs() < 1e-12));
            if !dup {
                front.push(p.clone());
            }
        }
    }
    front
}

/// Computes the reference point r: per dimension, `max_{p∈P} g_k(p) + REF_MARGIN`.
fn compute_ref_point(front: &[Vec<f64>], n_obj: usize) -> Vec<f64> {
    (0..n_obj)
        .map(|k| {
            let nadir = front.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
            // Fallback for an empty front (callers already exclude this case, but stay safe).
            let base = if nadir.is_finite() { nadir } else { 0.0 };
            base + REF_MARGIN
        })
        .collect()
}

/// Draws an S×n_obj standard normal matrix from a fixed-seed RNG.
///
/// Standard normal samples reuse the shared `SeededRng::next_gaussian` (Box-Muller).
fn draw_standard_normal_matrix(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    let mut rng = SeededRng::from_seed(EHVI_SEED);
    (0..rows)
        .map(|_| (0..cols).map(|_| rng.next_gaussian()).collect())
        .collect()
}

/// Computes the mean of observed parameters (original units) and returns it
/// as the starting point in normalized space.
///
/// Since we cannot find the observed trial corresponding to a front point,
/// we use the mean of all observed parameters (original units) as a robust
/// starting point (per the spec: "a simple robust start is the mean of
/// observed param rows").
fn mean_param_start(surrogate: &FittedSurrogate, x_matrix: &[Vec<f64>], n_dims: usize) -> Vec<f64> {
    if x_matrix.is_empty() {
        return vec![0.5; n_dims];
    }
    let n = x_matrix.len() as f64;
    let mean: Vec<f64> = (0..n_dims)
        .map(|d| {
            x_matrix
                .iter()
                .map(|r| r.get(d).copied().unwrap_or(0.0))
                .sum::<f64>()
                / n
        })
        .collect();
    surrogate.to_norm_x(&mean)
}

/// Proposes the next trial's candidate points via EHVI from a set of trained surrogates.
///
/// - `trained[k]`: surrogate for objective k (all objectives must be GP-family).
/// - `minimize[k]`: true = minimize objective k.
/// - `n_candidates`: number of candidate points to propose (≥ 1).
///
/// For batches (n > 1), uses the Constant Liar strategy: after selecting
/// each candidate, its parameters and per-objective predicted mean (raw
/// units) are appended to each objective's (x, y) working copy, each
/// objective's surrogate is refit, P and r are recomputed, and the next
/// candidate is searched for.
pub fn suggest_candidates_multi(
    trained: &[TrainedSurrogate],
    minimize: &[bool],
    n_candidates: usize,
) -> Result<Vec<MultiSuggestedCandidate>, String> {
    if trained.is_empty() {
        return Err("EHVI requires at least one objective surrogate".to_string());
    }
    if trained.len() != minimize.len() {
        return Err("trained and minimize length mismatch".to_string());
    }
    if n_candidates == 0 {
        return Err("n_candidates must be ≥ 1".to_string());
    }

    let n_obj = trained.len();
    let n_dims = trained[0].surrogate.col_stats.len();
    // Confirm that all surrogates share the same dimensionality (same normalization transform).
    if trained
        .iter()
        .any(|t| t.surrogate.col_stats.len() != n_dims)
    {
        return Err("trained surrogates have inconsistent dimensions".to_string());
    }

    // Confirm each objective is GP-family (has posterior variance).
    for t in trained {
        let probe = t.x_matrix.first().map(|row| t.surrogate.to_norm_x(row));
        let has_var = probe
            .as_deref()
            .and_then(|xn| t.surrogate.predict_var_norm(xn))
            .is_some();
        if !has_var {
            return Err("EHVI requires Gaussian Process models for all objectives".to_string());
        }
    }

    let signs: Vec<f64> = minimize
        .iter()
        .map(|&m| if m { 1.0 } else { -1.0 })
        .collect();

    // Draw the common random matrix exactly once (for determinism and smoothness).
    let z_matrix = draw_standard_normal_matrix(EHVI_SAMPLES, n_obj);

    let model_kinds: Vec<_> = trained.iter().map(|t| t.model_kind).collect();
    // Working copies for Constant Liar (per-objective x, y). x is shared across all objectives.
    let mut work_x = trained[0].x_matrix.clone();
    let mut work_ys: Vec<Vec<f64>> = trained.iter().map(|t| t.y.clone()).collect();

    // Refitted surrogates used on each iteration (i=0 uses `trained` directly).
    let mut refitted: Vec<Vec<FittedSurrogate>> = Vec::new();

    let mut candidates: Vec<MultiSuggestedCandidate> = Vec::with_capacity(n_candidates);

    for i in 0..n_candidates {
        // Obtain the surrogate references to use for this iteration.
        let surrogates: Vec<&FittedSurrogate> = if i == 0 {
            trained.iter().map(|t| &t.surrogate).collect()
        } else {
            refitted[i - 1].iter().collect()
        };
        let ref_surrogate = surrogates[0];

        // Recompute the observed front P and reference point r (from the working data).
        let ys_refs: Vec<&[f64]> = work_ys.iter().map(|y| y.as_slice()).collect();
        let front = build_observed_front(&surrogates, &ys_refs, &signs);
        let ref_point = compute_ref_point(&front, n_obj);
        let hv_p = hypervolume_nd(&front, &ref_point);

        let ctx = EhviContext {
            surrogates: surrogates.clone(),
            signs: signs.clone(),
            front,
            ref_point,
            hv_p,
            z_matrix: &z_matrix,
        };

        // Starting point: mean of observed parameters (normalized space).
        let start_norm = mean_param_start(ref_surrogate, &work_x, n_dims);

        // Maximize EHVI (equivalent to minimizing -ehvi).
        let neg_ehvi = |x: &[f64]| -ctx.ehvi(x);
        let mut best_norm = minimize_scalar_fn(&neg_ehvi, n_dims, &start_norm);

        // Duplicate guard: retry with a different seed if the normalized L2 distance from the previous candidate is under 1e-6.
        let is_dup = candidates.iter().any(|prev| {
            let prev_norm = ref_surrogate.to_norm_x(&prev.params);
            let dist2: f64 = best_norm
                .iter()
                .zip(prev_norm.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            dist2 < 1e-12
        });
        if is_dup {
            let mut rng = SeededRng::from_seed(EHVI_SEED + i as u64 + 1);
            let alt_start: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
            best_norm = minimize_scalar_fn(&neg_ehvi, n_dims, &alt_start);
        }

        // Convert the candidate to original units.
        let params = ref_surrogate.to_original_x(&best_norm);
        let predicted_values: Vec<f64> = surrogates
            .iter()
            .map(|s| s.to_original_y(s.predict_norm(&best_norm)))
            .collect();
        let predicted_stds: Vec<Option<f64>> = surrogates
            .iter()
            .map(|s| {
                s.predict_var_norm(&best_norm)
                    .map(|v| v.max(0.0).sqrt() * s.y_std)
            })
            .collect();
        let ehvi_score = ctx.ehvi(&best_norm);

        candidates.push(MultiSuggestedCandidate {
            params: params.clone(),
            predicted_values: predicted_values.clone(),
            predicted_stds,
            ehvi_score,
        });

        // Constant Liar: append to the working data and refit each objective for the next candidate.
        if i + 1 < n_candidates {
            work_x.push(params);
            for (k, yk) in work_ys.iter_mut().enumerate() {
                yk.push(predicted_values[k]);
            }
            let mut new_surrogates = Vec::with_capacity(n_obj);
            let mut refit_ok = true;
            for (k, yk) in work_ys.iter().enumerate() {
                match fit_surrogate(model_kinds[k], &work_x, yk) {
                    Ok(s) => new_surrogates.push(s),
                    Err(_) => {
                        refit_ok = false;
                        break;
                    }
                }
            }
            if refit_ok {
                refitted.push(new_surrogates);
            } else {
                // Refit failed → return the candidates gathered so far as Ok (≥ 1 item).
                return Ok(candidates);
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

    /// Generates data for a two-objective conflicting problem.
    /// f1 = x0² + x1² (minimize), f2 = (x0−1)² + (x1−1)² (minimize).
    fn conflicting_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<f64>) {
        let mut rng = SeededRng::from_seed(42);
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|_| vec![rng.next_f64(), rng.next_f64()])
            .collect();
        let f1: Vec<f64> = x_matrix
            .iter()
            .map(|r| r[0].powi(2) + r[1].powi(2))
            .collect();
        let f2: Vec<f64> = x_matrix
            .iter()
            .map(|r| (r[0] - 1.0).powi(2) + (r[1] - 1.0).powi(2))
            .collect();
        (x_matrix, f1, f2)
    }

    // ── Verifying EHVI processing with an analytic mock (no refit: n=1) ──────
    // Injects the two-objective conflicting surfaces f1 = x0²+x1², f2 = (x0−1)²+(x1−1)²
    // with a constant variance σ²=0.05, and verifies EHVI optimization, determinism,
    // and reference-point/front construction without any GP fitting.

    fn ehvi_f1(x: &[f64]) -> f64 {
        x[0].powi(2) + x[1].powi(2)
    }
    fn ehvi_f2(x: &[f64]) -> f64 {
        (x[0] - 1.0).powi(2) + (x[1] - 1.0).powi(2)
    }

    /// A pair of two-objective analytic mock `TrainedSurrogate`s. With
    /// `with_variance=false`, there is no posterior variance.
    fn analytic_two_objectives(with_variance: bool) -> Vec<TrainedSurrogate> {
        let x_matrix = vec![
            vec![0.1, 0.1],
            vec![0.9, 0.9],
            vec![0.5, 0.5],
            vec![0.2, 0.8],
            vec![0.8, 0.2],
        ];
        let mk = |surface: fn(&[f64]) -> f64| {
            let var: Option<crate::surrogate_opt::models::AnalyticFn> = if with_variance {
                Some(Box::new(|_x: &[f64]| 0.05))
            } else {
                None
            };
            let s = FittedSurrogate::analytic(2, surface, var);
            let y: Vec<f64> = x_matrix.iter().map(|r| surface(r)).collect();
            TrainedSurrogate::analytic_mock(x_matrix.clone(), y, s)
        };
        vec![mk(ehvi_f1), mk(ehvi_f2)]
    }

    /// Trains a pair of two-objective GP-FITC surrogates (for batch tests
    /// involving Constant Liar refitting; n=1 tests use the analytic mock).
    fn fit_two_objectives(
        x_matrix: Vec<Vec<f64>>,
        f1: Vec<f64>,
        f2: Vec<f64>,
    ) -> Vec<TrainedSurrogate> {
        let names = vec!["x0".to_string(), "x1".to_string()];
        let t1 = fit_surrogate_with_validation(&SurrogateFitRequest {
            x_matrix: x_matrix.clone(),
            y: f1,
            param_names: names.clone(),
            objective_name: "f1".to_string(),
            model: SurrogateModelKind::GpFitc,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
        })
        .expect("fit f1 should succeed");
        let t2 = fit_surrogate_with_validation(&SurrogateFitRequest {
            x_matrix,
            y: f2,
            param_names: names,
            objective_name: "f2".to_string(),
            model: SurrogateModelKind::GpFitc,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
        })
        .expect("fit f2 should succeed");
        vec![t1, t2]
    }

    #[test]
    fn single_candidate_in_box_with_stds() {
        let trained = analytic_two_objectives(true);
        let candidates = suggest_candidates_multi(&trained, &[true, true], 1)
            .expect("EHVI suggest should succeed");

        assert_eq!(candidates.len(), 1);
        let c = &candidates[0];
        assert!(
            c.params.iter().all(|&v| (0.0..=1.0).contains(&v)),
            "params out of [0,1]: {:?}",
            c.params
        );
        assert!(
            c.ehvi_score >= 0.0,
            "EHVI should be ≥ 0, got {}",
            c.ehvi_score
        );
        assert_eq!(c.predicted_values.len(), 2);
        assert_eq!(c.predicted_stds.len(), 2);
        // Since the mock has constant posterior variance, every std should be Some(√0.05).
        assert!(c
            .predicted_stds
            .iter()
            .all(|s| s.is_some_and(|v| (v - 0.05_f64.sqrt()).abs() < 1e-9)));
        // Since the surface is known, predicted values should match f(params) exactly.
        assert!((c.predicted_values[0] - ehvi_f1(&c.params)).abs() < 1e-9);
        assert!((c.predicted_values[1] - ehvi_f2(&c.params)).abs() < 1e-9);
    }

    #[test]
    fn deterministic_across_two_runs() {
        let trained = analytic_two_objectives(true);
        let c1 = suggest_candidates_multi(&trained, &[true, true], 1).expect("run 1");
        let c2 = suggest_candidates_multi(&trained, &[true, true], 1).expect("run 2");
        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            for (pa, pb) in a.params.iter().zip(b.params.iter()) {
                assert!(
                    (pa - pb).abs() < 1e-9,
                    "EHVI suggest must be deterministic: {pa} vs {pb}"
                );
            }
            assert!((a.ehvi_score - b.ehvi_score).abs() < 1e-9);
        }
    }

    #[test]
    fn batch_3_candidates_pairwise_diverse() {
        let (x, f1, f2) = conflicting_samples(40);
        let trained = fit_two_objectives(x, f1, f2);
        let candidates = suggest_candidates_multi(&trained, &[true, true], 3)
            .expect("batch EHVI suggest should succeed");
        assert_eq!(candidates.len(), 3);

        let ref_s = &trained[0].surrogate;
        for i in 0..3 {
            for j in (i + 1)..3 {
                let ni = ref_s.to_norm_x(&candidates[i].params);
                let nj = ref_s.to_norm_x(&candidates[j].params);
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
    fn suggested_beats_worst_observed_point() {
        // The suggested candidate's EHVI should be at least the EHVI of the
        // worst observed point (the point closest to the nadir in the
        // z-score minimization frame) — a sanity check that the optimizer is
        // actually improving.
        let trained = analytic_two_objectives(true);
        let candidates = suggest_candidates_multi(&trained, &[true, true], 1).expect("suggest");
        let suggested = &candidates[0];

        // Rebuild the same context to measure the EHVI of the worst observed point.
        let surrogates: Vec<&FittedSurrogate> = trained.iter().map(|t| &t.surrogate).collect();
        let signs = vec![1.0, 1.0];
        let ys_refs: Vec<&[f64]> = trained.iter().map(|t| t.y.as_slice()).collect();
        let front = build_observed_front(&surrogates, &ys_refs, &signs);
        let ref_point = compute_ref_point(&front, 2);
        let hv_p = hypervolume_nd(&front, &ref_point);
        let z_matrix = draw_standard_normal_matrix(EHVI_SAMPLES, 2);
        let ctx = EhviContext {
            surrogates: surrogates.clone(),
            signs,
            front,
            ref_point,
            hv_p,
            z_matrix: &z_matrix,
        };

        // Worst observed point: the normalized parameters of the trial with the largest first objective (worst under the minimization frame).
        let worst_idx = (0..trained[0].y.len())
            .max_by(|&a, &b| {
                trained[0].y[a]
                    .partial_cmp(&trained[0].y[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        let worst_norm = trained[0]
            .surrogate
            .to_norm_x(&trained[0].x_matrix[worst_idx]);
        let worst_ehvi = ctx.ehvi(&worst_norm);

        assert!(
            suggested.ehvi_score >= worst_ehvi - 1e-9,
            "suggested EHVI {} should be >= worst-point EHVI {}",
            suggested.ehvi_score,
            worst_ehvi
        );
    }

    #[test]
    fn ehvi_matches_naive_clone_reference() {
        // Confirms that the zero-allocation in-place implementation matches
        // the naive reference implementation, which clones the front on
        // every sample, exactly (bit-for-bit) — numerical equivalence of the
        // M1 refactor.
        let trained = analytic_two_objectives(true);
        let surrogates: Vec<&FittedSurrogate> = trained.iter().map(|t| &t.surrogate).collect();
        let signs = vec![1.0, 1.0];
        let ys_refs: Vec<&[f64]> = trained.iter().map(|t| t.y.as_slice()).collect();
        let front = build_observed_front(&surrogates, &ys_refs, &signs);
        let ref_point = compute_ref_point(&front, 2);
        let hv_p = hypervolume_nd(&front, &ref_point);
        let z_matrix = draw_standard_normal_matrix(EHVI_SAMPLES, 2);
        let ctx = EhviContext {
            surrogates: surrogates.clone(),
            signs: signs.clone(),
            front: front.clone(),
            ref_point: ref_point.clone(),
            hv_p,
            z_matrix: &z_matrix,
        };

        // Naive reference implementation (clones the entire front and pushes on every sample).
        let naive_ehvi = |x_norm: &[f64]| -> f64 {
            let n_obj = surrogates.len();
            let g: Vec<f64> = (0..n_obj)
                .map(|k| signs[k] * surrogates[k].predict_norm(x_norm))
                .collect();
            let s: Vec<f64> = (0..n_obj)
                .map(|k| {
                    surrogates[k]
                        .predict_var_norm(x_norm)
                        .map(|v| v.max(0.0).sqrt())
                        .unwrap_or(0.0)
                })
                .collect();
            let mut acc = 0.0;
            for z_row in &z_matrix {
                let v_s: Vec<f64> = (0..n_obj).map(|k| g[k] + s[k] * z_row[k]).collect();
                let mut augmented = front.clone();
                augmented.push(v_s);
                let hv_aug = hypervolume_nd(&augmented, &ref_point);
                let improvement = hv_aug - hv_p;
                if improvement > 0.0 {
                    acc += improvement;
                }
            }
            acc / z_matrix.len() as f64
        };

        for x in [
            vec![0.3, 0.3],
            vec![0.5, 0.5],
            vec![0.7, 0.2],
            vec![0.1, 0.9],
        ] {
            let fast = ctx.ehvi(&x);
            let naive = naive_ehvi(&x);
            assert_eq!(fast, naive, "EHVI must be bit-identical at x = {x:?}");
        }
    }

    #[test]
    fn non_gp_models_return_error() {
        // A model without posterior variance (equivalent to Ridge) cannot use EHVI and should return an error.
        let trained = analytic_two_objectives(false);
        let err = suggest_candidates_multi(&trained, &[true, true], 1).unwrap_err();
        assert!(
            err.contains("Gaussian Process"),
            "expected GP error, got: {err}"
        );
    }

    #[test]
    fn mixed_min_max_valid() {
        // Candidates should still be valid (EHVI finite, within bounds) even
        // with a mixed direction that maximizes f2. Uses a real fit since
        // this is an n=2 batch involving Constant Liar refitting.
        let (x, f1, f2) = conflicting_samples(40);
        let trained = fit_two_objectives(x, f1, f2);
        let candidates = suggest_candidates_multi(&trained, &[true, false], 2)
            .expect("mixed min/max suggest should succeed");
        assert_eq!(candidates.len(), 2);
        for c in &candidates {
            assert!(c.ehvi_score.is_finite(), "EHVI should be finite");
            assert!(c.ehvi_score >= 0.0);
            assert!(
                c.params.iter().all(|&v| (0.0..=1.0).contains(&v)),
                "params out of [0,1]: {:?}",
                c.params
            );
        }
    }

    #[test]
    fn errors_on_empty_and_mismatch() {
        assert!(suggest_candidates_multi(&[], &[], 1).is_err());

        let trained = analytic_two_objectives(true);
        // n_candidates == 0
        assert!(suggest_candidates_multi(&trained, &[true, true], 0).is_err());
        // length mismatch
        assert!(suggest_candidates_multi(&trained, &[true], 1).is_err());
    }
}
