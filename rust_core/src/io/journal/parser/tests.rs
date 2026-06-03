use super::*;
use serde_json::Value;
use std::collections::HashMap;

// ── parse_single_study / scan_study_list ──────────────────────────────────

fn two_study_log() -> Vec<u8> {
    // Study 0: "alpha", 2 trials, param "x"
    // Study 1: "beta",  3 trials, param "y"
    to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"alpha\",\"directions\":[1]}\n",
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"beta\",\"directions\":[2]}\n",
        // --- alpha trials (study_id=0) ---
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",\"param_value_internal\":0.1,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":1,\"param_name\":\"x\",\"param_value_internal\":0.2,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[2.0],\"datetime_complete\":\"2024-01-01T00:00:03.000000\"}\n",
        // --- beta trials (study_id=1) ---
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:04.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":2,\"param_name\":\"y\",\"param_value_internal\":10.0,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":100.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[3.0],\"datetime_complete\":\"2024-01-01T00:00:05.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:06.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":3,\"param_name\":\"y\",\"param_value_internal\":20.0,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":100.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":3,\"state\":1,\"values\":[4.0],\"datetime_complete\":\"2024-01-01T00:00:07.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:08.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":4,\"param_name\":\"y\",\"param_value_internal\":30.0,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":100.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":4,\"state\":1,\"values\":[5.0],\"datetime_complete\":\"2024-01-01T00:00:09.000000\"}\n"
    ))
}

#[test]
fn scan_study_list_returns_names_only() {
    let data = two_study_log();
    let studies = scan_study_list(&data).unwrap();
    assert_eq!(studies.len(), 2);
    assert_eq!(studies[0].name, "alpha");
    assert_eq!(studies[1].name, "beta");
    // Phase 1 では completed_trials は未確定
    assert_eq!(studies[0].completed_trials, 0);
    assert_eq!(studies[1].completed_trials, 0);
}

#[test]
fn parse_single_study_alpha_returns_correct_trials() {
    let data = two_study_log();
    let (meta, df) = parse_single_study(&data, 0).unwrap();
    assert_eq!(meta.name, "alpha");
    assert_eq!(meta.completed_trials, 2);
    assert_eq!(meta.total_trials, 2);
    assert!(meta.param_names.contains(&"x".to_string()));
    assert_eq!(df.row_count(), 2);
}

#[test]
fn parse_single_study_beta_skips_alpha_trials() {
    let data = two_study_log();
    let (meta, df) = parse_single_study(&data, 1).unwrap();
    assert_eq!(meta.name, "beta");
    assert_eq!(meta.completed_trials, 3);
    assert_eq!(meta.total_trials, 3);
    // beta の param は "y" のみ（alpha の "x" は含まれない）
    assert!(meta.param_names.contains(&"y".to_string()));
    assert!(!meta.param_names.contains(&"x".to_string()));
    assert_eq!(df.row_count(), 3);
}

#[test]
fn parse_single_study_objective_values_correct() {
    let data = two_study_log();
    let (_meta, df) = parse_single_study(&data, 0).unwrap();
    let obj: Vec<f64> = df
        .get_numeric_column("obj0")
        .map(|c| c.to_vec())
        .unwrap_or_default();
    assert_eq!(obj, vec![1.0, 2.0]);
}

#[test]
fn parse_single_study_nonexistent_id_returns_error() {
    let data = two_study_log();
    let result = parse_single_study(&data, 99);
    assert!(result.is_err());
}

#[test]
fn quick_extract_u32_basic() {
    assert_eq!(
        quick_extract_u32(r#"{"op_code":4,"study_id":2,"x":0}"#, "study_id"),
        Some(2)
    );
    assert_eq!(
        quick_extract_u32(r#"{"trial_id":  42,"state":1}"#, "trial_id"),
        Some(42)
    );
    assert_eq!(quick_extract_u32(r#"{"no_field":1}"#, "study_id"), None);
}

fn to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[test]
fn tc_101_01_create_study_basic() {
    let data =
        to_bytes(r#"{"op_code":0,"worker_id":"w1","study_name":"my_study","directions":[1,2]}"#);
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].name, "my_study");
    assert_eq!(
        result.studies[0].directions,
        vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Maximize,
        ]
    );
}

#[test]
fn tc_101_02_create_trial_complete() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies[0].completed_trials, 1);
    assert_eq!(result.studies[0].total_trials, 1);
}

#[test]
fn tc_101_03_float_distribution_no_log() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",\"param_value_internal\":0.5,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0,\"log\":false}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0].param_names.contains(&"x".to_string()));
}

