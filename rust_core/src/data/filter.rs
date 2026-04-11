//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-103/filter-requirements.md

use std::collections::HashMap;

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
#[derive(Debug, Clone)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct TrialData {
    /// Documentation.
    pub trial_id: u32,
    /// Documentation.
    pub params_numeric: Vec<(String, f64)>,
    /// Documentation.
    pub params_categorical: Vec<(String, String)>,
    /// objectivevalue list（obj0, obj1, ...）
    pub values: Vec<f64>,
    /// Documentation.
    pub is_feasible: Option<bool>,
    /// user_attr numeric type
    pub user_attrs_numeric: Vec<(String, f64)>,
    /// user_attr string type
    pub user_attrs_string: Vec<(String, String)>,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
pub fn parse_ranges(ranges_json: &str) -> HashMap<String, Range> {
    // Documentation.
    let parsed: serde_json::Value = match serde_json::from_str(ranges_json) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    let obj = match parsed.as_object() {
        Some(o) => o,
        None => return HashMap::new(),
    };

    let mut result = HashMap::new();
    for (col_name, range_val) in obj {
        // Documentation.
        let min = range_val.get("min").and_then(|v| v.as_f64());
        let max = range_val.get("max").and_then(|v| v.as_f64());
        result.insert(col_name.clone(), Range { min, max });
    }
    result
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn filter_rows(df: &crate::dataframe::DataFrame, ranges: &HashMap<String, Range>) -> Vec<u32> {
    let n = df.row_count();
    // Documentation.
    if n == 0 {
        return vec![];
    }
    // Documentation.
    if ranges.is_empty() {
        return (0..n as u32).collect();
    }

    // Documentation.
    // Documentation.
    for (col_name, _) in ranges {
        if df.get_numeric_column(col_name).is_none() {
            return vec![]; // Documentation.
        }
    }

    // Documentation.
    for range in ranges.values() {
        if let (Some(min), Some(max)) = (range.min, range.max) {
            if min > max {
                return vec![]; // Documentation.
            }
        }
    }

    // Documentation.
    // Documentation.
    let col_ranges: Vec<(&[f64], &Range)> = ranges
        .iter()
        .map(|(name, range)| (df.get_numeric_column(name).unwrap(), range))
        .collect();

    // Documentation.
    let mut result = Vec::with_capacity(n / 4); // Documentation.
    'outer: for row in 0..n {
        for (col, range) in &col_ranges {
            let val = col[row];
            // Documentation.
            if val.is_nan() {
                continue 'outer;
            }
            // Documentation.
            if let Some(min) = range.min {
                if val < min {
                    continue 'outer;
                }
            }
            // Documentation.
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

/// Documentation.
pub fn build_trial_data(df: &crate::dataframe::DataFrame, row: usize) -> Option<TrialData> {
    if row >= df.row_count() {
        return None; // Documentation.
    }

    let trial_id = df.get_trial_id(row)?;

    // Documentation.
    let params_numeric: Vec<(String, f64)> = df
        .param_col_names()
        .iter()
        .filter_map(|name| {
            df.get_numeric_column(name)
                .map(|col| (name.clone(), col[row]))
        })
        .collect();

    // Documentation.
    let params_categorical: Vec<(String, String)> = df
        .param_col_names()
        .iter()
        .filter_map(|name| {
            df.get_string_column(name)
                .map(|col| (name.clone(), col[row].clone()))
        })
        .collect();

    // Documentation.
    let values: Vec<f64> = df
        .objective_col_names()
        .iter()
        .filter_map(|name| df.get_numeric_column(name).map(|col| col[row]))
        .collect();

    // Documentation.
    let is_feasible = df
        .get_numeric_column("is_feasible")
        .map(|col| col[row] == 1.0);

    // Documentation.
    let user_attrs_numeric: Vec<(String, f64)> = df
        .user_attr_numeric_col_names()
        .iter()
        .filter_map(|name| {
            df.get_numeric_column(name)
                .map(|col| (name.clone(), col[row]))
        })
        .collect();

    // Documentation.
    let user_attrs_string: Vec<(String, String)> = df
        .user_attr_string_col_names()
        .iter()
        .filter_map(|name| {
            df.get_string_column(name)
                .map(|col| (name.clone(), col[row].clone()))
        })
        .collect();

    Some(TrialData {
        trial_id,
        params_numeric,
        params_categorical,
        values,
        is_feasible,
        user_attrs_numeric,
        user_attrs_string,
    })
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
pub fn filter_by_ranges(ranges_json: &str) -> Vec<u32> {
    let ranges = parse_ranges(ranges_json);
    crate::dataframe::with_active_df(|df| filter_rows(df, &ranges)).unwrap_or_default()
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn get_trial(index: u32) -> Option<TrialData> {
    crate::dataframe::with_active_df(|df| build_trial_data(df, index as usize)).flatten()
}

/// Documentation.
///
/// Documentation.
pub fn get_trials_batch(indices: &[u32]) -> Vec<TrialData> {
    crate::dataframe::with_active_df(|df| {
        indices
            .iter()
            .filter_map(|&idx| build_trial_data(df, idx as usize))
            .collect()
    })
    .unwrap_or_default()
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

    /// Documentation.
    fn make_row(trial_id: u32, params: &[(&str, f64)], obj: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values: obj,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    /// Documentation.
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
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_103_01_single_range_filter() {
        // Documentation.
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![0.0]),
            make_row(1, &[("x", 3.0)], vec![0.0]),
            make_row(2, &[("x", 5.0)], vec![0.0]),
            make_row(3, &[("x", 7.0)], vec![0.0]),
            make_row(4, &[("x", 9.0)], vec![0.0]),
        ];
        let df = setup_df(rows, &["x"], &["obj0"]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(3.0),
                max: Some(7.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn tc_103_02_multi_column_and_filter() {
        // Documentation.
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![10.0]),
            make_row(1, &[("x", 3.0)], vec![20.0]),
            make_row(2, &[("x", 5.0)], vec![30.0]),
            make_row(3, &[("x", 7.0)], vec![40.0]),
        ];
        let df = setup_df(rows, &["x"], &["obj0"]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(3.0),
                max: Some(7.0),
            },
        );
        ranges.insert(
            "obj0".to_string(),
            Range {
                min: Some(20.0),
                max: Some(30.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn tc_103_03_null_min_open_lower_bound() {
        // Documentation.
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
                min: None,
                max: Some(6.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn tc_103_04_null_max_open_upper_bound() {
        // Documentation.
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
                min: Some(4.0),
                max: None,
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn tc_103_05_both_null_all_pass() {
        // Documentation.
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
                min: None,
                max: None,
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![0, 1, 2]);
    }

    #[test]
    fn tc_103_06_boundary_min_inclusive() {
        // Documentation.
        let rows = vec![
            make_row(0, &[("x", 2.0)], vec![]),
            make_row(1, &[("x", 4.0)], vec![]),
            make_row(2, &[("x", 6.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(4.0),
                max: Some(8.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn tc_103_07_boundary_max_inclusive() {
        // Documentation.
        let rows = vec![
            make_row(0, &[("x", 2.0)], vec![]),
            make_row(1, &[("x", 4.0)], vec![]),
            make_row(2, &[("x", 6.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.0),
                max: Some(4.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn tc_103_08_objective_column_filter() {
        // Documentation.
        let rows = vec![
            make_row(0, &[], vec![0.1]),
            make_row(1, &[], vec![0.5]),
            make_row(2, &[], vec![0.9]),
        ];
        let df = setup_df(rows, &[], &["obj0"]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "obj0".to_string(),
            Range {
                min: Some(0.3),
                max: Some(0.7),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn tc_103_09_get_trial_returns_correct_data() {
        // Documentation.
        let rows = vec![
            make_row(10, &[("x", 0.5)], vec![2.0]),
            make_row(20, &[("x", 1.5)], vec![3.0]),
        ];
        setup_df(rows, &["x"], &["obj0"]);

        let trial = get_trial(0).expect("translated0translated");

        // Documentation.
        assert_eq!(trial.trial_id, 10); // Documentation.
        assert_eq!(trial.values, vec![2.0]); // Documentation.
        let x_val = trial
            .params_numeric
            .iter()
            .find(|(k, _)| k == "x")
            .map(|(_, v)| *v)
            .expect("xparametertranslated");
        assert!((x_val - 0.5).abs() < 1e-9); // Documentation.
    }

    #[test]
    fn tc_103_10_get_trials_batch_returns_two_trials() {
        // Documentation.
        let rows = vec![
            make_row(10, &[("x", 0.5)], vec![2.0]),
            make_row(20, &[("x", 1.5)], vec![3.0]),
        ];
        setup_df(rows, &["x"], &["obj0"]);

        let trials = get_trials_batch(&[0, 1]);

        // Documentation.
        assert_eq!(trials.len(), 2); // Documentation.
        assert_eq!(trials[0].trial_id, 10); // Documentation.
        assert_eq!(trials[1].trial_id, 20); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_103_e01_unknown_column_returns_empty() {
        // Documentation.
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

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, Vec::<u32>::new()); // Documentation.
    }

    #[test]
    fn tc_103_e02_min_greater_than_max_returns_empty() {
        // Documentation.
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
                min: Some(8.0),
                max: Some(2.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, Vec::<u32>::new()); // Documentation.
    }

    #[test]
    fn tc_103_e03_get_trial_out_of_range_returns_none() {
        // Documentation.
        let rows = vec![make_row(0, &[("x", 1.0)], vec![])];
        setup_df(rows, &["x"], &[]);

        let result = get_trial(99);

        // Documentation.
        assert!(result.is_none()); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_103_b01_empty_dataframe_returns_empty() {
        // Documentation.
        let df = DataFrame::empty();
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.0),
                max: Some(1.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, Vec::<u32>::new()); // Documentation.
    }

    #[test]
    fn tc_103_b02_all_rows_match_returns_all() {
        // Documentation.
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
                min: Some(0.0),
                max: Some(100.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, vec![0, 1, 2]); // Documentation.
    }

    #[test]
    fn tc_103_b03_no_rows_match_returns_empty() {
        // Documentation.
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
                min: Some(20.0),
                max: Some(30.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // Documentation.
        assert_eq!(result, Vec::<u32>::new()); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_103_p01_performance_50000_rows_under_5ms() {
        // Documentation.
        let n = 50_000usize;
        let rows: Vec<TrialRow> = (0..n)
            .map(|i| {
                let x = (i % 100) as f64 / 100.0;
                let y = (i % 50) as f64 / 50.0;
                TrialRow {
                    trial_id: i as u32,
                    param_display: [("x".to_string(), x), ("y".to_string(), y)]
                        .into_iter()
                        .collect(),
                    param_category_label: HashMap::new(),
                    objective_values: vec![x + y],
                    user_attrs_numeric: HashMap::new(),
                    user_attrs_string: HashMap::new(),
                    constraint_values: vec![],
                }
            })
            .collect();
        let df = setup_df(rows, &["x", "y"], &["obj0"]);

        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.2),
                max: Some(0.8),
            },
        );
        ranges.insert(
            "y".to_string(),
            Range {
                min: Some(0.1),
                max: Some(0.9),
            },
        );
        ranges.insert(
            "obj0".to_string(),
            Range {
                min: Some(0.3),
                max: Some(1.5),
            },
        );

        let start = std::time::Instant::now();
        let result = filter_rows(&df, &ranges);
        let elapsed = start.elapsed();

        // Documentation.
        assert!(
            elapsed.as_millis() <= 5,
            "filter_rows translated {}ms translated（translated: ≤5ms）",
            elapsed.as_millis()
        );
        assert!(!result.is_empty(), "translated — translated");
    }
}
