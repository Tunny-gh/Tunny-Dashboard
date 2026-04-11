use super::*;
use crate::dataframe::{DataFrame, TrialRow};
use std::collections::HashMap;

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

#[test]
fn tc_1101_01_csv_header_row() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[0, 1], &["trial_id".to_string(), "x1".to_string()]);

    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "trial_id,x1", "Header row should match");
}

#[test]
fn tc_1101_02_trial_id_column() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[0, 1], &["trial_id".to_string()]);

    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[1], "0", "row=0 trial_id should be 0");
    assert_eq!(lines[2], "5", "row=1 trial_id should be 5");
}

#[test]
fn tc_1101_03_numeric_column_values() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[0], &["x1".to_string(), "obj0".to_string()]);

    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[1], "1.5,10",
        "Numeric values should be formatted correctly"
    );
}

#[test]
fn tc_1101_04_index_filtering() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[1], &["x1".to_string()]);

    let lines: Vec<&str> = csv.lines().collect();
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

#[test]
fn tc_1101_05_out_of_range_index_skipped() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[99], &["x1".to_string()]);

    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1, "Out-of-range indices should be skipped");
}

#[test]
fn tc_1101_06_csv_field_escaping() {
    assert_eq!(escape_csv_field("hello,world"), "\"hello,world\"");
    assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    assert_eq!(escape_csv_field("normal"), "normal");
}

#[test]
fn tc_1101_07_parse_columns_json() {
    let cols = parse_columns_json("[\"x1\", \"obj0\", \"trial_id\"]");
    assert_eq!(cols, vec!["x1", "obj0", "trial_id"]);
}

#[test]
fn tc_1101_08_empty_indices_header_only() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[], &["x1".to_string()]);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1, "translated");
}

#[test]
fn tc_1101_09_format_f64_cases() {
    assert_eq!(format_f64(1.0), "1");
    assert_eq!(format_f64(1.5), "1.5");
    assert_eq!(format_f64(0.1 + 0.2), "0.3");
    assert_eq!(format_f64(f64::NAN), "");
    assert_eq!(format_f64(f64::INFINITY), "");
}

#[test]
fn tc_1101_10_nonexistent_column_empty() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[0], &["nonexistent".to_string()]);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[1], "", "translated");
}

#[test]
fn tc_1102_01_report_stats_numeric_columns() {
    let df = make_test_df();
    let json = compute_report_stats_from_df(&df);

    assert!(!json.is_empty(), "translated JSON translated");
    assert!(json.contains("\"x1\""), "x1 translated JSON translated");
    assert!(json.contains("\"min\""), "min translated JSON translated");
    assert!(json.contains("\"max\""), "max translated JSON translated");
    assert!(json.contains("\"mean\""), "mean translated JSON translated");
    assert!(json.contains("\"std\""), "std translated JSON translated");
    assert!(
        json.contains("\"count\""),
        "count translated JSON translated"
    );
}

#[test]
fn tc_1102_02_report_stats_empty_df() {
    let empty_df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
    let json = compute_report_stats_from_df(&empty_df);
    assert_eq!(
        json, "{}",
        "translated DataFrame translated {{}} translated"
    );
}

#[test]
fn tc_1102_03_report_stats_correct_values() {
    let df = make_test_df();
    let json = compute_report_stats_from_df(&df);

    assert!(
        json.contains("\"min\":1.5"),
        "x1 translated min=1.5 translated: {}",
        json
    );
    assert!(
        json.contains("\"max\":3"),
        "x1 translated max=3 translated: {}",
        json
    );
    assert!(
        json.contains("\"count\":2"),
        "x1 translated count=2 translated: {}",
        json
    );
}

#[test]
fn tc_1102_04_report_stats_valid_json_structure() {
    let df = make_test_df();
    let json = compute_report_stats_from_df(&df);
    assert!(json.starts_with('{'), "JSON translated {{ translated");
    assert!(json.ends_with('}'), "JSON translated }} translated");
}
