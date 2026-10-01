//! Three-state constraint feasibility. Missing evaluations are never violations.
use super::model::DataFrame;

/// Derived column: 1 = feasible, 0 = confirmed violation, NaN = unverified.
pub(crate) const IS_FEASIBLE_COL: &str = "is_feasible";

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FeasibilityState {
    /// Complete finite evaluation, with every value <= 0.
    Feasible,
    /// At least one finite positive value, regardless of incomplete peers.
    Infeasible,
    /// No confirmed violation, but evaluation is missing, partial, or non-finite.
    #[serde(rename = "Feasibility unverified")]
    Unverified,
}

impl FeasibilityState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Feasible => "Feasible",
            Self::Infeasible => "Infeasible",
            Self::Unverified => "Feasibility unverified",
        }
    }

    pub(crate) fn classify(values: &[f64], expected: usize) -> Self {
        if values.iter().any(|v| v.is_finite() && *v > 0.0) {
            Self::Infeasible
        } else if expected == 0 || values.len() != expected || values.iter().any(|v| !v.is_finite())
        {
            Self::Unverified
        } else {
            Self::Feasible
        }
    }

    pub(crate) fn numeric(self) -> f64 {
        match self {
            Self::Feasible => 1.0,
            Self::Infeasible => 0.0,
            Self::Unverified => f64::NAN,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Feasibility<'a> {
    col: Option<&'a [f64]>,
}

impl<'a> Feasibility<'a> {
    /// Wraps the derived column; absence means genuinely unconstrained.
    pub fn from_column(col: Option<&'a [f64]>) -> Self {
        Self { col }
    }

    pub fn has_constraints(&self) -> bool {
        self.col.is_some()
    }

    pub fn state(&self, row: usize) -> FeasibilityState {
        match self.col {
            None => FeasibilityState::Feasible,
            Some(col) => match col.get(row) {
                Some(v) if *v == 1.0 => FeasibilityState::Feasible,
                Some(v) if *v == 0.0 => FeasibilityState::Infeasible,
                _ => FeasibilityState::Unverified,
            },
        }
    }

    pub fn is_feasible(&self, row: usize) -> bool {
        self.state(row) == FeasibilityState::Feasible
    }

    /// Splits into (feasible, infeasible, unverified), preserving input order.
    pub fn partition_indices(&self, n: usize) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let (mut feasible, mut infeasible, mut unverified) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..n {
            match self.state(i) {
                FeasibilityState::Feasible => feasible.push(i),
                FeasibilityState::Infeasible => infeasible.push(i),
                FeasibilityState::Unverified => unverified.push(i),
            }
        }
        (feasible, infeasible, unverified)
    }
}

impl DataFrame {
    pub fn feasibility(&self) -> Feasibility<'_> {
        Feasibility::from_column(self.get_numeric_column(IS_FEASIBLE_COL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_requires_complete_finite_evidence_but_violation_wins() {
        use FeasibilityState::*;
        for (values, expected, state) in [
            (vec![-1.0, 0.0], 2, Feasible),
            (vec![-1.0, 0.1], 2, Infeasible),
            (vec![], 2, Unverified),
            (vec![], 0, Unverified),
            (vec![-1.0], 2, Unverified),
            (vec![f64::NAN, -1.0], 2, Unverified),
            (vec![f64::INFINITY, -1.0], 2, Unverified),
            (vec![f64::NEG_INFINITY, -1.0], 2, Unverified),
            (vec![0.1], 2, Infeasible),
            (vec![0.1, f64::NAN], 2, Infeasible),
            (vec![f64::INFINITY, 0.1], 2, Infeasible),
            (vec![f64::NEG_INFINITY, 0.1], 2, Infeasible),
        ] {
            assert_eq!(FeasibilityState::classify(&values, expected), state);
        }
    }

    #[test]
    fn view_preserves_unconstrained_and_partitions_three_states() {
        let unconstrained = Feasibility::from_column(None);
        assert!(!unconstrained.has_constraints());
        assert!(unconstrained.is_feasible(999));
        assert_eq!(
            unconstrained.partition_indices(3),
            (vec![0, 1, 2], vec![], vec![])
        );
        let col = [1.0, 0.0, f64::NAN];
        let constrained = Feasibility::from_column(Some(&col));
        assert_eq!(
            constrained.partition_indices(3),
            (vec![0], vec![1], vec![2])
        );
        assert_eq!(constrained.state(999), FeasibilityState::Unverified);
        assert_eq!(constrained.state(2).label(), "Feasibility unverified");
    }
}
