//! Optimization methods on the surrogate surface.
//!
//! Implemented as minimization within the normalized space [0,1]^d (maximize flips the sign).
//! Add new methods here as additional variants.

mod cma_es;
// Exposed within the crate because the gh runner (crate::gh::runner) repurposes it for real objective function evaluation
pub(crate) mod nsga2;

use argmin::core::{CostFunction, Error, Gradient};

use super::models::FittedSurrogate;
use crate::math::rng::SeededRng;
use crate::optimization::LbfgsOptimizer;

/// Kind of optimization method used on the surrogate surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OptimizerKind {
    /// Multi-start L-BFGS (numerical gradient) from the observed best point plus random points.
    MultiStartLbfgs,
    /// Random search with a fixed seed (an always-working baseline).
    RandomSearch,
    /// NSGA-II (SBX crossover, polynomial mutation, binary tournament selection).
    Nsga2,
    /// CMA-ES (Covariance Matrix Adaptation Evolution Strategy).
    CmaEs,
}

/// Number of multi-start starting points (1 observed best point + 7 random).
pub(crate) const N_RANDOM_STARTS: usize = 7;
/// Number of evaluation points for random search.
const N_RANDOM_SEARCH: usize = 4096;
/// Step size for the numerical gradient (central difference).
pub(crate) const FD_STEP: f64 = 1e-4;
/// Weight of the out-of-bounds penalty.
pub(crate) const BOUND_PENALTY: f64 = 1e3;
/// Random seed (fixed for reproducibility).
pub(crate) const SEED: u64 = 42;

/// Weight of the constraint penalty (in z-score units).
/// A scalar multiplied by the constraint violation amount (excess over the normalized z-score).
const CONSTRAINT_PENALTY: f64 = 100.0;

/// Quadratic penalty `Σ max(0, v−1)² + max(0, −v)²` for components outside the box [0,1]^d.
/// Each point is clamped to [0,1] for evaluation, while this term provides a smooth
/// gradient pointing back inward from outside the box.
fn box_penalty(t: &[f64]) -> f64 {
    t.iter()
        .map(|&v| {
            let over = (v - 1.0).max(0.0);
            let under = (-v).max(0.0);
            over * over + under * under
        })
        .sum()
}

/// Optimizes on the surrogate surface and returns the optimal point in normalized space [0,1]^d.
/// `minimize=false` (maximization) is handled as minimization of the sign-flipped surface.
///
/// When `constraint_models` is non-empty, a constraint penalty is added to the cost function:
///
/// ```text
/// cost = sign * mu_y_norm(x) + CONSTRAINT_PENALTY * Σ max(0, mu_ci_norm(x) - z0_i)
/// ```
///
/// z0_i = (0 - c_mean_i) / c_std_i is the feasibility boundary (in normalized z-score units).
pub(crate) fn minimize_on_surrogate(
    surrogate: &FittedSurrogate,
    minimize: bool,
    optimizer: OptimizerKind,
    start_norm: &[f64],
    constraint_models: &[FittedSurrogate],
) -> Vec<f64> {
    let sign = if minimize { 1.0 } else { -1.0 };

    // Feasibility boundary z0_i = (0 - c_mean_i) / c_std_i (normalized z-score
    // units). Empty when there are no constraints.
    let z0s: Vec<f64> = constraint_models
        .iter()
        .map(|cm| {
            if cm.y_std > 1e-12 {
                (0.0 - cm.y_mean) / cm.y_std
            } else if cm.y_mean <= 0.0 {
                f64::INFINITY // Always feasible
            } else {
                f64::NEG_INFINITY // Always violated
            }
        })
        .collect();

    // Base cost on an already-in-box point: the signed objective plus the
    // constraint penalty. The out-of-bounds box penalty is applied by the
    // optimizer wrappers (`penalized_fn` / `penalized_cost`), so this closure
    // only ever sees clamped inputs.
    //
    // Routing every optimizer through this one closure means the user's
    // optimizer choice is honored whether or not constraints are present.
    // Previously the constrained path always fell back to gradient-based
    // L-BFGS, silently discarding the requested optimizer; that stalls on
    // non-smooth surrogates (e.g. LightGBM), whose finite-difference gradient
    // is zero almost everywhere, so a constrained CMA-ES/NSGA-II request did no
    // real search.
    let base_cost = |c: &[f64]| -> f64 {
        let obj = sign * surrogate.predict_norm(c);
        let con_pen: f64 = constraint_models
            .iter()
            .zip(z0s.iter())
            .map(|(cm, &z0)| CONSTRAINT_PENALTY * (cm.predict_norm(c) - z0).max(0.0))
            .sum();
        obj + con_pen
    };

    let n_dims = start_norm.len();
    let t = match optimizer {
        OptimizerKind::MultiStartLbfgs => minimize_scalar_fn(&base_cost, n_dims, start_norm),
        OptimizerKind::RandomSearch => random_search(&base_cost, n_dims, start_norm),
        OptimizerKind::Nsga2 => run_nsga2(&base_cost, n_dims, start_norm),
        OptimizerKind::CmaEs => run_cma_es(&base_cost, start_norm),
    };
    t.iter().map(|v| v.clamp(0.0, 1.0)).collect()
}

