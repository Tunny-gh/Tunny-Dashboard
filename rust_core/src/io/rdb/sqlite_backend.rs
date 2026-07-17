//! SQLite (`rusqlite`) implementation.
//!
//! Implements the `OptunaBackend` trait on top of rusqlite's `Connection`.
//! Since the query-building logic is already consolidated in `generic.rs`,
//! this file only handles executing SQL with canonical `?` placeholders and
//! converting `rusqlite::types::ValueRef` to `SqlValue`.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use super::backend::{OptunaBackend, SqlParam, SqlValue};

/// An `OptunaBackend` implementation that has opened a SQLite file.
pub struct SqliteBackend {
    conn: Connection,
}

impl SqliteBackend {
    /// Opens the file read-only (same flags as the old `open_readonly`).
    pub fn open_readonly(path: &Path) -> Result<Self, String> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("Failed to open SQLite database: {e}"))?;
        Ok(Self { conn })
    }
}

fn to_rusqlite_value(param: &SqlParam) -> rusqlite::types::Value {
    match param {
        SqlParam::I64(v) => rusqlite::types::Value::Integer(*v),
        SqlParam::Text(s) => rusqlite::types::Value::Text(s.clone()),
    }
}

fn value_ref_to_sql_value(value_ref: rusqlite::types::ValueRef<'_>) -> SqlValue {
    match value_ref {
        rusqlite::types::ValueRef::Null => SqlValue::Null,
        rusqlite::types::ValueRef::Integer(v) => SqlValue::I64(v),
        rusqlite::types::ValueRef::Real(v) => SqlValue::F64(v),
        rusqlite::types::ValueRef::Text(t) => {
            SqlValue::Text(String::from_utf8_lossy(t).into_owned())
        }
        // The Optuna schema has no BLOB columns, so this is not expected to occur in practice. Treat it as NULL to be safe.
        rusqlite::types::ValueRef::Blob(_) => SqlValue::Null,
    }
}

impl OptunaBackend for SqliteBackend {
    fn query_for_each(
        &mut self,
        sql: &str,
        params: &[SqlParam],
        on_row: &mut dyn FnMut(&[SqlValue]) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| format!("Failed to prepare query: {e}"))?;
        let column_count = stmt.column_count();
        let bound_params: Vec<rusqlite::types::Value> =
            params.iter().map(to_rusqlite_value).collect();
        // Fetch rows one at a time via rusqlite's `Rows` cursor, reusing the row
        // buffer so all rows are never loaded into memory at once (avoids OOM on large DBs).
        let mut rows = stmt
            .query(rusqlite::params_from_iter(bound_params))
            .map_err(|e| format!("Failed to execute query: {e}"))?;
        let mut buf: Vec<SqlValue> = Vec::with_capacity(column_count);
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Failed to read query results: {e}"))?
        {
            buf.clear();
            for i in 0..column_count {
                let value_ref = row
                    .get_ref(i)
                    .map_err(|e| format!("Failed to read column {i}: {e}"))?;
                buf.push(value_ref_to_sql_value(value_ref));
            }
            on_row(&buf)?;
        }
        Ok(())
    }

    fn table_exists(&mut self, table: &str) -> Result<bool, String> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?1 LIMIT 1",
                [table],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|e| format!("Failed to query sqlite_master: {e}"))?
            .is_some();
        Ok(exists)
    }
}
