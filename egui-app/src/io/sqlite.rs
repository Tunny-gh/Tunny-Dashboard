use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;

use crate::state::messages::AppMessage;

/// Determines whether a path is an Optuna SQLite storage (extension db/sqlite/sqlite3,
/// case-insensitive).
pub fn is_sqlite_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        e.eq_ignore_ascii_case("db")
            || e.eq_ignore_ascii_case("sqlite")
            || e.eq_ignore_ascii_case("sqlite3")
    })
}

/// Phase 1: opens the SQLite storage and returns the Study list (same role as
/// journal's `scan_journal_task`).
///
/// Unlike journal, no raw byte buffer cache is needed (Phase 2 re-queries directly
/// from `path`), so the return value is just `AppMessage`.
pub fn scan_sqlite_task(path: PathBuf) -> AppMessage {
    match tunny_core::sqlite::scan_study_list(&path) {
        Ok(studies) => {
            let app_studies: Vec<crate::state::app_state::StudyMeta> = studies
                .into_iter()
                .map(crate::io::journal::convert_study_meta)
                .collect();
            AppMessage::JournalParsed {
                studies: app_studies,
                path,
            }
        }
        Err(e) => AppMessage::Error(e),
    }
}

/// Phase 2: reads all COMPLETE trials for the specified study and sends them as a
/// single chunk via `StudyChunkLoaded` (same role as journal's
/// `stream_single_study_task`).
///
/// Since SQLite can fetch all rows for the target study at once via a row-oriented
/// query, batch-split streaming like journal's is unnecessary. By packing all rows
/// into one message with `is_first = is_final = true`, `MessageHandler::handle_study_chunk`
/// is shared with journal, making UI state such as the param filter sliders and
/// Pareto rank computation exactly the same as the journal path.
pub fn load_single_study_task(path: &Path, study_id: u32, tx: &SyncSender<AppMessage>) -> bool {
    match tunny_core::sqlite::parse_single_study_rows(path, study_id) {
        Ok(rows) => {
            // Store the extra info for all trials (all states) into the shared store,
            // keyed by the actual study_id.
            tunny_core::dataframe::store_extras_for(study_id, rows.extras);
            let _ = tx.send(AppMessage::StudyChunkLoaded {
                study_id,
                meta: crate::io::journal::convert_study_meta(rows.meta),
                new_rows: rows.rows,
                param_names: rows.param_names,
                objective_names: rows.objective_names,
                user_attr_numeric_names: rows.user_attr_numeric_names,
                user_attr_string_names: rows.user_attr_string_names,
                max_constraints: rows.max_constraints,
                is_first: true,
                is_final: true,
            });
            true
        }
        Err(e) => {
            let _ = tx.send(AppMessage::Error(e));
            false
        }
    }
}

/// Live update: fully re-parses the study whose fingerprint change was detected,
/// swaps out the shared store, and sends `AppMessage::SqliteLiveReloadDone`.
///
/// Unlike journal's live update (incremental append), SQLite updates trial state
/// in place, so incremental application isn't possible. This is why the target study
/// is fully re-read every time with `parse_single_study` (the same function as
/// Phase 2). Parsing a single SQLite study is lightweight (on the order of a few ms),
/// so this re-parse itself runs on the worker thread and doesn't block the UI thread.
/// `swap_snapshot` / `store_extras_for` are also performed within this function (on
/// the worker thread), following the same pattern as `LoadComparisonStudy`, so the
/// `MessageHandler` side only needs to re-fetch `snapshot(study_id)` after receiving it.
pub fn reload_single_study_task(path: &Path, study_id: u32, tx: &SyncSender<AppMessage>) -> bool {
    match tunny_core::sqlite::parse_single_study(path, study_id) {
        Ok((meta, df, extras)) => {
            tunny_core::dataframe::swap_snapshot(study_id, std::sync::Arc::new(df));
            tunny_core::dataframe::store_extras_for(study_id, extras);
            let _ = tx.send(AppMessage::SqliteLiveReloadDone {
                study_id,
                meta: crate::io::journal::convert_study_meta(meta),
            });
            true
        }
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
    fn is_sqlite_path_detects_extensions() {
        assert!(is_sqlite_path(Path::new("study.db")));
        assert!(is_sqlite_path(Path::new("STUDY.DB")));
        assert!(is_sqlite_path(Path::new("study.sqlite")));
        assert!(is_sqlite_path(Path::new("study.sqlite3")));
        assert!(!is_sqlite_path(Path::new("study.log")));
        assert!(!is_sqlite_path(Path::new("study.csv")));
        assert!(!is_sqlite_path(Path::new("noext")));
    }

    #[test]
    fn scan_sqlite_task_nonexistent_path_returns_error() {
        let path = PathBuf::from("/nonexistent/study.db");
        let msg = scan_sqlite_task(path);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn load_single_study_task_nonexistent_path_sends_error() {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let ok = load_single_study_task(Path::new("/nonexistent/study.db"), 1, &tx);
        assert!(!ok);
        match rx.try_recv() {
            Ok(AppMessage::Error(_)) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn reload_single_study_task_nonexistent_path_sends_error() {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let ok = reload_single_study_task(Path::new("/nonexistent/study.db"), 1, &tx);
        assert!(!ok);
        match rx.try_recv() {
            Ok(AppMessage::Error(_)) => {}
            _ => panic!("Expected Error message"),
        }
    }
}
