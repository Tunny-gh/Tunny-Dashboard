//! Live update module for incremental Journal diff parsing.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-1201

use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// Result types
// =============================================================================

#[derive(Debug, Clone)]
pub struct AppendDiffResult {
    pub consumed_bytes: usize,
    pub pending_running: usize,
    pub new_trial_rows: Vec<TrialRow>,
    pub updated_study_counts: Vec<(u32, usize)>,
}

/// Trial row data built from incremental diff parsing.
#[derive(Debug, Clone)]
pub struct TrialRow {
    pub trial_id: u32,
    pub trial_number: u32,
    pub params: HashMap<String, f64>,
    pub param_categories: HashMap<String, String>,
    pub objectives: Vec<f64>,
    pub user_attrs_numeric: HashMap<String, f64>,
    pub user_attrs_string: HashMap<String, String>,
    pub constraint_values: Vec<f64>,
    pub study_id: u32,
}

// =============================================================================
// Internal state
// =============================================================================

#[derive(Debug, Default)]
struct PendingTrial {
    study_idx: u32,
    values: Option<Vec<f64>>,
    param_display: HashMap<String, f64>,
    param_category_label: HashMap<String, String>,
    user_attrs_numeric: HashMap<String, f64>,
    user_attrs_string: HashMap<String, String>,
    constraint_values: Vec<f64>,
}

#[derive(Debug, Default)]
struct LiveUpdateState {
    next_trial_id: u32,
    pending: HashMap<u32, PendingTrial>,
}

// =============================================================================
// Context types
// =============================================================================

/// Context passed to the polling thread for incremental parsing.
#[derive(Debug, Clone)]
pub struct LiveUpdateContext {
    pub file_path: PathBuf,
    pub initial_byte_offset: u64,
    pub next_trial_id: u32,
    pub study_distributions: Vec<StudyDistributionInfo>,
    /// Milliseconds of no file change before sending completion hint (default: 60_000)
    pub no_change_timeout_ms: u64,
}

/// Per-study distribution info needed for incremental TrialRow construction.
#[derive(Debug, Clone)]
pub struct StudyDistributionInfo {
    pub study_id: u32,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub distributions: HashMap<String, Value>,
}

// =============================================================================
// thread_local state
// =============================================================================

thread_local! {
    static STATE: RefCell<LiveUpdateState> = RefCell::new(LiveUpdateState::default());
}

// =============================================================================
// Core function
// =============================================================================

/// Parse incremental Journal diff and build TrialRow data for completed trials.
pub fn append_journal_diff(data: &[u8]) -> AppendDiffResult {
    let consumed = find_consumed_bytes(data);
    if consumed == 0 {
        let pending_running = STATE.with(|s| s.borrow().pending.len());
        return AppendDiffResult {
            consumed_bytes: 0,
            pending_running,
            new_trial_rows: vec![],
            updated_study_counts: vec![],
        };
    }

    let complete_data = &data[..consumed];
    let mut new_trial_rows: Vec<TrialRow> = Vec::new();

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        for line in complete_data.split(|&b| b == b'\n') {
            let trimmed = line
                .iter()
                .position(|&b| b != b' ' && b != b'\r' && b != b'\t')
                .map(|i| &line[i..])
                .unwrap_or(line);
            if trimmed.is_empty() {
                continue;
            }

            let json: Value = match serde_json::from_slice(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let op = json
                .get("op_code")
                .and_then(|v| v.as_u64())
                .unwrap_or(u64::MAX) as u8;

            match op {
                4 => {
                    let study_idx =
                        json.get("study_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let trial_id = s.next_trial_id;
                    s.next_trial_id += 1;
                    s.pending.insert(
                        trial_id,
                        PendingTrial {
                            study_idx,
                            ..Default::default()
                        },
                    );
                }
                5 => {
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    if let Some(pending) = s.pending.get_mut(&trial_id) {
                        if let (Some(name), Some(val)) = (
                            json.get("param_name").and_then(|v| v.as_str()),
                            json.get("param_value_internal").and_then(|v| v.as_f64()),
                        ) {
                            let display_val = decode_param_value(val, json.get("distribution"));
                            let label = extract_categorical_label(val, json.get("distribution"));
                            if let Some(lbl) = label {
                                pending.param_category_label.insert(name.to_string(), lbl);
                            } else {
                                pending.param_display.insert(name.to_string(), display_val);
                            }
                        }
                    }
                }
                6 => {
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    let state_val = json.get("state").and_then(|v| v.as_u64()).unwrap_or(0) as u8;

                    if state_val == 1 {
                        if let Some(mut pending) = s.pending.remove(&trial_id) {
                            if let Some(vals_json) = json.get("values").and_then(|v| v.as_array()) {
                                pending.values =
                                    Some(vals_json.iter().filter_map(|v| v.as_f64()).collect());
                            }
                            let row = TrialRow {
                                trial_id,
                                trial_number: trial_id,
                                params: pending.param_display,
                                param_categories: pending.param_category_label,
                                objectives: pending.values.unwrap_or_default(),
                                user_attrs_numeric: pending.user_attrs_numeric,
                                user_attrs_string: pending.user_attrs_string,
                                constraint_values: pending.constraint_values,
                                study_id: pending.study_idx,
                            };
                            new_trial_rows.push(row);
                        } else {
                            new_trial_rows.push(TrialRow {
                                trial_id,
                                trial_number: trial_id,
                                params: HashMap::new(),
                                param_categories: HashMap::new(),
                                objectives: vec![],
                                user_attrs_numeric: HashMap::new(),
                                user_attrs_string: HashMap::new(),
                                constraint_values: vec![],
                                study_id: 0,
                            });
                        }
                    } else if state_val == 2 || state_val == 3 {
                        s.pending.remove(&trial_id);
                    }
                }
                _ => {}
            }
        }
    });

    let pending_running = STATE.with(|s| s.borrow().pending.len());

    let mut study_counts: HashMap<u32, usize> = HashMap::new();
    for row in &new_trial_rows {
        *study_counts.entry(row.study_id).or_insert(0) += 1;
    }
    let updated_study_counts: Vec<(u32, usize)> = study_counts.into_iter().collect();

    AppendDiffResult {
        consumed_bytes: consumed,
        pending_running,
        new_trial_rows,
        updated_study_counts,
    }
}

