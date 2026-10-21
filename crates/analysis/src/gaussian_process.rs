//! Gaussian process regression (egobox-gp / egobox-moe backend).
//!
//! A single GP is implemented via FITC / VFE sparse approximation
//! ([`egobox_gp::SparseGaussianProcess`], Matérn 5/2 ARD, with noise variance
//! estimation), while the mixture of experts (MoE) is implemented via
//! [`egobox_moe::GpMixture`] (SparseGp experts).
//!
//! egobox's plain GP (`GaussianProcess`) is an interpolator that does not
//! estimate noise variance, so it is unsuitable for PDP use cases where we
//! fit on only a subset of columns of high-dimensional data (variation in the
//! other dimensions shows up as noise) — hence it is not used. When
//! N ≤ max_inducing, the inducing points become Z = X, making FITC / VFE
//! mathematically equivalent to an exact GP with noise estimation, so this
//! path also covers full-GP functionality.
//!
//! - FITC / VFE: number of inducing points M = min(N, max_inducing). For
//!   large N, k-means inducing points are used (verified that M=100 yields
//!   θ and noise estimates nearly identical to the exact solution).
//! - MoE: clusters the input space with a GMM, trains a FITC expert per
//!   cluster, and smoothly recombines them. The number of clusters is chosen
//!   by cross-validation (up to 3) on a subsample of at most 500 points.
//!   Experts' inducing points are passed as an explicit `Located` (because
//!   egobox-moe 0.35 propagates the seed to experts via an `Option<u64>`
//!   random number, and `Randomized` would make the result non-deterministic).
//!
//! Training is deterministic (k-means uses a fixed seed, egobox's multi-start
//! θ optimization uses a fixed grid, and SGP / MoE random seeds are
//! explicitly specified).

use egobox_gp::{
    correlation_models::Matern52Corr, Inducings, ParamTuning, SparseGaussianProcess, SparseMethod,
};
use egobox_moe::{
    find_best_number_of_clusters, CorrelationSpec, GpMixture, GpMixtureParams, GpType, NbClusters,
    Recombination, RegressionSpec,
};
use linfa::prelude::*;
use ndarray::{Array1, Array2, Axis};
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;

use crate::clustering::{run_kmeans, InitStrategy};

/// Gaussian process training method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GpMethod {
    /// FITC (Fully Independent Training Conditional) approximation.
    Fitc,
    /// VFE (Variational Free Energy) approximation. Tends to estimate noise
    /// more conservatively than FITC.
    Vfe,
    /// Mixture of experts (per-cluster FITC GPs smoothly recombined).
    Moe,
}

/// Candidate lower bounds for the noise variance search (assumes `y` is
/// z-score normalized, i.e. variance 1).
///
/// With egobox's default lower bound (~1e-14), noise-free smooth functions
/// can make the covariance matrix lose positive-definiteness and cause
/// training to panic. 1e-6 is -120dB relative to variance 1, improving the
/// matrix's condition number while leaving prediction bias essentially zero.
/// If training still fails, retry with the bound raised to 1e-3 (slightly
/// smooths the fit, but better than a training failure that shows nothing).
const NOISE_FLOORS: [f64; 2] = [1e-6, 1e-3];

/// Upper bound on the subsample size used for MoE cluster-count search.
/// The search performs O(N) repeated k-fold cross-validation, which is
/// expensive (about 10 seconds at N=1000), so it is capped at 500 points
/// (about 1.5 seconds at N=500).
const MOE_CLUSTER_SEARCH_MAX_N: usize = 500;

/// Maximum number of MoE clusters.
const MOE_MAX_CLUSTERS: usize = 3;

/// A trained Gaussian process model.
pub(crate) struct GpModel {
    inner: GpInner,
    n_dims: usize,
}

enum GpInner {
    Sgp(Box<SparseGaussianProcess<f64, Matern52Corr>>),
    Moe(Box<GpMixture>),
}

