//! Self-contained optimization runner (no Python / Optuna at runtime).
//!
//! Drives a sampler loop over an arbitrary [`Evaluator`] and writes every trial
//! to an Optuna-compatible journal using the dashboard's own Rust writer, so an
//! optimization runs with **only the Dashboard and the target process** — the
//! samplers (Random / NSGA-II) are Rust implementations and the journal is just
//! a file format, with no dependency on the Optuna Python package.
//!
//! This generalizes the Grasshopper runner (`crate::gh::runner`) so any
//! evaluator — an external command via [`crate::process::ProcessEvaluator`], a
//! mock, or Rhino.Compute — can be optimized the same way. The problem is
//! described by real-valued [`Variable`] ranges; the runner normalizes to
//! `[0,1]^d` internally (the convention the samplers use) and denormalizes back
//! to real units (with per-variable rounding) before each evaluation.
//!
//! Usage mirrors the GH runner: [`prepare_run`] creates the study (so the
//! journal can be opened for live updates immediately), then [`run_prepared`]
//! blocks on the sampler loop.

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

/// Fitness returned to the sampler on evaluation failure / cancellation.
/// Strictly worse than any constraint-violating fitness so the search never
/// prefers a failing region over a merely infeasible one. A large finite value
/// (infinity would produce NaN in crowding-distance normalization).
const FAIL_PENALTY: f64 = 1e15;
/// Base fitness for a constraint-violating trial.
const CONSTRAINT_PENALTY_BASE: f64 = 1e12;
/// Cap on the violation added to the base, keeping infeasible fitness `<` `FAIL_PENALTY`.
const MAX_COUNTED_VIOLATION: f64 = 1e14;

/// A per-trial attribute value recorded as an Optuna trial user attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Number(f64),
    Text(String),
}

/// An optimization variable (real-valued, with slider-style rounding).
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// Journal parameter name.
    pub name: String,
    pub low: f64,
    pub high: f64,
    /// Starting value used to seed NSGA-II's initial individual (clamped into range).
    pub value: f64,
    /// Decimal digits for rounding (ignored when `is_integer`).
    pub digits: u32,
    /// Whether the value is rounded to an integer.
    pub is_integer: bool,
}

impl Variable {
    /// A continuous variable spanning `[low, high]` with `digits` decimals,
    /// seeded at the midpoint.
    pub fn float(name: impl Into<String>, low: f64, high: f64, digits: u32) -> Self {
        Self {
            name: name.into(),
            low,
            high,
            value: 0.5 * (low + high),
            digits,
            is_integer: false,
        }
    }

    /// An integer variable spanning `[low, high]`, seeded at the midpoint.
    pub fn integer(name: impl Into<String>, low: f64, high: f64) -> Self {
        Self {
            name: name.into(),
            low,
            high,
            value: ((low + high) * 0.5).round(),
            digits: 0,
            is_integer: true,
        }
    }
}

/// The optimization problem: variable ranges plus the names of objectives,
/// constraints, and per-trial attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub variables: Vec<Variable>,
    pub objective_names: Vec<String>,
    /// Feasible when every constraint value is `<= 0` (empty = unconstrained).
    pub constraint_names: Vec<String>,
    pub attribute_names: Vec<String>,
}

/// Result of evaluating one trial.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub objectives: Vec<f64>,
    /// Constraint values (feasible when every value `<= 0`; empty = unconstrained).
    pub constraints: Vec<f64>,
    /// Per-trial attribute values, `None` for an empty output.
    pub attributes: Vec<Option<AttrValue>>,
}

/// Evaluator for the real objective function. `values` are real-unit parameter
/// values in the problem's variable order.
pub trait Evaluator: Sync {
    fn evaluate(&self, values: &[f64]) -> Result<Evaluation, String>;
}

/// The kind of sampler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sampler {
    /// Uniform random sampling (trial count = `n_trials`).
    Random,
    /// NSGA-II (trial count = even population × (generations + 1)).
    Nsga2,
}

