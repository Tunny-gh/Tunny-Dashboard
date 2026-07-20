//! Common query-building logic for Optuna's RDBStorage.
//!
//! Consolidates the query strings and row-assembly logic common to
//! SQLite / PostgreSQL / MySQL here. Dialect differences in connection
//! handling and value representation are only queried against the backend via
//! the `OptunaBackend` trait, so the core logic is backend-agnostic.
//!
//! Split by responsibility into submodules:
//! - `study_list`: study-list scanning (Phase 1).
//! - `fingerprint`: study fingerprinting for live-update polling.
//! - `study_rows`: single-study row parsing (Phase 2).
//!
//! Low-level helpers shared by more than one submodule stay here in the
//! module root.

mod fingerprint;
mod study_list;
mod study_rows;

pub use fingerprint::{study_fingerprint, StudyFingerprint};
pub use study_list::scan_study_list;
pub use study_rows::{parse_single_study, parse_single_study_rows, RdbStudyRows};

use super::backend::{OptunaBackend, SqlParam};

/// Determines whether this is an Optuna schema by checking for the `studies` table.
pub(super) fn ensure_optuna_schema(backend: &mut dyn OptunaBackend) -> Result<(), String> {
    let exists = backend
        .table_exists("studies")
        .map_err(|e| format!("Failed to inspect database schema: {e}"))?;
    if !exists {
        return Err("Not an Optuna storage: 'studies' table not found".to_string());
    }
    Ok(())
}

/// Executes a single-row, single-column aggregate query (`COUNT`/`MAX`, etc.) and returns it as `i64`.
pub(super) fn query_scalar_i64(
    backend: &mut dyn OptunaBackend,
    sql: &str,
    params: &[SqlParam],
    context: &str,
) -> Result<i64, String> {
    let rows = backend
        .query(sql, params)
        .map_err(|e| format!("{context}: {e}"))?;
    rows.into_iter()
        .next()
        .and_then(|row| row.into_iter().next())
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("{context}: expected a single integer row"))
}
