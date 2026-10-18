//! Parser for Optuna JournalStorage (JSON Lines).
//!
//! Provides bulk parsing (`parse_journal`), fast study-list scanning (`scan_study_list`),
//! and on-demand parsing of a given study (`parse_single_study` / `parse_single_study_streaming`).
//!
//! Reference: docs/implements/TASK-101/journal-parser-requirements.md

mod builders;
pub(crate) mod distribution;
mod finalize;
mod state;
pub mod study;
pub mod study_projection;
mod types;

use crate::io::optuna_json::parse as parse_json;
use serde_json::Value;

use super::line_u32_field;
use finalize::finalize_state;
use state::{get_str, get_u64, ParserState};

pub use types::{OptimizationDirection, ParseResult, StudyMeta};

#[cfg(test)]
use builders::TrialBuilder;
#[cfg(test)]
use distribution::Distribution;

/// Bulk-parses the entire journal and returns `StudyMeta` for all studies.
///
/// Also builds each study's `DataFrame` (COMPLETE trials) and `StudyExtras` (auxiliary
/// info for all states) and stores them in the shared store (`crate::dataframe`). Invalid
/// JSON lines are skipped; returns an error if there are no valid lines at all.
pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
    let start = std::time::Instant::now();

    if data.is_empty() {
        crate::dataframe::store_dataframes(vec![]);
        return Ok(ParseResult {
            studies: vec![],
            duration_ms: 0.0,
        });
    }

    let text = String::from_utf8_lossy(data);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.is_empty() {
        crate::dataframe::store_dataframes(vec![]);
        return Ok(ParseResult {
            studies: vec![],
            duration_ms: 0.0,
        });
    }

    let mut state = ParserState::new();
    let mut valid_lines: u32 = 0;

    for line in &lines {
        if let Ok(json) = parse_json(line.trim()) {
            valid_lines += 1;
            if let Some(op) = get_u64(&json, "op_code") {
                #[allow(clippy::cast_possible_truncation)]
                state.process_op(op as u8, &json);
            }
        }
    }

    if valid_lines == 0 {
        return Err("No valid JSON lines found in journal".to_string());
    }

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let finalized = finalize_state(state);
    let mut studies = Vec::with_capacity(finalized.len());
    let mut dataframes = Vec::with_capacity(finalized.len());
    let mut extras = Vec::with_capacity(finalized.len());
    for study in finalized {
        studies.push(study.meta);
        dataframes.push(study.dataframe);
        extras.push(study.extras);
    }
    crate::dataframe::store_dataframes(dataframes);
    crate::dataframe::store_extras(extras);

    Ok(ParseResult {
        studies,
        duration_ms,
    })
}