/// Configuration for a run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub study_name: String,
    /// Direction per objective (same count/order as `Problem.objective_names`).
    pub directions: Vec<OptimizationDirection>,
    pub sampler: Sampler,
    pub n_trials: usize,
    pub population_size: usize,
    pub generations: usize,
    pub seed: u64,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            study_name: "optimization".to_string(),
            directions: Vec::new(),
            sampler: Sampler::Nsga2,
            n_trials: 50,
            population_size: 16,
            generations: 10,
            seed: 42,
        }
    }
}

/// Summary of a run.
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub study_id: u32,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: bool,
}

/// A prepared run: the journal writer with the study already created.
pub struct PreparedRun {
    writer: Mutex<JournalWriter>,
    study_id: u32,
}

impl PreparedRun {
    pub fn study_id(&self) -> u32 {
        self.study_id
    }
}

/// Opens the journal and creates the study (synchronous, lightweight). The
/// study exists immediately, so the caller can open the journal for live
/// updates before [`run_prepared`] starts.
pub fn prepare_run(
    journal_path: &Path,
    problem: &Problem,
    cfg: &RunConfig,
) -> Result<PreparedRun, String> {
    if cfg.directions.len() != problem.objective_names.len() {
        return Err(format!(
            "directions ({}) do not match objectives ({})",
            cfg.directions.len(),
            problem.objective_names.len()
        ));
    }
    if problem.variables.is_empty() {
        return Err("no variables".to_string());
    }
    let mut writer = JournalWriter::open(journal_path)?;
    let study_id =
        writer.create_study(&cfg.study_name, &cfg.directions, &problem.objective_names)?;
    Ok(PreparedRun {
        writer: Mutex::new(writer),
        study_id,
    })
}

/// The main optimization loop (blocking; call from a background thread). Every
/// trial is written to the journal; evaluation failures are recorded as FAIL
/// and fed a penalty fitness so the sampler continues. Returns `Err` only when
/// writing to the journal fails.
pub fn run_prepared(
    prep: &PreparedRun,
    problem: &Problem,
    evaluator: &dyn Evaluator,
    cfg: &RunConfig,
    progress: &FitProgress,
) -> Result<RunSummary, String> {
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
    progress.set_stage("Evaluating");

    match cfg.sampler {
        Sampler::Random => {
            let n = cfg.n_trials.max(1);
            progress.set_total(n);
            (0..n).into_par_iter().for_each(|i| {
                let mut rng = SeededRng::from_seed(
                    cfg.seed
                        .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
                );
                let x: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
                recorder.eval_signed(&x);
            });
        }
        Sampler::Nsga2 => {
            let pop_even = (cfg.population_size.max(4) + 1) & !1;
            progress.set_total(pop_even * (cfg.generations + 1));
            let nsga_cfg = Nsga2Config {
                pop_size: cfg.population_size,
                generations: cfg.generations,
                seed: cfg.seed,
                ..Nsga2Config::for_objectives(cfg.directions.len())
            };
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
            "aborted because writing to the journal failed: {e}"
        ));
    }
    Ok(RunSummary {
        study_id: prep.study_id,
        completed: recorder.completed.load(Ordering::Relaxed),
        failed: recorder.failed.load(Ordering::Relaxed),
        cancelled: progress.is_cancelled(),
    })
}

/// Penalty fitness for a constraint-violating trial (see the GH runner for the
/// rationale of the three strict tiers). `None` when feasible.
fn constrained_penalty_fitness(n_obj: usize, constraints: &[f64]) -> Option<Vec<f64>> {
    let violation: f64 = constraints.iter().map(|c| c.max(0.0)).sum();
    (violation > 0.0)
        .then(|| vec![CONSTRAINT_PENALTY_BASE + violation.min(MAX_COUNTED_VIOLATION); n_obj])
}

