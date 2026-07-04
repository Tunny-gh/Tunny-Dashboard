use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;

use crate::state::messages::AppMessage;

/// パスが Optuna SQLite ストレージ（拡張子 db/sqlite/sqlite3、大文字小文字無視）かを判定する。
pub fn is_sqlite_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        e.eq_ignore_ascii_case("db")
            || e.eq_ignore_ascii_case("sqlite")
            || e.eq_ignore_ascii_case("sqlite3")
    })
}

/// Phase 1: SQLite ストレージを開いて Study 一覧を返す（journal の `scan_journal_task` と同じ役割）。
///
/// journal と異なり生バイト列のキャッシュは不要（Phase 2 は `path` から直接再クエリする）ため、
/// 戻り値は `AppMessage` のみ。
pub fn scan_sqlite_task(path: PathBuf) -> AppMessage {
    match tunny_core::sqlite::scan_study_list(&path) {
        Ok(studies) => {
            let app_studies: Vec<crate::state::app_state::StudyMeta> = studies
                .into_iter()
                .map(crate::io::journal::convert_study_meta)
                .collect();
            AppMessage::JournalParsed {
                studies: app_studies,
                path,
            }
        }
        Err(e) => AppMessage::Error(e),
    }
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、単一チャンクとして
/// `StudyChunkLoaded` を送信する（journal の `stream_single_study_task` と同じ役割）。
///
/// SQLite は行指向クエリで対象 study の全行を一括取得できるため journal のような
/// バッチ分割ストリーミングは不要。`is_first = is_final = true` の 1 通に全行を積んで送ることで、
/// `MessageHandler::handle_study_chunk` を journal と共有し、param フィルタスライダーや
/// pareto ランク計算などの UI 状態を journal 経路と完全に同一にする。
pub fn load_single_study_task(path: &Path, study_id: u32, tx: &SyncSender<AppMessage>) -> bool {
    match tunny_core::sqlite::parse_single_study_rows(path, study_id) {
        Ok(rows) => {
            let _ = tx.send(AppMessage::StudyChunkLoaded {
                study_id,
                meta: crate::io::journal::convert_study_meta(rows.meta),
                new_rows: rows.rows,
                param_names: rows.param_names,
                objective_names: rows.objective_names,
                user_attr_numeric_names: rows.user_attr_numeric_names,
                user_attr_string_names: rows.user_attr_string_names,
                max_constraints: rows.max_constraints,
                is_first: true,
                is_final: true,
            });
            true
        }
        Err(e) => {
            let _ = tx.send(AppMessage::Error(e));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_sqlite_path_detects_extensions() {
        assert!(is_sqlite_path(Path::new("study.db")));
        assert!(is_sqlite_path(Path::new("STUDY.DB")));
        assert!(is_sqlite_path(Path::new("study.sqlite")));
        assert!(is_sqlite_path(Path::new("study.sqlite3")));
        assert!(!is_sqlite_path(Path::new("study.log")));
        assert!(!is_sqlite_path(Path::new("study.csv")));
        assert!(!is_sqlite_path(Path::new("noext")));
    }

    #[test]
    fn scan_sqlite_task_nonexistent_path_returns_error() {
        let path = PathBuf::from("/nonexistent/study.db");
        let msg = scan_sqlite_task(path);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn load_single_study_task_nonexistent_path_sends_error() {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let ok = load_single_study_task(Path::new("/nonexistent/study.db"), 1, &tx);
        assert!(!ok);
        match rx.try_recv() {
            Ok(AppMessage::Error(_)) => {}
            _ => panic!("Expected Error message"),
        }
    }
}
