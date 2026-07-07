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
        // rusqlite の `Rows` カーソルで 1 行ずつ取り出し、行バッファを使い回して
        // 全行を同時にメモリへ載せない（大規模 DB での OOM 回避）。
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
