//! Filters that narrow down DataFrame rows by numeric range conditions.
//!
//! Applies a per-column range (min/max) as an AND condition and returns the indices of the rows that pass.
//!
//! Reference: docs/implements/TASK-103/filter-requirements.md

use std::collections::HashMap;

// =============================================================================
// Documentation.
// =============================================================================

/// Range for narrowing a numeric column (lower/upper bound, both optional).
#[derive(Debug, Clone)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// The same range-AND filter as filter_rows, but if a column specified in the
/// ranges doesn't exist in the DataFrame, that column is ignored and passed
/// through (not excluded). Same as filter_rows in that a value of NaN/Inf in an
/// existing column gets excluded by that filter.
pub fn filter_rows_permissive(
    df: &crate::dataframe::DataFrame,
    ranges: &HashMap<String, Range>,
) -> Vec<u32> {
    let n = df.row_count();
    if n == 0 {
        return vec![];
    }
    if ranges.is_empty() {
        return (0..n as u32).collect();
    }

    // If the column doesn't exist, keep it as None and pass it through during filtering.
    let col_ranges: Vec<(Option<&[f64]>, &Range)> = ranges
        .iter()
        .map(|(name, range)| (df.get_numeric_column(name), range))
        .collect();

    let mut result = Vec::with_capacity(n / 4);
    'outer: for row in 0..n {
        for (col, range) in &col_ranges {
            let Some(col) = col else {
                continue; // Don't exclude if the column doesn't exist
            };
            // A column shorter than row_count is treated the same as a missing
            // one rather than indexed directly: a short column is a bug
            // elsewhere, but it must not turn into a panic in the UI thread's
            // filter path.
            let Some(&val) = col.get(row) else {
                continue;
            };
            if !val.is_finite() {
                continue 'outer;
            }
            if let Some(min) = range.min {
                if val < min {
                    continue 'outer;
                }
            }
            if let Some(max) = range.max {
                if val > max {
                    continue 'outer;
                }
            }
        }
        result.push(row as u32);
    }
    result
}

// =============================================================================
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // Documentation.
    // -------------------------------------------------------------------------

    /// Helper to build a `TrialRow` for tests.
    fn make_row(trial_id: u32, params: &[(&str, f64)], obj: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id,
            trial_number: trial_id,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values: obj,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    /// Helper to build a `DataFrame` for tests and register it as the active Study.
    fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
        let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
        let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
        // Documentation.
        let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
        store_dataframes(vec![df.clone()]);
        select_study(0).expect("study 0 should exist");
        df
    }

    // =========================================================================
    // filter_rows_permissive
    // =========================================================================

    #[test]
    fn permissive_unknown_column_ignored() {
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "nonexistent".to_string(),
            Range {
                min: Some(0.0),
                max: Some(10.0),
            },
        );

        let result = filter_rows_permissive(&df, &ranges);

        // A non-existent column is ignored, and all rows pass through.
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn permissive_nan_value_rejected() {
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", f64::NAN)], vec![]),
            make_row(2, &[("x", 5.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.0),
                max: Some(10.0),
            },
        );

        let result = filter_rows_permissive(&df, &ranges);

        assert_eq!(result, vec![0, 2]);
    }

    #[test]
    fn permissive_in_and_out_of_range() {
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(3.0),
                max: Some(7.0),
            },
        );

        let result = filter_rows_permissive(&df, &ranges);

        assert_eq!(result, vec![1]);
    }

    #[test]
    fn permissive_mixed_known_and_missing_columns() {
        // Existing columns are filtered normally, and non-existent columns are ignored (AND condition).
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(3.0),
                max: Some(7.0),
            },
        );
        ranges.insert(
            "nonexistent".to_string(),
            Range {
                min: Some(0.0),
                max: Some(1.0),
            },
        );

        let result = filter_rows_permissive(&df, &ranges);

        assert_eq!(result, vec![1]);
    }

    #[test]
    fn permissive_empty_dataframe_returns_empty() {
        let df = DataFrame::empty();
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.0),
                max: Some(1.0),
            },
        );

        let result = filter_rows_permissive(&df, &ranges);

        assert_eq!(result, Vec::<u32>::new());
    }

    #[test]
    fn permissive_no_ranges_returns_all_rows() {
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);

        let result = filter_rows_permissive(&df, &HashMap::new());

        assert_eq!(result, vec![0, 1]);
    }
}
