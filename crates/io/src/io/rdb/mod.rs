//! Common foundation for reading Optuna's RDBStorage.
//!
//! Isolates the per-backend (SQLite/PostgreSQL/MySQL) differences in connection
//! handling and type conversion into the `OptunaBackend` trait (`backend.rs`),
//! consolidating the query-building logic itself into `generic.rs`. Phase A
//! provides only the SQLite implementation (`sqlite_backend.rs`), with
//! `crate::io::sqlite` acting as a thin compatibility layer that delegates here.

mod backend;
mod generic;
mod mysql_backend;
mod postgres_backend;
mod sqlite_backend;
mod url;

#[cfg(test)]
mod server_snapshot_tests;

pub(crate) use generic::study_rows_to_dataframe;

use crate::decoded::DecodedTable as DataFrame;
use crate::extras::StudyExtras;
use crate::io::journal::parser::StudyMeta;

pub use backend::{OptunaBackend, SqlParam, SqlValue};
pub use generic::{
    parse_single_study, parse_single_study_rows, scan_study_list, study_fingerprint, RdbStudyRows,
    StudyFingerprint,
};
// `RdbFingerprintSession` is defined within this module (mod.rs), so no re-export is needed
// (the `pub struct` can already be referenced directly as `tdashboard_io::rdb::RdbFingerprintSession`).
pub use mysql_backend::MysqlBackend;
pub use postgres_backend::PostgresBackend;
pub use sqlite_backend::SqliteBackend;
pub use url::{check_tls_precondition, has_explicit_plaintext_optin, is_rdb_url, RdbKind, RdbUrl};

/// Connects to the appropriate backend based on the `RdbUrl` kind.
///
/// Neither backend currently supports TLS (both are hard-coded to `NoTls`), so
/// before connecting we check whether a plaintext connection is acceptable via
/// `check_tls_precondition`: it errors (fail-closed) if `sslmode`/`ssl-mode`
/// requests encryption, errors for a plaintext connection to a non-loopback host
/// unless `sslmode=disable` is explicitly opted in, and allows loopback hosts
/// even without any explicit setting (see `url::check_tls_precondition` for details).
fn connect(url: &RdbUrl) -> Result<Box<dyn OptunaBackend>, String> {
    check_tls_precondition(&url.url)?;
    match url.kind {
        RdbKind::Postgres => Ok(Box::new(PostgresBackend::connect(&url.url)?)),
        RdbKind::Mysql => Ok(Box::new(MysqlBackend::connect(&url.url)?)),
    }
}

/// URL variant: opens the DB and returns the list of studies. See `scan_study_list` for details.
pub fn scan_study_list_url(url: &RdbUrl) -> Result<Vec<StudyMeta>, String> {
    let mut backend = connect(url)?;
    scan_study_list(backend.as_mut())
}

/// URL variant: reads all COMPLETE trials of the given study and returns the
/// finalized metadata and row data. See `parse_single_study_rows` for details.
/// Related queries share a read-only REPEATABLE READ transaction. The session
/// and connection end before returning owned rows. MySQL requires ordinary
/// transactional Optuna tables (validated with InnoDB); its first data read
/// establishes the snapshot. No automatic retry or mixed-read fallback is used.
pub fn parse_single_study_rows_url(url: &RdbUrl, study_id: u32) -> Result<RdbStudyRows, String> {
    check_tls_precondition(&url.url)?;
    match url.kind {
        RdbKind::Postgres => PostgresBackend::connect(&url.url)?
            .with_study_read(|session| parse_single_study_rows(session, study_id)),
        RdbKind::Mysql => MysqlBackend::connect(&url.url)?
            .with_study_read(|session| parse_single_study_rows(session, study_id)),
    }
}

/// URL variant: reads all COMPLETE trials of the given study and returns
/// (finalized metadata, `DataFrame`, `StudyExtras`). See `parse_single_study` for details.
/// Reuses the owned rows acquisition; its transaction and connection have ended
/// before DataFrame projection starts.
pub fn parse_single_study_url(
    url: &RdbUrl,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    Ok(study_rows_to_dataframe(parse_single_study_rows_url(
        url, study_id,
    )?))
}

/// A reusable connection session for public fingerprint queries.
///
/// `study_fingerprint_url` connects and disconnects on every call, which incurs
/// the cost of re-establishing a connection for callers that repeatedly query.
/// This session holds a
/// connection (`Box<dyn OptunaBackend>`) and reuses it across `fingerprint`
/// calls. If fetching the fingerprint fails, the connection itself may be
/// broken, so this type does not auto-reconnect internally; instead the caller
/// is expected to drop the session and `connect` again next time.
/// The Dashboard itself uses explicit Reload rather than polling.
pub struct RdbFingerprintSession {
    backend: Box<dyn OptunaBackend>,
}

impl RdbFingerprintSession {
    /// Connects to the `RdbUrl` and creates a reusable session.
    pub fn connect(url: &RdbUrl) -> Result<Self, String> {
        let backend = connect(url)?;
        Ok(Self { backend })
    }

    /// Fetches the fingerprint by reusing the held connection
    /// (does not reconnect; see `study_fingerprint` for details).
    pub fn fingerprint(&mut self, study_id: u32) -> Result<StudyFingerprint, String> {
        study_fingerprint(self.backend.as_mut(), study_id)
    }
}

/// URL variant of the public change-detection query. See `study_fingerprint`
/// for coverage, attribute scan cost and limitations.
///
/// A compatibility API for one-shot use that connects and disconnects on every
/// call. For repeated calls such as polling, use `RdbFingerprintSession`,
/// which reuses the connection.
pub fn study_fingerprint_url(url: &RdbUrl, study_id: u32) -> Result<StudyFingerprint, String> {
    let mut session = RdbFingerprintSession::connect(url)?;
    session.fingerprint(study_id)
}