impl GpModel {
    /// Trains a Gaussian process.
    ///
    /// - `x`: training inputs (rows = samples). Must be normalized (assumed
    ///   to be within [0,1]^d).
    /// - `y`: training objective values (assumed to be z-score normalized).
    /// - `method`: training method (FITC / VFE / MoE).
    /// - `max_inducing`: upper bound M on the number of inducing points.
    ///   If N ≤ M, Z = X (equivalent to an exact GP).
    /// - `seed`: seed for internal randomness (fixed for reproducibility).
    ///
    /// Returns `None` on training failure (numerical breakdown or invalid
    /// input). MoE does not fall back on its own (the caller must explicitly
    /// fall back to a different method).
    pub(crate) fn fit(
        x: &[Vec<f64>],
        y: &[f64],
        method: GpMethod,
        max_inducing: usize,
        seed: u64,
    ) -> Option<Self> {
        // No priority rows (priority = &[]): delegate to the usual uniform
        // inducing-point selection.
        Self::fit_impl(x, y, method, max_inducing, seed, &[])
    }

    /// Trains a GP while concentrating inducing points on priority rows
    /// (e.g. the Pareto front).
    ///
    /// `priority` is the set of row indices (into `x`) to prioritize as
    /// inducing points. It only has an effect when N > max_inducing (when
    /// N ≤ max_inducing, Z = X uses every point, so nothing changes).
    pub(crate) fn fit_front_focused(
        x: &[Vec<f64>],
        y: &[f64],
        method: GpMethod,
        max_inducing: usize,
        seed: u64,
        priority: &[usize],
    ) -> Option<Self> {
        Self::fit_impl(x, y, method, max_inducing, seed, priority)
    }

    /// Shared implementation for `fit` / `fit_front_focused`. Passes
    /// `priority` through to inducing-point selection.
    fn fit_impl(
        x: &[Vec<f64>],
        y: &[f64],
        method: GpMethod,
        max_inducing: usize,
        seed: u64,
        priority: &[usize],
    ) -> Option<Self> {
        let n = y.len();
        let n_dims = x.first()?.len();
        if n < 3 || x.len() != n || n_dims == 0 || max_inducing == 0 {
            return None;
        }
        if x.iter().any(|row| row.len() != n_dims) {
            return None;
        }

        let x_arr = Array2::from_shape_fn((n, n_dims), |(i, d)| x[i][d]);
        let y_arr = Array1::from_iter(y.iter().copied());

        // Inducing points: if N ≤ M, the training points themselves (Z = X);
        // otherwise select while taking `priority` into account.
        let z = if n <= max_inducing {
            x_arr.clone()
        } else {
            select_inducing_points(x, n_dims, max_inducing, priority, seed)?
        };

        let inner = match method {
            GpMethod::Fitc => Self::fit_sgp(&x_arr, &y_arr, &z, SparseMethod::Fitc, seed)?,
            GpMethod::Vfe => Self::fit_sgp(&x_arr, &y_arr, &z, SparseMethod::Vfe, seed)?,
            GpMethod::Moe => Self::fit_moe(&x_arr, &y_arr, &z, seed)?,
        };

        // Detect numerical breakdown: predictions at the training points
        // must be finite.
        let model = GpModel { inner, n_dims };
        let check = model.predict_mean_batch(&x[..1]);
        if check.iter().any(|v| !v.is_finite()) {
            return None;
        }
        Some(model)
    }

