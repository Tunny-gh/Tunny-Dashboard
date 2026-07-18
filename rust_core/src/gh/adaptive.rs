//! Adaptive surrogate loop for Grasshopper runs (ROADMAP Phase 2C, item 16).
//!
//! Closes the "suggest → evaluate → refit" loop automatically: after a random
//! bootstrap phase, each iteration fits a surrogate per objective on all
//! successful evaluations so far, asks the acquisition function for the next
//! batch of candidates (Expected Improvement for single-objective runs, EHVI
//! for multi-objective runs), evaluates them through the same
//! `TrialRecorder` as the other samplers (so every trial lands in the journal
//! and streams into the live view), and repeats — the commercial "adaptive
//! sampling" workflow driven entirely by the dashboard.
//!
//! Design notes:
//! - Surrogates are fitted with automatic model selection (the same Auto
//!   candidates as the analysis widgets) on real-unit parameters, normalized
//!   internally via the sliders' declared ranges.
//! - Constraint models are attached for single-objective runs, so EI accounts
//!   for the feasibility probability. EHVI does not use constraint models
//!   (multi-objective feasibility steering is future work); infeasible trials
//!   still record their constraint values for the analysis side.
//! - Suggested candidates are rounded to the sliders' resolution first; a
//!   candidate that duplicates an already-evaluated point (or another
//!   candidate in the same batch) is dropped. If a whole batch dedupes away,
//!   the loop stops early — the acquisition function has no new points to
//!   propose at slider resolution.

use rayon::prelude::*;

use crate::io::journal::parser::OptimizationDirection;
use crate::math::rng::SeededRng;
use crate::surrogate_opt::{
    fit_surrogate_with_validation_tracked, suggest_candidates, suggest_candidates_multi,
    AcquisitionKind, ConstraintData, FitProgress, SurrogateFitRequest, SurrogateModelKind,
    TrainedSurrogate, AUTO_CANDIDATES, MIN_TRIALS_FOR_SURROGATE_OPT,
};

use super::problem::GhProblem;
use super::runner::{denormalize, round_variable, GhRunConfig, TrialRecorder};

/// Tolerance for treating two rounded parameter vectors as the same point.
const DEDUPE_EPS: f64 = 1e-12;

/// One successful evaluation: (parameter values, objective values, constraint values).
type EvaluatedPoint = (Vec<f64>, Vec<f64>, Vec<f64>);

/// Successful evaluations accumulated across the run (the surrogate's
/// training set). Rows are aligned: `xs[i]` produced `ys[i]` and `cons[i]`.
#[derive(Default)]
struct Dataset {
    xs: Vec<Vec<f64>>,
    /// Objective values per trial (inner length = objective count).
    ys: Vec<Vec<f64>>,
    /// Constraint values per trial (inner length = constraint count).
    cons: Vec<Vec<f64>>,
}

impl Dataset {
    fn push(&mut self, x: Vec<f64>, objectives: Vec<f64>, constraints: Vec<f64>) {
        self.xs.push(x);
        self.ys.push(objectives);
        self.cons.push(constraints);
    }

    fn contains(&self, point: &[f64]) -> bool {
        self.xs.iter().any(|x| points_equal(x, point))
    }
}

fn points_equal(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(lhs, rhs)| (lhs - rhs).abs() <= DEDUPE_EPS)
}

