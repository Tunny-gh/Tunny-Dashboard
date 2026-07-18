//! Optimization runner for Grasshopper definitions.
//!
//! Evaluates the real objective function via `GhEvaluator` (Rhino.Compute or a mock) and
//! records every trial to an Optuna-compatible journal. Writing to the journal means the
//! existing live-update, all analysis widgets, and reports work as-is without modification.
//!
//! Usage is a two-step process:
//! 1. `prepare_gh_run` — opens the journal and creates the study (synchronous, lightweight).
//!    The caller can open the journal right after this call and the study will already
//!    appear in the study list.
//! 2. `run_prepared` — the main optimization loop (blocking; call it from a background
//!    thread). Progress and cancellation are shared via `FitProgress`.
//!
//! The internal optimizer reuses the existing implementation: to match its convention of
//! normalized space [0,1]^d and minimizing all objectives, this module handles the
//! conversion to/from the real variable ranges and sign-flipping for Maximize.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use rayon::prelude::*;

use crate::data::extras::TrialState;
use crate::io::journal::parser::OptimizationDirection;
use crate::io::journal::writer::{JournalWriter, ParamDistribution};
use crate::math::rng::SeededRng;
use crate::surrogate_opt::optimizers::nsga2::{nsga2_minimize, Nsga2Config};
use crate::surrogate_opt::FitProgress;

use super::compute::GhEvaluator;
use super::problem::{GhProblem, GhVariable};

/// Penalty value returned to the optimization algorithm on evaluation failure or
/// cancellation. Strictly worse than any constraint-violating trial's fitness
/// (`CONSTRAINT_PENALTY_BASE + MAX_COUNTED_VIOLATION < FAIL_PENALTY`), so the
/// search never prefers a crashing region over a merely infeasible one.
/// Infinity would produce NaN when normalizing the crowding distance, so a large
/// finite value is used instead.
const FAIL_PENALTY: f64 = 1e15;

/// Base fitness for constraint-violating trials (see `constrained_penalty_fitness`).
const CONSTRAINT_PENALTY_BASE: f64 = 1e12;

/// Cap on the violation added to `CONSTRAINT_PENALTY_BASE`, keeping every
/// infeasible fitness strictly below `FAIL_PENALTY` no matter how large the
/// user's constraint values are.
const MAX_COUNTED_VIOLATION: f64 = 1e14;

/// Fitness returned to the optimization algorithm for a constraint-violating trial:
/// `CONSTRAINT_PENALTY_BASE + total violation` on every objective (violation =
/// the amount above 0, per Tunny's convention). This emulates Deb's constrained
/// domination with a generic minimizer, with three strict tiers: any feasible
/// solution (objectives far below the base) dominates every infeasible one;
/// among infeasible solutions the one with less total violation dominates; and
/// evaluation failures (`FAIL_PENALTY`) are worse than any infeasible solution.
/// Violations below f64 resolution at 1e12 (~1e-4) tie, which is acceptable for
/// ranking. The trial itself is still recorded as COMPLETE with its real
/// objective values (Tunny's constraints are soft — feasibility steers the
/// search, not validity).
fn constrained_penalty_fitness(n_obj: usize, constraints: &[f64]) -> Option<Vec<f64>> {
    let violation: f64 = constraints.iter().map(|c| c.max(0.0)).sum();
    if violation > 0.0 {
        Some(vec![
            CONSTRAINT_PENALTY_BASE
                + violation.min(MAX_COUNTED_VIOLATION);
            n_obj
        ])
    } else {
        None
    }
}

/// Checks a successful evaluation against the problem before recording it:
/// constraint/attribute arity must match the problem definition, and objective /
/// constraint values must be finite. Returns the failure reason, or `None` when
/// the evaluation is recordable.
fn validate_evaluation(eval: &super::compute::GhEvaluation, problem: &GhProblem) -> Option<String> {
    if eval.constraints.len() != problem.constraints.len() {
        return Some(format!(
            "Constraint count mismatch (expected {}, got {})",
            problem.constraints.len(),
            eval.constraints.len()
        ));
    }
    if eval.attributes.len() != problem.attributes.len() {
        return Some(format!(
            "Attribute count mismatch (expected {}, got {})",
            problem.attributes.len(),
            eval.attributes.len()
        ));
    }
    if eval.objectives.iter().any(|v| !v.is_finite()) {
        return Some("Objective value is not finite".to_string());
    }
    if eval.constraints.iter().any(|c| !c.is_finite()) {
        return Some("Constraint value is not finite".to_string());
    }
    None
}

