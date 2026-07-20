//! Run configuration: the sampler kind and `GhRunConfig`.

use crate::io::journal::parser::OptimizationDirection;

/// The kind of sampler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhSampler {
    /// Uniform random sampling (trial count = `n_trials`)
    Random,
    /// NSGA-II (trial count = population size rounded to even × (generations + 1))
    Nsga2,
    /// Adaptive surrogate loop: random bootstrap, then repeat
    /// fit surrogate → suggest candidates (EI single-objective / EHVI
    /// multi-objective) → evaluate → refit (`crate::gh::adaptive`).
    Adaptive,
}

/// Configuration for an optimization run.
#[derive(Debug, Clone)]
pub struct GhRunConfig {
    pub study_name: String,
    /// Optimization direction per objective (same count and order as `GhProblem.objectives`)
    pub directions: Vec<OptimizationDirection>,
    pub sampler: GhSampler,
    /// Trial count for the Random sampler
    pub n_trials: usize,
    /// Population size for NSGA-II
    pub population_size: usize,
    /// Number of generations for NSGA-II
    pub generations: usize,
    pub seed: u64,
    /// Adaptive sampler: number of random bootstrap trials before the first fit.
    pub adaptive_initial: usize,
    /// Adaptive sampler: candidates evaluated per iteration.
    pub adaptive_batch: usize,
    /// Adaptive sampler: number of fit → suggest → evaluate iterations.
    pub adaptive_iterations: usize,
    /// Adaptive sampler: stop early after this many consecutive iterations whose
    /// relative improvement in the convergence metric (hypervolume for
    /// multi-objective, shifted best value for single-objective) stays below
    /// `adaptive_min_improvement`. `0` disables convergence-based stopping (the
    /// loop always runs the full `adaptive_iterations`).
    pub adaptive_patience: usize,
    /// Adaptive sampler: relative-improvement threshold for the convergence
    /// check (e.g. `0.01` = 1%). Only used when `adaptive_patience > 0`.
    pub adaptive_min_improvement: f64,
}

impl Default for GhRunConfig {
    fn default() -> Self {
        Self {
            study_name: "gh-optimization".to_string(),
            directions: Vec::new(),
            sampler: GhSampler::Nsga2,
            n_trials: 50,
            population_size: 16,
            generations: 10,
            seed: 42,
            adaptive_initial: 10,
            adaptive_batch: 4,
            adaptive_iterations: 10,
            adaptive_patience: 0,
            adaptive_min_improvement: 0.01,
        }
    }
}
