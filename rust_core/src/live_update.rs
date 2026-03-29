//! Module documentation.
//!
//! Module documentation.
//! Design:
//! Module documentation.
//! Module documentation.
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
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-1201

use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// 🟢 REQ-130〜REQ-133
#[derive(Debug, Clone)]
pub struct AppendDiffResult {
    /// Documentation.
    pub new_completed: usize,
    /// Documentation.
    /// Documentation.
    pub consumed_bytes: usize,
    /// Documentation.
    pub pending_running: usize,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
/// Documentation.
#[derive(Debug, Default)]
#[allow(dead_code)]
struct PendingTrial {
    /// Documentation.
    study_idx: u32,
    /// Documentation.
    values: Option<Vec<f64>>,
    /// Documentation.
    param_display: HashMap<String, f64>,
    /// Documentation.
    param_category_label: HashMap<String, String>,
    /// user_attr numeric type
    user_attrs_numeric: HashMap<String, f64>,
    /// user_attr string type
    user_attrs_string: HashMap<String, String>,
    /// Documentation.
    constraint_values: Vec<f64>,
}

/// Documentation.
#[derive(Debug, Default)]
struct LiveUpdateState {
    /// Documentation.
    next_trial_id: u32,
    /// Documentation.
    pending: HashMap<u32, PendingTrial>,
}

// =============================================================================
// Documentation.
// =============================================================================

thread_local! {
/// Documentation.
/// Documentation.
    static STATE: RefCell<LiveUpdateState> = RefCell::new(LiveUpdateState::default());
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
///
/// 🟢 REQ-130〜REQ-133
pub fn append_journal_diff(data: &[u8]) -> AppendDiffResult {
    // Documentation.
    let consumed = find_consumed_bytes(data);
    if consumed == 0 {
        let pending_running = STATE.with(|s| s.borrow().pending.len());
        return AppendDiffResult {
            new_completed: 0,
            consumed_bytes: 0,
            pending_running,
        };
    }

    let complete_data = &data[..consumed];
    let mut new_completed = 0usize;

    STATE.with(|state| {
        let mut s = state.borrow_mut();

        // Documentation.
        for line in complete_data.split(|&b| b == b'\n') {
            let trimmed = line
                .iter()
                .position(|&b| b != b' ' && b != b'\r' && b != b'\t')
                .map(|i| &line[i..])
                .unwrap_or(line);
            if trimmed.is_empty() {
                continue;
            }

            // Documentation.
            let json: Value = match serde_json::from_slice(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let op = json
                .get("op_code")
                .and_then(|v| v.as_u64())
                .unwrap_or(u64::MAX) as u8;

            match op {
                // Documentation.
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

                // Documentation.
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
                            // Documentation.
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

                // Documentation.
                6 => {
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    let state_val = json.get("state").and_then(|v| v.as_u64()).unwrap_or(0) as u8;

                    if state_val == 1 {
                        // Documentation.
                        if let Some(mut pending) = s.pending.remove(&trial_id) {
                            // Documentation.
                            if let Some(vals_json) = json.get("values").and_then(|v| v.as_array()) {
                                pending.values =
                                    Some(vals_json.iter().filter_map(|v| v.as_f64()).collect());
                            }
                            // Documentation.
                            // Documentation.
                            new_completed += 1;
                        } else {
                            // Documentation.
                            new_completed += 1;
                        }
                    } else if state_val == 2 || state_val == 3 {
                        // Documentation.
                        s.pending.remove(&trial_id);
                    }
                }

                // Documentation.
                _ => {}
            }
        }
    });

    let pending_running = STATE.with(|s| s.borrow().pending.len());

    AppendDiffResult {
        new_completed,
        consumed_bytes: consumed,
        pending_running,
    }
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
pub fn reset_live_update_state() {
    STATE.with(|s| *s.borrow_mut() = LiveUpdateState::default());
}

/// Documentation.
pub fn set_next_trial_id(id: u32) {
    STATE.with(|s| s.borrow_mut().next_trial_id = id);
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
fn find_consumed_bytes(data: &[u8]) -> usize {
    // Documentation.
    match data.iter().rposition(|&b| b == b'\n') {
        Some(pos) => pos + 1,
        None => 0, // Documentation.
    }
}

/// Documentation.
///
/// Documentation.
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

/// Documentation.
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
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Documentation.
    fn with_fresh_state<F: FnOnce()>(f: F) {
        reset_live_update_state();
        f();
        reset_live_update_state();
    }

    // Documentation.
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
        data.push('\n'); // Documentation.
        data.into_bytes()
    }

    // Documentation.
    #[test]
    fn tc_1201_01_complete_trial_counted() {
        // Documentation.
        with_fresh_state(|| {
            let lines = vec![
                make_create_trial(0),
                make_set_param(0, "x1", 0.5),
                make_complete(0, &[1.23]),
            ];
            let data = make_diff_bytes(&lines);
            let result = append_journal_diff(&data);
            // Documentation.
            assert_eq!(result.new_completed, 1, "COMPLETE translated1translated");
            assert_eq!(result.pending_running, 0, "COMPLETEtranslated");
        });
    }

    // Documentation.
    #[test]
    fn tc_1201_02_incomplete_last_line_skipped() {
        // Documentation.
        with_fresh_state(|| {
            let complete = make_create_trial(0);
            let incomplete = r#"{"op_code":4,"study_id":0"#; // Documentation.

            let data = format!("{}\n{}", complete, incomplete).into_bytes();
            let result = append_journal_diff(&data);

            // Documentation.
            let expected_consumed = complete.len() + 1; // +1 for '\n'
            assert_eq!(
                result.consumed_bytes, expected_consumed,
                "translated consumed_bytes translated"
            );
        });
    }

    // Documentation.
    #[test]
    fn tc_1201_03_running_trial_pending() {
        // Documentation.
        with_fresh_state(|| {
            let lines = vec![
                make_create_trial(0), // Documentation.
                make_create_trial(0),
                make_complete(1, &[0.5]), // Documentation.
            ];
            let data = make_diff_bytes(&lines);
            let result = append_journal_diff(&data);

            // Documentation.
            assert_eq!(result.new_completed, 1);
            assert_eq!(result.pending_running, 1, "RUNNING translated1translated");
        });
    }

    // Documentation.
    #[test]
    fn tc_1201_04_no_newline_consumed_zero() {
        // Documentation.
        with_fresh_state(|| {
            let data = b"incomplete line without newline";
            let result = append_journal_diff(data);
            // Documentation.
            assert_eq!(result.consumed_bytes, 0);
            assert_eq!(result.new_completed, 0);
        });
    }

    // Documentation.
    #[test]
    fn tc_1201_05_invalid_json_ignored() {
        // Documentation.
        with_fresh_state(|| {
            let lines = vec![
                "not valid json".to_string(), // Documentation.
                make_create_trial(0),
                make_complete(0, &[1.0]),
            ];
            let data = make_diff_bytes(&lines);
            let result = append_journal_diff(&data);
            // Documentation.
            assert_eq!(result.new_completed, 1);
        });
    }

    // Documentation.
    #[test]
    fn tc_1201_06_cross_diff_running_to_complete() {
        // Documentation.
        with_fresh_state(|| {
            // Documentation.
            let diff1 = make_diff_bytes(&[make_create_trial(0)]);
            let r1 = append_journal_diff(&diff1);
            assert_eq!(r1.new_completed, 0, "1translated RUNNING translated");
            assert_eq!(r1.pending_running, 1);

            // Documentation.
            let diff2 = make_diff_bytes(&[make_complete(0, &[2.0])]);
            let r2 = append_journal_diff(&diff2);
            assert_eq!(r2.new_completed, 1, "2translated COMPLETE translated");
            assert_eq!(r2.pending_running, 0);
        });
    }

    // Documentation.
    #[test]
    fn tc_1201_07_reset_clears_state() {
        // Documentation.
        with_fresh_state(|| {
            let diff1 = make_diff_bytes(&[make_create_trial(0)]);
            append_journal_diff(&diff1);
            // Documentation.
            reset_live_update_state();
            // Documentation.
            let diff2 = make_diff_bytes(&[make_create_trial(0), make_complete(0, &[1.0])]);
            let result = append_journal_diff(&diff2);
            // Documentation.
            assert_eq!(result.new_completed, 1);
        });
    }

    // Documentation.
    #[test]
    fn tc_1201_p01_performance_1000_lines() {
        // Documentation.
        with_fresh_state(|| {
            #[cfg(debug_assertions)]
            let n = 200; // Documentation.
            #[cfg(not(debug_assertions))]
            let n = 1000;

            // Documentation.
            let mut lines = Vec::new();
            for i in 0..n {
                lines.push(make_create_trial(0));
                lines.push(make_set_param(i as u32, "x1", (i as f64) / (n as f64)));
                lines.push(make_complete(i as u32, &[i as f64 * 0.01]));
            }
            let data = make_diff_bytes(&lines);

            let start = std::time::Instant::now();
            let result = append_journal_diff(&data);
            let elapsed = start.elapsed().as_millis();

            // Documentation.
            assert!(elapsed < 100, "{}ms translated 100ms translated", elapsed);
            // Documentation.
            assert_eq!(result.new_completed, n, "COMPLETE translated");
        });
    }
}
