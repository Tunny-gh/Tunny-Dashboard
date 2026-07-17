//! Centralizes feasibility (constraint-satisfaction) determination.
//!
//! Consolidates access to the `is_feasible` derived column (1.0 = feasible /
//! 0.0 = infeasible, present only for constrained studies) into a
//! `Feasibility` view. Defines the column name, threshold (> 0.5), and the
//! "no column = all rows feasible" fallback rule in this one place, and every
//! chart/computation path makes its determination through this view.

use super::model::DataFrame;

/// Name of the `is_feasible` column (must match the derived column name used when building the DataFrame).
pub(crate) const IS_FEASIBLE_COL: &str = "is_feasible";

/// Feasibility view. Obtained from `DataFrame::feasibility()`.
/// Since it can wrap a column slice directly, it can also be constructed via
/// [`Feasibility::from_column`] in tests or contexts that only have the column.
#[derive(Clone, Copy)]
pub struct Feasibility<'a> {
    col: Option<&'a [f64]>,
}

impl<'a> Feasibility<'a> {
    /// Constructs from an `is_feasible` column slice (`None` if absent).
    pub fn from_column(col: Option<&'a [f64]>) -> Self {
        Self { col }
    }

    /// Whether constraints are defined for the study (i.e., whether the `is_feasible` column exists).
    pub fn has_constraints(&self) -> bool {
        self.col.is_some()
    }

    /// Whether the given row is feasible. Returns `true` if the column is absent (no constraints) or the row is out of range.
    pub fn is_feasible(&self, row: usize) -> bool {
        self.col
            .and_then(|c| c.get(row))
            .map(|&v| v > 0.5)
            .unwrap_or(true)
    }

    /// Splits rows `0..n` into (feasible, infeasible) index lists.
    pub fn partition_indices(&self, n: usize) -> (Vec<usize>, Vec<usize>) {
        let mut feasible = Vec::with_capacity(n);
        let mut infeasible = Vec::new();
        for i in 0..n {
            if self.is_feasible(i) {
                feasible.push(i);
            } else {
                infeasible.push(i);
            }
        }
        (feasible, infeasible)
    }
}

impl DataFrame {
    /// Returns the feasibility view for this DataFrame.
    pub fn feasibility(&self) -> Feasibility<'_> {
        Feasibility::from_column(self.get_numeric_column(IS_FEASIBLE_COL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_column_means_unconstrained_and_all_feasible() {
        let feas = Feasibility::from_column(None);
        assert!(!feas.has_constraints());
        assert!(feas.is_feasible(0));
        assert!(feas.is_feasible(999));
        let (f, inf) = feas.partition_indices(3);
        assert_eq!(f, vec![0, 1, 2]);
        assert!(inf.is_empty());
    }

    #[test]
    fn threshold_is_half() {
        let col = vec![1.0, 0.0, 0.6, 0.5];
        let feas = Feasibility::from_column(Some(&col));
        assert!(feas.has_constraints());
        assert!(feas.is_feasible(0));
        assert!(!feas.is_feasible(1));
        assert!(feas.is_feasible(2));
        assert!(!feas.is_feasible(3)); // Exactly 0.5 is on the infeasible side
    }

    #[test]
    fn out_of_range_row_defaults_to_feasible() {
        let col = vec![0.0];
        let feas = Feasibility::from_column(Some(&col));
        assert!(feas.is_feasible(5));
    }

    #[test]
    fn partition_indices_splits_correctly() {
        let col = vec![1.0, 0.0, 1.0];
        let feas = Feasibility::from_column(Some(&col));
        let (f, inf) = feas.partition_indices(3);
        assert_eq!(f, vec![0, 2]);
        assert_eq!(inf, vec![1]);
    }
}