/// Evaluates the surrogate at a point clamped into the box, applying a quadratic penalty outside it.
pub(crate) fn penalized_cost(surrogate: &FittedSurrogate, sign: f64, t: &[f64]) -> f64 {
    let clamped: Vec<f64> = t.iter().map(|v| v.clamp(0.0, 1.0)).collect();
    sign * surrogate.predict_norm(&clamped) + BOUND_PENALTY * box_penalty(t)
}

/// Random search with a fixed seed. The observed best point is also included as
/// a candidate. `f` is the base cost on an in-box point; the box penalty is
/// applied via `penalized_fn`.
fn random_search(
    f: &(dyn Fn(&[f64]) -> f64 + Sync),
    n_dims: usize,
    start_norm: &[f64],
) -> Vec<f64> {
    let mut rng = SeededRng::from_seed(SEED);

    let mut best = start_norm.to_vec();
    let mut best_cost = penalized_fn(f, &best);

    for _ in 0..N_RANDOM_SEARCH {
        let t: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
        let cost = penalized_fn(f, &t);
        if cost < best_cost {
            best_cost = cost;
            best = t;
        }
    }
    best
}

/// Runs NSGA-II as a single-objective minimization of `f`.
/// Fitness is a length-1 vector (using the generic implementation in preparation for future
/// multi-objective surrogate support).
fn run_nsga2(f: &(dyn Fn(&[f64]) -> f64 + Sync), n_dims: usize, start_norm: &[f64]) -> Vec<f64> {
    // Currently a single-objective surrogate, so use the η_c = 20 configuration.
    let cfg = nsga2::Nsga2Config::for_objectives(1);
    let front = nsga2::nsga2_minimize(
        |t| vec![penalized_fn(f, t)],
        n_dims,
        std::slice::from_ref(&start_norm.to_vec()),
        &cfg,
    );
    front
        .into_iter()
        .min_by(|a, b| {
            a.1[0]
                .partial_cmp(&b.1[0])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(genome, _)| genome)
        .unwrap_or_else(|| start_norm.to_vec())
}

/// Runs CMA-ES with the observed best point as the initial mean.
fn run_cma_es(f: &(dyn Fn(&[f64]) -> f64 + Sync), start_norm: &[f64]) -> Vec<f64> {
    let cfg = cma_es::CmaEsConfig::default();
    cma_es::cma_es_minimize(|t| penalized_fn(f, t), start_norm, &cfg)
}

/// Minimizes an arbitrary scalar function `f: [0,1]^d → f64` via multi-start L-BFGS.
///
/// - `start_norm`: the supplied initial point (normalized coordinates in [0,1]^d).
/// - `n_dims`: number of dimensions.
/// - Return value: the best point, clamped to `[0,1]^d`.
///
/// Internally multi-starts from `start_norm` plus `N_RANDOM_STARTS` fixed-seed random points.
pub(crate) fn minimize_scalar_fn(
    f: &(dyn Fn(&[f64]) -> f64 + Sync),
    n_dims: usize,
    start_norm: &[f64],
) -> Vec<f64> {
    let mut rng = SeededRng::from_seed(SEED);

    let mut starts: Vec<Vec<f64>> = vec![start_norm.to_vec()];
    for _ in 0..N_RANDOM_STARTS {
        starts.push((0..n_dims).map(|_| rng.next_f64()).collect());
    }

    /// Cost function for argmin (wraps an arbitrary closure).
    struct ScalarProblem<'a> {
        f: &'a (dyn Fn(&[f64]) -> f64 + Sync),
    }

    impl CostFunction for ScalarProblem<'_> {
        type Param = Vec<f64>;
        type Output = f64;
        fn cost(&self, p: &Vec<f64>) -> Result<f64, Error> {
            Ok(penalized_fn(self.f, p))
        }
    }

    impl Gradient for ScalarProblem<'_> {
        type Param = Vec<f64>;
        type Gradient = Vec<f64>;
        fn gradient(&self, p: &Vec<f64>) -> Result<Vec<f64>, Error> {
            let mut grad = vec![0.0; p.len()];
            let mut pt = p.clone();
            for d in 0..p.len() {
                pt[d] = p[d] + FD_STEP;
                let plus = penalized_fn(self.f, &pt);
                pt[d] = p[d] - FD_STEP;
                let minus = penalized_fn(self.f, &pt);
                pt[d] = p[d];
                grad[d] = (plus - minus) / (2.0 * FD_STEP);
            }
            Ok(grad)
        }
    }

    let lbfgs = LbfgsOptimizer::new(100, 5);
    let mut best = start_norm.to_vec();
    let mut best_cost = penalized_fn(f, &best);

    for start in starts {
        let start_cost = penalized_fn(f, &start);
        if start_cost < best_cost {
            best_cost = start_cost;
            best = start.clone();
        }
        let problem = ScalarProblem { f };
        let candidate = lbfgs.optimize(start, problem);
        if candidate.iter().all(|v| v.is_finite()) {
            let cost = penalized_fn(f, &candidate);
            if cost < best_cost {
                best_cost = cost;
                best = candidate;
            }
        }
    }

    best.iter().map(|v| v.clamp(0.0, 1.0)).collect()
}

