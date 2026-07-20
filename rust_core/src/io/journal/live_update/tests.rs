use super::*;
use std::path::PathBuf;

fn with_fresh_state<F: FnOnce()>(f: F) {
    reset_live_update_state();
    f();
    reset_live_update_state();
}

fn make_create_trial(study_id: u32) -> String {
    format!(r#"{{"op_code":4,"study_id":{}}}"#, study_id)
}

fn make_set_param(trial_id: u32, name: &str, val: f64) -> String {
    format!(
        r#"{{"op_code":5,"trial_id":{},"param_name":"{}","param_value_internal":{},"distribution":{{"name":"FloatDistribution","low":0.0,"high":1.0,"log":false}}}}"#,
        trial_id, name, val
    )
}

fn make_complete(trial_id: u32, values: &[f64]) -> String {
    let vals = values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"op_code":6,"trial_id":{},"state":1,"values":[{}]}}"#,
        trial_id, vals
    )
}

fn make_diff_bytes(lines: &[String]) -> Vec<u8> {
    let mut data = lines.join("\n");
    data.push('\n');
    data.into_bytes()
}

// ── Original tests (TASK-1201) ──────────────────────────────────────

#[test]
fn count_created_trials_counts_all_op4_across_studies_and_states() {
    // Counts the total number of op_code=4 even with 2 studies and a mix of running/pruned/failed.
    let lines = vec![
        r#"{"op_code":0,"study_name":"a","directions":[1]}"#.to_string(),
        r#"{"op_code":0,"study_name":"b","directions":[1]}"#.to_string(),
        make_create_trial(0), // tid 0 (completed)
        make_complete(0, &[1.0]),
        make_create_trial(0), // tid 1 (pruned)
        r#"{"op_code":6,"trial_id":1,"state":2}"#.to_string(),
        make_create_trial(1), // tid 2 (running, no op6)
        make_set_param(2, "x", 0.3),
        make_create_trial(1), // tid 3 (failed)
        r#"{"op_code":6,"trial_id":3,"state":3}"#.to_string(),
    ];
    let data = make_diff_bytes(&lines);
    // op_code=4 appears 4 times → the next trial_id is 4.
    assert_eq!(count_created_trials(&data), 4);
}

#[test]
fn count_created_trials_per_study_counts_op4_per_study() {
    // 2 op_code=4 records for study 0, 2 for study 1 (state doesn't matter).
    let lines = vec![
        r#"{"op_code":0,"study_name":"a","directions":[1]}"#.to_string(),
        r#"{"op_code":0,"study_name":"b","directions":[1]}"#.to_string(),
        make_create_trial(0),
        make_create_trial(1),
        make_create_trial(0),
        make_create_trial(1),
    ];
    let data = make_diff_bytes(&lines);
    let per_study = count_created_trials_per_study(&data);
    assert_eq!(per_study.get(&0), Some(&2));
    assert_eq!(per_study.get(&1), Some(&2));
}

#[test]
fn live_trial_number_continues_from_seed() {
    with_fresh_state(|| {
        // The existing file already has 5 trials created for study 0 (trial.number 0..4).
        // The next live trial's trial.number must be 5 (a separate sequence from the global trial_id).
        set_next_trial_id(5);
        set_study_trial_number_seeds(HashMap::from([(0, 5)]));
        let data = make_diff_bytes(&[make_create_trial(0), make_complete(5, &[1.0])]);
        let result = append_journal_diff(&data);
        assert_eq!(result.new_trial_rows.len(), 1);
        let row = &result.new_trial_rows[0];
        assert_eq!(row.trial_id, 5);
        assert_eq!(row.trial_number, 5);
    });
}

