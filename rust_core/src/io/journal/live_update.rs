//! Live update module for incremental Journal diff parsing.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-1201

mod types;

use super::line_u32_field;
use super::parser::distribution::Distribution;
use crate::io::datetime::parse_naive_datetime;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use types::{LiveUpdateState, PendingTrial};

pub use types::{AppendDiffResult, ExtrasDiff, LiveUpdateContext, StudyDistributionInfo, TrialRow};

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
                8 => {
                    // SET_TRIAL_USER_ATTR: classified by the helper shared with
                    // the full parser, so live rows and reloaded rows put a
                    // given record into the same column type.
                    let trial_id = json
                        .get("trial_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(u64::MAX) as u32;
                    if let Some(pending) = s.pending.get_mut(&trial_id) {
                        if let Some(attrs) = json.get("user_attr").and_then(|v| v.as_object()) {
                            crate::io::journal::classify_user_attrs(
                                attrs,
                                &mut pending.user_attrs_numeric,
                                &mut pending.user_attrs_string,
                            );
                        }
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
                        if let Some(values) =
                            crate::io::journal::constraints_from_system_attr(&json)
                        {
                            pending.constraint_values = values;
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
mod tests;
