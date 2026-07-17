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
mod types;

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
        if let Ok(json) = serde_json::from_str::<Value>(line.trim()) {
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
        let Ok(json) = serde_json::from_str::<Value>(line) else {
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
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                state.process_op(0, &json);
                true
            } else {
                false
            }
        }
        3 => {
            if line.contains("study:metric_names") {
                if let Ok(json) = serde_json::from_str::<Value>(line) {
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
                        if let Ok(json) = serde_json::from_str::<Value>(line) {
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
        .filter_map(|(line, op, pre_tid)| {
            serde_json::from_str::<Value>(line)
                .ok()
                .map(|v| (*op, v, *pre_tid))
        })
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

/// A batch that `parse_single_study_streaming` passes to the sequential callback.
///
/// Emits completed trials forward-streamed in groups of `batch_size`. Each batch is
/// bundled with cumulative metadata up to that point (column name sets and counts) so
/// the UI side can rebuild the DataFrame, including newly added columns. The final batch
/// has `is_final = true` (emitted even with 0 remaining rows).
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
    /// The maximum number of constraints observed so far.
    pub max_constraints: usize,
    /// Whether this is the first batch (used by the UI side to decide whether to create a new StudyContext).
    pub is_first: bool,
    /// Whether this is the final batch (used by the UI side to finalize the Pareto computation and end loading).
    pub is_final: bool,
}

/// Accumulated state for `parse_single_study_streaming` (batch, column name sets, counters).
///
/// Consolidates into a single struct the mutable state that the old
/// `stream_emit_completed_trial` used to pass around via roughly 15 arguments
/// (behavior is unchanged).
struct StreamAccum {
    batch: Vec<crate::dataframe::TrialRow>,
    batch_size: usize,
    param_set: std::collections::BTreeSet<String>,
    uan_set: std::collections::BTreeSet<String>,
    uas_set: std::collections::BTreeSet<String>,
    derived_objective_names: Vec<String>,
    has_constraints: bool,
    max_constraints: usize,
    completed: u32,
    first_sent: bool,
}

impl StreamAccum {
    fn new(batch_size: usize) -> Self {
        let batch_size = batch_size.max(1);
        StreamAccum {
            batch: Vec::with_capacity(batch_size),
            batch_size,
            param_set: std::collections::BTreeSet::new(),
            uan_set: std::collections::BTreeSet::new(),
            uas_set: std::collections::BTreeSet::new(),
            derived_objective_names: Vec::new(),
            has_constraints: false,
            max_constraints: 0,
            completed: 0,
            first_sent: false,
        }
    }

    /// Chooses the objective names: uses the confirmed names from the study builder
    /// if available, otherwise `obj{i}` derived from the number of values in completed trials.
    fn objective_names(&self, state: &ParserState, target: u32) -> Vec<String> {
        let builder_names = &state.studies[target as usize].objective_names;
        if builder_names.is_empty() {
            self.derived_objective_names.clone()
        } else {
            builder_names.clone()
        }
    }

    /// Builds a StudyMeta snapshot from the accumulated sets.
    fn snapshot_meta(&self, state: &ParserState, target: u32) -> StudyMeta {
        let builder = &state.studies[target as usize];
        // user_attr_names merges numeric and string names and sorts them (same as the existing finalize step).
        let mut user_attr_names: Vec<String> = self
            .uan_set
            .iter()
            .chain(self.uas_set.iter())
            .cloned()
            .collect();
        user_attr_names.sort();
        user_attr_names.dedup();
        StudyMeta {
            study_id: target,
            name: builder.name.clone(),
            directions: builder.directions.clone(),
            completed_trials: self.completed,
            total_trials: builder.total_trials,
            param_names: self.param_set.iter().cloned().collect(),
            objective_names: self.objective_names(state, target),
            user_attr_names,
            has_constraints: self.has_constraints,
            param_bounds: builder.param_bounds.clone(),
        }
    }

    /// Builds and emits a `StudyStreamBatch` from the current accumulated content
    /// (`batch` is taken and left empty).
    fn send_batch<F>(&mut self, state: &ParserState, target: u32, is_final: bool, on_batch: &mut F)
    where
        F: FnMut(StudyStreamBatch),
    {
        let meta = self.snapshot_meta(state, target);
        let objective_names = self.objective_names(state, target);
        on_batch(StudyStreamBatch {
            meta,
            new_rows: std::mem::take(&mut self.batch),
            param_names: self.param_set.iter().cloned().collect(),
            objective_names,
            user_attr_numeric_names: self.uan_set.iter().cloned().collect(),
            user_attr_string_names: self.uas_set.iter().cloned().collect(),
            max_constraints: self.max_constraints,
            is_first: !self.first_sent,
            is_final,
        });
        self.first_sent = true;
    }

    /// Takes a completed trial's (state==1) builder into the batch, updating the column
    /// name sets and counters. Emits via `on_batch` once the batch is full.
    fn emit_completed_trial<F>(
        &mut self,
        tid: u32,
        builder: builders::TrialBuilder,
        state: &ParserState,
        target: u32,
        on_batch: &mut F,
    ) where
        F: FnMut(StudyStreamBatch),
    {
        use crate::dataframe::TrialRow;

        let b = builder;
        for name in b.param_display.keys() {
            self.param_set.insert(name.clone());
        }
        for name in b.param_category_label.keys() {
            self.param_set.insert(name.clone());
        }
        for name in b.user_attrs_numeric.keys() {
            self.uan_set.insert(name.clone());
        }
        for name in b.user_attrs_string.keys() {
            self.uas_set.insert(name.clone());
        }
        if b.has_constraints {
            self.has_constraints = true;
        }
        self.max_constraints = self.max_constraints.max(b.constraint_values.len());
        if self.derived_objective_names.is_empty() {
            if let Some(values) = &b.values {
                self.derived_objective_names =
                    (0..values.len()).map(|i| format!("obj{i}")).collect();
            }
        }
        self.completed += 1;
        self.batch.push(TrialRow {
            trial_id: tid,
            trial_number: b.trial_number,
            param_display: b.param_display,
            param_category_label: b.param_category_label,
            objective_values: b.values.unwrap_or_default(),
            user_attrs_numeric: b.user_attrs_numeric,
            user_attrs_string: b.user_attrs_string,
            constraint_values: b.constraint_values,
        });

        if self.batch.len() >= self.batch_size {
            self.send_batch(state, target, false, on_batch);
        }
    }

    /// Finalizes a trial's state (shared by op_code=4's inline state and op_code=6).
    ///
    /// In-memory storage carries all trial data (state / values / params) inline in
    /// op_code=4, with no subsequent op_code=6. File storage, on the other hand, completes
    /// via op_code=6. If state==1 (complete), emits the finalized row; if 2/3 (prune/fail),
    /// discards the builder and only records extras. Otherwise (RUNNING), keeps the builder
    /// and waits for a subsequent op.
    fn resolve_trial_state<F>(
        &mut self,
        state: &mut ParserState,
        tid: u32,
        target: u32,
        extras_trials: &mut Vec<crate::data::extras::TrialExtra>,
        on_batch: &mut F,
    ) where
        F: FnMut(StudyStreamBatch),
    {
        let trial_state = state.trial_builders.get(&tid).map(|b| b.state).unwrap_or(0);
        if trial_state == 1 {
            let b = state.trial_builders.remove(&tid).unwrap();
            extras_trials.push(trial_extra_from_builder(tid, &b));
            self.emit_completed_trial(tid, b, state, target, on_batch);
        } else if trial_state == 2 || trial_state == 3 {
            if let Some(b) = state.trial_builders.remove(&tid) {
                extras_trials.push(trial_extra_from_builder(tid, &b));
            }
        }
    }
}

/// Builds a `TrialExtra` from a `TrialBuilder` that's being removed (finalized as
/// complete/prune/fail, or at EOF). Intermediate values are sorted in ascending step order.
fn trial_extra_from_builder(
    trial_id: u32,
    b: &builders::TrialBuilder,
) -> crate::data::extras::TrialExtra {
    let mut intermediate_values = b.intermediate_values.clone();
    intermediate_values.sort_by_key(|(step, _)| *step);
    crate::data::extras::TrialExtra {
        trial_id,
        trial_number: b.trial_number,
        state: crate::data::extras::TrialState::from_journal(b.state),
        datetime_start: b.datetime_start,
        datetime_complete: b.datetime_complete,
        intermediate_values,
    }
}

/// Parses the target study's completed trials in a single forward pass, emitting to
/// `on_batch` every `batch_size` trials.
///
/// Unlike `parse_single_study`, this emits completed trials incrementally without
/// waiting for the entire parse to finish, letting the UI side "render while loading".
/// A trial is finalized as complete at op_code=6 / state==1, incorporating the
/// params/attrs/constraints set up to that point (the same incremental semantics as
/// live_update).
///
/// The column name sets (param/user_attr) accumulate across trials and are bundled
/// into each batch.
///
/// Auxiliary info for all trials (all states) ([`crate::data::extras::StudyExtras`]) is
/// also collected, and stored into the shared store via `store_extras_for` upon
/// completion (the counterpart, for the single-study load path, to `parse_journal`'s
/// storage via `store_extras`).
pub fn parse_single_study_streaming<F>(
    data: &[u8],
    target_study_id: u32,
    batch_size: usize,
    mut on_batch: F,
) -> Result<(), String>
where
    F: FnMut(StudyStreamBatch),
{
    if data.is_empty() {
        return Err("Empty journal data".to_string());
    }
    let text = String::from_utf8_lossy(data);
    let mut state = ParserState::new_with_target(target_study_id);
    let mut acc = StreamAccum::new(batch_size);
    let mut any_valid = false;
    // Auxiliary info for all trials (all states). Appended as trials are removed (complete/prune/fail/EOF).
    let mut extras_trials: Vec<crate::data::extras::TrialExtra> = Vec::new();

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
                let mut parsed_target = false;
                match line_u32_field(line, "study_id") {
                    Some(sid) if sid == target_study_id => {
                        if let Ok(json) = serde_json::from_str::<Value>(line) {
                            state.process_op(4, &json);
                            parsed_target = true;
                        }
                    }
                    Some(sid) => {
                        // Other study: only keep the trial_id counter consistent (no JSON needed).
                        count_other_study_trial(&mut state, sid);
                    }
                    None => {
                        if let Ok(json) = serde_json::from_str::<Value>(line) {
                            state.process_op(4, &json);
                            parsed_target = true;
                        }
                    }
                }
                // In-memory storage carries state/values/params inline in op_code=4, with
                // no subsequent op_code=6. Finalize and emit the completed trial (state==1) here.
                // File storage's op_code=4 has state==0, so the builder is kept, waiting for 5/6.
                if parsed_target {
                    let tid = state.next_trial_id.wrapping_sub(1);
                    acc.resolve_trial_state(
                        &mut state,
                        tid,
                        target_study_id,
                        &mut extras_trials,
                        &mut on_batch,
                    );
                }
            }
            5..=9 => {
                any_valid = true;
                let Some(tid) = line_u32_field(line, "trial_id") else {
                    continue;
                };
                if !state.trial_builders.contains_key(&tid) {
                    continue; // Trial not from the target study → ignore
                }
                let Ok(json) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                state.process_op(op, &json);

                if op != 6 {
                    continue;
                }
                // op6 completion check: emit the finalized row when state==1; discard for 2/3 (prune/fail).
                acc.resolve_trial_state(
                    &mut state,
                    tid,
                    target_study_id,
                    &mut extras_trials,
                    &mut on_batch,
                );
            }
            _ => {}
        }
    }

    if !any_valid {
        return Err("No valid JSON lines found in journal".to_string());
    }
    if (target_study_id as usize) >= state.studies.len() {
        return Err(format!("study_id {target_study_id} not found in journal"));
    }

    // trial_builders still remaining at EOF (only generated for the target study) are
    // still Running, i.e. not yet finalized as complete/prune/fail. Fold them into extras.
    for (tid, b) in state.trial_builders.drain() {
        extras_trials.push(trial_extra_from_builder(tid, &b));
    }
    extras_trials.sort_by_key(|t| t.trial_id);
    crate::dataframe::store_extras_for(
        target_study_id,
        crate::data::extras::StudyExtras {
            trials: extras_trials,
        },
    );

    // Emit the final batch (remainder). Notifies is_final even with 0 completions.
    acc.send_batch(&state, target_study_id, true, &mut on_batch);

    Ok(())
}

#[cfg(test)]
mod tests;
