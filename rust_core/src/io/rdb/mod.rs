//! Optuna RDBStorage 読み取りの共通基盤。
//!
//! バックエンド（SQLite/PostgreSQL/MySQL）ごとの接続・型変換の差分を
//! `OptunaBackend` trait（`backend.rs`）に隔離し、クエリ組み立てロジック本体は
//! `generic.rs` へ 1 本化する。Phase A では SQLite 実装（`sqlite_backend.rs`）のみを
//! 提供し、`crate::io::sqlite` はここへ委譲する薄い互換層になる。

mod backend;
mod generic;
mod mysql_backend;
mod postgres_backend;
mod sqlite_backend;
mod url;

use crate::data::dataframe::DataFrame;
use crate::data::extras::StudyExtras;
use crate::io::journal::parser::StudyMeta;

pub use backend::{OptunaBackend, SqlParam, SqlValue};
pub use generic::{
    parse_single_study, parse_single_study_rows, scan_study_list, study_fingerprint, RdbStudyRows,
    StudyFingerprint,
};
pub use mysql_backend::MysqlBackend;
pub use postgres_backend::PostgresBackend;
pub use sqlite_backend::SqliteBackend;
pub use url::{is_rdb_url, RdbKind, RdbUrl};

/// `RdbUrl` の種別に応じて対応するバックエンドへ接続する。
fn connect(url: &RdbUrl) -> Result<Box<dyn OptunaBackend>, String> {
    match url.kind {
        RdbKind::Postgres => Ok(Box::new(PostgresBackend::connect(&url.url)?)),
        RdbKind::Mysql => Ok(Box::new(MysqlBackend::connect(&url.url)?)),
    }
}

/// URL 版: DB を開いて Study 一覧を返す。詳細は `scan_study_list` を参照。
pub fn scan_study_list_url(url: &RdbUrl) -> Result<Vec<StudyMeta>, String> {
    let mut backend = connect(url)?;
    scan_study_list(backend.as_mut())
}

/// URL 版: 指定 study の COMPLETE trial を全件読み、確定メタと行データを返す。
/// 詳細は `parse_single_study_rows` を参照。
pub fn parse_single_study_rows_url(url: &RdbUrl, study_id: u32) -> Result<RdbStudyRows, String> {
    let mut backend = connect(url)?;
    parse_single_study_rows(backend.as_mut(), study_id)
}

/// URL 版: 指定 study の COMPLETE trial を全件読み、(確定メタ, `DataFrame`, `StudyExtras`) を返す。
/// 詳細は `parse_single_study` を参照。
pub fn parse_single_study_url(
    url: &RdbUrl,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let mut backend = connect(url)?;
    parse_single_study(backend.as_mut(), study_id)
}

/// URL 版: ライブ更新のポーリングで呼ぶ軽量フィンガープリント取得。
/// 詳細は `study_fingerprint` を参照。
pub fn study_fingerprint_url(url: &RdbUrl, study_id: u32) -> Result<StudyFingerprint, String> {
    let mut backend = connect(url)?;
    study_fingerprint(backend.as_mut(), study_id)
}
