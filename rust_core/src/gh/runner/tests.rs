use super::validate::{constrained_penalty_fitness, FAIL_PENALTY};
use super::*;
use crate::gh::fixtures::sample_ghx;
use crate::gh::problem::extract_problem;
use crate::io::journal::parser::{parse_single_study, OptimizationDirection};

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
        ..GhRunConfig::default()
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
    let summary = run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

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
    let summary = run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

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
    let summary = run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

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

fn single_problem() -> GhProblem {
    let mut problem = extract_problem(&sample_ghx()).unwrap();
    problem.objectives.truncate(1);
    problem
}

#[test]
fn cma_rejects_multi_and_zero_generations_before_creating_journal() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("run.log");
    let mut cfg = test_cfg(GhSampler::CmaEs);
    let problem = extract_problem(&sample_ghx()).unwrap();
    assert!(prepare_gh_run(&journal, &problem, &cfg)
        .err()
        .unwrap()
        .contains("exactly one objective"));
    assert!(!journal.exists());
    cfg.directions.truncate(1);
    cfg.cma_generations = 0;
    assert!(prepare_gh_run(&journal, &single_problem(), &cfg)
        .err()
        .unwrap()
        .contains("generations"));
    assert!(!journal.exists());
}

#[test]
fn cma_budget_seed_bounds_precision_start_and_constraint_journal() {
    let problem = single_problem();
    let mut cfg = test_cfg(GhSampler::CmaEs);
    cfg.directions.truncate(1);
    cfg.cma_generations = 3;
    let run = || {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let seen = Mutex::new(Vec::new());
        let eval = FnEvaluator(|v: &[f64]| {
            seen.lock().unwrap().push(v.to_vec());
            Ok(GhEvaluation {
                objectives: vec![v[0] + v[1]],
                constraints: vec![v[0] - 8.0],
                attributes: vec![Some(GhAttrValue::Number(v[0] * v[1]))],
            })
        });
        let summary = run_prepared(&prep, &problem, &eval, &cfg, &progress).unwrap();
        assert_eq!(summary.completed, 1 + 6 * 3);
        assert_eq!(summary.failed, 0);
        assert_eq!(progress.snapshot().total, 19);
        assert_eq!(progress.snapshot().done, 19);
        let seen = seen.into_inner().unwrap();
        assert_eq!(seen[0], denormalize(&problem, &normalize_current(&problem)));
        let (_, df, _) = parse_single_study(&std::fs::read(&journal).unwrap(), 0).unwrap();
        let span = df.get_numeric_column("span").unwrap().to_vec();
        let count = df.get_numeric_column("count").unwrap().to_vec();
        let objectives = df.get_numeric_column("weight").unwrap().to_vec();
        let constraints = df.get_numeric_column("c1").unwrap().to_vec();
        let area = df.get_numeric_column("area").unwrap().to_vec();
        for i in 0..df.row_count() {
            assert!((3.0..=12.0).contains(&span[i]));
            assert!((1.0..=10.0).contains(&count[i]));
            assert_eq!(count[i], count[i].round());
            assert_eq!(span[i], round_variable(&problem.variables[0], span[i]));
            assert!((objectives[i] - span[i] - count[i]).abs() < 1e-9);
            assert!((constraints[i] - (span[i] - 8.0)).abs() < 1e-9);
            assert!((area[i] - span[i] * count[i]).abs() < 1e-9);
        }
        assert!(constraints.iter().any(|c| *c > 0.0));
        let mut points = seen;
        points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        points
    };
    assert_eq!(run(), run());
}

#[test]
fn cma_minimizes_and_maximizes_real_objective() {
    let mut problem = single_problem();
    problem.constraints.clear();
    problem.attributes.clear();
    for direction in [
        OptimizationDirection::Minimize,
        OptimizationDirection::Maximize,
    ] {
        let cfg = GhRunConfig {
            sampler: GhSampler::CmaEs,
            directions: vec![direction.clone()],
            cma_generations: 10,
            ..GhRunConfig::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let eval = FnEvaluator(|v: &[f64]| {
            Ok(GhEvaluation {
                objectives: vec![v[0]],
                constraints: vec![],
                attributes: vec![],
            })
        });
        run_prepared(&prep, &problem, &eval, &cfg, &FitProgress::new()).unwrap();
        let (_, df, _) = parse_single_study(&std::fs::read(journal).unwrap(), 0).unwrap();
        let ys = df.get_numeric_column("weight").unwrap().to_vec();
        match direction {
            OptimizationDirection::Minimize => assert!(ys.iter().any(|y| *y <= 3.1)),
            OptimizationDirection::Maximize => assert!(ys.iter().any(|y| *y >= 11.9)),
        }
    }
}

#[test]
fn cma_cancellation_records_only_in_flight_trial() {
    let problem = single_problem();
    let mut cfg = test_cfg(GhSampler::CmaEs);
    cfg.directions.truncate(1);
    for cancel_before in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        if cancel_before {
            progress.request_cancel();
        }
        let eval = FnEvaluator(|v: &[f64]| {
            progress.request_cancel();
            Ok(GhEvaluation {
                objectives: vec![v[0]],
                constraints: vec![-1.0],
                attributes: vec![None],
            })
        });
        let summary = run_prepared(&prep, &problem, &eval, &cfg, &progress).unwrap();
        assert_eq!(summary.stop_reason, GhStopReason::Cancelled);
        assert_eq!(summary.completed, usize::from(!cancel_before));
        let (meta, _, _) = parse_single_study(&std::fs::read(journal).unwrap(), 0).unwrap();
        assert_eq!(meta.total_trials, u32::from(!cancel_before));
    }
}

