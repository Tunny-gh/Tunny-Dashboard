//! Journal scanning and owned all-state bulk/incremental replay.
mod builders;
pub mod distribution;
mod snapshot;
mod state;
pub mod study;
mod types;

use super::line_u32_field;
use crate::io::optuna_json::parse as parse_json;
pub use snapshot::JournalSnapshot;
use state::{get_u64, ParserState};
pub use types::{OptimizationDirection, StudyMeta};

/// Scan Study creation, deletion, and metric names without decoding trial payloads.
pub fn scan_study_list(data: &[u8]) -> Result<Vec<StudyMeta>, String> {
    if data.is_empty() {
        return Ok(vec![]);
    }
    let mut state = ParserState::new();
    for line in String::from_utf8_lossy(data).lines() {
        let line = line.trim();
        let op = match line_u32_field(line, "op_code") {
            Some(op @ (0 | 1)) => op as u8,
            Some(3) if line.contains("study:metric_names") => 3,
            _ => continue,
        };
        if let Ok(json) = parse_json(line) {
            state.process_op(op, &json);
        }
    }
    study_list(state, data.is_empty())
}

fn study_list(state: ParserState, empty: bool) -> Result<Vec<StudyMeta>, String> {
    if state.studies.is_empty() && !empty {
        return Err("No studies found in journal".into());
    }
    Ok(state
        .studies
        .into_iter()
        .flatten()
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

#[cfg(test)]
mod tests {
    use super::{scan_study_list, study, JournalSnapshot};
    use tdashboard_study::{
        IdentityAllocator, Number, ObjectiveIndex, StudyBatch, StudyKey, StudyTrialNumber,
        TrialState,
    };

    #[test]
    fn recreating_a_deleted_study_preserves_study_and_trial_ids() {
        let journal = [
            r#"{"op_code":0,"study_name":"recreated","directions":[1]}"#,
            r#"{"op_code":0,"study_name":"survivor","directions":[1]}"#,
            r#"{"op_code":4,"study_id":0}"#,
            r#"{"op_code":6,"trial_id":0,"state":1,"values":[10]}"#,
            r#"{"op_code":1,"study_id":0}"#,
            r#"{"op_code":4,"study_id":0}"#,
            r#"{"op_code":0,"study_name":"recreated","directions":[2]}"#,
            r#"{"op_code":0,"study_name":"recreated","directions":[1]}"#,
            r#"{"op_code":0,"study_name":"last","directions":[1]}"#,
            r#"{"op_code":3,"study_id":2,"system_attr":{"study:metric_names":["New metric"]}}"#,
            r#"{"op_code":4,"study_id":1}"#,
            r#"{"op_code":4,"study_id":2,"params":{"truncated":"#,
            r#"{"op_code":4,"study_id":2}"#,
            r#"{"op_code":4,"study_id":3}"#,
            r#"{"op_code":6,"trial_id":1,"state":1,"values":[20]}"#,
            r#"{"op_code":6,"trial_id":2,"state":1,"values":[30]}"#,
            r#"{"op_code":6,"trial_id":3,"state":1,"values":[40]}"#,
        ]
        .join("\n");
        let bytes = journal.as_bytes();
        let studies = scan_study_list(bytes).unwrap();
        let snapshot = JournalSnapshot::new(bytes.to_vec()).unwrap();
        let names_and_ids: Vec<_> = studies
            .iter()
            .map(|meta| (meta.name.as_str(), meta.study_id))
            .collect();
        assert_eq!(
            names_and_ids,
            [("survivor", 1), ("recreated", 2), ("last", 3)]
        );
        assert_eq!(studies[1].objective_names, ["New metric"]);
        assert_eq!(
            snapshot
                .studies()
                .iter()
                .map(|meta| (meta.name.as_str(), meta.study_id))
                .collect::<Vec<_>>(),
            names_and_ids
        );

        let source = IdentityAllocator::new().unwrap().source().unwrap();
        let deleted = StudyKey {
            source,
            stored_id: 0,
        };
        assert!(snapshot.read(deleted, false, |_| Ok(())).is_err());
        assert!(study::read_full(
            bytes,
            StudyKey {
                source,
                stored_id: 0
            }
        )
        .is_err());
        for (meta, expected_value) in studies.iter().zip([20.0, 30.0, 40.0]) {
            let key = StudyKey {
                source,
                stored_id: meta.study_id,
            };
            let StudyBatch::Replace { trials, metadata } = study::read_full(bytes, key).unwrap()
            else {
                panic!("Expected a full Study batch");
            };
            assert_eq!(metadata.name, meta.name);
            assert_eq!(trials.len(), 1);
            let trial = &trials[0];
            assert_eq!(trial.key.stored_id, meta.study_id);
            assert_eq!(trial.number, StudyTrialNumber(0));
            assert_eq!(trial.state, TrialState::Complete);
            assert_eq!(trial.objectives[&ObjectiveIndex(0)], Number(expected_value));

            snapshot
                .read(key, false, |batch| {
                    let StudyBatch::Replace {
                        trials: indexed,
                        metadata: indexed_meta,
                    } = batch
                    else {
                        panic!("Expected a full Study batch");
                    };
                    assert_eq!(indexed, trials);
                    assert_eq!(indexed_meta, metadata);
                    Ok(())
                })
                .unwrap();

            let mut streamed = Vec::new();
            study::read_batches(bytes, key, 1, |batch| {
                match batch {
                    StudyBatch::Replace { trials, .. } => streamed.extend(trials),
                    StudyBatch::Delta { upserts, .. } => streamed.extend(upserts),
                }
                Ok(())
            })
            .unwrap();
            assert_eq!(streamed.as_slice(), trials.as_slice());
        }
    }
}
