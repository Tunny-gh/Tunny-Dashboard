//! DataFrame の行を数値レンジ条件で絞り込むフィルタ。
//!
//! 列ごとのレンジ (min/max) を AND 条件として適用し、通過した行のインデックスを返す。
//!
//! Reference: docs/implements/TASK-103/filter-requirements.md

use std::collections::HashMap;

// =============================================================================
// Documentation.
// =============================================================================

/// 数値列を絞り込む範囲（下限・上限、いずれも省略可）。
#[derive(Debug, Clone)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// filter_rows と同様の範囲 AND フィルタだが、レンジに指定された列が
/// DataFrame に存在しない場合はその列を無視して素通しする（除外しない）。
/// 存在する列の値が NaN/Inf の場合はそのフィルタで除外される点は filter_rows と同じ。
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

    // 列が存在しない場合は None として保持し、フィルタ時に素通しする。
    let col_ranges: Vec<(Option<&[f64]>, &Range)> = ranges
        .iter()
        .map(|(name, range)| (df.get_numeric_column(name), range))
        .collect();

    let mut result = Vec::with_capacity(n / 4);
    'outer: for row in 0..n {
        for (col, range) in &col_ranges {
            let Some(col) = col else {
                continue; // 列が存在しない場合は除外しない
            };
            let val = col[row];
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

    /// テスト用の `TrialRow` を組み立てるヘルパー。
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

    /// テスト用の `DataFrame` を構築し、アクティブ Study として登録するヘルパー。
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

        // 存在しない列は無視され、全行が通過する。
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
        // 既存列は通常通りフィルタし、存在しない列は無視する（AND 条件）。
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