/// Runs the adaptive loop. Trials are recorded through `recorder` exactly like
/// the Random / NSGA-II samplers; `Err` is returned only for setup-level
/// failures (surrogate fitting / suggestion errors), after which the journal
/// keeps whatever trials were already recorded.
pub(super) fn run_loop(
    recorder: &TrialRecorder<'_>,
    problem: &GhProblem,
    cfg: &GhRunConfig,
    progress: &FitProgress,
) -> Result<(), String> {
    let n_dims = problem.variables.len();
    let n_obj = cfg.directions.len();
    // Surrogate fitting requires MIN_TRIALS_FOR_SURROGATE_OPT successful
    // points, so the bootstrap phase never samples fewer (a handful of extra
    // random trials is cheap next to a stalled fit).
    let initial = cfg.adaptive_initial.max(MIN_TRIALS_FOR_SURROGATE_OPT);
    let batch = cfg.adaptive_batch.max(1);
    let iterations = cfg.adaptive_iterations;
    progress.set_total(initial + batch * iterations);

    let minimize: Vec<bool> = cfg
        .directions
        .iter()
        .map(|d| matches!(d, OptimizationDirection::Minimize))
        .collect();
    let param_names: Vec<String> = problem.variables.iter().map(|v| v.name.clone()).collect();
    let param_bounds: Vec<Option<(f64, f64)>> = problem
        .variables
        .iter()
        .map(|v| Some((v.low, v.high)))
        .collect();

    // ── Bootstrap: random sampling (same per-trial seed derivation as the
    //    Random sampler, offset so the two samplers don't reuse points) ──────
    progress.set_stage(format!("Adaptive: random bootstrap ({initial} trials)"));
    let mut data = Dataset::default();
    let bootstrap: Vec<Option<EvaluatedPoint>> = (0..initial)
        .into_par_iter()
        .map(|i| {
            let mut rng = SeededRng::from_seed(
                cfg.seed
                    .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
            );
            let x_norm: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
            let values = denormalize(problem, &x_norm);
            recorder
                .evaluate_and_record(&values)
                .map(|eval| (values, eval.objectives, eval.constraints))
        })
        .collect();
    for entry in bootstrap.into_iter().flatten() {
        let (x, y, c) = entry;
        // Duplicate rounded points can occur at coarse slider resolution;
        // keep only the first so the fit matrix stays well-conditioned.
        if !data.contains(&x) {
            data.push(x, y, c);
        }
    }

    // ── Iterate: fit → suggest → evaluate ──────────────────────────────────
    for iteration in 1..=iterations {
        if progress.is_cancelled() {
            return Ok(());
        }
        if data.xs.len() < MIN_TRIALS_FOR_SURROGATE_OPT {
            return Err(format!(
                "Adaptive loop needs at least {MIN_TRIALS_FOR_SURROGATE_OPT} successful \
                 evaluations to fit a surrogate ({} succeeded so far)",
                data.xs.len()
            ));
        }

        progress.set_stage(format!(
            "Adaptive: fitting surrogate (iteration {iteration}/{iterations}, {} trials)",
            data.xs.len()
        ));
        let trained = fit_objective_surrogates(problem, cfg, &data, &param_names, &param_bounds)?;

        let suggested: Vec<Vec<f64>> = if n_obj == 1 {
            suggest_candidates(
                &trained[0],
                batch,
                AcquisitionKind::ExpectedImprovement,
                minimize[0],
            )?
            .into_iter()
            .map(|c| c.params)
            .collect()
        } else {
            suggest_candidates_multi(&trained, &minimize, batch)?
                .into_iter()
                .map(|c| c.params)
                .collect()
        };

        // Round to slider resolution, then drop points we already evaluated
        // (or that repeat within the batch).
        let mut batch_points: Vec<Vec<f64>> = Vec::with_capacity(suggested.len());
        for mut point in suggested {
            for (value, var) in point.iter_mut().zip(&problem.variables) {
                *value = round_variable(var, value.clamp(var.low, var.high));
            }
            if !data.contains(&point) && !batch_points.iter().any(|p| points_equal(p, &point)) {
                batch_points.push(point);
            }
        }
        if batch_points.is_empty() {
            progress.set_stage(format!(
                "Adaptive: stopped after iteration {iteration}/{iterations} \
                 (no new candidates at slider resolution)"
            ));
            return Ok(());
        }

        progress.set_stage(format!(
            "Adaptive: evaluating {} candidates (iteration {iteration}/{iterations})",
            batch_points.len()
        ));
        let results: Vec<Option<EvaluatedPoint>> = batch_points
            .into_par_iter()
            .map(|values| {
                recorder
                    .evaluate_and_record(&values)
                    .map(|eval| (values, eval.objectives, eval.constraints))
            })
            .collect();
        for entry in results.into_iter().flatten() {
            let (x, y, c) = entry;
            data.push(x, y, c);
        }
    }
    Ok(())
}

