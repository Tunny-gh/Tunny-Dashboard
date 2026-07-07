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
// `RdbFingerprintSession` は本モジュール（mod.rs）内で定義しているため re-export 不要
// （`pub struct` はそのまま `tunny_core::rdb::RdbFingerprintSession` として参照できる）。
pub use mysql_backend::MysqlBackend;
pub use postgres_backend::PostgresBackend;
pub use sqlite_backend::SqliteBackend;
pub use url::{check_tls_precondition, has_explicit_plaintext_optin, is_rdb_url, RdbKind, RdbUrl};

/// `RdbUrl` の種別に応じて対応するバックエンドへ接続する。
///
/// 両バックエンドとも現状 TLS 未対応（`NoTls` 固定）のため、接続前に
/// `check_tls_precondition` で平文接続の可否を確認する: `sslmode`/`ssl-mode` が
/// 暗号化を要求していればエラー（フェイルクローズ）、非ローカルホストへの平文接続は
/// `sslmode=disable` の明示（opt-in）が無ければエラー、ループバックホストは無指定でも
/// 許可する（詳細は `url::check_tls_precondition` を参照）。
fn connect(url: &RdbUrl) -> Result<Box<dyn OptunaBackend>, String> {
    check_tls_precondition(&url.url)?;
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

/// RDB ライブ更新ポーリング用の再利用可能な接続セッション。
///
/// `study_fingerprint_url` は呼び出す度に接続・切断するため、ポーリング間隔ごとに
/// 接続を張り直すコストが掛かる（`RdbLivePoller` は tick 毎にこれを呼んでいた）。
/// 本セッションは接続（`Box<dyn OptunaBackend>`）を保持し、`fingerprint` 呼び出し間で
/// 使い回す。フィンガープリント取得がエラーになった場合、接続そのものが壊れている
/// 可能性があるため、内部では自動再接続せず、呼び出し側がセッションを破棄して
/// 次回改めて `connect` し直す設計とする（`RdbLivePoller` 側の挙動を参照）。
pub struct RdbFingerprintSession {
    backend: Box<dyn OptunaBackend>,
}

impl RdbFingerprintSession {
    /// `RdbUrl` へ接続し、再利用可能なセッションを作る。
    pub fn connect(url: &RdbUrl) -> Result<Self, String> {
        let backend = connect(url)?;
        Ok(Self { backend })
    }

    /// 保持している接続を再利用してフィンガープリントを取得する
    /// （接続の張り直しは行わない。詳細は `study_fingerprint` を参照）。
    pub fn fingerprint(&mut self, study_id: u32) -> Result<StudyFingerprint, String> {
        study_fingerprint(self.backend.as_mut(), study_id)
    }
}

/// URL 版: ライブ更新のポーリングで呼ぶ軽量フィンガープリント取得。
/// 詳細は `study_fingerprint` を参照。
///
/// 呼ぶ度に接続・切断するワンショット用途向けの互換 API。ポーリングのように
/// 繰り返し呼ぶ場合は、接続を使い回せる `RdbFingerprintSession` を使うこと。
pub fn study_fingerprint_url(url: &RdbUrl, study_id: u32) -> Result<StudyFingerprint, String> {
    let mut session = RdbFingerprintSession::connect(url)?;
    session.fingerprint(study_id)
}
