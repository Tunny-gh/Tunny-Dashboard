use std::ops::Range;

use serde::Deserialize;
use tdashboard_study::{StudyBatch, StudyKey};

use super::{get_u64, study, study_list, ParserState, StudyMeta};
use crate::io::optuna_json::{parse, record};

/// An immutable Journal and the record locations for each live Study.
pub struct JournalSnapshot {
    text: String,
    studies: Vec<StudyMeta>,
    records: Vec<Vec<RecordLocation>>,
}

struct RecordLocation {
    range: Range<usize>,
    created_trial_id: Option<u32>,
}

#[derive(Deserialize)]
struct Header {
    op_code: u8,
    trial_id: Option<u64>,
}

impl std::fmt::Debug for JournalSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JournalSnapshot")
            .field("bytes", &self.text.len())
            .field("studies", &self.studies.len())
            .finish_non_exhaustive()
    }
}

impl JournalSnapshot {
    pub fn new(bytes: Vec<u8>) -> Result<Self, String> {
        let text = String::from_utf8(bytes)
            .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned());
        let mut state = ParserState::new();
        let mut records: Vec<Vec<RecordLocation>> = Vec::new();
        let mut trial_studies = Vec::new();
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let range = offset..offset + line.len();
            offset = range.end;
            let line = line.trim();
            let Some(header) = read_header(line) else {
                continue;
            };
            let mut created_trial_id = None;
            let study_id = match header.op_code {
                0 => {
                    let Ok(json) = parse(line) else { continue };
                    state.process_op(0, &json);
                    if records.len() == state.studies.len() {
                        continue;
                    }
                    records.push(Vec::new());
                    records.len() - 1
                }
                1 | 3 => {
                    let Ok(json) = parse(line) else { continue };
                    let id = get_u64(&json, "study_id");
                    state.process_op(header.op_code, &json);
                    if header.op_code == 1 {
                        if let Some(entries) = id
                            .and_then(|id| usize::try_from(id).ok())
                            .and_then(|id| records.get_mut(id))
                        {
                            *entries = Vec::new();
                        }
                        continue;
                    }
                    id.unwrap_or(0) as u32 as usize
                }
                4 => {
                    // A truncated or malformed CREATE_TRIAL must not consume a global ID.
                    let Ok(entry) = record(line) else { continue };
                    let id = get_u64(&entry.json, "study_id").unwrap_or(0) as u32 as usize;
                    if !state.studies.get(id).is_some_and(Option::is_some) {
                        continue;
                    }
                    let trial = u32::try_from(trial_studies.len())
                        .ok()
                        .filter(|id| *id < u32::MAX)
                        .ok_or("Journal trial identity exhausted")?;
                    trial_studies.push(id);
                    created_trial_id = Some(trial);
                    id
                }
                5..=9 => {
                    let Some(&id) = header
                        .trial_id
                        .and_then(|id| u32::try_from(id).ok())
                        .and_then(|id| trial_studies.get(id as usize))
                    else {
                        continue;
                    };
                    id
                }
                _ => continue,
            };
            if state.studies.get(study_id).is_some_and(Option::is_some) {
                records[study_id].push(RecordLocation {
                    range,
                    created_trial_id,
                });
            }
        }
        let studies = study_list(state, text.is_empty())?;
        Ok(Self {
            text,
            studies,
            records,
        })
    }

    pub fn studies(&self) -> &[StudyMeta] {
        &self.studies
    }

    pub(crate) fn read(
        &self,
        key: StudyKey,
        streaming: bool,
        mut sink: impl FnMut(StudyBatch) -> Result<(), String>,
    ) -> Result<(), String> {
        let id = key.stored_id as usize;
        let locations = self
            .records
            .get(id)
            .filter(|records| !records.is_empty())
            .ok_or_else(|| format!("study_id {} not found", key.stored_id))?;
        let records = locations
            .iter()
            .map(|entry| (&self.text[entry.range.clone()], entry.created_trial_id));
        let mut state = ParserState::new_with_target(key.stored_id);
        // The first entry creates this Study. Earlier slots preserve its original ID.
        state.studies.resize_with(id, || None);
        if streaming {
            study::replay(records, state, key, 1000, |batch, _| sink(batch))
        } else {
            sink(study::replay_full(records, state, key)?)
        }
    }
}

fn read_header(line: &str) -> Option<Header> {
    if !line.starts_with('{') {
        return None;
    }
    serde_json::from_str(line).ok().or_else(|| {
        // Optuna permits bare NaN and Infinity in payloads.
        let json = parse(line).ok()?;
        Some(Header {
            op_code: u8::try_from(get_u64(&json, "op_code")?).ok()?,
            trial_id: get_u64(&json, "trial_id"),
        })
    })
}
