//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-101/journal-parser-requirements.md

mod builders;
mod distribution;
mod finalize;
mod state;
mod types;

use serde_json::Value;

use finalize::finalize_state;
use state::{get_u64, ParserState};

pub use types::{JournalParser, OptimizationDirection, ParseResult, StudyMeta};

#[cfg(test)]
use builders::TrialBuilder;
#[cfg(test)]
use distribution::Distribution;

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;

    let (studies, dataframes) = finalize_state(state);
    crate::dataframe::store_dataframes(dataframes);

    Ok(ParseResult {
        studies,
        duration_ms,
    })
}

#[cfg(test)]
mod tests;