pub fn reset_live_update_state() {
    STATE.with(|s| *s.borrow_mut() = LiveUpdateState::default());
}

pub fn set_next_trial_id(id: u32) {
    STATE.with(|s| s.borrow_mut().next_trial_id = id);
}

// =============================================================================
// Helpers
// =============================================================================

fn find_consumed_bytes(data: &[u8]) -> usize {
    match data.iter().rposition(|&b| b == b'\n') {
        Some(pos) => pos + 1,
        None => 0,
    }
}

fn decode_param_value(internal: f64, dist: Option<&Value>) -> f64 {
    let Some(dist) = dist else { return internal };
    match dist.get("name").and_then(|v| v.as_str()).unwrap_or("") {
        "FloatDistribution" => {
            if dist.get("log").and_then(|v| v.as_bool()).unwrap_or(false) {
                internal.exp()
            } else {
                internal
            }
        }
        "IntDistribution" => {
            let low = dist.get("low").and_then(|v| v.as_i64()).unwrap_or(0);
            let step = dist
                .get("step")
                .and_then(|v| v.as_i64())
                .unwrap_or(1)
                .max(1);
            let log = dist.get("log").and_then(|v| v.as_bool()).unwrap_or(false);
            let rounded = if log {
                internal.exp().round() as i64
            } else {
                internal.round() as i64
            };
            (low + rounded * step) as f64
        }
        _ => internal,
    }
}

fn extract_categorical_label(internal: f64, dist: Option<&Value>) -> Option<String> {
    let dist = dist?;
    if dist.get("name").and_then(|v| v.as_str())? != "CategoricalDistribution" {
        return None;
    }
    let choices = dist.get("choices")?.as_array()?;
    let idx = internal.round() as usize;
    choices.get(idx).map(|v| match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
    fn tc_2218_08_log_param_is_exp_decoded() {
        with_fresh_state(|| {
            let create = make_create_trial(0);
            let internal = std::f64::consts::LN_2; // ln(2.0)
            let set_log = format!(
                r#"{{"op_code":5,"trial_id":0,"param_name":"lr","param_value_internal":{},"distribution":{{"name":"FloatDistribution","low":0.0001,"high":1.0,"log":true}}}}"#,
                internal
            );
            let complete = make_complete(0, &[1.0]);
            let data = make_diff_bytes(&[create, set_log, complete]);
            let result = append_journal_diff(&data);

            assert_eq!(result.new_trial_rows.len(), 1);
            let row = &result.new_trial_rows[0];
            let decoded = row.params.get("lr").copied().unwrap_or(0.0);
            assert!(
                (decoded - 2.0).abs() < 1e-6,
                "expected ~2.0, got {}",
                decoded
            );
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
}