    /// Trains a single SGP (FITC / VFE).
    fn fit_sgp(
        x: &Array2<f64>,
        y: &Array1<f64>,
        z: &Array2<f64>,
        sparse_method: SparseMethod,
        seed: u64,
    ) -> Option<GpInner> {
        let dataset = Dataset::new(x.clone(), y.clone());
        // egobox-gp may panic (e.g. NotPositiveDefinite in COBYLA loop) instead of
        // returning Err for ill-conditioned data.  Catch such panics and treat them
        // as a training failure, then retry with a higher noise floor before
        // giving up (→ None), so callers can fall back gracefully.
        let sgp = NOISE_FLOORS.iter().find_map(|&floor| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                SparseGaussianProcess::<f64, Matern52Corr>::params(
                    Matern52Corr::default(),
                    Inducings::Located(z.clone()),
                )
                .sparse_method(sparse_method)
                .noise_variance(ParamTuning::Optimized {
                    init: 1e-2_f64.max(floor),
                    bounds: (floor, 1e2),
                })
                .seed(Some(seed))
                .fit(&dataset)
            }))
            .ok()
            .and_then(|r| r.ok())
        })?;
        Some(GpInner::Sgp(Box::new(sgp)))
    }

    /// Trains a mixture-of-experts GP.
    ///
    /// The number of clusters is chosen by cross-validation (capped at
    /// [`MOE_MAX_CLUSTERS`]) on an evenly-spaced subsample of at most
    /// [`MOE_CLUSTER_SEARCH_MAX_N`] points. Experts are FITC SGPs (sharing
    /// the full data's k-means / Z=X inducing points).
    ///
    /// The noise variance lower bound for MoE experts cannot be configured
    /// because egobox-moe does not expose it. Training may panic on
    /// noise-free data, but this is caught via `catch_unwind` and returns
    /// `None`.
    fn fit_moe(x: &Array2<f64>, y: &Array1<f64>, z: &Array2<f64>, seed: u64) -> Option<GpInner> {
        let n = x.nrows();

        // Search for the number of clusters (evenly-spaced subsample, deterministic).
        let n_clusters = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (x_sub, y_sub) = if n > MOE_CLUSTER_SEARCH_MAX_N {
                let idx: Vec<usize> = (0..MOE_CLUSTER_SEARCH_MAX_N)
                    .map(|j| j * n / MOE_CLUSTER_SEARCH_MAX_N)
                    .collect();
                (x.select(Axis(0), &idx), y.select(Axis(0), &idx))
            } else {
                (x.clone(), y.clone())
            };
            let (k, _recombination) = find_best_number_of_clusters(
                &x_sub,
                &y_sub,
                MOE_MAX_CLUSTERS,
                None,
                RegressionSpec::CONSTANT,
                CorrelationSpec::MATERN52,
                Xoshiro256Plus::seed_from_u64(seed),
            );
            k.max(1)
        }))
        .unwrap_or(1);

        let dataset = Dataset::new(x.clone(), y.clone());
        let moe = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            GpMixtureParams::<f64>::new_with_rng(
                GpType::SparseGp {
                    sparse_method: SparseMethod::Fitc,
                    inducings: Inducings::Located(z.clone()),
                },
                Xoshiro256Plus::seed_from_u64(seed),
            )
            .n_clusters(NbClusters::fixed(n_clusters))
            .recombination(Recombination::Smooth(None))
            .correlation_spec(CorrelationSpec::MATERN52)
            .fit(&dataset)
        }))
        .ok()
        .and_then(|r| r.ok())?;
        Some(GpInner::Moe(Box::new(moe)))
    }

    /// Raw prediction of the posterior mean (`None` on error). Absorbs the
    /// differing error types of SGP and MoE.
    fn predict_raw(&self, x: &Array2<f64>) -> Option<Array1<f64>> {
        match &self.inner {
            GpInner::Sgp(sgp) => sgp.predict(x).ok(),
            GpInner::Moe(moe) => moe.predict(x).ok(),
        }
    }

    /// Raw prediction of the posterior variance (`None` on error).
    fn predict_var_raw(&self, x: &Array2<f64>) -> Option<Array1<f64>> {
        match &self.inner {
            GpInner::Sgp(sgp) => sgp.predict_var(x).ok(),
            GpInner::Moe(moe) => moe.predict_var(x).ok(),
        }
    }

    /// Predicts the posterior mean for a batch of points.
    pub(crate) fn predict_mean_batch(&self, rows: &[Vec<f64>]) -> Vec<f64> {
        let x = Array2::from_shape_fn((rows.len(), self.n_dims), |(i, d)| rows[i][d]);
        match self.predict_raw(&x) {
            Some(mean) => mean.to_vec(),
            None => vec![f64::NAN; rows.len()],
        }
    }

    /// Predicts the posterior mean for a single point.
    pub(crate) fn predict_mean(&self, x: &[f64]) -> f64 {
        let arr = Array2::from_shape_fn((1, self.n_dims), |(_, d)| x[d]);
        self.predict_raw(&arr).map(|m| m[0]).unwrap_or(f64::NAN)
    }

    /// Predicts the posterior variance for a single point (negative values
    /// are clamped to 0).
    pub(crate) fn predict_variance(&self, x: &[f64]) -> f64 {
        let arr = Array2::from_shape_fn((1, self.n_dims), |(_, d)| x[d]);
        self.predict_var_raw(&arr)
            .map(|v| v[0].max(0.0))
            .unwrap_or(f64::NAN)
    }

    /// Returns the ARD correlation parameters θ (one per input dimension,
    /// over normalized [0,1] inputs).
    ///
    /// By egobox / SMT convention, a larger θ_d means a shorter length scale
    /// for dimension d, i.e. the surrogate is more sensitive to that
    /// dimension. Only a single SGP (FITC / VFE) returns `Some`. MoE has a θ
    /// per expert with no unique aggregation, so it returns `None`.
    pub(crate) fn ard_theta(&self) -> Option<Vec<f64>> {
        match &self.inner {
            GpInner::Sgp(sgp) => Some(sgp.theta().to_vec()),
            GpInner::Moe(_) => None,
        }
    }
}

