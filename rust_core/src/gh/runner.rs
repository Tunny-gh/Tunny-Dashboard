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

use crate::io::journal::writer::JournalWriter;
use crate::math::rng::SeededRng;
use crate::surrogate_opt::optimizers::nsga2::{nsga2_minimize, Nsga2Config};
use crate::surrogate_opt::FitProgress;

use super::compute::GhEvaluator;
use super::problem::GhProblem;

mod config;
mod recorder;
mod summary;
mod validate;

#[cfg(test)]
mod tests;

pub use config::{GhRunConfig, GhSampler};
pub use summary::{GhIterationDiagnostic, GhRunSummary, GhStopReason};

pub(in crate::gh) use recorder::TrialRecorder;
pub(in crate::gh) use validate::{denormalize, round_variable};

use validate::normalize_current;

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

    let mut adaptive_diagnostics = Vec::new();
    let mut stop_reason = GhStopReason::Completed;

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
        GhSampler::Adaptive => {
            let outcome = super::adaptive::run_loop(&recorder, problem, cfg, progress)?;
            adaptive_diagnostics = outcome.diagnostics;
            stop_reason = outcome.stop_reason;
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
    // Cancellation overrides the sampler's own stop reason.
    if progress.is_cancelled() {
        stop_reason = GhStopReason::Cancelled;
    }
    Ok(GhRunSummary {
        study_id: prep.study_id,
        completed: recorder.completed.load(Ordering::Relaxed),
        failed: recorder.failed.load(Ordering::Relaxed),
        cancelled: progress.is_cancelled(),
        adaptive_diagnostics,
        stop_reason,
    })
}