#[test]
fn tc_101_04_float_distribution_log_true() {
    let ln2: f64 = std::f64::consts::LN_2;
    let line = format!(
        "{{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"lr\",\"param_value_internal\":{ln2},\"distribution\":{{\"name\":\"FloatDistribution\",\"low\":1e-5,\"high\":1.0,\"log\":true}}}}"
    );
    let data = to_bytes(&format!(
        "{}\n{}\n{}\n{}\n",
        r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#,
        r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#,
        line,
        r#"{"op_code":6,"worker_id":"w","trial_id":0,"state":1,"values":[0.5],"datetime_complete":"2024-01-01T00:00:01.000000"}"#,
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0].param_names.contains(&"lr".to_string()));
}

#[test]
fn tc_101_05_int_distribution_basic() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"n\",\"param_value_internal\":3.0,\"distribution\":{\"name\":\"IntDistribution\",\"low\":0,\"high\":10,\"step\":1,\"log\":false}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0].param_names.contains(&"n".to_string()));
}

#[test]
fn tc_101_07_categorical_distribution_string() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":1.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0].param_names.contains(&"cat".to_string()));
}

#[test]
fn tc_101_10_multiple_studies() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[0.2],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n",
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:03.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies.len(), 2);
    let sa = result
        .studies
        .iter()
        .find(|study| study.name == "A")
        .unwrap();
    let sb = result
        .studies
        .iter()
        .find(|study| study.name == "B")
        .unwrap();
    assert_eq!(sa.completed_trials, 2);
    assert_eq!(sb.completed_trials, 1);
}

#[test]
fn tc_101_11_trial_id_sequential() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies[0].total_trials, 3);
}

#[test]
fn tc_101_12_user_attr_numeric() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"loss\":0.123}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"loss".to_string()));
}

#[test]
fn tc_101_13_user_attr_string() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"tag\":\"run_a\"}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"tag".to_string()));
}

#[test]
fn tc_101_14_constraints_expansion() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-0.5,0.3]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0].has_constraints);
}

#[test]
fn tc_101_15_constraints_all_feasible() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-1.0,-0.5,0.0]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0].has_constraints);
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_101_16_multi_objective_values() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1,2]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1,0.9],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies[0].objective_names.len(), 2);
}

#[test]
fn tc_101_17_duration_ms_returned() {
    let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
    let result = parse_journal(&data).expect("translated");
    assert!(result.duration_ms >= 0.0);
}

#[test]
fn tc_101_e01_incomplete_json_line_skipped() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().studies[0].completed_trials, 1);
}

#[test]
fn tc_101_e02_non_json_line_skipped() {
    let mut data = Vec::new();
    data.extend_from_slice(
        b"{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
    );
    data.extend_from_slice(b"not-json-at-all\n");
    data.extend_from_slice(b"\xff\xfe\x00\n");
    data.extend_from_slice(b"{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n");
    data.extend_from_slice(b"{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n");
    let result = parse_journal(&data);
    assert!(result.is_ok());
}

#[test]
fn tc_101_e03_unknown_opcode_ignored() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":99,\"worker_id\":\"w\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data);
    assert!(result.is_ok());
}

#[test]
fn tc_101_e04_all_lines_invalid_returns_error() {
    let data: Vec<u8> = vec![0xff, 0xfe, 0x00, 0x01, 0x02];
    let result = parse_journal(&data);
    assert!(result.is_err());
}

#[test]
fn tc_101_e06_all_trials_not_complete() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies[0].completed_trials, 0);
    assert_eq!(result.studies[0].total_trials, 2);
}

#[test]
fn tc_101_e07_distributed_optimization_overwrite() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w1\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w2\",\"trial_id\":0,\"state\":1,\"values\":[0.3],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_101_b01_empty_file() {
    let data: Vec<u8> = Vec::new();
    let result = parse_journal(&data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().studies.len(), 0);
}

#[test]
fn tc_101_b02_study_only_no_trials() {
    let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].completed_trials, 0);
}

#[test]
fn tc_101_b03_categorical_boundary_indices() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":0.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert!(result.studies[0].param_names.contains(&"cat".to_string()));
}

