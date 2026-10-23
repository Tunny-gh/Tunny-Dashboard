use std::collections::HashMap;
use tdashboard_analysis::dataframe::{DataFrame, TrialRow};
use tdashboard_io::csv::formatting::format_f64;

fn serialize_csv_from_df(df: &DataFrame, rows: &[u32], columns: &[String]) -> String {
    use tdashboard_analysis::export::{select_rows, ExportCell};
    use tdashboard_io::csv::{CsvField, CsvWriter};
    let mut writer = CsvWriter::new();
    writer.header(columns.iter().map(String::as_str));
    for row in select_rows(df, rows, columns) {
        writer.row(row.iter().map(|cell| match cell {
            ExportCell::Numeric(value) => CsvField::Num(*value),
            ExportCell::Text(value) => CsvField::Text(value),
            ExportCell::Empty => CsvField::Empty,
        }));
    }
    writer.finish()
}

fn compute_report_stats_from_df(df: &DataFrame) -> String {
    let stats: serde_json::Map<_, _> = tdashboard_analysis::export::column_stats(df).into_iter().map(|s| {
        (s.name, serde_json::json!({"min":s.min,"max":s.max,"mean":s.mean,"std":s.std,"count":s.count}))
    }).collect();
    serde_json::to_string(&stats).unwrap()
}

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
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![10.0, 20.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
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
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![30.0, 40.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
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
fn tc_1101_02_trial_id_column() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(
        &df,
        &[0, 1],
        &["trial_id".to_string(), "x1".to_string(), "obj0".to_string()],
    );

    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines, ["trial_id,x1,obj0", "0,1.5,10", "5,3,30"]);
}

#[test]
fn tc_1101_04_index_filtering() {
    let df = make_test_df();
    let csv = serialize_csv_from_df(&df, &[99, 1], &["x1".to_string()]);

    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Invalid indices should be skipped while valid selected rows remain"
    );
    assert_eq!(
        lines[1], "3",
        "row=1 x1=3.0 should be rendered as integer '3'"
    );
    for indices in [&[][..], &[99][..]] {
        assert_eq!(serialize_csv_from_df(&df, indices, &["x1".into()]), "x1\n");
    }
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
