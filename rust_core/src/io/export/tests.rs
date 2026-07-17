use super::*;
use crate::dataframe::{DataFrame, TrialRow};
use std::collections::HashMap;

fn make_test_df() -> DataFrame {
    let rows = vec![
        TrialRow {
            trial_id: 0,
            trial_number: 0,
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
            trial_number: 5,
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
    assert_eq!(lines.len(), 1, "empty indices should produce header only");
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
    assert_eq!(
        lines[1], "",
        "nonexistent column should render as empty field"
    );
}

#[test]
fn tc_1102_01_report_stats_numeric_columns() {
    let df = make_test_df();
    let json = compute_report_stats_from_df(&df);

    assert!(!json.is_empty(), "report stats JSON should not be empty");
    assert!(
        json.contains("\"x1\""),
        "report stats JSON should contain key x1"
    );
    assert!(
        json.contains("\"min\""),
        "report stats JSON should contain key min"
    );
    assert!(
        json.contains("\"max\""),
        "report stats JSON should contain key max"
    );
    assert!(
        json.contains("\"mean\""),
        "report stats JSON should contain key mean"
    );
    assert!(
        json.contains("\"std\""),
        "report stats JSON should contain key std"
    );
    assert!(
        json.contains("\"count\""),
        "report stats JSON should contain key count"
    );
}

#[test]
fn tc_1102_02_report_stats_empty_df() {
    let empty_df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
    let json = compute_report_stats_from_df(&empty_df);
    assert_eq!(
        json, "{}",
        "empty DataFrame should produce report stats {{}}"
    );
}

#[test]
fn tc_1102_03_report_stats_correct_values() {
    let df = make_test_df();
    let json = compute_report_stats_from_df(&df);

    assert!(
        json.contains("\"min\":1.5"),
        "x1 report stats should have min=1.5: {}",
        json
    );
    assert!(
        json.contains("\"max\":3"),
        "x1 report stats should have max=3: {}",
        json
    );
    assert!(
        json.contains("\"count\":2"),
        "x1 report stats should have count=2: {}",
        json
    );
}

#[test]
fn tc_1102_04_report_stats_valid_json_structure() {
    let df = make_test_df();
    let json = compute_report_stats_from_df(&df);
    assert!(json.starts_with('{'), "JSON should start with {{");
    assert!(json.ends_with('}'), "JSON should end with }}");
}