#[test]
fn live_update_with_correct_next_trial_id_builds_full_row() {
    with_fresh_state(|| {
        // The existing file has 10 op_code=4 records; when next_trial_id=10 is correctly
        // set at the start of live updates, new trials' params/objectives are built without loss (regression test).
        set_next_trial_id(10);
        let create = make_create_trial(0);
        let set_param = make_set_param(10, "x1", 0.5);
        let complete = make_complete(10, &[1.23]);
        let data = make_diff_bytes(&[create, set_param, complete]);

        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        let row = &result.new_trial_rows[0];
        assert_eq!(row.params.get("x1"), Some(&0.5));
        assert_eq!(row.objectives, vec![1.23]);
    });
}

#[test]
fn tc_1201_01_complete_trial_counted() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            make_set_param(0, "x1", 0.5),
            make_complete(0, &[1.23]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);
        assert_eq!(result.new_trial_rows.len(), 1);
        assert_eq!(result.pending_running, 0);
    });
}

#[test]
fn tc_1201_02_incomplete_last_line_skipped() {
    with_fresh_state(|| {
        let complete = make_create_trial(0);
        let incomplete = r#"{"op_code":4,"study_id":0"#;

        let data = format!("{}\n{}", complete, incomplete).into_bytes();
        let result = append_journal_diff(&data);

        let expected_consumed = complete.len() + 1;
        assert_eq!(result.consumed_bytes, expected_consumed);
    });
}

#[test]
fn tc_1201_03_running_trial_pending() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            make_create_trial(0),
            make_complete(1, &[0.5]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        assert_eq!(result.pending_running, 1);
    });
}

#[test]
fn tc_1201_04_no_newline_consumed_zero() {
    with_fresh_state(|| {
        let data = b"incomplete line without newline";
        let result = append_journal_diff(data);
        assert_eq!(result.consumed_bytes, 0);
        assert_eq!(result.new_trial_rows.len(), 0);
    });
}

#[test]
fn tc_1201_05_invalid_json_ignored() {
    with_fresh_state(|| {
        let lines = vec![
            "not valid json".to_string(),
            make_create_trial(0),
            make_complete(0, &[1.0]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);
        assert_eq!(result.new_trial_rows.len(), 1);
    });
}

#[test]
fn tc_1201_06_cross_diff_running_to_complete() {
    with_fresh_state(|| {
        let diff1 = make_diff_bytes(&[make_create_trial(0)]);
        let r1 = append_journal_diff(&diff1);
        assert_eq!(r1.new_trial_rows.len(), 0);
        assert_eq!(r1.pending_running, 1);

        let diff2 = make_diff_bytes(&[make_complete(0, &[2.0])]);
        let r2 = append_journal_diff(&diff2);
        assert_eq!(r2.new_trial_rows.len(), 1);
        assert_eq!(r2.pending_running, 0);
    });
}

#[test]
fn tc_1201_07_reset_clears_state() {
    with_fresh_state(|| {
        let diff1 = make_diff_bytes(&[make_create_trial(0)]);
        append_journal_diff(&diff1);
        reset_live_update_state();
        let diff2 = make_diff_bytes(&[make_create_trial(0), make_complete(0, &[1.0])]);
        let result = append_journal_diff(&diff2);
        assert_eq!(result.new_trial_rows.len(), 1);
    });
}

#[test]
fn tc_1201_p01_performance_1000_lines() {
    with_fresh_state(|| {
        #[cfg(debug_assertions)]
        let n = 200;
        #[cfg(not(debug_assertions))]
        let n = 1000;

        let mut lines = Vec::new();
        for i in 0..n {
            lines.push(make_create_trial(0));
            lines.push(make_set_param(i as u32, "x1", (i as f64) / (n as f64)));
            lines.push(make_complete(i as u32, &[i as f64 * 0.01]));
        }
        let data = make_diff_bytes(&lines);

        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), n);
    });
}

// ── TASK-2217: Context type tests ──────────────────────────────────

