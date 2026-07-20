//! Per-trial evaluation and journal recording (`TrialRecorder`), shared across
//! the built-in samplers (via `super::run_prepared`) and the adaptive loop
//! (`crate::gh::adaptive`).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::data::extras::TrialState;
use crate::gh::compute::{GhEvaluation, GhEvaluator};
use crate::gh::problem::GhProblem;
use crate::io::journal::parser::OptimizationDirection;
use crate::io::journal::writer::{JournalWriter, ParamDistribution};
use crate::surrogate_opt::FitProgress;

use super::validate::{
    constrained_penalty_fitness, denormalize, validate_evaluation, FAIL_PENALTY,
};

/// Evaluates a single trial and records it to the journal. Shared across parallel
/// evaluation threads and reused by the adaptive loop (`crate::gh::adaptive`).
pub(in crate::gh) struct TrialRecorder<'a> {
    pub(super) writer: &'a Mutex<JournalWriter>,
    pub(super) study_id: u32,
    pub(super) problem: &'a GhProblem,
    pub(super) directions: &'a [OptimizationDirection],
    pub(super) evaluator: &'a dyn GhEvaluator,
    pub(super) progress: &'a FitProgress,
    pub(super) completed: AtomicUsize,
    pub(super) failed: AtomicUsize,
    /// Journal write error (the first one). Once set, no new evaluations are started.
    pub(super) io_error: Mutex<Option<String>>,
}

impl TrialRecorder<'_> {
    /// Evaluates a normalized point and returns objective values sign-adjusted to the minimize convention.
    pub(super) fn eval_signed(&self, x_norm: &[f64]) -> Vec<f64> {
        let n_obj = self.directions.len();
        if self.progress.is_cancelled() || self.has_io_error() {
            return vec![FAIL_PENALTY; n_obj];
        }
        let values = denormalize(self.problem, x_norm);

        match self.evaluate_and_record(&values) {
            Some(eval) => {
                // Constraint-violating trials feed a penalty fitness to the
                // algorithm (see constrained_penalty_fitness); the journal
                // record keeps the real objective values.
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

    /// Evaluates real-unit parameter values, records the trial to the journal
    /// (COMPLETE with constraints/attributes, or FAIL), and returns the
    /// evaluation on success. `None` covers every non-success: cancellation,
    /// journal I/O errors, evaluator errors, and invalid evaluations (wrong
    /// arity / non-finite values — see `validate_evaluation`). Shared by the
    /// samplers in this module and the adaptive loop.
    pub(in crate::gh) fn evaluate_and_record(&self, values: &[f64]) -> Option<GhEvaluation> {
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
                // Validate the evaluation the same way the objective count is
                // validated: wrong constraint/attribute arity or non-finite
                // values would journal misaligned columns or silently vanish
                // (serde_json writes non-finite f64 as null), so record FAIL
                // instead. ComputeEvaluator guarantees all of this; the checks
                // protect other GhEvaluator implementations.
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
                        "Objective count mismatch (expected {n_obj}, got {})",
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
    fn finish_complete(&self, trial_id: u32, eval: &GhEvaluation) -> Result<(), String> {
        use crate::gh::compute::GhAttrValue;

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
