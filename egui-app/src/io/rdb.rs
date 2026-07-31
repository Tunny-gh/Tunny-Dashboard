//! Bridge for RDB (PostgreSQL/MySQL) connection URLs. Mirrors the shape of
//! `io/sqlite.rs`, only swapping the callees for the `tunny_core::rdb::*_url` family
//! (the versions that take an `RdbUrl`).
//!
//! Difference from SQLite: SQLite can simply re-query the local file path as-is each
//! time, but RDB stores the connection URL (including the password) as a string in
//! `journal_path: Option<PathBuf>` (to minimize the plumbing diff; see the Phase C
//! design document for details). `path_as_rdb_url` runs that string through
//! `RdbUrl::parse` each time to detect and reconstruct it.

use std::path::Path;
use std::sync::mpsc::SyncSender;

use tunny_core::rdb::RdbUrl;

use crate::state::messages::AppMessage;

/// Determines whether the path stored in `journal_path` can be interpreted as an RDB
/// connection URL, and reconstructs an `RdbUrl` if possible. This is the counterpart
/// to SQLite's `is_sqlite_path`, but instead of checking the extension, it runs the
/// entire string through `RdbUrl::parse`.
pub fn path_as_rdb_url(path: &Path) -> Option<RdbUrl> {
    path.to_str().and_then(RdbUrl::parse)
}

/// Phase 1: opens the RDB storage and returns the Study list (same role as
/// `io::sqlite::scan_sqlite_task`).
pub fn scan_rdb_task(url: RdbUrl) -> AppMessage {
    match tunny_core::rdb::scan_study_list_url(&url) {
        Ok(studies) => {
            let app_studies: Vec<crate::state::app_state::StudyMeta> = studies
                .into_iter()
                .map(crate::io::journal::convert_study_meta)
                .collect();
            AppMessage::JournalParsed {
                studies: app_studies,
                path: std::path::PathBuf::from(url.url),
            }
        }
        Err(e) => AppMessage::Error(e),
    }
}

/// Phase 2: reads all COMPLETE trials for the specified study and sends them as a
/// single chunk via `StudyChunkLoaded` (same role as `io::sqlite::load_single_study_task`).
pub fn load_single_study_task(url: &RdbUrl, study_id: u32, tx: &SyncSender<AppMessage>) -> bool {
    match tunny_core::rdb::parse_single_study_rows_url(url, study_id) {
        Ok(rows) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn path_as_rdb_url_accepts_postgresql_and_mysql() {
        assert!(
            path_as_rdb_url(Path::new("postgresql://u:p@localhost:5432/db")).is_some(),
            "postgresql:// should be recognized as RDB URL"
        );
        assert!(
            path_as_rdb_url(Path::new("mysql://u:p@localhost:3306/db")).is_some(),
            "mysql:// should be recognized as RDB URL"
        );
        assert!(path_as_rdb_url(Path::new("postgresql+psycopg2://u:p@localhost/db")).is_some());
    }

    #[test]
    fn path_as_rdb_url_rejects_local_paths() {
        assert!(path_as_rdb_url(Path::new("study.log")).is_none());
        assert!(path_as_rdb_url(Path::new("study.db")).is_none());
        assert!(path_as_rdb_url(Path::new("study.csv")).is_none());
        assert!(path_as_rdb_url(Path::new("/abs/path/study.db")).is_none());
        assert!(path_as_rdb_url(Path::new("sqlite:///a.db")).is_none());
    }

    #[test]
    fn scan_rdb_task_unreachable_host_returns_error() {
        // A URL that can't connect to a real DB (unused port) returns an error message.
        let url = RdbUrl::parse("postgresql://u:p@127.0.0.1:1/nope").unwrap();
        let msg = scan_rdb_task(url);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn load_single_study_task_unreachable_host_sends_error() {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let url = RdbUrl::parse("postgresql://u:p@127.0.0.1:1/nope").unwrap();
        let ok = load_single_study_task(&url, 1, &tx);
        assert!(!ok);
        match rx.try_recv() {
            Ok(AppMessage::Error(_)) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn scan_rdb_task_uses_normalized_url_as_path() {
        // postgres:// (the shorthand form) is normalized to postgresql://.
        // The connection itself will fail, but the normalization logic runs
        // regardless of the error path, so here we directly verify the
        // normalization result of RdbUrl::parse.
        let url = RdbUrl::parse("postgres://u:p@127.0.0.1:1/nope").unwrap();
        assert_eq!(url.url, "postgresql://u:p@127.0.0.1:1/nope");
        let _ = PathBuf::from(url.url.clone());
    }
}
