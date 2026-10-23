//! Three-state constraint feasibility. Missing evaluations are never violations.
use super::model::DataFrame;

/// Derived column: 1 = feasible, 0 = confirmed violation, NaN = unverified.
pub(crate) const IS_FEASIBLE_COL: &str = "is_feasible";

pub use tdashboard_study::FeasibilityState;

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
