//! RDB（PostgreSQL）URL 対応の手動 E2E 確認テスト。
//!
//! CI には実 DB が無いため `#[ignore]` で外し、ローカルで手動実行する:
//! `cargo test -p tunny-desktop --test rdb_integration -- --ignored`
//!
//! 事前条件: `postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test` に
//! 3 studies（mo_constrained / pruned_lc / messy_states）が投入済みであること。
//! 破壊的操作（UPDATE/INSERT）は行わず、読み取り専用の疎通・整合性確認に留める。

use std::sync::mpsc;
use std::time::Duration;

use tunny_core::rdb::RdbUrl;
use tunny_desktop::io::rdb::{load_single_study_task, scan_rdb_task};
use tunny_desktop::state::messages::AppMessage;

const TEST_DB_URL: &str = "postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test";

fn test_url() -> RdbUrl {
    RdbUrl::parse(TEST_DB_URL).expect("TEST_DB_URL must parse as a valid RDB URL")
}

/// `scan_rdb_task` が実 DB に接続し、既知の 3 studies を返すことを確認する
/// （study worker の `ScanRdb` ハンドラが呼ぶのと同じ関数）。
#[test]
#[ignore]
fn scan_rdb_task_lists_known_studies() {
    let msg = scan_rdb_task(test_url());
    let AppMessage::JournalParsed { studies, path } = msg else {
        panic!("Expected JournalParsed, got a different/Error message");
    };

    let mut names: Vec<&str> = studies.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["messy_states", "mo_constrained", "pruned_lc"]);

    // JournalParsed.path には正規化済み URL 文字列がそのまま格納される
    // （`journal_path` 配管の設計どおり）。
    assert_eq!(path, std::path::PathBuf::from(TEST_DB_URL));
}

/// `load_single_study_task`（study worker の `SelectStudy` RDB 分岐が呼ぶ関数）が
/// 指定 study の全 COMPLETE trial を単一チャンクとして送ることを確認する。
#[test]
#[ignore]
fn load_single_study_task_loads_mo_constrained() {
    let url = test_url();
    // scan で study_id を確定してから対象 study を選ぶ（ハードコードした ID に依存しない）。
    let studies = match scan_rdb_task(url.clone()) {
        AppMessage::JournalParsed { studies, .. } => studies,
        AppMessage::Error(e) => panic!("scan_rdb_task failed: {e}"),
        _ => panic!("unexpected message"),
    };
    let target = studies
        .iter()
        .find(|s| s.name == "mo_constrained")
        .expect("mo_constrained study must exist in the fixture DB");

    let (tx, rx) = mpsc::sync_channel::<AppMessage>(4);
    let ok = load_single_study_task(&url, target.study_id, &tx);
    assert!(
        ok,
        "load_single_study_task should succeed against a live DB"
    );

    let msg = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("StudyChunkLoaded should arrive");
    match msg {
        AppMessage::StudyChunkLoaded {
            study_id,
            new_rows,
            is_first,
            is_final,
            ..
        } => {
            assert_eq!(study_id, target.study_id);
            assert!(is_first && is_final, "RDB load is always a single chunk");
            assert!(
                !new_rows.is_empty(),
                "mo_constrained fixture should have COMPLETE trials"
            );
        }
        _ => panic!("Expected StudyChunkLoaded"),
    }
}

/// ライブ更新ポーリングが使うフィンガープリント取得（`study_fingerprint_url`）が、
/// DB に変化が無ければ同一値を返すことを確認する（ポーリングループの
/// `fingerprint_fn` クロージャに相当する呼び出しの安定性チェック）。
#[test]
#[ignore]
fn study_fingerprint_url_is_stable_without_changes() {
    let url = test_url();
    let studies = match scan_rdb_task(url.clone()) {
        AppMessage::JournalParsed { studies, .. } => studies,
        AppMessage::Error(e) => panic!("scan_rdb_task failed: {e}"),
        _ => panic!("unexpected message"),
    };
    let study_id = studies
        .iter()
        .find(|s| s.name == "messy_states")
        .expect("messy_states study must exist in the fixture DB")
        .study_id;

    let first = tunny_core::rdb::study_fingerprint_url(&url, study_id)
        .expect("fingerprint should succeed against a live DB");
    let second = tunny_core::rdb::study_fingerprint_url(&url, study_id)
        .expect("fingerprint should succeed against a live DB");
    assert_eq!(
        first, second,
        "fingerprint must be stable across consecutive calls when nothing changed"
    );
}