/// Fits one surrogate per objective with automatic model selection. Constraint
/// models are attached only for single-objective runs (EI uses them; EHVI does
/// not).
fn fit_objective_surrogates(
    problem: &GhProblem,
    cfg: &GhRunConfig,
    data: &Dataset,
    param_names: &[String],
    param_bounds: &[Option<(f64, f64)>],
) -> Result<Vec<TrainedSurrogate>, String> {
    let n_obj = cfg.directions.len();
    // ConstraintData is not Clone; rebuild the vector per objective fit.
    let build_constraints = || -> Vec<ConstraintData> {
        if n_obj == 1 {
            problem
                .constraints
                .iter()
                .enumerate()
                .map(|(ci, con)| ConstraintData {
                    name: con.name.clone(),
                    values: data.cons.iter().map(|row| row[ci]).collect(),
                })
                .collect()
        } else {
            Vec::new()
        }
    };

    let mut trained = Vec::with_capacity(n_obj);
    for (k, objective) in problem.objectives.iter().enumerate() {
        let req = SurrogateFitRequest {
            x_matrix: data.xs.clone(),
            y: data.ys.iter().map(|row| row[k]).collect(),
            param_names: param_names.to_vec(),
            objective_name: objective.name.clone(),
            // Ignored when auto_select is true; any Auto candidate works as
            // the placeholder.
            model: default_model_kind(),
            auto_select: true,
            constraints: build_constraints(),
            priority_rows: vec![],
            param_bounds: Some(param_bounds.to_vec()),
        };
        // Fitting uses its own progress handle: the shared one tracks trial
        // evaluations, and the fit routine would overwrite its totals.
        trained.push(
            fit_surrogate_with_validation_tracked(&req, &FitProgress::new())
                .map_err(|e| format!("Surrogate fit failed for \"{}\": {e}", objective.name))?,
        );
    }
    Ok(trained)
}