/// Validates an evaluation before recording (arity + finiteness).
fn validate_evaluation(eval: &Evaluation, problem: &Problem) -> Option<String> {
    if eval.constraints.len() != problem.constraint_names.len() {
        return Some(format!(
            "constraint count mismatch (expected {}, got {})",
            problem.constraint_names.len(),
            eval.constraints.len()
        ));
    }
    if eval.attributes.len() != problem.attribute_names.len() {
        return Some(format!(
            "attribute count mismatch (expected {}, got {})",
            problem.attribute_names.len(),
            eval.attributes.len()
        ));
    }
    if eval.objectives.iter().any(|v| !v.is_finite()) {
        return Some("objective value is not finite".to_string());
    }
    if eval.constraints.iter().any(|c| !c.is_finite()) {
        return Some("constraint value is not finite".to_string());
    }
    None
}

/// Evaluates one trial and records it to the journal. Shared across parallel
/// evaluation threads.
struct TrialRecorder<'a> {
    writer: &'a Mutex<JournalWriter>,
    study_id: u32,
    problem: &'a Problem,
    directions: &'a [OptimizationDirection],
    evaluator: &'a dyn Evaluator,
    progress: &'a FitProgress,
    completed: AtomicUsize,
    failed: AtomicUsize,
    io_error: Mutex<Option<String>>,
}