#[test]
fn tc_2217_01_live_update_context_construction() {
    let ctx = LiveUpdateContext {
        file_path: PathBuf::from("test.log"),
        initial_byte_offset: 1024,
        next_trial_id: 42,
        study_trial_number_seeds: std::collections::HashMap::new(),
        study_distributions: vec![],
        no_change_timeout_ms: 60_000,
    };
    assert_eq!(ctx.file_path, PathBuf::from("test.log"));
    assert_eq!(ctx.initial_byte_offset, 1024);
    assert_eq!(ctx.next_trial_id, 42);
    assert!(ctx.study_distributions.is_empty());
    assert_eq!(ctx.no_change_timeout_ms, 60_000);
}

#[test]
fn tc_2217_02_study_distribution_info_construction() {
    let info = StudyDistributionInfo {
        study_id: 0,
        param_names: vec!["x1".to_string(), "x2".to_string()],
        objective_names: vec!["obj1".to_string()],
        distributions: HashMap::new(),
    };
    assert_eq!(info.study_id, 0);
    assert_eq!(info.param_names.len(), 2);
    assert!(info.distributions.is_empty());
}

// ── TASK-2218: TrialRow building tests ─────────────────────────────

#[test]
fn tc_2218_01_complete_trial_builds_trial_row() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            make_set_param(0, "x1", 0.5),
            make_complete(0, &[1.23]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);

        let row = &result.new_trial_rows[0];
        assert_eq!(row.trial_id, 0);
        assert_eq!(row.study_id, 0);
        assert_eq!(row.objectives, vec![1.23]);
        assert_eq!(row.params.get("x1"), Some(&0.5));
    });
}

#[test]
fn tc_2218_02_running_trial_not_in_rows() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            make_create_trial(0),
            make_complete(1, &[0.5]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        assert_eq!(result.new_trial_rows[0].trial_id, 1);
        assert_eq!(result.pending_running, 1);
    });
}

#[test]
fn tc_2218_03_incomplete_line_carried_over() {
    with_fresh_state(|| {
        let complete = make_create_trial(0);
        let incomplete = r#"{"op_code":4,"study_id":0"#;

        let data = format!("{}\n{}", complete, incomplete).into_bytes();
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 0);
        assert_eq!(result.pending_running, 1);
    });
}

#[test]
fn tc_2218_04_multi_study_trial_rows() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            make_set_param(0, "x", 0.1),
            make_complete(0, &[1.0]),
            make_create_trial(1),
            make_set_param(1, "y", 0.2),
            make_complete(1, &[2.0]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 2);

        assert_eq!(result.new_trial_rows[0].study_id, 0);
        assert_eq!(result.new_trial_rows[0].objectives, vec![1.0]);

        assert_eq!(result.new_trial_rows[1].study_id, 1);
        assert_eq!(result.new_trial_rows[1].objectives, vec![2.0]);

        assert_eq!(result.updated_study_counts.len(), 2);
    });
}

