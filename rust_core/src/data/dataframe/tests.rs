use super::*;
use std::collections::HashMap;

fn make_trial(params: &[(&str, f64)], objective_values: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id: 0,
        trial_number: 0,
        param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        param_category_label: HashMap::new(),
        objective_values,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        user_attrs_json: HashMap::new(),
        constraint_values: vec![],
    }
}

fn to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[test]
fn tc_102_01_row_count_single_trial() {
    let rows = vec![make_trial(&[("x", 0.5)], vec![1.0])];
    let df = DataFrame::from_trials(
        &rows,
        &["x".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        0,
    );
    assert_eq!(df.row_count(), 1);
}

#[test]
fn tc_102_02_param_column_values() {
    let rows = vec![
        make_trial(&[("x", 0.5), ("y", 2.0)], vec![1.0]),
        make_trial(&[("x", 1.5), ("y", 3.0)], vec![2.0]),
    ];
    let param_names = vec!["x".to_string(), "y".to_string()];
    let df = DataFrame::from_trials(&rows, &param_names, &["obj0".to_string()], &[], &[], 0);
    let x_col = df.get_numeric_column("x").expect("x column should exist");
    assert!((x_col[0] - 0.5).abs() < 1e-9);
    assert!((x_col[1] - 1.5).abs() < 1e-9);
}

#[test]
fn tc_102_03_objective_column_values() {
    let rows = vec![make_trial(&[], vec![0.1, 0.9])];
    let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
    let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
    let obj0 = df
        .get_numeric_column("obj0")
        .expect("obj0 column should exist");
    let obj1 = df
        .get_numeric_column("obj1")
        .expect("obj1 column should exist");
    assert!((obj0[0] - 0.1).abs() < 1e-9);
    assert!((obj1[0] - 0.9).abs() < 1e-9);
}

#[test]
fn tc_102_04_user_attr_numeric() {
    let mut row = make_trial(&[], vec![1.0]);
    row.user_attrs_numeric.insert("loss".to_string(), 0.123);
    let df = DataFrame::from_trials(
        &[row],
        &[],
        &["obj0".to_string()],
        &["loss".to_string()],
        &[],
        0,
    );
    let loss = df
        .get_numeric_column("loss")
        .expect("loss column should exist");
    assert!((loss[0] - 0.123).abs() < 1e-9);
}

#[test]
fn tc_102_05_user_attr_string() {
    let mut row = make_trial(&[], vec![1.0]);
    row.user_attrs_string
        .insert("tag".to_string(), "run_a".to_string());
    let df = DataFrame::from_trials(
        &[row],
        &[],
        &["obj0".to_string()],
        &[],
        &["tag".to_string()],
        0,
    );
    let tag = df
        .get_string_column("tag")
        .expect("tag column should exist");
    assert_eq!(tag[0], "run_a");
}

#[test]
fn user_attribute_accessors_ignore_same_named_other_categories() {
    let mut row = make_trial(&[("shared", 1.0)], vec![2.0]);
    row.param_category_label
        .insert("tag".to_string(), "parameter".to_string());
    row.user_attrs_numeric.insert("shared".to_string(), 3.0);
    row.user_attrs_numeric.insert("objective".to_string(), 4.0);
    row.user_attrs_string
        .insert("tag".to_string(), "attribute".to_string());
    let df = DataFrame::from_trials(
        &[row],
        &["shared".to_string(), "tag".to_string()],
        &["objective".to_string()],
        &["shared".to_string(), "objective".to_string()],
        &["tag".to_string()],
        0,
    );
    let numeric: Vec<_> = df.user_attr_numeric_columns().collect();
    let text: Vec<_> = df.user_attr_string_columns().collect();
    assert_eq!(numeric[0], ("shared", &[3.0][..]));
    assert_eq!(numeric[1], ("objective", &[4.0][..]));
    assert_eq!(text[0], ("tag", &["attribute".to_string()][..]));
}

#[test]
fn user_attribute_indices_survive_late_same_named_parameter() {
    let mut first = make_trial(&[], vec![]);
    first.user_attrs_numeric.insert("shared".to_string(), 7.0);
    let mut df = DataFrame::from_trials(&[first], &[], &[], &["shared".to_string()], &[], 0);
    let mut second = make_trial(&[("shared", 2.0)], vec![]);
    second.user_attrs_numeric.insert("shared".to_string(), 8.0);
    df.append_trials(
        &[second],
        &["shared".to_string()],
        &[],
        &["shared".to_string()],
        &[],
        0,
    );
    let mut third = make_trial(&[("shared", 3.0)], vec![]);
    third.user_attrs_numeric.insert("shared".to_string(), 9.0);
    df.append_trials(
        &[third],
        &["shared".to_string()],
        &[],
        &["shared".to_string()],
        &[],
        0,
    );
    let attrs: Vec<_> = df.user_attr_numeric_columns().collect();
    assert_eq!(attrs[0], ("shared", &[7.0, 8.0, 9.0][..]));
}

#[test]
fn user_attribute_string_stays_separate_when_parameter_becomes_categorical() {
    let mut first = make_trial(&[("shared", 1.0)], vec![]);
    first
        .user_attrs_string
        .insert("shared".to_string(), "attr-one".to_string());
    let mut df = DataFrame::from_trials(
        &[first],
        &["shared".to_string()],
        &[],
        &[],
        &["shared".to_string()],
        0,
    );
    let mut second = make_trial(&[], vec![]);
    second
        .param_category_label
        .insert("shared".to_string(), "category".to_string());
    second
        .user_attrs_string
        .insert("shared".to_string(), "attr-two".to_string());
    df.append_trials(
        &[second],
        &["shared".to_string()],
        &[],
        &[],
        &["shared".to_string()],
        0,
    );
    let attrs: Vec<_> = df.user_attr_string_columns().collect();
    assert_eq!(
        attrs[0],
        (
            "shared",
            &["attr-one".to_string(), "attr-two".to_string()][..]
        )
    );
}

#[test]
fn user_attribute_indices_survive_parameter_type_change() {
    let mut first = make_trial(&[("choice", 0.0)], vec![]);
    first.user_attrs_numeric.insert("score".to_string(), 7.0);
    let mut df = DataFrame::from_trials(
        &[first],
        &["choice".to_string()],
        &[],
        &["score".to_string()],
        &[],
        0,
    );
    let mut second = make_trial(&[], vec![]);
    second
        .param_category_label
        .insert("choice".to_string(), "b".to_string());
    second.user_attrs_numeric.insert("score".to_string(), 8.0);
    df.append_trials(
        &[second],
        &["choice".to_string()],
        &[],
        &["score".to_string()],
        &[],
        0,
    );
    let attrs: Vec<_> = df.user_attr_numeric_columns().collect();
    assert_eq!(attrs[0], ("score", &[7.0, 8.0][..]));
}

#[test]
fn tc_102_06_constraint_columns() {
    let mut row = make_trial(&[], vec![1.0]);
    row.constraint_values = vec![-0.5, 0.3];
    let df = DataFrame::from_trials(&[row], &[], &["obj0".to_string()], &[], &[], 2);
    let c1 = df.get_numeric_column("c1").expect("c1 column should exist");
    let c2 = df.get_numeric_column("c2").expect("c2 column should exist");
    let is_feas = df
        .get_numeric_column("is_feasible")
        .expect("is_feasible column should exist");
    let csum = df
        .get_numeric_column("constraint_sum")
        .expect("constraint_sum column should exist");
    assert!((c1[0] - (-0.5)).abs() < 1e-9);
    assert!((c2[0] - 0.3).abs() < 1e-9);
    assert!((is_feas[0] - 0.0).abs() < 1e-9);
    assert!((csum[0] - (-0.2)).abs() < 1e-6);
}

#[test]
fn filter_feasible_keeps_only_feasible_rows() {
    // c <= 0 is feasible (Optuna convention). row0/row2 are feasible.
    let mut rows = vec![
        make_trial(&[("x", 1.0)], vec![10.0]),
        make_trial(&[("x", 2.0)], vec![20.0]),
        make_trial(&[("x", 3.0)], vec![30.0]),
    ];
    rows[0].constraint_values = vec![-1.0];
    rows[1].constraint_values = vec![0.5];
    rows[2].constraint_values = vec![0.0];
    for (i, r) in rows.iter_mut().enumerate() {
        r.trial_id = i as u32;
    }
    let df = DataFrame::from_trials(
        &rows,
        &["x".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        1,
    );

    let filtered = df.filter_feasible();
    assert_eq!(filtered.row_count(), 2);
    assert_eq!(filtered.get_trial_id(0), Some(0));
    assert_eq!(filtered.get_trial_id(1), Some(2));
    let x = filtered.get_numeric_column("x").expect("x column");
    assert_eq!(x, &vec![1.0, 3.0]);
    let obj = filtered.get_numeric_column("obj0").expect("obj0 column");
    assert_eq!(obj, &vec![10.0, 30.0]);
    // The column name list is preserved.
    assert_eq!(filtered.param_col_names(), df.param_col_names());
    assert_eq!(filtered.objective_col_names(), df.objective_col_names());
}

#[test]
fn filter_feasible_without_constraints_returns_all_rows() {
    let rows = vec![
        make_trial(&[("x", 1.0)], vec![10.0]),
        make_trial(&[("x", 2.0)], vec![20.0]),
    ];
    let df = DataFrame::from_trials(
        &rows,
        &["x".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        0,
    );
    let filtered = df.filter_feasible();
    assert_eq!(filtered.row_count(), 2);
}

#[test]
fn filter_feasible_all_infeasible_returns_empty() {
    let mut rows = vec![make_trial(&[("x", 1.0)], vec![10.0])];
    rows[0].constraint_values = vec![1.0];
    let df = DataFrame::from_trials(
        &rows,
        &["x".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        1,
    );
    let filtered = df.filter_feasible();
    assert_eq!(filtered.row_count(), 0);
    assert!(filtered
        .get_numeric_column("x")
        .expect("x column")
        .is_empty());
}

#[test]
fn tc_102_13_select_study_returns_result() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",",
        "\"param_value_internal\":0.5,",
        "\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0,\"log\":false}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5]}\n"
    ));
    crate::journal_parser::parse_journal(&data).expect("journal should parse successfully");
    select_study(0).expect("select_study(0) should succeed");
    assert!(snapshot(0).expect("study 0 resident").row_count() >= 1);
}

#[test]
fn tc_102_14_select_study_multiple_studies() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[2.0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[3.0]}\n",
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":3,\"state\":1,\"values\":[4.0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":4,\"state\":1,\"values\":[5.0]}\n"
    ));
    crate::journal_parser::parse_journal(&data).expect("journal should parse successfully");
    select_study(1).expect("StudyBretrieval");
    assert_eq!(snapshot(1).expect("study 1 resident").row_count(), 2);
}