#[test]
fn tc_101_b07_minimal_journal() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_101_p01_performance_50000_lines() {
    let mut lines = Vec::with_capacity(150_002);
    lines.push(r#"{"op_code":0,"worker_id":"w","study_name":"perf","directions":[0]}"#.to_string());
    for i in 0u32..50_000 {
        let val = f64::from(i) / 50_000.0;
        lines.push(r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#.to_string());
        lines.push(format!(r#"{{"op_code":5,"worker_id":"w","trial_id":{i},"param_name":"x","param_value_internal":{val},"distribution":{{"name":"FloatDistribution","low":0.0,"high":1.0,"log":false}}}}"#));
        lines.push(format!(r#"{{"op_code":6,"worker_id":"w","trial_id":{i},"state":1,"values":[{val}],"datetime_complete":"2024-01-01T00:00:01.000000"}}"#));
    }
    let data = lines.join("\n").into_bytes();

    let start = std::time::Instant::now();
    let result = parse_journal(&data).expect("50,000 translated");
    let elapsed_ms = start.elapsed().as_millis() as f64;

    assert_eq!(result.studies[0].completed_trials, 50_000);
    assert!(
        elapsed_ms < 5_000.0,
        "50,000 translated 5,000ms translated（translated: {elapsed_ms}ms）"
    );
}

#[test]
fn distribution_float_log_false_identity() {
    let dist = Distribution::Float { log: false };
    assert!((dist.to_display_f64(0.5) - 0.5).abs() < 1e-10);
}

#[test]
fn distribution_float_log_true_exp() {
    let dist = Distribution::Float { log: true };
    let expected = std::f64::consts::LN_2.exp();
    assert!((dist.to_display_f64(std::f64::consts::LN_2) - expected).abs() < 1e-10);
}

#[test]
fn distribution_int_step1() {
    let dist = Distribution::Int {
        low: 0,
        step: 1,
        log: false,
    };
    assert!((dist.to_display_f64(3.0) - 3.0).abs() < 1e-10);
}

#[test]
fn distribution_int_step2() {
    let dist = Distribution::Int {
        low: 0,
        step: 2,
        log: false,
    };
    assert!((dist.to_display_f64(2.0) - 4.0).abs() < 1e-10);
}

#[test]
fn distribution_categorical_label() {
    let dist = Distribution::Categorical {
        choices: vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
            Value::String("c".to_string()),
        ],
    };
    assert_eq!(dist.categorical_label(1.0), Some("b".to_string()));
    assert_eq!(dist.categorical_label(0.0), Some("a".to_string()));
    assert_eq!(dist.categorical_label(2.0), Some("c".to_string()));
}

#[test]
fn trial_builder_constraint_values_stored() {
    let trial = TrialBuilder {
        study_id: 0,
        state: 1,
        values: None,
        param_display: HashMap::new(),
        param_category_label: HashMap::new(),
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![-1.0, -0.5, 0.0],
        has_constraints: true,
    };
    assert_eq!(trial.constraint_values.len(), 3);
    assert!(trial.constraint_values.iter().all(|&value| value <= 0.0));
    let sum: f64 = trial.constraint_values.iter().sum();
    assert!((sum - (-1.5)).abs() < 1e-10);
}

#[test]
fn distribution_from_json_string_with_attributes() {
    let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.01, \"low\": -32.77, \"high\": 32.77, \"log\": false}}""#;
    let val: Value = serde_json::from_str(json_str).unwrap();
    let dist = Distribution::from_json(&val);
    assert!(matches!(dist, Distribution::Float { log: false }));
    assert!((dist.to_display_f64(7.4) - 7.4).abs() < 1e-10);
}

#[test]
fn distribution_from_json_string_log_true() {
    let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.0, \"low\": 1e-5, \"high\": 1.0, \"log\": true}}""#;
    let val: Value = serde_json::from_str(json_str).unwrap();
    let dist = Distribution::from_json(&val);
    assert!(matches!(dist, Distribution::Float { log: true }));
    let ln2 = std::f64::consts::LN_2;
    assert!((dist.to_display_f64(ln2) - 2.0).abs() < 1e-10);
}

#[test]
fn distribution_from_json_object_with_attributes() {
    let val: Value = serde_json::from_str(
        r#"{"name": "IntDistribution", "attributes": {"low": 0, "high": 10, "step": 2, "log": false}}"#,
    )
    .unwrap();
    let dist = Distribution::from_json(&val);
    assert!(matches!(
        dist,
        Distribution::Int {
            low: 0,
            step: 2,
            log: false
        }
    ));
    assert!((dist.to_display_f64(3.0) - 6.0).abs() < 1e-10);
}

#[test]
fn parse_real_log_format_param_values() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-03-28T11:58:48.485367\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x0\",\"param_value_internal\":7.4,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x1\",\"param_value_internal\":17.43,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[21.64],\"datetime_complete\":\"2026-03-28T11:58:48.612043\"}\n"
    ));
    let result = parse_journal(&data).expect("translated");
    assert_eq!(result.studies[0].completed_trials, 1);
    assert!(result.studies[0].param_names.contains(&"x0".to_string()));
    assert!(result.studies[0].param_names.contains(&"x1".to_string()));
}

#[test]
fn tc_inmem_01_basic() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"dtlz\",\"directions\":[1,1]}\n",
        "{\"op_code\":3,\"worker_id\":\"w\",\"study_id\":0,\"system_attr\":{\"study:metric_names\":[\"Obj1\",\"Obj2\"]}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":1,\"value\":null,\"values\":[1.0,2.0],\"datetime_complete\":\"2026-01-01T00:00:01.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.5},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:01.000000\",\"state\":1,\"value\":null,\"values\":[3.0,0.5],\"datetime_complete\":\"2026-01-01T00:00:02.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.7},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("inmem parse");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].name, "dtlz");
    assert_eq!(result.studies[0].completed_trials, 2);
    assert_eq!(result.studies[0].total_trials, 2);
    assert!(result.studies[0].param_names.contains(&"x".to_string()));
    assert_eq!(result.studies[0].objective_names, vec!["Obj1", "Obj2"]);
}

#[test]
fn tc_inmem_02_incomplete_state_not_counted() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":3,\"value\":null,\"values\":null,\"datetime_complete\":null,\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.5},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:01.000000\",\"state\":1,\"value\":null,\"values\":[0.8],\"datetime_complete\":\"2026-01-01T00:00:02.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.3},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("inmem incomplete state parse");
    assert_eq!(result.studies[0].total_trials, 2);
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_inmem_03_user_attrs_inline() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":1,\"value\":null,\"values\":[0.5],\"datetime_complete\":\"2026-01-01T00:00:01.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.5},\"user_attrs\":{\"loss\":0.123,\"tag\":\"run_a\"},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("inmem user_attrs parse");
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"loss".to_string()));
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"tag".to_string()));
}