/// Evaluates an arbitrary function at a point clamped into the box, applying a quadratic penalty outside it.
fn penalized_fn(f: &(dyn Fn(&[f64]) -> f64 + Sync), t: &[f64]) -> f64 {
    let clamped: Vec<f64> = t.iter().map(|v| v.clamp(0.0, 1.0)).collect();
    f(&clamped) + BOUND_PENALTY * box_penalty(t)
}

/// Runs NSGA-II on multi-objective surrogate surfaces and returns the first Pareto front.
///
/// - `surrogates`: the fitted surrogate for each objective (minimize if `signs[k]` is 1.0, maximize if -1.0).
/// - `signs`: the sign for each objective (minimize = 1.0, maximize = −1.0).
/// - `initial_seeds`: points in normalized space used to seed the initial population.
///
/// Returns a list of `(genome, fitness vector)` pairs (first front only).
pub(crate) fn multi_objective_nsga2(
    surrogates: &[&super::models::FittedSurrogate],
    signs: &[f64],
    initial_seeds: &[Vec<f64>],
) -> Vec<(Vec<f64>, Vec<f64>)> {
    let n_obj = surrogates.len();
    let cfg = nsga2::Nsga2Config::for_objectives(n_obj);
    nsga2::nsga2_minimize(
        |t| {
            signs
                .iter()
                .zip(surrogates.iter())
                .map(|(&sign, &surrogate)| penalized_cost(surrogate, sign, t))
                .collect()
        },
        surrogates[0].col_stats.len(),
        initial_seeds,
        &cfg,
    )
}
