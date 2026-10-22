//! Pure evaluation helpers: constraint-violation penalty fitness, evaluation
//! validation, and conversions between normalized `[0,1]^d` space and the
//! sliders' real-unit values.

use crate::gh::compute::GhEvaluation;
use crate::gh::problem::{GhProblem, GhVariable};

/// Penalty value returned to the optimization algorithm on evaluation failure or
/// cancellation. Strictly worse than any constraint-violating trial's fitness
/// (`CONSTRAINT_PENALTY_BASE + MAX_COUNTED_VIOLATION < FAIL_PENALTY`), so the
/// search never prefers a crashing region over a merely infeasible one.
/// Infinity would produce NaN when normalizing the crowding distance, so a large
/// finite value is used instead.
pub(super) const FAIL_PENALTY: f64 = 1e15;

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
pub(super) fn constrained_penalty_fitness(n_obj: usize, constraints: &[f64]) -> Option<Vec<f64>> {
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
pub(super) fn validate_evaluation(eval: &GhEvaluation, problem: &GhProblem) -> Option<String> {
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

/// Converts a normalized point [0,1]^d into the slider's real value.
/// Applies the slider's rounding (integer / decimal digits) so that the value
/// recorded to the journal matches the value sent to Compute.
pub(in crate::gh) fn denormalize(problem: &GhProblem, x_norm: &[f64]) -> Vec<f64> {
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
pub(super) fn normalize_current(problem: &GhProblem) -> Vec<f64> {
    problem
        .variables
        .iter()
        .map(|var| ((var.value - var.low) / (var.high - var.low)).clamp(0.0, 1.0))
        .collect()
}

pub(in crate::gh) fn round_variable(var: &GhVariable, raw: f64) -> f64 {
    if var.is_integer {
        raw.round()
    } else {
        let scale = 10f64.powi(var.digits.min(15) as i32);
        (raw * scale).round() / scale
    }
}