fn default_model_kind() -> SurrogateModelKind {
    AUTO_CANDIDATES[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::extras::TrialState;
    use crate::gh::compute::{GhAttrValue, GhEvaluation, GhEvaluator};
    use crate::gh::fixtures::sample_ghx;
    use crate::gh::problem::extract_problem;
    use crate::gh::runner::{prepare_gh_run, run_prepared, GhSampler};
    use crate::io::journal::parser::parse_single_study;

    struct FnEvaluator<F: Fn(&[f64]) -> Result<GhEvaluation, String> + Send + Sync>(F);

    impl<F: Fn(&[f64]) -> Result<GhEvaluation, String> + Send + Sync> GhEvaluator for FnEvaluator<F> {
        fn evaluate(&self, values: &[f64]) -> Result<GhEvaluation, String> {
            (self.0)(values)
        }
    }

    fn adaptive_cfg(directions: Vec<OptimizationDirection>) -> GhRunConfig {
        GhRunConfig {
            study_name: "adaptive-test".to_string(),
            directions,
            sampler: GhSampler::Adaptive,
            adaptive_initial: 12,
            adaptive_batch: 2,
            adaptive_iterations: 2,
            seed: 11,
            ..GhRunConfig::default()
        }
    }

    /// Fixture-compatible evaluator: 2 objectives, 1 constraint, 1 attribute.
    fn quadratic_evaluator() -> impl GhEvaluator {
        FnEvaluator(|v: &[f64]| {
            let span = v[0];
            let count = v[1];
            Ok(GhEvaluation {
                objectives: vec![(span - 7.0).powi(2) + count, span + count],
                constraints: vec![span - 11.0],
                attributes: vec![Some(GhAttrValue::Number(span * count))],
            })
        })
    }

    /// Multi-objective adaptive run: bootstrap + 2 iterations complete, every
    /// trial is journaled COMPLETE with params/constraints, and the summary
    /// matches the journal.
    #[test]
    fn adaptive_multi_objective_records_all_trials() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = adaptive_cfg(vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Minimize,
        ]);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = crate::surrogate_opt::FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &quadratic_evaluator(), &cfg, &progress).unwrap();

        assert_eq!(summary.failed, 0);
        assert!(!summary.cancelled);
        // Bootstrap trials always run; each iteration adds at most `batch`
        // (dedupe may drop candidates or stop the loop early).
        assert!(summary.completed >= cfg.adaptive_initial);
        assert!(
            summary.completed
                <= cfg.adaptive_initial + cfg.adaptive_batch * cfg.adaptive_iterations
        );

        let data = std::fs::read(&journal).unwrap();
        let (meta, df, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.completed_trials, summary.completed as u32);
        assert_eq!(df.row_count(), summary.completed);
        assert!(df.get_numeric_column("c1").is_some());
        assert!(extras
            .trials
            .iter()
            .all(|t| t.state == TrialState::Complete));
    }

    /// Single-objective adaptive run (EI with a constraint model) completes and
    /// the suggested points stay within the slider ranges/resolution.
    #[test]
    fn adaptive_single_objective_respects_slider_ranges() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = adaptive_cfg(vec![OptimizationDirection::Minimize]);

        let single_obj = FnEvaluator(|v: &[f64]| {
            Ok(GhEvaluation {
                objectives: vec![(v[0] - 7.0).powi(2) + v[1]],
                constraints: vec![v[0] - 11.0],
                attributes: vec![None],
            })
        });
        // The fixture declares 2 objectives; restrict the problem to one so
        // directions/objectives lengths agree.
        let mut problem = problem;
        problem.objectives.truncate(1);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = crate::surrogate_opt::FitProgress::new();
        let summary = run_prepared(&prep, &problem, &single_obj, &cfg, &progress).unwrap();
        assert!(summary.completed >= cfg.adaptive_initial);

        let data = std::fs::read(&journal).unwrap();
        let (_, df, _) = parse_single_study(&data, 0).unwrap();
        let span = df.get_numeric_column("span").unwrap();
        let count = df.get_numeric_column("count").unwrap();
        for i in 0..df.row_count() {
            assert!((3.0..=12.0).contains(&span[i]), "span out of range");
            assert!((1.0..=10.0).contains(&count[i]), "count out of range");
            // Slider resolution: span has 2 digits, count is an integer.
            assert!(((span[i] * 100.0).round() / 100.0 - span[i]).abs() < 1e-9);
            assert_eq!(count[i], count[i].round());
        }
    }

    /// Cancellation before the run starts records nothing and reports
    /// cancelled (same contract as the other samplers).
    #[test]
    fn adaptive_cancel_before_run_records_nothing() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = adaptive_cfg(vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Minimize,
        ]);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = crate::surrogate_opt::FitProgress::new();
        progress.request_cancel();
        let summary =
            run_prepared(&prep, &problem, &quadratic_evaluator(), &cfg, &progress).unwrap();
        assert!(summary.cancelled);
        assert_eq!(summary.completed, 0);
    }

    /// If every bootstrap evaluation fails, the loop aborts with a clear error
    /// instead of trying to fit on an empty dataset.
    #[test]
    fn adaptive_reports_error_when_bootstrap_fails() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = adaptive_cfg(vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Minimize,
        ]);

        let failing = FnEvaluator(|_: &[f64]| Err("solve failed".to_string()));
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = crate::surrogate_opt::FitProgress::new();
        let err = run_prepared(&prep, &problem, &failing, &cfg, &progress).unwrap_err();
        assert!(err.contains("successful"), "unexpected error: {err}");
    }
}
