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
use crate::multi_objective::pareto::hypervolume_nd;
use crate::surrogate_opt::{
    fit_surrogate_with_validation_tracked, suggest_candidates, suggest_candidates_multi,
    AcquisitionKind, ConstraintData, FitProgress, SurrogateFitRequest, SurrogateModelKind,
    TrainedSurrogate, AUTO_CANDIDATES, MIN_TRIALS_FOR_SURROGATE_OPT,
};

use super::problem::GhProblem;
use super::runner::{
    denormalize, round_variable, GhIterationDiagnostic, GhRunConfig, GhStopReason, TrialRecorder,
};

/// Tolerance for treating two rounded parameter vectors as the same point.
const DEDUPE_EPS: f64 = 1e-12;

/// Fractional margin added to each objective's observed range when fixing the
/// hypervolume reference point (so the initial feasible front has positive HV).
const REF_MARGIN: f64 = 0.1;

/// Fallback margin when an objective's observed range is zero.
const REF_MIN_MARGIN: f64 = 1.0;

/// One successful evaluation: (parameter values, objective values, constraint values).
type EvaluatedPoint = (Vec<f64>, Vec<f64>, Vec<f64>);

/// Result of the adaptive loop, threaded into `GhRunSummary`.
pub(super) struct AdaptiveOutcome {
    pub diagnostics: Vec<GhIterationDiagnostic>,
    pub stop_reason: GhStopReason,
}

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
/// keeps whatever trials were already recorded. The returned outcome carries
/// per-iteration diagnostics and why the loop stopped.
pub(super) fn run_loop(
    recorder: &TrialRecorder<'_>,
    problem: &GhProblem,
    cfg: &GhRunConfig,
    progress: &FitProgress,
) -> Result<AdaptiveOutcome, String> {
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

    // Fix the hypervolume reference point once from the bootstrap data, so the
    // convergence metric is monotonically non-decreasing as trials are added
    // (a moving reference could make HV drop and mislead the stopping check).
    let ref_point = fixed_reference(&data, &minimize);
    let mut diagnostics = Vec::with_capacity(iterations + 1);
    let mut prev_metric = convergence_metric(&data, &minimize, &ref_point);
    diagnostics.push(GhIterationDiagnostic {
        iteration: 0,
        trials_completed: data.xs.len(),
        metric: prev_metric,
        relative_improvement: f64::INFINITY,
    });

    let mut low_improvement_streak = 0usize;
    let mut stop_reason = GhStopReason::Completed;

    // ── Iterate: fit → suggest → evaluate → measure ─────────────────────────
    for iteration in 1..=iterations {
        if progress.is_cancelled() {
            stop_reason = GhStopReason::Cancelled;
            break;
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
            stop_reason = GhStopReason::NoNewCandidates;
            break;
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

        // ── Diagnostics + convergence check ─────────────────────────────────
        let metric = convergence_metric(&data, &minimize, &ref_point);
        let relative_improvement = relative_improvement(prev_metric, metric);
        diagnostics.push(GhIterationDiagnostic {
            iteration,
            trials_completed: data.xs.len(),
            metric,
            relative_improvement,
        });
        progress.set_stage(format!(
            "Adaptive: iteration {iteration}/{iterations} done \
             (metric {metric:.4}, +{:.2}%)",
            relative_improvement * 100.0
        ));

        // Only count low-improvement iterations once a feasible front exists
        // (metric > 0), so a run that has not yet found any feasible point
        // keeps exploring instead of stopping on a flat zero metric.
        if cfg.adaptive_patience > 0 && metric > 0.0 {
            if relative_improvement < cfg.adaptive_min_improvement {
                low_improvement_streak += 1;
                if low_improvement_streak >= cfg.adaptive_patience {
                    stop_reason = GhStopReason::Converged;
                    progress.set_stage(format!(
                        "Adaptive: converged after iteration {iteration}/{iterations} \
                         ({low_improvement_streak} iterations below \
                         {:.1}% improvement)",
                        cfg.adaptive_min_improvement * 100.0
                    ));
                    break;
                }
            } else {
                low_improvement_streak = 0;
            }
        }
        prev_metric = metric;
    }

    Ok(AdaptiveOutcome {
        diagnostics,
        stop_reason,
    })
}

/// A point is feasible when every constraint value is `<= 0` (Tunny's
/// convention). No constraints ⇒ always feasible.
fn is_feasible(constraints: &[f64]) -> bool {
    constraints.iter().all(|&c| c <= 0.0)
}

/// Objective values converted to the all-minimize convention used by the
/// hypervolume routine (maximize objectives are negated).
fn to_minimized(objectives: &[f64], minimize: &[bool]) -> Vec<f64> {
    objectives
        .iter()
        .zip(minimize)
        .map(|(v, &min)| if min { *v } else { -*v })
        .collect()
}

/// Computes a fixed hypervolume reference point from the (minimized) feasible
/// objective values — the per-objective nadir plus a margin. Falls back to all
/// points when nothing is feasible yet, and to a unit margin for a
/// zero-range objective.
fn fixed_reference(data: &Dataset, minimize: &[bool]) -> Vec<f64> {
    let n_obj = minimize.len();
    let rows: Vec<Vec<f64>> = data
        .ys
        .iter()
        .zip(&data.cons)
        .filter(|(_, c)| is_feasible(c))
        .map(|(y, _)| to_minimized(y, minimize))
        .collect();
    let rows = if rows.is_empty() {
        data.ys.iter().map(|y| to_minimized(y, minimize)).collect()
    } else {
        rows
    };

    let mut nadir = vec![f64::NEG_INFINITY; n_obj];
    let mut ideal = vec![f64::INFINITY; n_obj];
    for row in &rows {
        for k in 0..n_obj {
            nadir[k] = nadir[k].max(row[k]);
            ideal[k] = ideal[k].min(row[k]);
        }
    }
    for k in 0..n_obj {
        let range = nadir[k] - ideal[k];
        let margin = if range > 0.0 {
            REF_MARGIN * range
        } else {
            REF_MIN_MARGIN
        };
        // A non-finite nadir (no data at all) collapses to the margin.
        nadir[k] = if nadir[k].is_finite() {
            nadir[k] + margin
        } else {
            margin
        };
    }
    nadir
}

/// The feasible Pareto front's hypervolume against the fixed reference point.
/// For a single objective this reduces to `ref - best` (in the minimize
/// convention). Returns 0 when no feasible point exists.
fn convergence_metric(data: &Dataset, minimize: &[bool], ref_point: &[f64]) -> f64 {
    let points: Vec<Vec<f64>> = data
        .ys
        .iter()
        .zip(&data.cons)
        .filter(|(_, c)| is_feasible(c))
        .map(|(y, _)| to_minimized(y, minimize))
        .collect();
    if points.is_empty() {
        return 0.0;
    }
    hypervolume_nd(&points, ref_point)
}

/// Relative improvement `(current - prev) / |prev|`. The metric is
/// monotonically non-decreasing, so this is `>= 0`; the first move away from a
/// zero baseline is reported as `+inf` (an unambiguous "improved").
fn relative_improvement(prev: f64, current: f64) -> f64 {
    if prev.abs() < DEDUPE_EPS {
        if current > prev {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        (current - prev) / prev.abs()
    }
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
    use crate::gh::runner::{prepare_gh_run, run_prepared, GhSampler, GhStopReason};
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

    /// Same objectives as `quadratic_evaluator` but always feasible, so every
    /// bootstrap trial contributes to the hypervolume metric (deterministic
    /// convergence behavior for the diagnostics tests).
    fn always_feasible_evaluator() -> impl GhEvaluator {
        FnEvaluator(|v: &[f64]| {
            Ok(GhEvaluation {
                objectives: vec![(v[0] - 7.0).powi(2) + v[1], v[0] + v[1]],
                constraints: vec![-1.0],
                attributes: vec![Some(GhAttrValue::Number(v[0] * v[1]))],
            })
        })
    }

    /// Diagnostics: a bootstrap baseline (iteration 0) plus one entry per
    /// completed iteration, with a monotonically non-decreasing metric and
    /// non-negative relative improvement (the reference point is fixed).
    #[test]
    fn adaptive_records_monotonic_diagnostics() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let mut cfg = adaptive_cfg(vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Minimize,
        ]);
        cfg.adaptive_iterations = 3;
        cfg.adaptive_patience = 0; // no early stop: run the full budget

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = crate::surrogate_opt::FitProgress::new();
        let summary = run_prepared(
            &prep,
            &problem,
            &always_feasible_evaluator(),
            &cfg,
            &progress,
        )
        .unwrap();

        let diags = &summary.adaptive_diagnostics;
        assert_eq!(
            diags[0].iteration, 0,
            "first entry is the bootstrap baseline"
        );
        assert!(diags[0].metric > 0.0, "feasible bootstrap has positive HV");
        assert_eq!(summary.stop_reason, GhStopReason::Completed);
        // One entry per iteration that ran (plus the baseline).
        assert!(diags.len() >= 2);
        for w in diags.windows(2) {
            assert!(
                w[1].metric >= w[0].metric - 1e-9,
                "metric must not decrease: {} -> {}",
                w[0].metric,
                w[1].metric
            );
            assert!(w[1].relative_improvement >= -1e-9);
        }
    }

    /// Convergence: with a tiny patience and a huge improvement threshold, the
    /// loop stops early (before the full iteration budget) and reports
    /// `Converged`.
    #[test]
    fn adaptive_stops_early_on_convergence() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let mut cfg = adaptive_cfg(vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Minimize,
        ]);
        cfg.adaptive_iterations = 6;
        cfg.adaptive_patience = 1;
        // 10000%: no single-batch hypervolume gain reaches this, so the very
        // first measured iteration counts as "converged".
        cfg.adaptive_min_improvement = 100.0;

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = crate::surrogate_opt::FitProgress::new();
        let summary = run_prepared(
            &prep,
            &problem,
            &always_feasible_evaluator(),
            &cfg,
            &progress,
        )
        .unwrap();

        assert_eq!(summary.stop_reason, GhStopReason::Converged);
        // Stopped before running all 6 iterations.
        assert!(summary.adaptive_diagnostics.len() < 1 + cfg.adaptive_iterations);
        assert!(summary.adaptive_diagnostics.len() >= 2);
    }

    /// The non-adaptive samplers report no diagnostics and a `Completed` reason.
    #[test]
    fn non_adaptive_samplers_have_no_diagnostics() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = GhRunConfig {
            study_name: "random".to_string(),
            directions: vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Minimize,
            ],
            sampler: GhSampler::Random,
            n_trials: 4,
            ..GhRunConfig::default()
        };
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = crate::surrogate_opt::FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &quadratic_evaluator(), &cfg, &progress).unwrap();
        assert!(summary.adaptive_diagnostics.is_empty());
        assert_eq!(summary.stop_reason, GhStopReason::Completed);
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