#[test]
fn tc_inmem_04_single_objective_value_singular() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"ackley\",\"directions\":[1]}\n",
        "{\"op_code\":3,\"worker_id\":\"w\",\"study_id\":0,\"system_attr\":{\"study:metric_names\":[\"Obj\"]}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":1,\"value\":21.13,\"values\":null,\"datetime_complete\":\"2026-01-01T00:00:01.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": null, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"},\"params\":{\"x\":1.5},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:01.000000\",\"state\":1,\"value\":5.42,\"values\":null,\"datetime_complete\":\"2026-01-01T00:00:02.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": null, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"},\"params\":{\"x\":0.1},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("single-objective inmem parse");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].completed_trials, 2);
    assert_eq!(result.studies[0].objective_names, vec!["Obj"]);
    assert!(result.studies[0].param_names.contains(&"x".to_string()));
}

#[test]
fn parse_real_inmem_log_file() {
    let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("test_inmem.log");
    if !log_path.exists() {
        eprintln!("test_inmem.log not found at {:?}, skipping", log_path);
        return;
    }
    let data = std::fs::read(&log_path).expect("test_inmem.log read");
    let result = parse_journal(&data).expect("test_inmem.log parse");

    assert!(!result.studies.is_empty(), "at least 1 study");
    let study = &result.studies[0];
    assert!(study.completed_trials > 0, "should have completed trials");
    assert_eq!(study.param_names.len(), 10, "DTLZ1 has 10 params");
    for i in 0..10 {
        let name = format!("DTLZ1_Variable{i}");
        assert!(study.param_names.contains(&name), "missing param {name}");
    }
    assert_eq!(
        study.objective_names,
        vec!["Obj1", "Obj2"],
        "objective names from study:metric_names"
    );
}

#[test]
fn parse_real_log_file() {
    let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("test.log");
    if !log_path.exists() {
        eprintln!("test.log not found at {:?}, skipping", log_path);
        return;
    }
    let data = std::fs::read(&log_path).expect("test.log translated");
    let result = parse_journal(&data).expect("test.log translated");

    assert!(result.studies.len() >= 2, "translated 2 Study translated");

    let ackley = &result.studies[0];
    assert!(ackley.completed_trials > 0, "Ackley translated");
    assert_eq!(
        ackley.param_names.len(),
        10,
        "Ackley translated 10 parameter"
    );
    for i in 0..10 {
        let name = format!("Ackley_Variable{i}");
        assert!(
            ackley.param_names.contains(&name),
            "Ackley translated {name} translated"
        );
    }

    let dtlz = &result.studies[1];
    assert!(dtlz.completed_trials > 0, "DTLZ1 translated");
    assert_eq!(dtlz.param_names.len(), 10, "DTLZ1 translated 10 parameter");
    for i in 0..10 {
        let name = format!("DTLZ1_Variable{i}");
        assert!(
            dtlz.param_names.contains(&name),
            "DTLZ1 translated {name} translated"
        );
    }

    use crate::dataframe::with_df;
    let df_check = with_df(0, |df| {
        let param_cols = df.param_col_names();
        assert_eq!(
            param_cols.len(),
            10,
            "Ackley DataFrame translated 10 parametertranslated"
        );
        let col = df
            .get_numeric_column("Ackley_Variable0")
            .expect("translated");
        assert!(
            col[0].abs() > 1e-10,
            "Ackley_Variable0 translated: {}",
            col[0]
        );
    });
    assert!(df_check.is_some(), "DataFrame translated");
}