#[test]
fn cma_evaluator_and_invalid_results_record_failures() {
    let problem = single_problem();
    let mut cfg = test_cfg(GhSampler::CmaEs);
    cfg.directions.truncate(1);
    cfg.cma_generations = 1;
    for mode in 0..5 {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let eval = FnEvaluator(|_: &[f64]| {
            if mode == 0 {
                return Err("mock evaluator failure".to_string());
            }
            Ok(GhEvaluation {
                objectives: if mode == 1 {
                    vec![]
                } else if mode == 2 {
                    vec![f64::NAN]
                } else {
                    vec![1.0]
                },
                constraints: if mode == 3 { vec![] } else { vec![0.0] },
                attributes: if mode == 4 { vec![] } else { vec![None] },
            })
        });
        let summary = run_prepared(&prep, &problem, &eval, &cfg, &FitProgress::new()).unwrap();
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 7);
        let (_, _, extras) = parse_single_study(&std::fs::read(journal).unwrap(), 0).unwrap();
        assert_eq!(extras.trials.len(), 7);
        assert!(extras
            .trials
            .iter()
            .all(|t| t.state == crate::data::extras::TrialState::Fail));
    }
}

#[test]
fn cma_journal_open_error_is_returned() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = test_cfg(GhSampler::CmaEs);
    cfg.directions.truncate(1);
    assert!(prepare_gh_run(dir.path(), &single_problem(), &cfg).is_err());
}

#[test]
fn cma_journal_write_error_aborts_without_evaluating() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("run.log");
    let mut cfg = test_cfg(GhSampler::CmaEs);
    cfg.directions.truncate(1);
    let problem = single_problem();
    let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
    prep.writer
        .lock()
        .unwrap()
        .replace_file_for_test(std::fs::File::open(&journal).unwrap());
    let calls = AtomicUsize::new(0);
    let eval = FnEvaluator(|_: &[f64]| {
        calls.fetch_add(1, Ordering::Relaxed);
        Err("must not evaluate".to_string())
    });
    let error = run_prepared(&prep, &problem, &eval, &cfg, &FitProgress::new())
        .err()
        .unwrap();
    assert!(error.contains("writing to the journal failed"), "{error}");
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let (meta, _, _) = parse_single_study(&std::fs::read(journal).unwrap(), 0).unwrap();
    assert_eq!(meta.total_trials, 0);
}

#[test]
fn cma_cancel_during_generation_stops_new_trials_and_preserves_journal() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    pool.install(|| {
        let problem = single_problem();
        let mut cfg = test_cfg(GhSampler::CmaEs);
        cfg.directions.truncate(1);
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let calls = AtomicUsize::new(0);
        let eval = FnEvaluator(|v: &[f64]| {
            if calls.fetch_add(1, Ordering::Relaxed) == 4 {
                progress.request_cancel();
            }
            Ok(GhEvaluation {
                objectives: vec![v[0]],
                constraints: vec![-1.0],
                attributes: vec![None],
            })
        });
        let summary = run_prepared(&prep, &problem, &eval, &cfg, &progress).unwrap();
        assert_eq!(summary.stop_reason, GhStopReason::Cancelled);
        assert_eq!(summary.completed, 5);
        assert_eq!(calls.load(Ordering::Relaxed), 5);
        let (meta, _, extras) = parse_single_study(&std::fs::read(journal).unwrap(), 0).unwrap();
        assert_eq!(meta.total_trials, 5);
        assert!(extras
            .trials
            .iter()
            .all(|t| t.state == crate::data::extras::TrialState::Complete));
    });
}

#[test]
fn cma_journal_finish_error_aborts_after_in_flight_evaluation() {
    let problem = single_problem();
    let mut cfg = test_cfg(GhSampler::CmaEs);
    cfg.directions.truncate(1);
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("run.log");
    let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
    let calls = AtomicUsize::new(0);
    let eval = FnEvaluator(|v: &[f64]| {
        calls.fetch_add(1, Ordering::Relaxed);
        prep.writer
            .lock()
            .unwrap()
            .replace_file_for_test(std::fs::File::open(&journal).unwrap());
        Ok(GhEvaluation {
            objectives: vec![v[0]],
            constraints: vec![-1.0],
            attributes: vec![None],
        })
    });
    let error = run_prepared(&prep, &problem, &eval, &cfg, &FitProgress::new())
        .err()
        .unwrap();
    assert!(error.contains("writing to the journal failed"), "{error}");
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    let (meta, _, _) = parse_single_study(&std::fs::read(journal).unwrap(), 0).unwrap();
    assert_eq!(meta.total_trials, 1);
    assert_eq!(meta.completed_trials, 0);
}