/// Whether two points match in every dimension (exact equality, used to
/// detect duplicate inducing points).
fn rows_equal(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x == y)
}

/// Builds the inducing points for the N > max_inducing case (pure and
/// testable).
///
/// `priority` is the set of row indices to prioritize (e.g. the Pareto
/// front). Duplicates and out-of-range indices are removed. There are three
/// cases:
/// - P is empty: k-means(max_inducing) over all rows (the original
///   behavior).
/// - |P| ≥ max_inducing: k-means(max_inducing) over the priority rows only
///   (fully concentrated on the front).
/// - 0 < |P| < max_inducing: adopt all priority rows as inducing points, and
///   fill the remaining budget with k-means over the non-priority rows
///   (coarsely covering the rest of the space). Non-priority centroids that
///   coincide with a priority point are deduplicated, so the final count may
///   fall slightly short of max_inducing.
///
/// Returns an `Array2` of shape (number of inducing points, n_dims).
/// `None` on failure.
fn select_inducing_points(
    x: &[Vec<f64>],
    n_dims: usize,
    max_inducing: usize,
    priority: &[usize],
    seed: u64,
) -> Option<Array2<f64>> {
    let n = x.len();
    let _ = seed; // k-means is effectively fixed-seed (deterministic); accepted for signature uniformity.

    // Extract the priority rows as unique points, removing duplicates and out-of-range indices.
    let mut priority_rows: Vec<usize> = Vec::new();
    let mut seen = vec![false; n];
    for &idx in priority {
        if idx < n && !seen[idx] {
            seen[idx] = true;
            priority_rows.push(idx);
        }
    }

    // Helper for k-means over all rows (the original behavior).
    let kmeans_over = |rows: &[Vec<f64>], k: usize| -> Option<Vec<Vec<f64>>> {
        if rows.is_empty() || k == 0 {
            return None;
        }
        let flat: Vec<f64> = rows.iter().flatten().copied().collect();
        let result = run_kmeans(k, &flat, n_dims, InitStrategy::KMeansPlusPlus);
        if result.centroids.is_empty() {
            None
        } else {
            Some(result.centroids)
        }
    };

    let centroids: Vec<Vec<f64>> = if priority_rows.is_empty() {
        // P is empty: original behavior (k-means over all rows).
        kmeans_over(x, max_inducing)?
    } else if priority_rows.len() >= max_inducing {
        // |P| ≥ M: k-means over the priority rows only, fully concentrated on the front.
        let p_points: Vec<Vec<f64>> = priority_rows.iter().map(|&i| x[i].clone()).collect();
        kmeans_over(&p_points, max_inducing)?
    } else {
        // 0 < |P| < M: adopt all priority rows, fill the rest with k-means over non-priority rows.
        let mut points: Vec<Vec<f64>> = priority_rows.iter().map(|&i| x[i].clone()).collect();
        let remaining = max_inducing - points.len();
        let non_priority: Vec<Vec<f64>> =
            (0..n).filter(|i| !seen[*i]).map(|i| x[i].clone()).collect();
        if remaining > 0 {
            if let Some(fill) = kmeans_over(&non_priority, remaining) {
                // Deduplicate centroids that coincide with a priority point (avoid double-counting).
                for c in fill {
                    if !points.iter().any(|p| rows_equal(p, &c)) {
                        points.push(c);
                    }
                }
            }
        }
        points
    };

    if centroids.is_empty() {
        return None;
    }
    Some(Array2::from_shape_fn(
        (centroids.len(), n_dims),
        |(j, d)| centroids[j][d],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates data from a smooth test function (deterministic pseudo-random numbers).
    fn make_data(n: usize, d: usize, seed: u64) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut state = seed;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let x: Vec<Vec<f64>> = (0..n).map(|_| (0..d).map(|_| next()).collect()).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|row| {
                row.iter().map(|v| (v - 0.5) * (v - 0.5)).sum::<f64>()
                    + 0.01 * (row[0] * 20.0).sin()
            })
            .collect();
        (x, y)
    }

    // NOTE: GP fit quality (e.g. high R² on smooth functions, increasing
    // variance away from the data, MoE fitting piecewise functions well, not
    // interpolating noise dimensions) is the responsibility of egobox, the
    // surrogate backend, and is not verified here. Tests in this module are
    // limited to checking our own logic (input validation, inducing-point
    // selection, fallback, determinism, Send/Sync, ard_theta plumbing).

    #[test]
    fn fit_is_deterministic() {
        // Determinism is our own responsibility (fixed seed), so check all 3 methods, but N can be small.
        let (x, y) = make_data(60, 2, 3);
        for method in [GpMethod::Fitc, GpMethod::Vfe, GpMethod::Moe] {
            let m1 = GpModel::fit(&x, &y, method, 50, 42).expect("fit 1");
            let m2 = GpModel::fit(&x, &y, method, 50, 42).expect("fit 2");
            let p = vec![vec![0.3, 0.7], vec![0.9, 0.1]];
            assert_eq!(
                m1.predict_mean_batch(&p),
                m2.predict_mean_batch(&p),
                "{method:?}"
            );
            let v1: Vec<f64> = p.iter().map(|pt| m1.predict_variance(pt)).collect();
            let v2: Vec<f64> = p.iter().map(|pt| m2.predict_variance(pt)).collect();
            assert_eq!(v1, v2, "{method:?}");
        }
    }

    #[test]
    fn moe_handles_small_n() {
        let (x, y) = make_data(12, 2, 5);
        // Must return Some/None without panicking even for small N.
        if let Some(model) = GpModel::fit(&x, &y, GpMethod::Moe, 100, 42) {
            let pred = model.predict_mean_batch(&x);
            assert!(pred.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn degenerate_inputs_return_none() {
        for method in [GpMethod::Fitc, GpMethod::Vfe, GpMethod::Moe] {
            // n < 3
            assert!(GpModel::fit(&[vec![0.0], vec![1.0]], &[0.0, 1.0], method, 10, 42).is_none());
            // empty input
            assert!(GpModel::fit(&[], &[], method, 10, 42).is_none());
            // mismatched column count
            let x = vec![vec![0.0, 1.0], vec![0.5], vec![1.0, 0.0]];
            assert!(GpModel::fit(&x, &[0.0, 0.5, 1.0], method, 10, 42).is_none());
            // max_inducing = 0
            let (x, y) = make_data(10, 2, 1);
            assert!(GpModel::fit(&x, &y, method, 0, 42).is_none());
        }
    }

    #[test]
    fn duplicate_rows_do_not_break_fit() {
        // Training should succeed via noise estimation + nugget even with duplicate points (K_ZZ near-singular).
        let (mut x, mut y) = make_data(40, 2, 5);
        for i in 0..10 {
            x.push(x[i].clone());
            y.push(y[i]);
        }
        let model = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42);
        if let Some(m) = model {
            let pred = m.predict_mean_batch(&x);
            assert!(pred.iter().all(|v| v.is_finite()));
        }
        // The requirement is that it must not panic even when it returns None.
    }

    #[test]
    fn ard_theta_is_some_for_sgp_none_for_moe() {
        // A function that depends strongly on x0 and barely on x1 → expect θ_0 > θ_1.
        let mut state = 12345u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let x: Vec<Vec<f64>> = (0..50).map(|_| vec![next(), next()]).collect();
        let y: Vec<f64> = x.iter().map(|r| 3.0 * r[0] + 0.05 * r[1]).collect();

        // SGP's θ plumbing does not depend on SparseMethod, so check only FITC as a representative case.
        let model = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42).expect("fit");
        let theta = model.ard_theta().expect("SGP should expose theta");
        assert_eq!(theta.len(), 2);
        assert!(theta.iter().all(|t| t.is_finite() && *t > 0.0));
        // Sensitive to x0 ⇒ θ_0 is larger (shorter length scale).
        assert!(theta[0] > theta[1], "theta={theta:?}");

        // MoE returns None.
        let moe = GpModel::fit(&x, &y, GpMethod::Moe, 100, 42).expect("MoE fit");
        assert!(moe.ard_theta().is_none());
    }

    #[test]
    fn model_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GpModel>();
    }

    // ────────────────────────────────────────────────────────────
    // Front-focused inducing points (select_inducing_points / fit_front_focused)
    // ────────────────────────────────────────────────────────────

    #[test]
    fn select_inducing_points_includes_priority_rows() {
        // N=200, M=50, 10 priority rows → should include the 10 priority points, total ≤ 50.
        let (x, _) = make_data(200, 2, 5);
        let priority: Vec<usize> = vec![0, 3, 7, 11, 20, 33, 55, 88, 120, 199];
        let z = select_inducing_points(&x, 2, 50, &priority, 42).expect("should select");
        assert!(z.nrows() <= 50, "count {} should be ≤ 50", z.nrows());
        // Each priority point exists as an inducing-point row (exact match).
        for &p in &priority {
            let found = (0..z.nrows()).any(|r| (0..2).all(|d| z[[r, d]] == x[p][d]));
            assert!(found, "priority row {p} should be an inducing point");
        }
        assert!(z.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn select_inducing_points_priority_exceeds_budget() {
        // 80 priority rows (> M=50) → result ≤ 50 rows, all within the convex range of the priority set (k-means constraint).
        let (x, _) = make_data(200, 2, 9);
        let priority: Vec<usize> = (0..80).collect();
        let z = select_inducing_points(&x, 2, 50, &priority, 42).expect("should select");
        assert!(z.nrows() <= 50, "count {} should be ≤ 50", z.nrows());
        assert!(z.iter().all(|v| v.is_finite()));
        // Since each centroid comes from k-means over priority rows only, each dimension falls within the priority rows' range.
        for d in 0..2 {
            let lo = priority
                .iter()
                .map(|&i| x[i][d])
                .fold(f64::INFINITY, f64::min);
            let hi = priority
                .iter()
                .map(|&i| x[i][d])
                .fold(f64::NEG_INFINITY, f64::max);
            for r in 0..z.nrows() {
                assert!(
                    z[[r, d]] >= lo - 1e-9 && z[[r, d]] <= hi + 1e-9,
                    "centroid out of priority range"
                );
            }
        }
    }

    #[test]
    fn select_inducing_points_empty_priority_behaves_as_before() {
        // No priority → original behavior (k-means over all rows, ≤ M rows).
        let (x, _) = make_data(200, 2, 13);
        let z = select_inducing_points(&x, 2, 50, &[], 42).expect("should select");
        assert!(z.nrows() <= 50);
        assert!(z.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn fit_front_focused_trains_and_is_deterministic() {
        // N=80, M=50, with a small number of priority rows: training/prediction are finite and deterministic across two runs.
        let (x, y) = make_data(80, 2, 21);
        let priority: Vec<usize> = vec![0, 5, 10, 17, 42];
        let m1 =
            GpModel::fit_front_focused(&x, &y, GpMethod::Fitc, 50, 42, &priority).expect("fit 1");
        let m2 =
            GpModel::fit_front_focused(&x, &y, GpMethod::Fitc, 50, 42, &priority).expect("fit 2");
        let probe = vec![vec![0.3, 0.7], vec![0.9, 0.1]];
        let p1 = m1.predict_mean_batch(&probe);
        let p2 = m2.predict_mean_batch(&probe);
        assert!(p1.iter().all(|v| v.is_finite()));
        assert_eq!(p1, p2, "front-focused fit should be deterministic");
    }

    #[test]
    fn fit_front_focused_equals_fit_when_n_le_max_inducing() {
        // When N ≤ M, Z = X and priority is ignored (identical to `fit`).
        let (x, y) = make_data(40, 2, 3);
        let with_priority =
            GpModel::fit_front_focused(&x, &y, GpMethod::Fitc, 100, 42, &[0, 1, 2]).expect("fit");
        let plain = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42).expect("fit");
        let probe = vec![vec![0.4, 0.6]];
        assert_eq!(
            with_priority.predict_mean_batch(&probe),
            plain.predict_mean_batch(&probe),
            "priority must not change result when N ≤ max_inducing"
        );
    }
}
