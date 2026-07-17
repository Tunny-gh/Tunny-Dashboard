use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;

use tunny_core::io::journal::parser::{self, OptimizationDirection};

/// Converts a rust_core StudyMeta into an egui-app StudyMeta.
pub fn convert_study_meta(meta: parser::StudyMeta) -> StudyMeta {
    StudyMeta {
        study_id: meta.study_id,
        name: meta.name,
        directions: meta
            .directions
            .into_iter()
            .map(|d| match d {
                OptimizationDirection::Minimize => Direction::Minimize,
                OptimizationDirection::Maximize => Direction::Maximize,
            })
            .collect(),
        completed_trials: meta.completed_trials as usize,
        param_names: meta.param_names,
        objective_names: meta.objective_names,
        param_bounds: meta.param_bounds,
    }
}

/// Phase 1: reads the file, scans only op_code=0/3, and returns the Study list.
/// Also returns the raw byte buffer so Phase 2 doesn't need to re-read the file.
pub fn scan_journal_task(path: PathBuf) -> (Vec<u8>, AppMessage) {
    match crate::io::file::read_journal_file(&path) {
        Ok(data) => match parser::scan_study_list(&data) {
            Ok(studies) => {
                let app_studies: Vec<StudyMeta> =
                    studies.into_iter().map(convert_study_meta).collect();
                let msg = AppMessage::JournalParsed {
                    studies: app_studies,
                    path,
                };
                (data, msg)
            }
            Err(e) => (vec![], AppMessage::Error(e)),
        },
        Err(e) => (vec![], AppMessage::Error(e)),
    }
}

/// Phase 2 (streaming): parses the target study from the cached byte buffer in a single
/// forward pass, sending completed trials incrementally as `StudyChunkLoaded` every
/// `BATCH_SIZE` entries.
///
/// Does not re-read the file. Since `tx` is a bounded channel, natural backpressure is
/// applied until the UI catches up with rendering. Returns `true` on success (the caller
/// uses this to register it as loaded).
pub fn stream_single_study_task(data: &[u8], meta: StudyMeta, tx: &SyncSender<AppMessage>) -> bool {
    /// Number of completed trials per batch reflected in the graph.
    const BATCH_SIZE: usize = 1000;

    let study_id = meta.study_id;
    let result = parser::parse_single_study_streaming(data, study_id, BATCH_SIZE, |batch| {
        let _ = tx.send(AppMessage::StudyChunkLoaded {
            study_id,
            meta: convert_study_meta(batch.meta),
            new_rows: batch.new_rows,
            param_names: batch.param_names,
            objective_names: batch.objective_names,
            user_attr_numeric_names: batch.user_attr_numeric_names,
            user_attr_string_names: batch.user_attr_string_names,
            max_constraints: batch.max_constraints,
            is_first: batch.is_first,
            is_final: batch.is_final,
        });
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            let _ = tx.send(AppMessage::Error(e));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_journal_nonexistent_path_returns_error() {
        let path = PathBuf::from("/nonexistent/file.log");
        let (_data, msg) = scan_journal_task(path);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn convert_minimize_direction() {
        let core_meta = parser::StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize,
            ],
            completed_trials: 5,
            total_trials: 10,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: Default::default(),
        };
        let app_meta = convert_study_meta(core_meta);
        assert_eq!(app_meta.directions[0], Direction::Minimize);
        assert_eq!(app_meta.directions[1], Direction::Maximize);
        assert_eq!(app_meta.completed_trials, 5usize);
    }
}
