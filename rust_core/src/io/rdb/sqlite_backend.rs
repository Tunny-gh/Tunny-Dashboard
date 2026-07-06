//! SQLite (`rusqlite`) 実装。
//!
//! `OptunaBackend` trait を rusqlite の `Connection` 上に実装する。クエリ組み立て
//! ロジックは `generic.rs` に集約済みのため、ここでは canonical `?` プレースホルダの
//! 実行と `rusqlite::types::ValueRef` → `SqlValue` の変換のみを担う。

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use super::backend::{OptunaBackend, SqlParam, SqlValue};

/// SQLite ファイルを開いた `OptunaBackend` 実装。
pub struct SqliteBackend {
    conn: Connection,
}

impl SqliteBackend {
    /// 読み取り専用でファイルを開く（旧 `open_readonly` と同一フラグ）。
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
        // Optuna スキーマに BLOB 列は無いため実際には来ない想定。安全側で NULL 扱いにする。
        rusqlite::types::ValueRef::Blob(_) => SqlValue::Null,
    }
}

impl OptunaBackend for SqliteBackend {
    fn query(&mut self, sql: &str, params: &[SqlParam]) -> Result<Vec<Vec<SqlValue>>, String> {
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| format!("Failed to prepare query: {e}"))?;
        let column_count = stmt.column_count();
        let bound_params: Vec<rusqlite::types::Value> =
            params.iter().map(to_rusqlite_value).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bound_params), |row| {
                let mut values = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    values.push(value_ref_to_sql_value(row.get_ref(i)?));
                }
                Ok(values)
            })
            .map_err(|e| format!("Failed to execute query: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read query results: {e}"))?;
        Ok(rows)
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
