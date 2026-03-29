//! Module documentation.
//!
//! Module documentation.
//! Design:
//! Module documentation.
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-1101

// =============================================================================
// Constants
// =============================================================================

/// Documentation.
const CSV_DELIMITER: char = ',';

/// Documentation.
const NEEDS_QUOTING_CHARS: [char; 3] = [',', '\n', '"'];

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
fn escape_csv_field(s: &str) -> String {
    if s.chars().any(|c| NEEDS_QUOTING_CHARS.contains(&c)) {
        // Documentation.
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Documentation.
///
/// Design:
/// Documentation.
/// Documentation.
/// Documentation.
fn format_f64(v: f64) -> String {
    if v.is_nan() || v.is_infinite() {
        return String::new();
    }
    // Documentation.
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    // Documentation.
    let s = format!("{:.10}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
///
/// Documentation.
pub fn compute_report_stats() -> String {
    let result = crate::dataframe::with_active_df(|df| compute_report_stats_from_df(df));
    result.unwrap_or_else(|| "{}".to_string())
}

/// Documentation.
pub(crate) fn compute_report_stats_from_df(df: &crate::dataframe::DataFrame) -> String {
    if df.row_count() == 0 {
        return "{}".to_string();
    }

    let mut entries: Vec<String> = Vec::new();

    for col_name in df.column_names() {
        // Documentation.
        if let Some(vals) = df.get_numeric_column(&col_name) {
            // Documentation.
            let finite: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
            if finite.is_empty() {
                continue;
            }

            let count = finite.len();
            let min = finite.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = finite.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mean = finite.iter().sum::<f64>() / count as f64;

            // Documentation.
            let std = if count > 1 {
                let variance =
                    finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64;
                variance.sqrt()
            } else {
                0.0
            };

            // Documentation.
            let safe_name = col_name.replace('"', "\\\"");
            let entry = format!(
                r#""{}":{{"min":{},"max":{},"mean":{},"std":{},"count":{}}}"#,
                safe_name,
                format_f64(min),
                format_f64(max),
                format_f64(mean),
                format_f64(std),
                count
            );
            entries.push(entry);
        }
    }

    format!("{{{}}}", entries.join(","))
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
///
/// 🟢 REQ-150〜REQ-153
pub fn serialize_csv(indices: &[u32], columns_json: &str) -> String {
    // Documentation.
    let columns: Vec<String> = parse_columns_json(columns_json);
    if columns.is_empty() {
        return String::new();
    }

    // Documentation.
    let result =
        crate::dataframe::with_active_df(|df| serialize_csv_from_df(df, indices, &columns));

    result.unwrap_or_default()
}

/// Documentation.
///
/// Documentation.
pub(crate) fn serialize_csv_from_df(
    df: &crate::dataframe::DataFrame,
    indices: &[u32],
    columns: &[String],
) -> String {
    let n = df.row_count();
    let mut out = String::with_capacity(indices.len() * columns.len() * 10);

    // Documentation.
    let header_fields: Vec<String> = columns.iter().map(|c| escape_csv_field(c)).collect();
    out.push_str(&header_fields.join(&CSV_DELIMITER.to_string()));
    out.push('\n');

    // Documentation.
    for &idx in indices {
        let row = idx as usize;
        if row >= n {
            // Documentation.
            continue;
        }

        let mut fields = Vec::with_capacity(columns.len());
        for col in columns {
            let field = get_cell_value(df, row, col);
            fields.push(escape_csv_field(&field));
        }
        out.push_str(&fields.join(&CSV_DELIMITER.to_string()));
        out.push('\n');
    }

    out
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
fn get_cell_value(df: &crate::dataframe::DataFrame, row: usize, col: &str) -> String {
    // Documentation.
    if col == "trial_id" {
        return df
            .get_trial_id(row)
            .map(|id| id.to_string())
            .unwrap_or_default();
    }

    // Documentation.
    if let Some(vals) = df.get_numeric_column(col) {
        if let Some(&v) = vals.get(row) {
            return format_f64(v);
        }
        return String::new();
    }

    // Documentation.
    if let Some(vals) = df.get_string_column(col) {
        return vals.get(row).cloned().unwrap_or_default();
    }

    // Documentation.
    String::new()
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
fn parse_columns_json(json: &str) -> Vec<String> {
    let trimmed = json.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return vec![];
    }

    // Documentation.
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in inner.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            ',' if !in_string => {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    result.push(s);
                }
                current.clear();
            }
            _ if in_string => current.push(ch),
            _ => {} // Documentation.
        }
    }
    let s = current.trim().to_string();
    if !s.is_empty() {
        result.push(s);
    }

    result
}

// =============================================================================
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{DataFrame, TrialRow};
    use std::collections::HashMap;

    /// Documentation.
    fn make_test_df() -> DataFrame {
        let rows = vec![
            TrialRow {
                trial_id: 0,
                param_display: {
                    let mut m = HashMap::new();
                    m.insert("x1".to_string(), 1.5);
                    m.insert("x2".to_string(), 2.0);
                    m
                },
                param_category_label: HashMap::new(),
                objective_values: vec![10.0, 20.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            TrialRow {
                trial_id: 5,
                param_display: {
                    let mut m = HashMap::new();
                    m.insert("x1".to_string(), 3.0);
                    m.insert("x2".to_string(), 4.5);
                    m
                },
                param_category_label: HashMap::new(),
                objective_values: vec![30.0, 40.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];

        DataFrame::from_trials(
            &rows,
            &["x1".to_string(), "x2".to_string()],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        )
    }

    // Documentation.
    #[test]
    fn tc_1101_01_csv_header_row() {
        // Documentation.
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[0, 1], &["trial_id".to_string(), "x1".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // Documentation.
        assert_eq!(lines[0], "trial_id,x1", "Header row should match");
    }

    // Documentation.
    #[test]
    fn tc_1101_02_trial_id_column() {
        // Documentation.
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[0, 1], &["trial_id".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[1], "0", "row=0 trial_id should be 0");
        assert_eq!(lines[2], "5", "row=1 trial_id should be 5");
    }

    // Documentation.
    #[test]
    fn tc_1101_03_numeric_column_values() {
        // Documentation.
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[0], &["x1".to_string(), "obj0".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // Documentation.
        assert_eq!(
            lines[1], "1.5,10",
            "Numeric values should be formatted correctly"
        );
    }

    // Documentation.
    #[test]
    fn tc_1101_04_index_filtering() {
        // Documentation.
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[1], &["x1".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // Documentation.
        assert_eq!(
            lines.len(),
            2,
            "index=[1] should produce 2 rows (header + 1 data row)"
        );
        assert_eq!(
            lines[1], "3",
            "row=1 x1=3.0 should be rendered as integer '3'"
        );
    }

    // Documentation.
    #[test]
    fn tc_1101_05_out_of_range_index_skipped() {
        // Documentation.
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[99], &["x1".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // Documentation.
        assert_eq!(lines.len(), 1, "Out-of-range indices should be skipped");
    }

    // Documentation.
    #[test]
    fn tc_1101_06_csv_field_escaping() {
        // Documentation.
        assert_eq!(escape_csv_field("hello,world"), "\"hello,world\"");
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape_csv_field("normal"), "normal");
    }

    // Documentation.
    #[test]
    fn tc_1101_07_parse_columns_json() {
        // Documentation.
        let cols = parse_columns_json("[\"x1\", \"obj0\", \"trial_id\"]");
        assert_eq!(cols, vec!["x1", "obj0", "trial_id"]);
    }

    // Documentation.
    #[test]
    fn tc_1101_08_empty_indices_header_only() {
        // Documentation.
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[], &["x1".to_string()]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1, "translated");
    }

    // Documentation.
    #[test]
    fn tc_1101_09_format_f64_cases() {
        // Documentation.
        assert_eq!(format_f64(1.0), "1");
        assert_eq!(format_f64(1.5), "1.5");
        assert_eq!(format_f64(0.1 + 0.2), "0.3"); // Documentation.
        assert_eq!(format_f64(f64::NAN), "");
        assert_eq!(format_f64(f64::INFINITY), "");
    }

    // Documentation.
    #[test]
    fn tc_1101_10_nonexistent_column_empty() {
        // Documentation.
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[0], &["nonexistent".to_string()]);
        let lines: Vec<&str> = csv.lines().collect();
        // Documentation.
        assert_eq!(lines[1], "", "translated");
    }

    // Documentation.
    #[test]
    fn tc_1102_01_report_stats_numeric_columns() {
        // Documentation.
        let df = make_test_df();
        let json = compute_report_stats_from_df(&df);

        // Documentation.
        assert!(!json.is_empty(), "translated JSON translated");
        // Documentation.
        assert!(json.contains("\"x1\""), "x1 translated JSON translated");
        // Documentation.
        assert!(json.contains("\"min\""), "min translated JSON translated");
        assert!(json.contains("\"max\""), "max translated JSON translated");
        assert!(json.contains("\"mean\""), "mean translated JSON translated");
        assert!(json.contains("\"std\""), "std translated JSON translated");
        assert!(
            json.contains("\"count\""),
            "count translated JSON translated"
        );
    }

    // Documentation.
    #[test]
    fn tc_1102_02_report_stats_empty_df() {
        // Documentation.
        let empty_df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        let json = compute_report_stats_from_df(&empty_df);
        // Documentation.
        assert_eq!(
            json, "{}",
            "translated DataFrame translated {{}} translated"
        );
    }

    // Documentation.
    #[test]
    fn tc_1102_03_report_stats_correct_values() {
        // Documentation.
        let df = make_test_df();
        let json = compute_report_stats_from_df(&df);

        // Documentation.
        // Documentation.
        assert!(
            json.contains("\"min\":1.5"),
            "x1 translated min=1.5 translated: {}",
            json
        );
        // Documentation.
        assert!(
            json.contains("\"max\":3"),
            "x1 translated max=3 translated: {}",
            json
        );
        // Documentation.
        assert!(
            json.contains("\"count\":2"),
            "x1 translated count=2 translated: {}",
            json
        );
    }

    // Documentation.
    #[test]
    fn tc_1102_04_report_stats_valid_json_structure() {
        // Documentation.
        let df = make_test_df();
        let json = compute_report_stats_from_df(&df);
        // Documentation.
        assert!(json.starts_with('{'), "JSON translated {{ translated");
        assert!(json.ends_with('}'), "JSON translated }} translated");
    }
}