impl TrialRecorder<'_> {
    /// Evaluates a normalized point, records the trial, and returns the
    /// minimize-convention fitness for the sampler.
    fn eval_signed(&self, x_norm: &[f64]) -> Vec<f64> {
        let n_obj = self.directions.len();
        if self.progress.is_cancelled() || self.has_io_error() {
            return vec![FAIL_PENALTY; n_obj];
        }
        let values = denormalize(self.problem, x_norm);
        match self.evaluate_and_record(&values) {
            Some(eval) => {
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
            None => vec![FAIL_PENALTY; n_obj],
        }
    }

    /// Evaluates real-unit values, records COMPLETE (with constraints/attrs) or
    /// FAIL, and returns the evaluation on success (`None` on any non-success).
    fn evaluate_and_record(&self, values: &[f64]) -> Option<Evaluation> {
        let n_obj = self.directions.len();
        if self.progress.is_cancelled() || self.has_io_error() {
            return None;
        }
        let trial_id = match self.begin_trial(values) {
            Ok(id) => id,
            Err(e) => {
                self.set_io_error(e);
                return None;
            }
        };
        match self.evaluator.evaluate(values) {
            Ok(eval) if eval.objectives.len() == n_obj => {
                if let Some(reason) = validate_evaluation(&eval, self.problem) {
                    self.record_failure(trial_id, reason);
                    return None;
                }
                if let Err(e) = self.finish_complete(trial_id, &eval) {
                    self.set_io_error(e);
                    return None;
                }
                self.completed.fetch_add(1, Ordering::Relaxed);
                self.progress.inc_done();
                Some(eval)
            }
            Ok(eval) => {
                self.record_failure(
                    trial_id,
                    format!(
                        "objective count mismatch (expected {n_obj}, got {})",
                        eval.objectives.len()
                    ),
                );
                None
            }
            Err(e) => {
                self.record_failure(trial_id, e);
                None
            }
        }
    }

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

    fn finish_fail(&self, trial_id: u32) -> Result<(), String> {
        self.writer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .finish_trial(trial_id, TrialState::Fail, &[])
    }

    fn finish_complete(&self, trial_id: u32, eval: &Evaluation) -> Result<(), String> {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if !eval.constraints.is_empty() {
            writer.set_trial_constraints(trial_id, &eval.constraints)?;
        }
        for (attr, value) in self.problem.attribute_names.iter().zip(&eval.attributes) {
            let json = match value {
                Some(AttrValue::Number(v)) if v.is_finite() => serde_json::json!(v),
                Some(AttrValue::Number(_)) | None => continue,
                Some(AttrValue::Text(s)) => serde_json::json!(s),
            };
            writer.set_trial_user_attr(trial_id, attr, &json)?;
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

/// Converts a normalized point `[0,1]^d` into real values, applying per-variable rounding.
fn denormalize(problem: &Problem, x_norm: &[f64]) -> Vec<f64> {
    problem
        .variables
        .iter()
        .zip(x_norm)
        .map(|(var, x)| {
            let x = x.clamp(0.0, 1.0);
            round_variable(var, var.low + x * (var.high - var.low))
        })
        .collect()
}

/// Maps the variables' starting values into normalized space (NSGA-II seed).
fn normalize_current(problem: &Problem) -> Vec<f64> {
    problem
        .variables
        .iter()
        .map(|var| {
            if var.high > var.low {
                ((var.value - var.low) / (var.high - var.low)).clamp(0.0, 1.0)
            } else {
                0.0
            }
        })
        .collect()
}

fn round_variable(var: &Variable, raw: f64) -> f64 {
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
    use crate::io::journal::parser::parse_single_study;

    struct FnEvaluator<F: Fn(&[f64]) -> Result<Evaluation, String> + Sync>(F);
    impl<F: Fn(&[f64]) -> Result<Evaluation, String> + Sync> Evaluator for FnEvaluator<F> {
        fn evaluate(&self, values: &[f64]) -> Result<Evaluation, String> {
            (self.0)(values)
        }
    }

    fn two_var_problem() -> Problem {
        Problem {
            variables: vec![
                Variable::float("x", 0.0, 10.0, 2),
                Variable::integer("n", 1.0, 5.0),
            ],
            objective_names: vec!["f".to_string()],
            constraint_names: vec!["g".to_string()],
            attribute_names: vec!["area".to_string()],
        }
    }

    fn cfg(sampler: Sampler) -> RunConfig {
        RunConfig {
            study_name: "generic".to_string(),
            directions: vec![OptimizationDirection::Minimize],
            sampler,
            n_trials: 8,
            population_size: 4,
            generations: 1,
            seed: 3,
        }
    }

    #[test]
    fn random_run_records_all_trials_to_journal() {
        let problem = two_var_problem();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let eval = FnEvaluator(|v: &[f64]| {
            Ok(Evaluation {
                objectives: vec![(v[0] - 5.0).powi(2) + v[1]],
                constraints: vec![v[0] - 8.0],
                attributes: vec![Some(AttrValue::Number(v[0] * v[1]))],
            })
        });
        let prep = prepare_run(&journal, &problem, &cfg(Sampler::Random)).unwrap();
        let progress = FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &eval, &cfg(Sampler::Random), &progress).unwrap();
        assert_eq!(summary.completed, 8);
        assert_eq!(summary.failed, 0);

        let data = std::fs::read(&journal).unwrap();
        let (meta, df, _) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.completed_trials, 8);
        // Params respect ranges/rounding: n is integer, x has 2 decimals.
        let x = df.get_numeric_column("x").unwrap();
        let n = df.get_numeric_column("n").unwrap();
        for i in 0..df.row_count() {
            assert!((0.0..=10.0).contains(&x[i]));
            assert_eq!(n[i], n[i].round());
            assert!((1.0..=5.0).contains(&n[i]));
        }
        // Constraints and attributes recorded.
        assert!(df.get_numeric_column("c1").is_some());
        assert!(df.get_numeric_column("area").is_some());
    }

    #[test]
    fn nsga2_run_completes_and_records() {
        let problem = two_var_problem();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let eval = FnEvaluator(|v: &[f64]| {
            Ok(Evaluation {
                objectives: vec![v[0] + v[1]],
                constraints: vec![],
                attributes: vec![],
            })
        });
        let mut problem = problem;
        problem.constraint_names.clear();
        problem.attribute_names.clear();
        let prep = prepare_run(&journal, &problem, &cfg(Sampler::Nsga2)).unwrap();
        let progress = FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &eval, &cfg(Sampler::Nsga2), &progress).unwrap();
        // even(4) x (1 gen + 1) = 8 evaluations.
        assert_eq!(summary.completed, 8);
    }

    #[test]
    fn evaluation_errors_recorded_as_fail() {
        let problem = two_var_problem();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let eval = FnEvaluator(|_: &[f64]| Err("boom".to_string()));
        let prep = prepare_run(&journal, &problem, &cfg(Sampler::Random)).unwrap();
        let progress = FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &eval, &cfg(Sampler::Random), &progress).unwrap();
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 8);
    }
}
