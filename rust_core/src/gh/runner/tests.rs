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
