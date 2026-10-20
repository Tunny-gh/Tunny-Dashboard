//! Journal scanning, bulk decoding and owned incremental replay.
mod builders;
pub mod distribution;
mod finalize;
mod state;
pub mod study;
mod types;

use super::line_u32_field;
use crate::io::optuna_json::parse as parse_json;
use finalize::finalize_state;
use state::{get_u64, ParserState};
pub use types::{OptimizationDirection, ParseResult, StudyMeta};

pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
    let start = std::time::Instant::now();
    let mut state = ParserState::new();
    let mut valid = 0;
    for line in String::from_utf8_lossy(data)
        .lines()
        .filter(|s| !s.trim().is_empty())
    {
        if let Ok(record) = crate::optuna_json::record(line.trim()) {
            valid += 1;
            if let Some(op) = get_u64(&record.json, "op_code") {
                state.process_record(op as u8, &record);
            }
        }
    }
    if valid == 0 && !String::from_utf8_lossy(data).trim().is_empty() {
        return Err("No valid JSON lines found in journal".into());
    }
    let contents: Vec<_> = finalize_state(state)
        .into_iter()
        .map(|s| (s.meta, s.dataframe, s.extras))
        .collect();
    Ok(ParseResult {
        studies: contents.iter().map(|s| s.0.clone()).collect(),
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
        contents,
    })
}

/// Scan only Study creation/metric-name records, without decoding trial payloads.
pub fn scan_study_list(data: &[u8]) -> Result<Vec<StudyMeta>, String> {
    if data.is_empty() {
        return Ok(vec![]);
    }
    let mut state = ParserState::new();
    for line in String::from_utf8_lossy(data).lines() {
        let line = line.trim();
        match line_u32_field(line, "op_code") {
            Some(0) => {
                if let Ok(json) = parse_json(line) {
                    state.process_op(0, &json);
                }
            }
            Some(3) if line.contains("study:metric_names") => {
                if let Ok(json) = parse_json(line) {
                    state.process_op(3, &json);
                }
            }
            _ => (),
        }
    }
    if state.studies.is_empty() {
        return Err("No studies found in journal".into());
    }
    Ok(state
        .studies
        .into_iter()
        .map(|s| StudyMeta {
            study_id: s.study_id,
            name: s.name,
            directions: s.directions,
            completed_trials: 0,
            total_trials: 0,
            param_names: vec![],
            objective_names: s.objective_names,
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: Default::default(),
        })
        .collect())
}

/// The established bulk decoder keeps its target filtering and parallel JSON
/// decoding. The result is owned data, not an installed table.
pub fn parse_single_study(
    data: &[u8],
    target: u32,
) -> Result<
    (
        StudyMeta,
        crate::decoded::DecodedTable,
        crate::extras::StudyExtras,
    ),
    String,
> {
    use rayon::prelude::*;
    if data.is_empty() {
        return Err("Empty journal data".into());
    }
    let text = String::from_utf8_lossy(data);
    let mut state = ParserState::new_with_target(target);
    let mut target_ids = std::collections::HashSet::new();
    let mut deferred = Vec::new();
    let mut valid = false;
    for line in text.lines().map(str::trim).filter(|s| !s.is_empty()) {
        let Some(op) = line_u32_field(line, "op_code").map(|n| n as u8) else {
            continue;
        };
        match op {
            0 | 3 => {
                if op == 3 {
                    valid = true;
                }
                if op == 0 || line.contains("study:metric_names") {
                    if let Ok(json) = parse_json(line) {
                        state.process_op(op, &json);
                        valid = true;
                    }
                }
            }
            4 => {
                valid = true;
                let tid = state.next_trial_id;
                match line_u32_field(line, "study_id") {
                    Some(sid) if sid == target => {
                        state.next_trial_id += 1;
                        target_ids.insert(tid);
                        deferred.push((line, op, Some(tid)));
                    }
                    Some(sid) => {
                        state.next_trial_id += 1;
                        if let Some(s) = state.studies.get_mut(sid as usize) {
                            s.total_trials += 1;
                        }
                    }
                    None => {
                        if let Ok(json) = parse_json(line) {
                            state.process_op(op, &json);
                            if state.trial_builders.contains_key(&tid) {
                                target_ids.insert(tid);
                            }
                        }
                    }
                }
            }
            5..=9 => {
                valid = true;
                if line_u32_field(line, "trial_id").is_some_and(|id| target_ids.contains(&id)) {
                    deferred.push((line, op, None));
                }
            }
            _ => (),
        }
    }
    if !valid {
        return Err("No valid JSON lines found in journal".into());
    }
    let parsed: Vec<_> = deferred
        .par_iter()
        .filter_map(|(line, op, tid)| {
            crate::optuna_json::record(line)
                .ok()
                .map(|v| (*op, v, *tid))
        })
        .collect();
    for (op, json, tid) in parsed {
        if let Some(tid) = tid {
            state.next_trial_id = tid;
        }
        state.process_record(op, &json);
    }
    let s = finalize_state(state)
        .into_iter()
        .find(|s| s.meta.study_id == target)
        .ok_or_else(|| format!("study_id {target} not found in journal"))?;
    Ok((s.meta, s.dataframe, s.extras))
}
