//! Optuna RDBStorage (SQLite) reader.
//!
//! Study acquisition uses one deferred read transaction and returns owned
//! decoded contents. The caller supplies a source key and applies the domain batch.

use std::path::Path;

use crate::io::journal::parser::StudyMeta;
use crate::io::rdb::{self, RdbStudyRows, SqliteBackend, StudyFingerprint};

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
pub fn parse_single_study_rows(path: &Path, study_id: u32) -> Result<RdbStudyRows, String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    backend.with_study_read(|session| rdb::parse_single_study_rows(session, study_id))
}
