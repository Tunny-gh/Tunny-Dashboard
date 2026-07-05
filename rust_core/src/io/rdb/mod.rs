//! Optuna RDBStorage 読み取りの共通基盤。
//!
//! バックエンド（SQLite/PostgreSQL/MySQL）ごとの接続・型変換の差分を
//! `OptunaBackend` trait（`backend.rs`）に隔離し、クエリ組み立てロジック本体は
//! `generic.rs` へ 1 本化する。Phase A では SQLite 実装（`sqlite_backend.rs`）のみを
//! 提供し、`crate::io::sqlite` はここへ委譲する薄い互換層になる。

mod backend;
mod generic;
mod sqlite_backend;

pub use backend::{OptunaBackend, SqlParam, SqlValue};
pub use generic::{
    parse_single_study, parse_single_study_rows, scan_study_list, study_fingerprint, RdbStudyRows,
    StudyFingerprint,
};
pub use sqlite_backend::SqliteBackend;
