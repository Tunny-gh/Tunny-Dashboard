//! RDB（PostgreSQL/MySQL）接続 URL のブリッジ。`io/sqlite.rs` と同型で、
//! 呼び先だけ `tunny_core::rdb::*_url` 系（`RdbUrl` を受け取る版）に差し替えてある。
//!
//! SQLite との違い: SQLite はローカルファイルパスを毎回そのまま再クエリすれば良いが、
//! RDB は接続 URL（パスワードを含む）を `journal_path: Option<PathBuf>` に文字列として
//! 格納している（配管の diff を最小化するため。詳細は Phase C 設計ドキュメント参照）。
//! `path_as_rdb_url` はその文字列を都度 `RdbUrl::parse` へ通して判定・復元する。

use std::path::Path;
use std::sync::mpsc::SyncSender;

use tunny_core::rdb::RdbUrl;

use crate::state::messages::AppMessage;

/// `journal_path` に格納されたパスが RDB 接続 URL として解釈できるかを判定し、
/// 可能なら `RdbUrl` を復元する。SQLite の `is_sqlite_path` に相当する判定関数だが、
/// こちらは拡張子ではなく文字列全体を `RdbUrl::parse` に通す。
pub fn path_as_rdb_url(path: &Path) -> Option<RdbUrl> {
    path.to_str().and_then(RdbUrl::parse)
}

/// Phase 1: RDB ストレージを開いて Study 一覧を返す（`io::sqlite::scan_sqlite_task` と同じ役割）。
pub fn scan_rdb_task(url: RdbUrl) -> AppMessage {
    match tunny_core::rdb::scan_study_list_url(&url) {
        Ok(studies) => {
            let app_studies: Vec<crate::state::app_state::StudyMeta> = studies
                .into_iter()
                .map(crate::io::journal::convert_study_meta)
                .collect();
            AppMessage::JournalParsed {
                studies: app_studies,
                path: std::path::PathBuf::from(url.url),
            }
        }
        Err(e) => AppMessage::Error(e),
    }
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、単一チャンクとして
/// `StudyChunkLoaded` を送信する（`io::sqlite::load_single_study_task` と同じ役割）。
pub fn load_single_study_task(url: &RdbUrl, study_id: u32, tx: &SyncSender<AppMessage>) -> bool {
    match tunny_core::rdb::parse_single_study_rows_url(url, study_id) {
        Ok(rows) => {
            tunny_core::dataframe::store_extras_for(study_id, rows.extras);
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

/// ライブ更新: フィンガープリント変化を検出した study を丸ごと再パースし、
/// 共有ストアを差し替えた上で `AppMessage::SqliteLiveReloadDone` を送る
/// （RDB ライブ更新もこのメッセージをそのまま流用する。`io::sqlite::reload_single_study_task`
/// と同じ役割）。
pub fn reload_single_study_task(url: &RdbUrl, study_id: u32, tx: &SyncSender<AppMessage>) -> bool {
    match tunny_core::rdb::parse_single_study_url(url, study_id) {
        Ok((meta, df, extras)) => {
            tunny_core::dataframe::swap_snapshot(study_id, std::sync::Arc::new(df));
            tunny_core::dataframe::store_extras_for(study_id, extras);
            let _ = tx.send(AppMessage::SqliteLiveReloadDone {
                study_id,
                meta: crate::io::journal::convert_study_meta(meta),
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
    use std::path::PathBuf;

    #[test]
    fn path_as_rdb_url_accepts_postgresql_and_mysql() {
        assert!(
            path_as_rdb_url(Path::new("postgresql://u:p@localhost:5432/db")).is_some(),
            "postgresql:// should be recognized as RDB URL"
        );
        assert!(
            path_as_rdb_url(Path::new("mysql://u:p@localhost:3306/db")).is_some(),
            "mysql:// should be recognized as RDB URL"
        );
        assert!(path_as_rdb_url(Path::new("postgresql+psycopg2://u:p@localhost/db")).is_some());
    }

    #[test]
    fn path_as_rdb_url_rejects_local_paths() {
        assert!(path_as_rdb_url(Path::new("study.log")).is_none());
        assert!(path_as_rdb_url(Path::new("study.db")).is_none());
        assert!(path_as_rdb_url(Path::new("study.csv")).is_none());
        assert!(path_as_rdb_url(Path::new("/abs/path/study.db")).is_none());
        assert!(path_as_rdb_url(Path::new("sqlite:///a.db")).is_none());
    }

    #[test]
    fn scan_rdb_task_unreachable_host_returns_error() {
        // 実 DB へ接続できない URL（未使用ポート）はエラーメッセージを返す。
        let url = RdbUrl::parse("postgresql://u:p@127.0.0.1:1/nope").unwrap();
        let msg = scan_rdb_task(url);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn load_single_study_task_unreachable_host_sends_error() {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let url = RdbUrl::parse("postgresql://u:p@127.0.0.1:1/nope").unwrap();
        let ok = load_single_study_task(&url, 1, &tx);
        assert!(!ok);
        match rx.try_recv() {
            Ok(AppMessage::Error(_)) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn reload_single_study_task_unreachable_host_sends_error() {
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let url = RdbUrl::parse("postgresql://u:p@127.0.0.1:1/nope").unwrap();
        let ok = reload_single_study_task(&url, 1, &tx);
        assert!(!ok);
        match rx.try_recv() {
            Ok(AppMessage::Error(_)) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn scan_rdb_task_uses_normalized_url_as_path() {
        // postgres:// (短縮形) は正規化されて postgresql:// になる。
        // 接続には失敗するが、正規化ロジック自体はエラーパスに関係なく通るため、
        // ここでは RdbUrl::parse の正規化結果を直接確認する。
        let url = RdbUrl::parse("postgres://u:p@127.0.0.1:1/nope").unwrap();
        assert_eq!(url.url, "postgresql://u:p@127.0.0.1:1/nope");
        let _ = PathBuf::from(url.url.clone());
    }
}
