//! Optuna RDBStorage (SQLite) reader.
//!
//! Reads an Optuna `sqlite:///xxx.db` storage file and exposes the same
//! output contract (`StudyMeta` / `DataFrame`) as the journal parser, so
//! downstream code (UI, export, ...) does not need to distinguish the
//! storage backend.
//!
//! クエリ組み立てロジック本体は `crate::io::rdb::generic` へ移動した
//! （PostgreSQL / MySQL とロジックを共有するため）。このモジュールは
//! `SqliteBackend` を開いて `rdb` 層へ委譲する薄い互換レイヤーであり、
//! 公開関数のシグネチャ・戻り値型・エラー文言は移行前と同一に保つ。

use std::path::Path;

// `rusqlite::Connection` / `OptimizationDirection` / `TrialState` はこのモジュール自体は
// 使わないが、`tests.rs` が `use super::*;` でフィクスチャ構築に使うため、テストビルドに
// 限定してスコープへ引き込む（tests.rs は無変更が受入条件のため）。
#[cfg(test)]
use rusqlite::Connection;

use crate::data::dataframe::DataFrame;
use crate::data::extras::StudyExtras;
#[cfg(test)]
use crate::data::extras::TrialState;
#[cfg(test)]
use crate::io::journal::parser::OptimizationDirection;
use crate::io::journal::parser::StudyMeta;
use crate::io::rdb::{self, RdbStudyRows, SqliteBackend};

#[cfg(test)]
mod tests;

/// `RdbStudyRows` の互換エイリアス（呼び出し側の参照名を変えないため）。
pub type SqliteStudyRows = RdbStudyRows;

/// `StudyFingerprint` の互換 re-export。
pub use crate::io::rdb::StudyFingerprint;

/// ライブ更新のポーリングで呼ぶ軽量フィンガープリント取得。
/// 詳細は `rdb::generic::study_fingerprint` を参照。
pub fn study_fingerprint(path: &Path, study_id: u32) -> Result<StudyFingerprint, String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    rdb::study_fingerprint(&mut backend, study_id)
}

/// Phase 1: DB を開いて Study 一覧を返す。詳細は `rdb::generic::scan_study_list` を参照。
pub fn scan_study_list(path: &Path) -> Result<Vec<StudyMeta>, String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    rdb::scan_study_list(&mut backend)
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、確定メタと行データを返す。
/// 詳細は `rdb::generic::parse_single_study_rows` を参照。
pub fn parse_single_study_rows(path: &Path, study_id: u32) -> Result<SqliteStudyRows, String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    rdb::parse_single_study_rows(&mut backend, study_id)
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、(確定メタ, `DataFrame`, `StudyExtras`) を返す
/// （journal の `parse_single_study` と同じ出力契約）。
pub fn parse_single_study(
    path: &Path,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let mut backend = SqliteBackend::open_readonly(path)?;
    rdb::parse_single_study(&mut backend, study_id)
}