#[test]
fn tc_2218_05_invalid_json_skipped() {
    with_fresh_state(|| {
        let lines = vec![
            "not valid json".to_string(),
            make_create_trial(0),
            make_complete(0, &[1.0]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
    });
}

#[test]
fn tc_2218_06_cross_diff_builds_rows() {
    with_fresh_state(|| {
        let diff1 = make_diff_bytes(&[make_create_trial(0)]);
        let r1 = append_journal_diff(&diff1);
        assert_eq!(r1.new_trial_rows.len(), 0);

        let diff2 = make_diff_bytes(&[make_complete(0, &[2.0])]);
        let r2 = append_journal_diff(&diff2);
        assert_eq!(r2.new_trial_rows.len(), 1);
        assert_eq!(r2.new_trial_rows[0].objectives, vec![2.0]);
    });
}

#[test]
fn tc_2218_07_categorical_param_goes_to_param_categories() {
    with_fresh_state(|| {
        let create = make_create_trial(0);
        let set_cat = r#"{"op_code":5,"trial_id":0,"param_name":"color","param_value_internal":1.0,"distribution":{"name":"CategoricalDistribution","choices":["red","green","blue"]}}"#.to_string();
        let complete = make_complete(0, &[0.5]);
        let data = make_diff_bytes(&[create, set_cat, complete]);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        let row = &result.new_trial_rows[0];
        assert_eq!(
            row.param_categories.get("color"),
            Some(&"green".to_string())
        );
        assert!(!row.params.contains_key("color"));
    });
}

#[test]
fn tc_2218_08_log_param_is_stored_as_external_value() {
    // Optuna stores the external representation (the actual value) directly in
    // param_value_internal even for log distributions, so the display value must match the stored value.
    with_fresh_state(|| {
        let create = make_create_trial(0);
        let stored = 0.125;
        let set_log = format!(
            r#"{{"op_code":5,"trial_id":0,"param_name":"lr","param_value_internal":{},"distribution":{{"name":"FloatDistribution","low":0.0001,"high":1.0,"log":true}}}}"#,
            stored
        );
        let complete = make_complete(0, &[1.0]);
        let data = make_diff_bytes(&[create, set_log, complete]);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        let row = &result.new_trial_rows[0];
        let decoded = row.params.get("lr").copied().unwrap_or(0.0);
        assert!(
            (decoded - stored).abs() < 1e-12,
            "expected {}, got {}",
            stored,
            decoded
        );
    });
}

#[test]
fn tc_2218_09_op9_constraints_flow_into_trial_row() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            make_set_param(0, "x1", 0.5),
            r#"{"op_code":9,"trial_id":0,"system_attr":{"constraints":[-0.5,0.25]}}"#.to_string(),
            make_complete(0, &[1.0]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        assert_eq!(result.new_trial_rows[0].constraint_values, vec![-0.5, 0.25]);
    });
}

#[test]
fn tc_2218_11_op8_user_attrs_flow_into_trial_row() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            r#"{"op_code":8,"trial_id":0,"user_attr":{"area":12.5}}"#.to_string(),
            r#"{"op_code":8,"trial_id":0,"user_attr":{"material":"steel"}}"#.to_string(),
            make_complete(0, &[1.0]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        let row = &result.new_trial_rows[0];
        assert_eq!(row.user_attrs_numeric.get("area"), Some(&12.5));
        assert_eq!(
            row.user_attrs_string.get("material"),
            Some(&"steel".to_string())
        );
    });
}

#[test]
fn tc_2218_10_op9_other_system_attrs_ignored() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            r#"{"op_code":9,"trial_id":0,"system_attr":{"nsga2:generation":3}}"#.to_string(),
            make_complete(0, &[1.0]),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), 1);
        assert!(result.new_trial_rows[0].constraint_values.is_empty());
    });
}

#[test]
fn tc_2218_p02_performance_1000_trials_builds_rows() {
    with_fresh_state(|| {
        #[cfg(debug_assertions)]
        let n = 200usize;
        #[cfg(not(debug_assertions))]
        let n = 1000usize;

        let mut lines = Vec::new();
        for i in 0..n {
            lines.push(make_create_trial(0));
            lines.push(make_set_param(i as u32, "x1", (i as f64) / (n as f64)));
            lines.push(make_complete(i as u32, &[i as f64 * 0.01]));
        }
        let data = make_diff_bytes(&lines);

        let result = append_journal_diff(&data);

        assert_eq!(result.new_trial_rows.len(), n);
        for (i, row) in result.new_trial_rows.iter().enumerate() {
            assert_eq!(row.trial_id, i as u32);
            assert!((row.objectives[0] - i as f64 * 0.01).abs() < 1e-9);
        }
    });
}

// ── extras_events (ExtrasDiff): op4/op7/op6 auxiliary info diffs ───────────

#[test]
fn extras_op4_records_new_trial_with_datetime_start() {
    with_fresh_state(|| {
        let line =
            r#"{"op_code":4,"study_id":0,"datetime_start":"2024-01-01T00:00:00"}"#.to_string();
        let data = make_diff_bytes(&[line]);
        let result = append_journal_diff(&data);

        assert_eq!(
            result.extras_events.new_trials,
            vec![(0u32, 0u32, 0u32, Some(1_704_067_200.0))]
        );
        assert!(result.extras_events.intermediate_values.is_empty());
        assert!(result.extras_events.state_changes.is_empty());
    });
}

