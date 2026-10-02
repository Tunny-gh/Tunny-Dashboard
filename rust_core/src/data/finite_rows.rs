/// Finite observations and their original, ascending source-row indices.
/// Auxiliary values must be selected with `source_indices`, not filtered independently.
pub(crate) struct FiniteRows {
    pub values: Vec<Vec<f64>>,
    pub source_indices: Vec<usize>,
}

/// Select rows finite in every required numeric column, preserving column order.
/// Callers resolve columns and choose pairwise or complete-case requirements and
/// the row limit. Missing cells in short columns are non-finite. An empty required
/// set retains all `n_rows` rows. This is exclusion, not a fitting-boundary validator.
pub(crate) fn finite_rows(columns: &[&[f64]], n_rows: usize) -> FiniteRows {
    finite_matrix_rows((0..n_rows).map(|i| {
        columns
            .iter()
            .map(move |col| col.get(i).copied().unwrap_or(f64::NAN))
    }))
}

/// The same selection for already row-oriented PCA and sensitivity inputs.
/// Callers retain responsibility for shape validation and rejection policies.
pub(crate) fn finite_matrix_rows(
    rows: impl IntoIterator<Item = impl IntoIterator<Item = f64>>,
) -> FiniteRows {
    let mut values = Vec::new();
    let mut source_indices = Vec::new();
    for (i, row) in rows.into_iter().enumerate() {
        let row: Vec<f64> = row.into_iter().collect();
        if row.iter().all(|v| v.is_finite()) {
            values.push(row);
            source_indices.push(i);
        }
    }
    FiniteRows {
        values,
        source_indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_columns_keep_zero_and_original_alignment() {
        let x = [0.0, f64::NAN, 2.0, f64::INFINITY, 4.0, f64::NEG_INFINITY];
        let y = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        // Non-required auxiliary values, including constraints, are not finite features.
        let auxiliary = [f64::NAN, 1.0, f64::INFINITY, 3.0, f64::NEG_INFINITY, 5.0];
        let selected = finite_rows(&[&x, &y], x.len());
        assert_eq!(selected.source_indices, vec![0, 2, 4]);
        assert_eq!(
            selected.values,
            vec![vec![0.0, 10.0], vec![2.0, 12.0], vec![4.0, 14.0]]
        );
        let aligned: Vec<_> = selected
            .source_indices
            .iter()
            .map(|&i| auxiliary[i])
            .collect();
        assert!(aligned[0].is_nan());
        assert_eq!(aligned[1..], [f64::INFINITY, f64::NEG_INFINITY]);
        assert!(finite_rows(&[&x, &auxiliary], x.len()).values.is_empty());
    }

    #[test]
    fn empty_and_missing_cells() {
        assert!(
            finite_rows(&[&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY]], 3)
                .values
                .is_empty()
        );
        assert!(finite_rows(&[&[]], 2).source_indices.is_empty());
        assert!(finite_rows(&[&[1.0]], 0).values.is_empty());
        let selected = finite_rows(&[&[0.0, 1.0], &[2.0]], 2);
        assert_eq!(selected.source_indices, vec![0]);
        assert_eq!(selected.values, vec![vec![0.0, 2.0]]);
        assert_eq!(finite_rows(&[], 2).source_indices, vec![0, 1]);
    }
}
