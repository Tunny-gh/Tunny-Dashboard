//! Journal scanning and owned all-state bulk/incremental replay.
mod builders;
pub mod distribution;
mod state;
pub mod study;
mod types;

use super::line_u32_field;
use crate::io::optuna_json::parse as parse_json;
use state::{get_u64, ParserState};
pub use types::{OptimizationDirection, StudyMeta};

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