/// The kind of sampler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhSampler {
    /// Uniform random sampling (trial count = `n_trials`)
    Random,
    /// NSGA-II (trial count = population size rounded to even × (generations + 1))
    Nsga2,
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
        }
    }
}

/// Summary of a run's results.
#[derive(Debug, Clone)]
pub struct GhRunSummary {
    pub study_id: u32,
    /// Number of trials recorded as COMPLETE
    pub completed: usize,
    /// Number of trials that failed evaluation (recorded as FAIL)
    pub failed: usize,
    /// Whether the run was cut short by cancellation
    pub cancelled: bool,
}

/// Result of `prepare_gh_run`. Holds a journal writer with the study already created.
pub struct PreparedGhRun {
    writer: Mutex<JournalWriter>,
    study_id: u32,
}

impl PreparedGhRun {
    /// ID of the created study (a sequential number within the journal).
    pub fn study_id(&self) -> u32 {
        self.study_id
    }
}

/// Opens the journal and creates the study (synchronous, lightweight).
///
/// Immediately after this call, the study already exists in the journal, so the caller
/// can open the journal to pick up live updates before starting `run_prepared` in the
/// background.
pub fn prepare_gh_run(
    journal_path: &Path,
    problem: &GhProblem,
    cfg: &GhRunConfig,
) -> Result<PreparedGhRun, String> {
    if cfg.directions.len() != problem.objectives.len() {
        return Err(format!(
            "The number of optimization directions ({}) does not match the number of objectives ({})",
            cfg.directions.len(),
            problem.objectives.len()
        ));
    }
    if problem.variables.is_empty() {
        return Err("No variables".to_string());
    }
    let mut writer = JournalWriter::open(journal_path)?;
    let objective_names: Vec<String> = problem.objectives.iter().map(|o| o.name.clone()).collect();
    let study_id = writer.create_study(&cfg.study_name, &cfg.directions, &objective_names)?;
    Ok(PreparedGhRun {
        writer: Mutex::new(writer),
        study_id,
    })
}

/// The main optimization loop (blocking). Call it from a background thread.
///
/// - Progress is reflected in `progress` (total = the planned number of evaluations)
/// - `progress.request_cancel()` cuts off further evaluations (an in-flight solve is
///   allowed to finish). Cancelled trials are not recorded to the journal
/// - Trials with an evaluation error are recorded as FAIL, and a penalty value is
///   returned to the optimization algorithm so it can continue
/// - If writing to the journal itself fails, the run aborts and returns Err
pub fn run_prepared(
    prep: &PreparedGhRun,
    problem: &GhProblem,
    evaluator: &dyn GhEvaluator,
    cfg: &GhRunConfig,
    progress: &FitProgress,
) -> Result<GhRunSummary, String> {
    let recorder = TrialRecorder {
        writer: &prep.writer,
        study_id: prep.study_id,
        problem,
        directions: &cfg.directions,
        evaluator,
        progress,
        completed: AtomicUsize::new(0),
        failed: AtomicUsize::new(0),
        io_error: Mutex::new(None),
    };
    let n_dims = problem.variables.len();
    progress.set_stage("Evaluating with Rhino.Compute");

    match cfg.sampler {
        GhSampler::Random => {
            let n = cfg.n_trials.max(1);
            progress.set_total(n);
            (0..n).into_par_iter().for_each(|i| {
                // Derive an independent seed per trial so results stay deterministic even in parallel
                let mut rng = SeededRng::from_seed(
                    cfg.seed
                        .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
                );
                let x: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
                recorder.eval_signed(&x);
            });
        }
        GhSampler::Nsga2 => {
            // nsga2_minimize rounds the population size up to an even number
            // (minimum 4), then evaluates the initial population plus each
            // generation's offspring population.
            let pop_even = (cfg.population_size.max(4) + 1) & !1;
            progress.set_total(pop_even * (cfg.generations + 1));
            let nsga_cfg = Nsga2Config {
                pop_size: cfg.population_size,
                generations: cfg.generations,
                seed: cfg.seed,
                ..Nsga2Config::for_objectives(cfg.directions.len())
            };
            // Seed the initial individual with the slider values at the time the definition was saved
            let initial = vec![normalize_current(problem)];
            nsga2_minimize(|x| recorder.eval_signed(x), n_dims, &initial, &nsga_cfg);
        }
    }

    if let Some(e) = recorder
        .io_error
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        return Err(format!(
            "Aborted because writing to the journal failed: {e}"
        ));
    }
    Ok(GhRunSummary {
        study_id: prep.study_id,
        completed: recorder.completed.load(Ordering::Relaxed),
        failed: recorder.failed.load(Ordering::Relaxed),
        cancelled: progress.is_cancelled(),
    })
}