#[test]
fn bo_bootstrap_journal_begin_error_preserves_existing_records() {
    bo_bootstrap_journal_error(false);
}

#[test]
fn bo_bootstrap_journal_finish_error_preserves_successful_and_started_trials() {
    bo_bootstrap_journal_error(true);
}

fn bo_bootstrap_journal_error(fail_finish: bool) {
    use crate::data::extras::TrialState;
    use std::io::Write as _;

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    for sampler in [GhSampler::BoGpFitc, GhSampler::BoGpVfe] {
        pool.install(|| {
            let mut problem = single_problem();
            problem.constraints.clear();
            problem.attributes.clear();
            let mut cfg = test_cfg(sampler);
            cfg.directions.truncate(1);
            cfg.adaptive_iterations = 1;
            let dir = tempfile::tempdir().unwrap();
            let journal = dir.path().join("run.log");
            let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
            let successful = FnEvaluator(|v: &[f64]| {
                Ok(GhEvaluation {
                    objectives: vec![v[0]],
                    constraints: vec![],
                    attributes: vec![],
                })
            });
            let seed_cfg = GhRunConfig {
                sampler: GhSampler::Random,
                n_trials: 2,
                ..cfg.clone()
            };
            run_prepared(&prep, &problem, &successful, &seed_cfg, &FitProgress::new())
                .unwrap();
            // A previously started trial must survive either failure too.
            prep.writer.lock().unwrap().create_trial(0).unwrap();
            let before = std::fs::read(&journal).unwrap();
            let mut read_only = std::fs::File::open(&journal).unwrap();
            let underlying = read_only.write_all(b"probe").unwrap_err().to_string();
            if !fail_finish {
                prep.writer.lock().unwrap().replace_file_for_test(read_only);
            }
            let calls = AtomicUsize::new(0);
            let eval = FnEvaluator(|v: &[f64]| {
                calls.fetch_add(1, Ordering::Relaxed);
                prep.writer
                    .lock()
                    .unwrap()
                    .replace_file_for_test(std::fs::File::open(&journal).unwrap());
                successful.evaluate(v)
            });
            let progress = FitProgress::new();
            let error = run_prepared(&prep, &problem, &eval, &cfg, &progress).unwrap_err();
            assert_eq!(
                error,
                format!(
                    "Aborted because writing to the journal failed: failed to write journal record to {}: {underlying}",
                    journal.display()
                )
            );
            assert!(!error.contains("successful evaluations"), "{error}");
            assert_eq!(calls.load(Ordering::Relaxed), usize::from(fail_finish));
            assert_eq!(progress.snapshot().done, 0);
            let after = std::fs::read(&journal).unwrap();
            assert!(after.starts_with(&before));
            if !fail_finish {
                assert_eq!(after, before);
            }
            let (meta, df, extras) = parse_single_study(&after, 0).unwrap();
            assert_eq!(meta.total_trials, 3 + u32::from(fail_finish));
            assert_eq!(meta.completed_trials, 2);
            assert_eq!(df.row_count(), 2);
            assert_eq!(extras.trials.len(), 3 + usize::from(fail_finish));
            assert!(extras.trials[..2].iter().all(|t| t.state == TrialState::Complete));
            assert!(extras.trials[2..].iter().all(|t| t.state == TrialState::Running));
        });
    }
}

#[test]
fn bo_bootstrap_evaluation_failure_still_returns_insufficient_data() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    for sampler in [GhSampler::BoGpFitc, GhSampler::BoGpVfe] {
        pool.install(|| {
            let problem = single_problem();
            let mut cfg = test_cfg(sampler);
            cfg.directions.truncate(1);
            cfg.adaptive_iterations = 1;
            let dir = tempfile::tempdir().unwrap();
            let journal = dir.path().join("run.log");
            let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
            let calls = AtomicUsize::new(0);
            let eval = FnEvaluator(|_: &[f64]| {
                calls.fetch_add(1, Ordering::Relaxed);
                Err("mock evaluation failure".to_string())
            });
            let progress = FitProgress::new();
            let error = run_prepared(&prep, &problem, &eval, &cfg, &progress).unwrap_err();
            let initial = cfg
                .adaptive_initial
                .max(crate::surrogate_opt::MIN_TRIALS_FOR_SURROGATE_OPT);
            assert!(
                error.contains("successful evaluations to fit a surrogate (0 succeeded so far)"),
                "{error}"
            );
            assert!(!error.contains("journal"), "{error}");
            assert_eq!(calls.load(Ordering::Relaxed), initial);
            assert_eq!(progress.snapshot().done, initial);
            let (meta, _, extras) =
                parse_single_study(&std::fs::read(journal).unwrap(), 0).unwrap();
            assert_eq!(meta.total_trials as usize, initial);
            assert!(extras
                .trials
                .iter()
                .all(|t| t.state == crate::data::extras::TrialState::Fail));
        });
    }
}