#[test]
fn extras_op4_without_datetime_start_records_none() {
    with_fresh_state(|| {
        let data = make_diff_bytes(&[make_create_trial(0)]);
        let result = append_journal_diff(&data);

        assert_eq!(result.extras_events.new_trials, vec![(0, 0, 0, None)]);
    });
}

#[test]
fn extras_op7_records_intermediate_value_without_affecting_pending() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            r#"{"op_code":7,"trial_id":0,"step":0,"intermediate_value":0.5}"#.to_string(),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(
            result.extras_events.intermediate_values,
            vec![(0u32, 0u64, 0.5)]
        );
        // op7 does not affect the completion logic (resolving pending trials).
        assert_eq!(result.new_trial_rows.len(), 0);
        assert_eq!(result.pending_running, 1);
    });
}

#[test]
fn extras_op7_multiple_steps_preserve_insertion_order() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            r#"{"op_code":7,"trial_id":0,"step":0,"intermediate_value":0.1}"#.to_string(),
            r#"{"op_code":7,"trial_id":0,"step":1,"intermediate_value":0.2}"#.to_string(),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(
            result.extras_events.intermediate_values,
            vec![(0u32, 0u64, 0.1), (0u32, 1u64, 0.2)]
        );
    });
}

#[test]
fn extras_op6_state1_records_state_change_and_datetime_complete() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            r#"{"op_code":6,"trial_id":0,"state":1,"values":[1.0],"datetime_complete":"2024-01-01T00:00:01"}"#.to_string(),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(
            result.extras_events.state_changes,
            vec![(0u32, 1u8, Some(1_704_067_201.0))]
        );
        // Normal completed-row construction still works as before.
        assert_eq!(result.new_trial_rows.len(), 1);
        assert_eq!(result.new_trial_rows[0].objectives, vec![1.0]);
    });
}

#[test]
fn extras_op6_state2_pruned_records_state_change_without_new_row() {
    with_fresh_state(|| {
        let lines = vec![
            make_create_trial(0),
            r#"{"op_code":6,"trial_id":0,"state":2}"#.to_string(),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(result.extras_events.state_changes, vec![(0u32, 2u8, None)]);
        assert_eq!(result.new_trial_rows.len(), 0);
        assert_eq!(
            result.pending_running, 0,
            "pruned trial must not stay pending"
        );
    });
}

#[test]
fn extras_full_lifecycle_populates_all_three_diffs() {
    with_fresh_state(|| {
        let lines = vec![
            r#"{"op_code":4,"study_id":0,"datetime_start":"2024-01-01T00:00:00"}"#.to_string(),
            r#"{"op_code":7,"trial_id":0,"step":0,"intermediate_value":0.5}"#.to_string(),
            r#"{"op_code":6,"trial_id":0,"state":1,"values":[2.0],"datetime_complete":"2024-01-01T00:00:02"}"#.to_string(),
        ];
        let data = make_diff_bytes(&lines);
        let result = append_journal_diff(&data);

        assert_eq!(
            result.extras_events.new_trials,
            vec![(0, 0, 0, Some(1_704_067_200.0))]
        );
        assert_eq!(result.extras_events.intermediate_values, vec![(0, 0, 0.5)]);
        assert_eq!(
            result.extras_events.state_changes,
            vec![(0, 1, Some(1_704_067_202.0))]
        );
        assert_eq!(result.new_trial_rows.len(), 1);
    });
}

#[test]
fn extras_events_default_when_no_complete_line() {
    with_fresh_state(|| {
        // When nothing is consumed due to no newline, extras_events also stays empty.
        let result = append_journal_diff(b"incomplete line without newline");
        assert_eq!(result.consumed_bytes, 0);
        assert!(result.extras_events.new_trials.is_empty());
        assert!(result.extras_events.intermediate_values.is_empty());
        assert!(result.extras_events.state_changes.is_empty());
    });
}