/// Phase 1: scans only op_code=0/3 to quickly get the list of studies.
/// Trial data is not processed at all, so this returns instantly even for large files.
/// StudyMeta's completed_trials / param_names etc. are 0 / empty (finalized in Phase 2).
pub fn scan_study_list(data: &[u8]) -> Result<Vec<StudyMeta>, String> {
    if data.is_empty() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(data);
    let mut studies: Vec<StudyMeta> = Vec::new();
    // For checking duplicate study names (avoids a linear Vec scan).
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // op_code is always the first field in each line of an Optuna journal (`{"op_code":N,...`).
        // Extract it once and branch on it, eliminating repeated full-line `contains` scans.
        // Trial lines (op_code 4/5/6/8/9) make up over 99% of the total, so exclude them immediately here.
        let op = match line_u32_field(line, "op_code") {
            Some(op) => u64::from(op),
            None => continue,
        };
        if op != 0 && op != 3 {
            continue;
        }
        // Most op3 lines are huge sampler attribute arrays. Only lines with metric_names
        // are actually needed, so JSON parsing is skipped entirely for the rest.
        if op == 3 && !line.contains("study:metric_names") {
            continue;
        }
        let Ok(json) = parse_json(line) else {
            continue;
        };
        match op {
            0 => {
                let name = get_str(&json, "study_name").unwrap_or("").to_string();
                // Skip duplicate create_study lines for the same study name (O(1) check via HashSet).
                if !seen_names.insert(name.clone()) {
                    continue;
                }
                let directions = json
                    .get("directions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|d| match d.as_u64() {
                                Some(1) => OptimizationDirection::Minimize,
                                Some(2) => OptimizationDirection::Maximize,
                                _ => OptimizationDirection::Minimize,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let study_id = studies.len() as u32;
                studies.push(StudyMeta {
                    study_id,
                    name,
                    directions,
                    completed_trials: 0,
                    total_trials: 0,
                    param_names: vec![],
                    objective_names: vec![],
                    user_attr_names: vec![],
                    has_constraints: false,
                    param_bounds: std::collections::HashMap::new(),
                });
            }
            3 => {
                let study_id = get_u64(&json, "study_id").unwrap_or(0) as usize;
                if study_id >= studies.len() {
                    continue;
                }
                if let Some(attrs) = json.get("system_attr").and_then(|v| v.as_object()) {
                    if let Some(names_arr) =
                        attrs.get("study:metric_names").and_then(|v| v.as_array())
                    {
                        let names: Vec<String> = names_arr
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect();
                        if !names.is_empty() {
                            studies[study_id].objective_names = names;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if studies.is_empty() {
        return Err("No studies found in journal".to_string());
    }
    Ok(studies)
}

/// Common handling of op_code=0 (CREATE_STUDY) / 3 (SET_STUDY_SYSTEM_ATTR)
/// (shared by `parse_single_study` Pass 1 and `parse_single_study_streaming`).
///
/// op0 is parsed as JSON immediately since it's rare. Most op3 lines are huge
/// sampler attribute lines, so only lines containing `study:metric_names` are
/// parsed. The return value indicates whether to count it as a valid line
/// (for op0 only on successful parse; for op3 always true).
fn process_study_meta_op(state: &mut ParserState, op: u8, line: &str) -> bool {
    match op {
        0 => {
            if let Ok(json) = parse_json(line) {
                state.process_op(0, &json);
                true
            } else {
                false
            }
        }
        3 => {
            if line.contains("study:metric_names") {
                if let Ok(json) = parse_json(line) {
                    state.process_op(3, &json);
                }
            }
            true
        }
        _ => false,
    }
}

/// Common handling of op_code=4 lines for non-target studies: skips JSON parsing and
/// only updates the trial_id counter and that study's total_trials (shared by
/// `parse_single_study` / streaming).
fn count_other_study_trial(state: &mut ParserState, sid: u32) {
    state.next_trial_id += 1;
    if (sid as usize) < state.studies.len() {
        state.studies[sid as usize].total_trials += 1;
    }
}

/// Phase 2: parses only the trial data for the given study_id and returns (StudyMeta, DataFrame).
///
/// Uses a 3-pass design for speed:
///   Pass 1 (sequential): string-scans all lines, collecting target lines and managing counters
///   Pass 2 (rayon parallel): parallel JSON-parses the collected lines
///   Pass 3 (sequential): applies the parsed results to state while preserving order
///
/// For an N-study file with a single target study, the amount of JSON parsing is reduced
/// to roughly 1/N. Parallelization via rayon gives further speedup depending on core count.
pub fn parse_single_study(
    data: &[u8],
    target_study_id: u32,
) -> Result<
    (
        StudyMeta,
        crate::data::dataframe::DataFrame,
        crate::data::extras::StudyExtras,
    ),
    String,
> {
    use rayon::prelude::*;

    if data.is_empty() {
        return Err("Empty journal data".to_string());
    }
    let text = String::from_utf8_lossy(data);
    let mut state = ParserState::new_with_target(target_study_id);
    // Set of trial_ids belonging to the target study (used to filter ops 5/6/8/9)
    let mut target_trial_ids = std::collections::HashSet::<u32>::new();
    // For Pass 2: (line_ref, op_code, pre_trial_id-for-op4)
    let mut deferred: Vec<(&str, u8, Option<u32>)> = Vec::new();
    let mut any_valid = false;

    // ── Pass 1: sequential string scan ──────────────────────────────────
    // op_code is the first field of each line. Extract it once and branch with match.
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let op = match line_u32_field(line, "op_code") {
            #[allow(clippy::cast_possible_truncation)]
            Some(op) => op as u8,
            None => continue,
        };

        match op {
            0 | 3 => {
                if process_study_meta_op(&mut state, op, line) {
                    any_valid = true;
                }
            }
            4 => {
                any_valid = true;
                let pre_trial_id = state.next_trial_id;
                match line_u32_field(line, "study_id") {
                    Some(sid) if sid == target_study_id => {
                        // Target study → defer to Pass 2 (advance the counter now)
                        state.next_trial_id += 1;
                        target_trial_ids.insert(pre_trial_id);
                        deferred.push((line, 4, Some(pre_trial_id)));
                    }
                    Some(sid) => {
                        // Other study → no JSON needed, just update the counter
                        count_other_study_trial(&mut state, sid);
                    }
                    None => {
                        // Extraction failed → fall back to full parsing for safety
                        let tid = state.next_trial_id;
                        if let Ok(json) = parse_json(line) {
                            state.process_op(4, &json);
                            if state.trial_builders.contains_key(&tid) {
                                target_trial_ids.insert(tid);
                            }
                        }
                    }
                }
            }
            5..=9 => {
                // Trial-update ops (including op7 intermediate values): only lines for target trial_ids go to Pass 2
                any_valid = true;
                if let Some(tid) = line_u32_field(line, "trial_id") {
                    if target_trial_ids.contains(&tid) {
                        deferred.push((line, op, None));
                    }
                }
            }
            _ => {}
        }
    }

    if !any_valid {
        return Err("No valid JSON lines found in journal".to_string());
    }

    // ── Pass 2: parallel JSON parse ──────────────────────────────────────
    // &str has the same lifetime as text and is Send, so it can be safely sent to rayon threads.
    let parsed: Vec<(u8, Value, Option<u32>)> = deferred
        .par_iter()
        .filter_map(|(line, op, pre_tid)| parse_json(line).ok().map(|v| (*op, v, *pre_tid)))
        .collect();
    drop(deferred);

    // ── Pass 3: sequential state update (preserving order) ───────────────────────
    for (op, json, pre_tid) in parsed {
        if op == 4 {
            // For op_code=4, use the trial_id assigned in Pass 1
            if let Some(tid) = pre_tid {
                state.next_trial_id = tid;
            }
        }
        state.process_op(op, &json);
    }

    let study = finalize_state(state)
        .into_iter()
        .find(|s| s.meta.study_id == target_study_id)
        .ok_or_else(|| format!("study_id {target_study_id} not found in journal"))?;

    Ok((study.meta, study.dataframe, study.extras))
}

/// Transitional table delivery over the owned Journal replay. Corrections replace
/// the derived table; use `apply_to` rather than assuming every batch only appends.
pub struct StudyStreamBatch {
    /// The cumulative StudyMeta up to this point (user_attr_names is merged and sorted).
    pub meta: StudyMeta,
    /// Trial rows newly completed in this batch.
    pub new_rows: Vec<crate::dataframe::TrialRow>,
    /// Cumulative parameter column names (sorted, for DataFrame construction).
    pub param_names: Vec<String>,
    /// Objective column names.
    pub objective_names: Vec<String>,
    /// Cumulative numeric user_attr column names (sorted).
    pub user_attr_numeric_names: Vec<String>,
    /// Cumulative string user_attr column names (sorted).
    pub user_attr_string_names: Vec<String>,
    /// Current all-state constraint width, which may shrink after replacement.
    pub max_constraints: usize,
    /// Whether this is the first batch (used by the UI side to decide whether to create a new StudyContext).
    pub is_first: bool,
    /// Whether this is the final batch (used by the UI side to finalize the Pareto computation and end loading).
    pub is_final: bool,
    /// Present when previously emitted rows or their constraint width changed.
    pub replacement: Option<crate::dataframe::DataFrame>,
}

impl StudyStreamBatch {
    pub fn apply_to(self, dataframe: &mut crate::dataframe::DataFrame) {
        if let Some(replacement) = self.replacement {
            *dataframe = replacement;
        } else {
            dataframe.append_trials(
                &self.new_rows,
                &self.param_names,
                &self.objective_names,
                &self.user_attr_numeric_names,
                &self.user_attr_string_names,
                self.max_constraints,
            );
            if self.meta.has_constraints && !dataframe.feasibility().has_constraints() {
                dataframe.mark_constrained();
            }
        }
    }
}

/// Emits early COMPLETE prefixes and batched corrections using the same replay
/// and projection as the GUI. The final callback always arrives, even empty.
/// The legacy extras publication remains until #251 removes this adapter.
pub fn parse_single_study_streaming<F>(
    data: &[u8],
    target_study_id: u32,
    batch_size: usize,
    mut on_batch: F,
) -> Result<(), String>
where
    F: FnMut(StudyStreamBatch),
{
    use tdashboard_study::*;
    let key = StudyKey {
        source: IdentityAllocator::new()
            .and_then(|mut a| a.source())
            .map_err(|e| e.to_string())?,
        stored_id: target_study_id,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager
        .begin(LoadKind::Initial)
        .map_err(|e| e.to_string())?;
    let mut header = manager.header(ticket).map_err(|e| e.to_string())?;
    let mut table = study_projection::TableProjection::default();
    let mut emitted = std::collections::HashSet::new();
    let mut width = 0;
    let mut first = true;
    // Reuse owned replay rather than maintaining a second late-update parser.
    let mut deliver = |batch: StudyBatch, is_final| -> Result<(), String> {
        let updates = match &batch {
            StudyBatch::Replace { trials, .. } => trials,
            StudyBatch::Delta { upserts, .. } => upserts,
        };
        let corrected = updates.iter().any(|t| emitted.contains(&t.key.stored_id));
        header = manager
            .apply(header, batch)
            .map_err(|e| e.to_string())?
            .next_header(ticket);
        if is_final {
            manager
                .finish(ticket, header.sequence)
                .map_err(|e| e.to_string())?;
        }
        let input = manager.acquire().ok_or("No acquired Journal input")?;
        let mut projection = table.project(&input);
        let new_rows = projection
            .trial_rows
            .drain(..)
            .filter(|r| emitted.insert(r.trial_id))
            .collect();
        let new_width = input.data().constraint_schema().width;
        let mut user_attr_numeric_names =
            projection.dataframe.user_attr_numeric_col_names().to_vec();
        user_attr_numeric_names.sort();
        let mut user_attr_string_names = projection.dataframe.user_attr_string_col_names().to_vec();
        user_attr_string_names.sort();
        let replacement = (corrected || new_width < width).then_some(projection.dataframe);
        width = new_width;
        if is_final {
            crate::dataframe::store_extras_for(target_study_id, projection.extras);
        }
        on_batch(StudyStreamBatch {
            param_names: projection.meta.param_names.clone(),
            objective_names: projection.meta.objective_names.clone(),
            user_attr_numeric_names,
            user_attr_string_names,
            meta: projection.meta,
            new_rows,
            max_constraints: width,
            is_first: first,
            is_final,
            replacement,
        });
        first = false;
        Ok(())
    };
    study::read_batches_with_final(data, key, batch_size, &mut deliver)
}

#[cfg(test)]
mod tests;