/// Evaluates a single trial and records it to the journal. Shared across parallel evaluation threads.
struct TrialRecorder<'a> {
    writer: &'a Mutex<JournalWriter>,
    study_id: u32,
    problem: &'a GhProblem,
    directions: &'a [OptimizationDirection],
    evaluator: &'a dyn GhEvaluator,
    progress: &'a FitProgress,
    completed: AtomicUsize,
    failed: AtomicUsize,
    /// Journal write error (the first one). Once set, no new evaluations are started.
    io_error: Mutex<Option<String>>,
}

impl TrialRecorder<'_> {
    /// Evaluates a normalized point and returns objective values sign-adjusted to the minimize convention.
    fn eval_signed(&self, x_norm: &[f64]) -> Vec<f64> {
        let n_obj = self.directions.len();
        if self.progress.is_cancelled() || self.has_io_error() {
            return vec![FAIL_PENALTY; n_obj];
        }
        let values = denormalize(self.problem, x_norm);

        let trial_id = match self.begin_trial(&values) {
            Ok(id) => id,
            Err(e) => {
                self.set_io_error(e);
                return vec![FAIL_PENALTY; n_obj];
            }
        };

        match self.evaluator.evaluate(&values) {
            Ok(eval) if eval.objectives.len() == n_obj => {
                // Validate the evaluation the same way the objective count is
                // validated: wrong constraint/attribute arity or non-finite
                // values would journal misaligned columns or silently vanish
                // (serde_json writes non-finite f64 as null), so record FAIL
                // instead. ComputeEvaluator guarantees all of this; the checks
                // protect other GhEvaluator implementations.
                if let Some(reason) = validate_evaluation(&eval, self.problem) {
                    self.record_failure(trial_id, reason);
                    return vec![FAIL_PENALTY; n_obj];
                }
                if let Err(e) = self.finish_complete(trial_id, &eval) {
                    self.set_io_error(e);
                    return vec![FAIL_PENALTY; n_obj];
                }
                self.completed.fetch_add(1, Ordering::Relaxed);
                self.progress.inc_done();
                // Constraint-violating trials feed a penalty fitness to the
                // algorithm (see constrained_penalty_fitness); the journal
                // record above keeps the real objective values.
                if let Some(penalized) = constrained_penalty_fitness(n_obj, &eval.constraints) {
                    return penalized;
                }
                eval.objectives
                    .iter()
                    .zip(self.directions)
                    .map(|(v, d)| match d {
                        OptimizationDirection::Minimize => *v,
                        OptimizationDirection::Maximize => -*v,
                    })
                    .collect()
            }
            Ok(eval) => {
                self.record_failure(
                    trial_id,
                    format!(
                        "Objective count mismatch (expected {n_obj}, got {})",
                        eval.objectives.len()
                    ),
                );
                vec![FAIL_PENALTY; n_obj]
            }
            Err(e) => {
                self.record_failure(trial_id, e);
                vec![FAIL_PENALTY; n_obj]
            }
        }
    }

    /// Creates the trial and records params (acquires the writer lock only once).
    fn begin_trial(&self, values: &[f64]) -> Result<u32, String> {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        let trial_id = writer.create_trial(self.study_id)?;
        for (var, value) in self.problem.variables.iter().zip(values) {
            let dist = if var.is_integer {
                ParamDistribution::Int {
                    low: var.low.round() as i64,
                    high: var.high.round() as i64,
                }
            } else {
                ParamDistribution::Float {
                    low: var.low,
                    high: var.high,
                }
            };
            writer.set_trial_param(trial_id, &var.name, *value, &dist)?;
        }
        Ok(trial_id)
    }

    /// Records a FAIL for the trial. The Complete path goes through
    /// `finish_complete` (which also writes op8/op9); keeping this FAIL-only
    /// prevents a Complete trial from bypassing those records.
    fn finish_fail(&self, trial_id: u32) -> Result<(), String> {
        self.writer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .finish_trial(trial_id, TrialState::Fail, &[])
    }

    /// Records a successful evaluation: constraint values (op9) and per-trial
    /// attributes (op8), if any, followed by COMPLETE with the objective values
    /// (holds the writer lock once so the records stay adjacent even under
    /// parallel evaluation). Attributes evaluated as empty (`None`) are skipped,
    /// as are non-finite numeric attributes (serde_json would turn them into
    /// null, which the parsers drop anyway).
    fn finish_complete(
        &self,
        trial_id: u32,
        eval: &super::compute::GhEvaluation,
    ) -> Result<(), String> {
        use super::compute::GhAttrValue;

        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if !eval.constraints.is_empty() {
            writer.set_trial_constraints(trial_id, &eval.constraints)?;
        }
        for (attr, value) in self.problem.attributes.iter().zip(&eval.attributes) {
            let json = match value {
                Some(GhAttrValue::Number(v)) if v.is_finite() => serde_json::json!(v),
                Some(GhAttrValue::Number(_)) | None => continue,
                Some(GhAttrValue::Text(s)) => serde_json::json!(s),
            };
            writer.set_trial_user_attr(trial_id, &attr.name, &json)?;
        }
        writer.finish_trial(trial_id, TrialState::Complete, &eval.objectives)
    }

    fn record_failure(&self, trial_id: u32, reason: String) {
        if let Err(e) = self.finish_fail(trial_id) {
            self.set_io_error(e);
            return;
        }
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.progress.inc_done();
        let short: String = reason.chars().take(120).collect();
        self.progress
            .set_stage(format!("Evaluation errors: {short}"));
    }

    fn has_io_error(&self) -> bool {
        self.io_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    fn set_io_error(&self, e: String) {
        let mut guard = self.io_error.lock().unwrap_or_else(|e| e.into_inner());
        guard.get_or_insert(e);
    }
}

