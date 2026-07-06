//! ストレージ種別（journal / SQLite / RDB URL）ディスパッチの一元化。
//!
//! 同じ判定規則が mcp-server・examples・egui-app に分散してドリフトしていた
//! ため、ここに集約する。判定規則:
//! 1. [`RdbUrl::parse`] が URL とみなせば PostgreSQL / MySQL
//! 2. 拡張子 `.log` / `.journal` なら Optuna journal ファイル
//! 3. それ以外はローカル SQLite ファイル
//!
//! ## 資格情報の扱い
//!
//! 返す表示用ストレージ名は RDB URL の場合必ず [`RdbUrl::masked`] を経由する。
//! エラーメッセージには storage 文字列そのものを埋め込まない — URL として
//! パースできなかった文字列にもパスワードが含まれ得るため（例: typo した
//! スキーム `postgresqll://user:pass@...`）、経路を問わずエコーしない。

use std::path::Path;

use crate::data::dataframe::DataFrame;
use crate::data::extras::StudyExtras;
use crate::io::journal::parser as journal_parser;
use crate::io::journal::parser::StudyMeta;
use crate::io::rdb::{parse_single_study_url, scan_study_list_url, RdbUrl};
use crate::io::sqlite;

/// storage 文字列から study 一覧を取得する。
pub fn scan_studies(storage: &str) -> Result<Vec<StudyMeta>, String> {
    if let Some(url) = RdbUrl::parse(storage) {
        return scan_study_list_url(&url);
    }
    let path = Path::new(storage);
    if is_journal(path) {
        let data = read_journal(path)?;
        return journal_parser::scan_study_list(&data);
    }
    sqlite::scan_study_list(path)
}

/// storage 文字列から単一 study を読み込む。
///
/// 返り値の 4 要素目は表示用ストレージ名（RDB URL はパスワードマスク済み）。
pub fn load_study(
    storage: &str,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras, String), String> {
    if let Some(url) = RdbUrl::parse(storage) {
        let masked = url.masked();
        let (meta, df, extras) = parse_single_study_url(&url, study_id)?;
        return Ok((meta, df, extras, masked));
    }
    let path = Path::new(storage);
    if is_journal(path) {
        let data = read_journal(path)?;
        let (meta, df, extras) = journal_parser::parse_single_study(&data, study_id)?;
        return Ok((meta, df, extras, storage.to_string()));
    }
    let (meta, df, extras) = sqlite::parse_single_study(path, study_id)?;
    Ok((meta, df, extras, storage.to_string()))
}

/// journal ファイルとみなす拡張子か。
pub fn is_journal(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("log") | Some("journal")
    )
}

/// journal ファイルを読み込む。エラーにパス文字列を埋め込まない
/// （モジュールドキュメントの資格情報方針を参照）。
fn read_journal(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("failed to read journal file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_extension_detection() {
        assert!(is_journal(Path::new("/tmp/study.log")));
        assert!(is_journal(Path::new("optuna.journal")));
        assert!(!is_journal(Path::new("/tmp/study.db")));
        assert!(!is_journal(Path::new("/tmp/study")));
    }

    #[test]
    fn missing_storage_is_error_not_panic() {
        assert!(scan_studies("/nonexistent/definitely_missing.db").is_err());
        assert!(load_study("/nonexistent/x.db", 1).is_err());
    }

    #[test]
    fn journal_error_does_not_echo_storage_string() {
        // typo スキーム + パスワード入りの文字列が journal 拡張子で終わるケース。
        // RdbUrl::parse は None → journal 分岐 → 読み込み失敗。エラーに
        // パスワード（および storage 文字列全体）が現れてはならない。
        let storage = "postgresqll://user:secret_pw@host/audit.journal";
        let err = scan_studies(storage).unwrap_err();
        assert!(!err.contains("secret_pw"), "leak: {err}");
        assert!(!err.contains(storage), "leak: {err}");
    }
}
