//! Optuna RDBStorage (SQLite) reader.
//!
//! Reads an Optuna `sqlite:///xxx.db` storage file and exposes the same
//! output contract (`StudyMeta` / `DataFrame`) as the journal parser, so
//! downstream code (UI, export, ...) does not need to distinguish the
//! storage backend.
//!
//! The core query-building logic has been moved to `crate::io::rdb::generic`
//! (to share logic with PostgreSQL / MySQL). This module is now a thin
//! compatibility layer that opens a `SqliteBackend` and delegates to the
//! `rdb` layer. Study acquisition uses one deferred read transaction; DataFrame
//! projection happens after that session and its connection have ended.

use std::path::Path;

// `rusqlite::Connection` / `OptimizationDirection` / `TrialState` are not used by
// this module itself, but `tests.rs` uses them via `use super::*;` to build
// fixtures, so they are pulled into scope only for test builds (leaving
// `tests.rs` unchanged is an acceptance condition).

use crate::decoded::DecodedTable as DataFrame;
use crate::extras::StudyExtras;
use crate::io::journal::parser::StudyMeta;
use crate::io::rdb::{self, RdbStudyRows, SqliteBackend};

/// Compatibility alias for `RdbStudyRows` (kept so callers' reference names don't change).
pub type SqliteStudyRows = RdbStudyRows;

/// Compatibility re-export of `StudyFingerprint`.
pub use crate::io::rdb::StudyFingerprint;

/// Public change-detection query, including COMPLETE user attributes and all-state
/// constraints. See [`rdb::study_fingerprint`] for scan cost and limitations.
pub fn study_fingerprint(path: &Path, study_id: u32) -> Result<StudyFingerprint, String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    rdb::study_fingerprint(&mut backend, study_id)
}

/// Phase 1: Opens the DB and returns the list of studies. See `rdb::generic::scan_study_list` for details.
pub fn scan_study_list(path: &Path) -> Result<Vec<StudyMeta>, String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    rdb::scan_study_list(&mut backend)
}

/// Phase 2: Reads all COMPLETE trials for the given study and returns the finalized
/// metadata and row data. See `rdb::generic::parse_single_study_rows` for details.
/// All related queries share one read snapshot, established by the first read.
/// The session ends before the owned result is returned.
pub fn parse_single_study_rows(path: &Path, study_id: u32) -> Result<SqliteStudyRows, String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    backend.with_study_read(|session| rdb::parse_single_study_rows(session, study_id))
}

/// Phase 2: Reads all COMPLETE trials for the given study and returns
/// (finalized metadata, `DataFrame`, `StudyExtras`) (same output contract as
/// the journal's `parse_single_study`).
pub fn parse_single_study(
    path: &Path,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    Ok(rdb::study_rows_to_dataframe(parse_single_study_rows(
        path, study_id,
    )?))
}