#[test]
fn tc_2330_all_studies_resident_by_id_after_parse() {
    // TASK-2330: after the initial parse, all studies are resident keyed
    // by study_id, so any study can be referenced via snapshot without
    // calling select_study (the foundation for removing re-parsing on
    // comparison).
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0]}\n",
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[4.0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[5.0]}\n"
    ));
    crate::journal_parser::parse_journal(&data).expect("parse ok");

    // Both studies can be obtained via snapshot without calling select_study.
    let s0 = snapshot(0).expect("study 0 resident");
    let s1 = snapshot(1).expect("study 1 resident");
    assert_eq!(s0.row_count(), 1);
    assert_eq!(s1.row_count(), 2);

    // with_df can also reference any study.
    let rc1 = with_df(1, |df| df.row_count());
    assert_eq!(rc1, Some(2));
}

#[test]
fn tc_102_e01_invalid_study_id_returns_err() {
    let data =
        to_bytes("{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n");
    crate::journal_parser::parse_journal(&data).expect("journal should parse successfully");
    let result = select_study(99);
    assert!(result.is_err());
}

#[test]
fn tc_102_e02_all_running_returns_empty() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":0,\"values\":null}\n"
    ));
    crate::journal_parser::parse_journal(&data).expect("journal should parse successfully");
    select_study(0).expect("select_study(0) should succeed");
    assert_eq!(snapshot(0).expect("study 0 resident").row_count(), 0);
}

