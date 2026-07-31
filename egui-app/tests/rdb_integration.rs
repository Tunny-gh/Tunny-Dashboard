//! Manual E2E verification tests for RDB (PostgreSQL) URL support.
//!
//! CI has no real DB, so these are excluded via `#[ignore]` and run manually locally:
//! `cargo test -p tunny-desktop --test rdb_integration -- --ignored`
//!
//! Prerequisite: 3 studies (mo_constrained / pruned_lc / messy_states) must already be
//! seeded at `postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test`.
//! Performs no destructive operations (UPDATE/INSERT); limited to read-only
//! connectivity/consistency checks.

use std::sync::mpsc;
use std::time::Duration;

use tunny_core::rdb::RdbUrl;
use tunny_desktop::io::rdb::{load_single_study_task, scan_rdb_task};
use tunny_desktop::state::messages::AppMessage;

const TEST_DB_URL: &str = "postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test";

fn test_url() -> RdbUrl {
    RdbUrl::parse(TEST_DB_URL).expect("TEST_DB_URL must parse as a valid RDB URL")
}

/// Verifies that `scan_rdb_task` connects to a real DB and returns the 3 known studies
/// (the same function the study worker's `ScanRdb` handler calls).
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

    // JournalParsed.path holds the normalized URL string directly (as designed for the
    // `journal_path` plumbing).
    assert_eq!(path, std::path::PathBuf::from(TEST_DB_URL));
}

/// Verifies that `load_single_study_task` (the function the study worker's
/// `SelectStudy` RDB branch calls) sends every COMPLETE trial of the given study as a
/// single chunk.
#[test]
#[ignore]
fn load_single_study_task_loads_mo_constrained() {
    let url = test_url();
    // Pin down study_id via scan first, then select the target study (avoids depending
    // on a hardcoded ID).
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

/// Verifies that the `study_fingerprint_url` change-detection query returns the
/// same value when the DB hasn't changed. The Dashboard itself no longer polls
/// for changes (Reload re-reads on demand), but the query stays part of
/// `tunny_core`'s public API, so its stability is still worth pinning.
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
