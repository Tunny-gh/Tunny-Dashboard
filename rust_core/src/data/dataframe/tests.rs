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
    let x_col = df.get_numeric_column("x").expect("xtranslated");
    assert!((x_col[0] - 0.5).abs() < 1e-9);
    assert!((x_col[1] - 1.5).abs() < 1e-9);
}

#[test]
fn tc_102_03_objective_column_values() {
    let rows = vec![make_trial(&[], vec![0.1, 0.9])];
    let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
    let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
    let obj0 = df.get_numeric_column("obj0").expect("obj0translated");
    let obj1 = df.get_numeric_column("obj1").expect("obj1translated");
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
    let loss = df.get_numeric_column("loss").expect("losstranslated");
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
    let tag = df.get_string_column("tag").expect("tagtranslated");
    assert_eq!(tag[0], "run_a");
}

#[test]
fn tc_102_06_constraint_columns() {
    let mut row = make_trial(&[], vec![1.0]);
    row.constraint_values = vec![-0.5, 0.3];
    let df = DataFrame::from_trials(&[row], &[], &["obj0".to_string()], &[], &[], 2);
    let c1 = df.get_numeric_column("c1").expect("c1translated");
    let c2 = df.get_numeric_column("c2").expect("c2translated");
    let is_feas = df
        .get_numeric_column("is_feasible")
        .expect("is_feasibletranslated");
    let csum = df
        .get_numeric_column("constraint_sum")
        .expect("constraint_sumtranslated");
    assert!((c1[0] - (-0.5)).abs() < 1e-9);
    assert!((c2[0] - 0.3).abs() < 1e-9);
    assert!((is_feas[0] - 0.0).abs() < 1e-9);
    assert!((csum[0] - (-0.2)).abs() < 1e-6);
}

