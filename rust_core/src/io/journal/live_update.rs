//! Live update module for incremental Journal diff parsing.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-1201

use super::line_u32_field;
use super::parser::distribution::Distribution;
use crate::io::datetime::parse_naive_datetime;
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
    /// Diff events to apply to the extras (auxiliary info) of all trials (all states).
    pub extras_events: ExtrasDiff,
}

/// Extras (state / datetime / intermediate value) update events extracted from a live diff.
///
/// While `new_trial_rows` only handles COMPLETE trials, this collects events for all states.
/// The consumer (egui-app) merges these into the study's [`crate::extras::StudyExtras`].
#[derive(Debug, Clone, Default)]
pub struct ExtrasDiff {
    /// op_code=4 (CREATE_TRIAL): (trial_id, study_id, trial_number, datetime_start).
    pub new_trials: Vec<(u32, u32, u32, Option<f64>)>,
    /// op_code=7 (SET_TRIAL_INTERMEDIATE_VALUE): (trial_id, step, value).
    pub intermediate_values: Vec<(u32, u64, f64)>,
    /// op_code=6 (SET_TRIAL_STATE_VALUES): (trial_id, state, datetime_complete). Records all states.
    pub state_changes: Vec<(u32, u8, Option<f64>)>,
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
    /// 0-based trial.number within the study (fixed at creation time).
    trial_number: u32,
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
    /// study_id → number of trials created so far (i.e. the next trial.number).
    /// Seeded from the existing file's per-study creation count when going live.
    next_trial_number: HashMap<u32, u32>,
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
    /// Per-study creation counts from the existing file (study_id → count). Seeds each study's next trial.number.
    pub study_trial_number_seeds: HashMap<u32, u32>,
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
            extras_events: ExtrasDiff::default(),
        };
    }

    let complete_data = &data[..consumed];
    let mut new_trial_rows: Vec<TrialRow> = Vec::new();
    let mut extras = ExtrasDiff::default();

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
                    // Assign the 0-based trial.number within the study from the per-study counter.
                    let counter = s.next_trial_number.entry(study_idx).or_insert(0);
                    let trial_number = *counter;
                    *counter += 1;
                    let datetime_start = json
                        .get("datetime_start")
                        .and_then(|v| v.as_str())
                        .and_then(parse_naive_datetime);
                    extras
                        .new_trials
                        .push((trial_id, study_idx, trial_number, datetime_start));
                    s.pending.insert(
                        trial_id,
                        PendingTrial {
                            study_idx,
                            trial_number,
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
                            // The journal's distribution field is a JSON string; from_json
                            // handles both re-parsing the string and the nested attributes.
                            let dist = json.get("distribution").map(Distribution::from_json);
                            let display_val = dist.as_ref().map_or(val, |d| d.to_display_f64(val));
                            let label = dist.as_ref().and_then(|d| d.categorical_label(val));
                            if let Some(lbl) = label {
                                pending.param_category_label.insert(name.to_string(), lbl);
                            } else {
                                pending.param_display.insert(name.to_string(), display_val);
                            }
                        }
                    }
                }
                7 => {
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    let step = json.get("step").and_then(|v| v.as_u64()).unwrap_or(0);
                    if let Some(value) = json.get("intermediate_value").and_then(|v| v.as_f64()) {
                        extras.intermediate_values.push((trial_id, step, value));
                    }
                }
                9 => {
                    // SET_TRIAL_SYSTEM_ATTR: only the "constraints" key matters
                    // for live rows (feasibility columns), same as the full parser.
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    if let Some(pending) = s.pending.get_mut(&trial_id) {
                        if let Some(values) = json
                            .get("system_attr")
                            .and_then(|attr| attr.get("constraints"))
                            .and_then(|v| v.as_array())
                        {
                            pending.constraint_values =
                                values.iter().filter_map(|v| v.as_f64()).collect();
                        }
                    }
                }
                6 => {
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    let state_val = json.get("state").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                    let datetime_complete = json
                        .get("datetime_complete")
                        .and_then(|v| v.as_str())
                        .and_then(parse_naive_datetime);
                    // Record all state changes regardless of state (including COMPLETE/PRUNED/FAIL).
                    extras
                        .state_changes
                        .push((trial_id, state_val, datetime_complete));

                    if state_val == 1 {
                        if let Some(mut pending) = s.pending.remove(&trial_id) {
                            if let Some(vals_json) = json.get("values").and_then(|v| v.as_array()) {
                                pending.values =
                                    Some(vals_json.iter().filter_map(|v| v.as_f64()).collect());
                            }
                            let row = TrialRow {
                                trial_id,
                                trial_number: pending.trial_number,
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
        extras_events: extras,
    }
}

pub fn reset_live_update_state() {
    STATE.with(|s| *s.borrow_mut() = LiveUpdateState::default());
}

pub fn set_next_trial_id(id: u32) {
    STATE.with(|s| s.borrow_mut().next_trial_id = id);
}

/// Seeds the per-study "next trial.number" counter.
/// Passing in the existing file's per-study creation counts
/// ([`count_created_trials_per_study`]) makes trials created during live updates
/// receive trial.numbers that continue consecutively within each study.
pub fn set_study_trial_number_seeds(seeds: HashMap<u32, u32>) {
    STATE.with(|s| s.borrow_mut().next_trial_number = seeds);
}

/// Counts op_code=4 (CREATE_TRIAL) records in the existing file, per study_id.
/// The returned `study_id → count` equals the trial.number of the next trial created in each study.
pub fn count_created_trials_per_study(data: &[u8]) -> HashMap<u32, u32> {
    let text = String::from_utf8_lossy(data);
    let mut counts: HashMap<u32, u32> = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if line_u32_field(trimmed, "op_code") == Some(4) {
            let study_id = line_u32_field(trimmed, "study_id").unwrap_or(0);
            *counts.entry(study_id).or_insert(0) += 1;
        }
    }
    counts
}

/// Computes the global trial_id that Optuna will assign to the next CREATE_TRIAL.
///
/// Optuna's Journal storage assigns trial_id sequentially in order of op_code=4
/// (CREATE_TRIAL) occurrence, **across all studies and all states (including
/// running/failed/pruned)**. So the total count of op_code=4 records in the file
/// equals the trial_id of the next trial to be created. The `next_trial_id` at the
/// start of live updates must equal this value, or it won't match the trial_id in
/// op_code=5/6 records and trials won't be built correctly.
pub fn count_created_trials(data: &[u8]) -> u32 {
    let text = String::from_utf8_lossy(data);
    let mut count = 0u32;
    for line in text.lines() {
        if line_op_code(line.trim_start()) == Some(4) {
            count += 1;
        }
    }
    count
}

/// Extracts the op_code value from a line (lightweight version, no JSON parsing required).
/// Extraction itself is done by [`line_u32_field`] (a shared `io::journal` helper, no per-line alloc).
fn line_op_code(line: &str) -> Option<u8> {
    line_u32_field(line, "op_code").and_then(|v| u8::try_from(v).ok())
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
                r#"{"op_code":9,"trial_id":0,"system_attr":{"constraints":[-0.5,0.25]}}"#
                    .to_string(),
                make_complete(0, &[1.0]),
            ];
            let data = make_diff_bytes(&lines);
            let result = append_journal_diff(&data);

            assert_eq!(result.new_trial_rows.len(), 1);
            assert_eq!(result.new_trial_rows[0].constraint_values, vec![-0.5, 0.25]);
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
}
