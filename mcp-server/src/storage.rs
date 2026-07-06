//! ストレージ種別（journal / SQLite / RDB URL）のディスパッチ。
//!
//! egui-app と report_smoke example が使う判定規則を踏襲する:
//! 1. `RdbUrl::parse` が URL とみなせば PostgreSQL / MySQL
//! 2. 拡張子 `.log` / `.journal` なら Optuna journal ファイル
//! 3. それ以外はローカル SQLite ファイル
//!
//! RDB URL のパスワードはエラー文字列・レポート表示に残さない
//! （`RdbUrl::masked()` を必ず経由する）。

use std::path::Path;

use tunny_core::data::dataframe::DataFrame;
use tunny_core::data::extras::StudyExtras;
use tunny_core::io::rdb::{parse_single_study_url, scan_study_list_url, RdbUrl};
use tunny_core::journal_parser::StudyMeta;
use tunny_core::{journal_parser, sqlite};

/// storage 文字列から study 一覧を取得する。
pub fn scan_studies(storage: &str) -> Result<Vec<StudyMeta>, String> {
    if let Some(url) = RdbUrl::parse(storage) {
        return scan_study_list_url(&url);
    }
    let path = Path::new(storage);
    if is_journal(path) {
        let data = std::fs::read(path).map_err(|e| format!("failed to read {storage}: {e}"))?;
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
        let data = std::fs::read(path).map_err(|e| format!("failed to read {storage}: {e}"))?;
        let (meta, df, extras) = journal_parser::parse_single_study(&data, study_id)?;
        return Ok((meta, df, extras, storage.to_string()));
    }
    let (meta, df, extras) = sqlite::parse_single_study(path, study_id)?;
    Ok((meta, df, extras, storage.to_string()))
}

/// journal ファイルとみなす拡張子か。
fn is_journal(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("log") | Some("journal")
    )
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
    fn missing_sqlite_file_is_error_not_panic() {
        let err = scan_studies("/nonexistent/definitely_missing.db").unwrap_err();
        assert!(!err.is_empty());
    }
}