/// Converts a normalized point [0,1]^d into the slider's real value.
/// Applies the slider's rounding (integer / decimal digits) so that the value
/// recorded to the journal matches the value sent to Compute.
fn denormalize(problem: &GhProblem, x_norm: &[f64]) -> Vec<f64> {
    problem
        .variables
        .iter()
        .zip(x_norm)
        .map(|(var, x)| {
            let x = x.clamp(0.0, 1.0);
            let raw = var.low + x * (var.high - var.low);
            round_variable(var, raw)
        })
        .collect()
}

/// Maps the current slider values into normalized space (for seeding NSGA-II's initial individual).
fn normalize_current(problem: &GhProblem) -> Vec<f64> {
    problem
        .variables
        .iter()
        .map(|var| ((var.value - var.low) / (var.high - var.low)).clamp(0.0, 1.0))
        .collect()
}

fn round_variable(var: &GhVariable, raw: f64) -> f64 {
    if var.is_integer {
        raw.round()
    } else {
        let scale = 10f64.powi(var.digits.min(15) as i32);
        (raw * scale).round() / scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::fixtures::sample_ghx;
    use crate::gh::problem::extract_problem;
    use crate::io::journal::parser::parse_single_study;

    use crate::gh::compute::{GhAttrValue, GhEvaluation};

    /// Mock evaluator that computes objective values via a closure.
    struct FnEvaluator<F: Fn(&[f64]) -> Result<GhEvaluation, String> + Send + Sync>(F);

    impl<F: Fn(&[f64]) -> Result<GhEvaluation, String> + Send + Sync> GhEvaluator for FnEvaluator<F> {
        fn evaluate(&self, values: &[f64]) -> Result<GhEvaluation, String> {
            (self.0)(values)
        }
    }

    fn test_cfg(sampler: GhSampler) -> GhRunConfig {
        GhRunConfig {
            study_name: "gh-test".to_string(),
            directions: vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize,
            ],
            sampler,
            n_trials: 6,
            population_size: 4,
            generations: 1,
            seed: 7,
        }
    }

    /// Objectives: [span+count, span-count]. Constraint (the fixture wires one):
    /// span - 8 (feasible when span <= 8). Attribute (the fixture wires one):
    /// area = span * count.
    fn sum_diff_evaluator() -> impl GhEvaluator {
        FnEvaluator(|v: &[f64]| {
            Ok(GhEvaluation {
                objectives: vec![v[0] + v[1], v[0] - v[1]],
                constraints: vec![v[0] - 8.0],
                attributes: vec![Some(GhAttrValue::Number(v[0] * v[1]))],
            })
        })
    }

    #[test]
    fn random_sampler_records_all_trials() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

        assert_eq!(summary.completed, 6);
        assert_eq!(summary.failed, 0);
        assert!(!summary.cancelled);

        let data = std::fs::read(&journal).unwrap();
        let (meta, df, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.name, "gh-test");
        assert_eq!(meta.completed_trials, 6);
        assert_eq!(meta.objective_names, vec!["weight", "disp"]);
        assert_eq!(
            meta.directions,
            vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize
            ]
        );
        assert_eq!(extras.trials.len(), 6);

        // Consistency between the params and objective values recorded in the journal (obj0 = span + count)
        let span = df.get_numeric_column("span").unwrap().to_vec();
        let count = df.get_numeric_column("count").unwrap().to_vec();
        let weight = df.get_numeric_column("weight").unwrap().to_vec();
        for i in 0..df.row_count() {
            assert!((span[i] + count[i] - weight[i]).abs() < 1e-9);
            // Integer sliders produce integer values; real-valued sliders stay within range
            assert_eq!(count[i], count[i].round());
            assert!((1.0..=10.0).contains(&count[i]));
            assert!((3.0..=12.0).contains(&span[i]));
        }
        // param_bounds reflects the slider range
        assert_eq!(meta.param_bounds.get("span"), Some(&(3.0, 12.0)));

        // Constraints recorded via op9: c1 = span - 8, feasibility matches
        let c1 = df.get_numeric_column("c1").unwrap().to_vec();
        let feasible = df.get_numeric_column("is_feasible").unwrap().to_vec();
        for i in 0..df.row_count() {
            assert!((c1[i] - (span[i] - 8.0)).abs() < 1e-9);
            assert_eq!(feasible[i], if c1[i] <= 0.0 { 1.0 } else { 0.0 });
        }

        // Attributes recorded via op8 as a numeric user-attr column: area = span * count
        let area = df.get_numeric_column("area").unwrap().to_vec();
        for i in 0..df.row_count() {
            assert!((area[i] - span[i] * count[i]).abs() < 1e-9);
        }
    }

    #[test]
    fn constrained_penalty_fitness_orders_by_violation() {
        // Feasible: no penalty
        assert_eq!(constrained_penalty_fitness(2, &[-1.0, 0.0]), None);
        assert_eq!(constrained_penalty_fitness(2, &[]), None);
        // Infeasible: identical penalized value on every objective,
        // ordered by total violation (constrained-domination emulation)
        let a = constrained_penalty_fitness(2, &[0.5, -1.0]).unwrap();
        let b = constrained_penalty_fitness(2, &[2.0, 1.0]).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a[0], a[1]);
        assert!(a[0] < b[0], "less violation must rank better");
        assert!(a[0] > 1e11, "penalty must dominate any real objective");
    }

    /// The three penalty tiers must be strictly ordered: feasible objectives
    /// < any infeasible fitness < the evaluation-failure fitness. In
    /// particular a crashed solve must never rank better than a merely
    /// constraint-violating trial (that would steer NSGA-II toward crash
    /// regions).
    #[test]
    fn evaluation_failure_ranks_worse_than_any_infeasible_trial() {
        let worst_infeasible = constrained_penalty_fitness(1, &[f64::MAX]).unwrap();
        assert!(
            worst_infeasible[0] < FAIL_PENALTY,
            "infeasible fitness {} must stay below FAIL_PENALTY {}",
            worst_infeasible[0],
            FAIL_PENALTY
        );
        let mild_infeasible = constrained_penalty_fitness(1, &[1e-3]).unwrap();
        assert!(mild_infeasible[0] < FAIL_PENALTY);
    }

    /// Wrong constraint arity from an evaluator is recorded as FAIL instead of
    /// journaling misaligned constraint columns.
    #[test]
    fn constraint_arity_mismatch_is_recorded_as_fail() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        assert_eq!(problem.constraints.len(), 1);
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let wrong_arity = FnEvaluator(|v: &[f64]| {
            Ok(GhEvaluation {
                objectives: vec![v[0] + v[1], v[0] - v[1]],
                constraints: vec![0.0, 0.0], // problem has 1 constraint
                attributes: vec![Some(GhAttrValue::Number(1.0))],
            })
        });
        let summary = run_prepared(&prep, &problem, &wrong_arity, &cfg, &progress).unwrap();
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 6);
    }

    /// A non-finite constraint value must not be treated as feasible (f64::max
    /// ignores NaN) nor journaled as null — the trial is recorded as FAIL.
    #[test]
    fn non_finite_constraint_is_recorded_as_fail() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let nan_constraint = FnEvaluator(|v: &[f64]| {
            Ok(GhEvaluation {
                objectives: vec![v[0] + v[1], v[0] - v[1]],
                constraints: vec![f64::NAN],
                attributes: vec![Some(GhAttrValue::Number(1.0))],
            })
        });
        let summary = run_prepared(&prep, &problem, &nan_constraint, &cfg, &progress).unwrap();
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 6);
    }

    /// An empty attribute output (None) does not fail the trial; the trial
    /// completes with its objectives and simply records no value for that
    /// attribute.
    #[test]
    fn empty_attribute_does_not_fail_the_trial() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let empty_attr = FnEvaluator(|v: &[f64]| {
            Ok(GhEvaluation {
                objectives: vec![v[0] + v[1], v[0] - v[1]],
                constraints: vec![v[0] - 8.0],
                attributes: vec![None],
            })
        });
        let summary = run_prepared(&prep, &problem, &empty_attr, &cfg, &progress).unwrap();
        assert_eq!(summary.completed, 6);
        assert_eq!(summary.failed, 0);

        let data = std::fs::read(&journal).unwrap();
        let (_, df, _) = parse_single_study(&data, 0).unwrap();
        // No attribute column, but constraints are still recorded.
        assert!(df.get_numeric_column("area").is_none());
        assert!(df.get_numeric_column("c1").is_some());
    }

    #[test]
    fn nsga2_sampler_runs_expected_evaluations() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Nsga2);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

        // Population size rounded to even (4) x (1 generation + 1 initial) = 8 evaluations
        assert_eq!(summary.completed, 8);
        let snapshot = progress.snapshot();
        assert_eq!(snapshot.total, 8);
        assert_eq!(snapshot.done, 8);
    }

    #[test]
    fn evaluation_errors_are_recorded_as_fail() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let failing = FnEvaluator(|_: &[f64]| Err("solve failed".to_string()));
        let summary = run_prepared(&prep, &problem, &failing, &cfg, &progress).unwrap();

        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 6);

        let data = std::fs::read(&journal).unwrap();
        let (meta, df, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.completed_trials, 0);
        assert_eq!(meta.total_trials, 6);
        assert_eq!(df.row_count(), 0);
        assert!(extras
            .trials
            .iter()
            .all(|t| t.state == crate::data::extras::TrialState::Fail));
    }

    #[test]
    fn cancel_before_run_records_nothing() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        progress.request_cancel();
        let summary =
            run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

        assert!(summary.cancelled);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 0);
        let data = std::fs::read(&journal).unwrap();
        let (meta, _, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.total_trials, 0);
        assert!(extras.trials.is_empty());
    }

    #[test]
    fn direction_mismatch_is_rejected() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let mut cfg = test_cfg(GhSampler::Random);
        cfg.directions.pop();
        assert!(prepare_gh_run(&journal, &problem, &cfg).is_err());
    }
}