#[test]
fn tc_102_b02_study_with_no_complete_trials() {
    let data =
        to_bytes("{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n");
    crate::journal_parser::parse_journal(&data).expect("journal should parse successfully");
    select_study(0).expect("select_study(0) should succeed for study with no complete trials");
    assert_eq!(snapshot(0).expect("study 0 resident").row_count(), 0);
}

#[test]
fn tc_102_p01_load_50000_trials_at_scale() {
    let mut lines = Vec::with_capacity(100_001);
    lines.push(r#"{"op_code":0,"worker_id":"w","study_name":"perf","directions":[0]}"#.to_string());
    for i in 0u32..50_000 {
        lines.push(
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}".to_string()
        );
        lines.push(format!(
            "{{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":{i},\"state\":1,\"values\":[{v}]}}",
            v = (i as f64) * 0.001
        ));
    }
    let data = lines.join("\n").into_bytes();

    crate::journal_parser::parse_journal(&data).expect("journal should parse successfully");

    select_study(0).expect("select_study(0) should succeed");

    assert_eq!(snapshot(0).expect("study 0 resident").row_count(), 50_000);
}

// ============================================================
// append_trials: equivalence with from_trials (all rows)
// ============================================================

/// Checks whether column contents match by name lookup (internal storage order does not matter).
fn assert_df_equivalent(appended: &DataFrame, rebuilt: &DataFrame) {
    assert_eq!(appended.row_count(), rebuilt.row_count());
    for i in 0..appended.row_count() {
        assert_eq!(appended.get_trial_id(i), rebuilt.get_trial_id(i));
        assert_eq!(appended.get_trial_number(i), rebuilt.get_trial_number(i));
    }
    assert_eq!(appended.param_col_names(), rebuilt.param_col_names());
    assert_eq!(
        appended.objective_col_names(),
        rebuilt.objective_col_names()
    );
    assert_eq!(
        appended.user_attr_numeric_col_names(),
        rebuilt.user_attr_numeric_col_names()
    );
    assert_eq!(
        appended.user_attr_string_col_names(),
        rebuilt.user_attr_string_col_names()
    );
    let attr_names: Vec<_> = rebuilt.user_attr_names().collect();
    assert_eq!(appended.user_attr_names().collect::<Vec<_>>(), attr_names);
    for name in attr_names {
        for row in 0..rebuilt.row_count() {
            assert_eq!(
                appended.user_attr_value(name, row),
                rebuilt.user_attr_value(name, row)
            );
        }
    }
    assert_eq!(
        appended.constraint_col_names(),
        rebuilt.constraint_col_names()
    );
    let mut names_a = appended.column_names();
    let mut names_b = rebuilt.column_names();
    names_a.sort();
    names_b.sort();
    assert_eq!(names_a, names_b);
    for name in &names_b {
        match (
            appended.get_numeric_column(name),
            rebuilt.get_numeric_column(name),
        ) {
            (Some(a), Some(b)) => {
                assert_eq!(a.len(), b.len(), "len mismatch: {name}");
                for (x, y) in a.iter().zip(b) {
                    assert!(
                        (x.is_nan() && y.is_nan()) || (x - y).abs() < 1e-12,
                        "column {name}: {x} != {y}"
                    );
                }
            }
            (None, None) => {
                assert_eq!(
                    appended.get_string_column(name),
                    rebuilt.get_string_column(name),
                    "string column mismatch: {name}"
                );
            }
            _ => panic!("column {name}: numeric/string type mismatch"),
        }
    }
}