#[test]
fn filter_feasible_keeps_only_feasible_rows() {
    // c <= 0 が実行可能（Optuna 規約）。row0/row2 が実行可能。
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
    // 列名リストは維持される
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
fn tc_102_07_positions_buffer_size() {
    let rows = vec![
        make_trial(&[], vec![1.0, 2.0]),
        make_trial(&[], vec![3.0, 4.0]),
        make_trial(&[], vec![5.0, 6.0]),
    ];
    let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
    let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
    let gpu = df.gpu_buffers();
    assert_eq!(gpu.positions.len(), 6);
    assert_eq!(gpu.trial_count, 3);
}

#[test]
fn tc_102_08_positions3d_buffer_size() {
    let rows = vec![
        make_trial(&[], vec![1.0, 2.0]),
        make_trial(&[], vec![3.0, 4.0]),
        make_trial(&[], vec![5.0, 6.0]),
    ];
    let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
    let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
    let gpu = df.gpu_buffers();
    assert_eq!(gpu.positions3d.len(), 9);
}

#[test]
fn tc_102_09_sizes_buffer() {
    let rows = vec![
        make_trial(&[], vec![1.0]),
        make_trial(&[], vec![2.0]),
        make_trial(&[], vec![3.0]),
    ];
    let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
    let gpu = df.gpu_buffers();
    assert_eq!(gpu.sizes.len(), 3);
    assert!(gpu.sizes.iter().all(|&s| (s - 1.0f32).abs() < 1e-6));
}

#[test]
fn tc_102_10_positions_two_objectives() {
    let rows = vec![make_trial(&[], vec![1.0, 2.0])];
    let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
    let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
    let gpu = df.gpu_buffers();
    assert!((gpu.positions[0] - 1.0f32).abs() < 1e-6);
    assert!((gpu.positions[1] - 2.0f32).abs() < 1e-6);
}

#[test]
fn tc_102_11_positions_single_objective() {
    let rows = vec![
        make_trial(&[], vec![1.0]),
        make_trial(&[], vec![2.0]),
        make_trial(&[], vec![3.0]),
    ];
    let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
    let gpu = df.gpu_buffers();
    assert!((gpu.positions[0] - 0.0f32).abs() < 1e-6);
    assert!((gpu.positions[1] - 1.0f32).abs() < 1e-6);
    assert!((gpu.positions[2] - 0.5f32).abs() < 1e-6);
    assert!((gpu.positions[3] - 2.0f32).abs() < 1e-6);
    assert!((gpu.positions[4] - 1.0f32).abs() < 1e-6);
    assert!((gpu.positions[5] - 3.0f32).abs() < 1e-6);
}

#[test]
fn tc_102_12_dataframe_info_column_classification() {
    let mut row = make_trial(&[("x", 0.5)], vec![1.0]);
    row.user_attrs_numeric.insert("loss".to_string(), 0.1);
    row.constraint_values = vec![-0.5];
    let df = DataFrame::from_trials(
        &[row],
        &["x".to_string()],
        &["obj0".to_string()],
        &["loss".to_string()],
        &[],
        1,
    );
    let info = df.info();
    assert_eq!(info.param_columns, vec!["x"]);
    assert_eq!(info.objective_columns, vec!["obj0"]);
    assert_eq!(info.user_attr_columns, vec!["loss"]);
    assert_eq!(info.constraint_columns, vec!["c1"]);
    assert!(info.derived_columns.contains(&"is_feasible".to_string()));
    assert!(info.derived_columns.contains(&"constraint_sum".to_string()));
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
    crate::journal_parser::parse_journal(&data).expect("translated");
    let result = select_study(0).expect("select_study(0)translated");
    assert!(result.data_frame_info.row_count >= 1);
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
    crate::journal_parser::parse_journal(&data).expect("translated");
    let result_b = select_study(1).expect("StudyBretrieval");
    assert_eq!(result_b.data_frame_info.row_count, 2);
}

#[test]
fn tc_2330_all_studies_resident_by_id_after_parse() {
    // TASK-2330: 初回パースで全 study が study_id キーで常駐し、
    // select_study せずとも任意 study を snapshot で参照できること（比較の再パース廃止の土台）。
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

    // select_study を呼ばずに両 study を snapshot で取得できる
    let s0 = snapshot(0).expect("study 0 resident");
    let s1 = snapshot(1).expect("study 1 resident");
    assert_eq!(s0.row_count(), 1);
    assert_eq!(s1.row_count(), 2);

    // with_df でも任意 study を参照できる
    let rc1 = with_df(1, |df| df.row_count());
    assert_eq!(rc1, Some(2));
}

#[test]
fn tc_102_e01_invalid_study_id_returns_err() {
    let data =
        to_bytes("{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n");
    crate::journal_parser::parse_journal(&data).expect("translated");
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
    crate::journal_parser::parse_journal(&data).expect("translated");
    let result = select_study(0).expect("translated Ok translated");
    assert_eq!(result.data_frame_info.row_count, 0);
}

#[test]
fn tc_102_b01_three_objectives_positions() {
    let rows = vec![make_trial(&[], vec![0.1, 0.2, 0.3])];
    let obj_names = vec!["obj0".to_string(), "obj1".to_string(), "obj2".to_string()];
    let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
    let gpu = df.gpu_buffers();
    assert!((gpu.positions[0] - 0.1f32).abs() < 1e-6);
    assert!((gpu.positions[1] - 0.2f32).abs() < 1e-6);
    assert!((gpu.positions3d[0] - 0.1f32).abs() < 1e-6);
    assert!((gpu.positions3d[1] - 0.2f32).abs() < 1e-6);
    assert!((gpu.positions3d[2] - 0.3f32).abs() < 1e-6);
}

#[test]
fn tc_102_b02_study_with_no_complete_trials() {
    let data =
        to_bytes("{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n");
    crate::journal_parser::parse_journal(&data).expect("translated");
    let result = select_study(0).expect("translatedStudytranslated Ok translated");
    assert_eq!(result.data_frame_info.row_count, 0);
    assert_eq!(result.gpu_buffer_data.trial_count, 0);
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

    crate::journal_parser::parse_journal(&data).expect("translated");

    let result = select_study(0).expect("select_study translated");

    assert_eq!(result.data_frame_info.row_count, 50_000);
}

// ============================================================
// append_trials: from_trials(全行) との等価性
// ============================================================

/// 列内容が名前引きで一致するか（内部格納順は問わない）。
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
fn append_trials_backfills_new_param_column() {
    let o = vec!["obj0".to_string()];
    let chunk1 = vec![make_trial_n(0, &[("x", 0.5)], vec![1.0])];
    // 2 チャンク目で param "z"（数値）と user attr が初出現する。
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
    // 既存数値行はラベル無しとして "" になる（from_trials と同じ規則）。
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
    // 制約を持たない既存行は feasible 扱い。
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