#[test]
fn json_user_attributes_keep_null_distinct_from_missing_and_append_matches_rebuild() {
    let mut first = make_trial_n(0, &[], vec![1.0]);
    first
        .user_attrs_json
        .insert("maybe".into(), serde_json::Value::Null);
    first.user_attrs_numeric.insert("mixed".into(), 1.25);
    first
        .user_attrs_json
        .insert("mixed".into(), serde_json::json!(1.25));
    let mut second = make_trial_n(1, &[], vec![2.0]);
    second.user_attrs_json.insert(
        "mixed".into(),
        serde_json::json!({"z": [2, true], "a": null}),
    );
    second
        .user_attrs_json
        .insert("late".into(), serde_json::json!([1, "x", false]));
    let mut third = make_trial_n(2, &[], vec![3.0]);
    third
        .user_attrs_string
        .insert("mixed".into(), "[1,2]".into());
    third
        .user_attrs_json
        .insert("mixed".into(), serde_json::json!("[1,2]"));

    let mut appended = DataFrame::from_trials(
        &[first.clone()],
        &[],
        &["obj0".into()],
        &["mixed".into()],
        &[],
        0,
    );
    appended.append_trials(
        &[second.clone()],
        &[],
        &["obj0".into()],
        &["mixed".into()],
        &[],
        0,
    );
    appended.append_trials(
        &[third.clone()],
        &[],
        &["obj0".into()],
        &["mixed".into()],
        &["mixed".into()],
        0,
    );
    let rebuilt = DataFrame::from_trials(
        &[first, second, third],
        &[],
        &["obj0".into()],
        &["mixed".into()],
        &["mixed".into()],
        0,
    );
    assert_df_equivalent(&appended, &rebuilt);
    assert_eq!(
        appended.user_attr_value("maybe", 0),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(appended.user_attr_value("maybe", 1), None);
    assert_eq!(appended.user_attr_value("late", 0), None);
    assert_eq!(
        appended.user_attr_value("late", 1),
        Some(&serde_json::json!([1, "x", false]))
    );
    assert_eq!(appended.user_attr_array_len("late"), Some(3));
    assert_eq!(appended.user_attr_array_len("mixed"), None);
    assert_eq!(
        appended.user_attr_value("mixed", 2),
        Some(&serde_json::json!("[1,2]"))
    );
    assert_eq!(
        appended.user_attr_value("mixed", 1).unwrap().to_string(),
        "{\"a\":null,\"z\":[2,true]}"
    );
    assert_eq!(
        appended.user_attr_numeric_columns().next().unwrap().1[0],
        1.25
    );
    assert!(appended.user_attr_numeric_columns().next().unwrap().1[1].is_nan());
}

fn make_trial_n(id: u32, params: &[(&str, f64)], objective_values: Vec<f64>) -> TrialRow {
    let mut t = make_trial(params, objective_values);
    t.trial_id = id;
    t.trial_number = id;
    t
}

#[test]
fn append_trials_equals_from_trials_basic() {
    let p = vec!["x".to_string(), "y".to_string()];
    let o = vec!["obj0".to_string()];
    let chunk1 = vec![
        make_trial_n(0, &[("x", 0.5), ("y", 2.0)], vec![1.0]),
        make_trial_n(1, &[("x", 1.5), ("y", 3.0)], vec![2.0]),
    ];
    let chunk2 = vec![make_trial_n(2, &[("x", 2.5), ("y", 4.0)], vec![3.0])];

    let mut df = DataFrame::from_trials(&chunk1, &p, &o, &[], &[], 0);
    df.append_trials(&chunk2, &p, &o, &[], &[], 0);

    let all: Vec<TrialRow> = chunk1.into_iter().chain(chunk2).collect();
    let rebuilt = DataFrame::from_trials(&all, &p, &o, &[], &[], 0);
    assert_df_equivalent(&df, &rebuilt);
}

#[test]
fn missing_numeric_params_match_initial_and_chunked_ingestion() {
    let rows = vec![
        make_trial_n(10, &[], vec![1.0]),
        make_trial_n(20, &[("x", 0.0)], vec![2.0]),
        make_trial_n(30, &[("x", 3.0), ("late", 0.0)], vec![3.0]),
        make_trial_n(40, &[], vec![4.0]),
        make_trial_n(50, &[("late", 7.0)], vec![5.0]),
    ];
    let names = vec!["x".to_string(), "late".to_string()];
    let initial = DataFrame::from_trials(&rows, &names, &[], &[], &[], 0);
    for chunk_size in [1, 2, 3, 5] {
        let mut appended = DataFrame::empty();
        let mut cumulative = Vec::new();
        for chunk in rows.chunks(chunk_size) {
            for name in &names {
                if !cumulative.contains(name)
                    && chunk.iter().any(|r| r.param_display.contains_key(name))
                {
                    cumulative.push(name.clone());
                }
            }
            appended.append_trials(chunk, &cumulative, &[], &[], &[], 0);
        }
        assert_df_equivalent(&initial, &appended);
    }
    let x = initial.get_numeric_column("x").unwrap();
    assert!(x[0].is_nan() && x[3].is_nan() && x[4].is_nan());
    assert_eq!(&x[1..3], &[0.0, 3.0]);
    let late = initial.get_numeric_column("late").unwrap();
    assert!(late[0].is_nan() && late[1].is_nan() && late[3].is_nan());
    assert_eq!(late[2], 0.0);
    assert_eq!(late[4], 7.0);
}

#[test]
fn append_trials_backfills_new_param_column() {
    let o = vec!["obj0".to_string()];
    let chunk1 = vec![make_trial_n(0, &[("x", 0.5)], vec![1.0])];
    // param "z" (numeric) and user attrs first appear in the 2nd chunk.
    let mut t = make_trial_n(1, &[("x", 1.5), ("z", 9.0)], vec![2.0]);
    t.user_attrs_numeric.insert("loss".into(), 0.5);
    t.user_attrs_string.insert("tag".into(), "b".into());
    let chunk2 = vec![t.clone()];

    let p1 = vec!["x".to_string()];
    let p2 = vec!["x".to_string(), "z".to_string()];
    let mut df = DataFrame::from_trials(&chunk1, &p1, &o, &[], &[], 0);
    df.append_trials(
        &chunk2,
        &p2,
        &o,
        &["loss".to_string()],
        &["tag".to_string()],
        0,
    );

    let all = vec![chunk1[0].clone(), t];
    let rebuilt = DataFrame::from_trials(
        &all,
        &p2,
        &o,
        &["loss".to_string()],
        &["tag".to_string()],
        0,
    );
    assert_df_equivalent(&df, &rebuilt);
}

#[test]
fn append_trials_flips_numeric_param_to_categorical() {
    let p = vec!["opt".to_string()];
    let o = vec!["obj0".to_string()];
    let chunk1 = vec![make_trial_n(0, &[("opt", 1.0)], vec![1.0])];
    let mut t = make_trial_n(1, &[], vec![2.0]);
    t.param_category_label.insert("opt".into(), "adam".into());
    let chunk2 = vec![t.clone()];

    let mut df = DataFrame::from_trials(&chunk1, &p, &o, &[], &[], 0);
    df.append_trials(&chunk2, &p, &o, &[], &[], 0);

    let all = vec![chunk1[0].clone(), t];
    let rebuilt = DataFrame::from_trials(&all, &p, &o, &[], &[], 0);
    assert_df_equivalent(&df, &rebuilt);
    // Existing numeric rows become "" as unlabeled (same rule as from_trials).
    assert_eq!(df.get_string_column("opt").unwrap()[0], "");
}

#[test]
fn append_trials_constraints_appear_mid_stream() {
    let p = vec!["x".to_string()];
    let o = vec!["obj0".to_string()];
    let chunk1 = vec![make_trial_n(0, &[("x", 0.5)], vec![1.0])];
    let mut t = make_trial_n(1, &[("x", 1.5)], vec![2.0]);
    t.constraint_values = vec![-1.0, 0.5];
    let chunk2 = vec![t.clone()];

    let mut df = DataFrame::from_trials(&chunk1, &p, &o, &[], &[], 0);
    df.append_trials(&chunk2, &p, &o, &[], &[], 2);

    let all = vec![chunk1[0].clone(), t];
    let rebuilt = DataFrame::from_trials(&all, &p, &o, &[], &[], 2);
    assert_df_equivalent(&df, &rebuilt);
    // Existing rows with no constraints are treated as feasible.
    assert!((df.get_numeric_column("is_feasible").unwrap()[0] - 1.0).abs() < 1e-12);
    assert!((df.get_numeric_column("is_feasible").unwrap()[1] - 0.0).abs() < 1e-12);
}

#[test]
fn append_trials_objective_added_mid_stream_backfills_nan() {
    let p = vec!["x".to_string()];
    let chunk1 = vec![make_trial_n(0, &[("x", 0.5)], vec![1.0])];
    let chunk2 = vec![make_trial_n(1, &[("x", 1.5)], vec![2.0, 5.0])];

    let o1 = vec!["obj0".to_string()];
    let o2 = vec!["obj0".to_string(), "obj1".to_string()];
    let mut df = DataFrame::from_trials(&chunk1, &p, &o1, &[], &[], 0);
    df.append_trials(&chunk2, &p, &o2, &[], &[], 0);

    let all = vec![chunk1[0].clone(), chunk2[0].clone()];
    let rebuilt = DataFrame::from_trials(&all, &p, &o2, &[], &[], 0);
    assert_df_equivalent(&df, &rebuilt);
    assert!(df.get_numeric_column("obj1").unwrap()[0].is_nan());
}

#[test]
fn append_trials_to_empty_dataframe() {
    let p = vec!["x".to_string()];
    let o = vec!["obj0".to_string()];
    let rows = vec![make_trial_n(0, &[("x", 0.5)], vec![1.0])];

    let mut df = DataFrame::empty();
    df.append_trials(&rows, &p, &o, &[], &[], 0);

    let rebuilt = DataFrame::from_trials(&rows, &p, &o, &[], &[], 0);
    assert_df_equivalent(&df, &rebuilt);
}

#[test]
fn append_trials_empty_rows_is_noop() {
    let p = vec!["x".to_string()];
    let o = vec!["obj0".to_string()];
    let rows = vec![make_trial_n(0, &[("x", 0.5)], vec![1.0])];
    let mut df = DataFrame::from_trials(&rows, &p, &o, &[], &[], 0);
    df.append_trials(&[], &p, &o, &[], &[], 0);
    assert_eq!(df.row_count(), 1);
}
