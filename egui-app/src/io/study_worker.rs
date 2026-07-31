use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, OnceLock};

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::StudyMeta;
use crate::state::messages::AppMessage;

use tunny_core::dataframe::DataFrame;
use tunny_core::rdb::RdbUrl;

enum StudyCommand {
    /// Phase 1: scans the file and retrieves only the list of Studies
    ScanJournal {
        path: PathBuf,
        tx: SyncSender<AppMessage>,
    },
    /// Loads a flat CSV (1 row = 1 trial) format and registers it as a single Study.
    ScanCsv {
        path: PathBuf,
        tx: SyncSender<AppMessage>,
    },
    /// Opens Optuna SQLite (RDBStorage) and retrieves only the list of Studies.
    ScanSqlite {
        path: PathBuf,
        tx: SyncSender<AppMessage>,
    },
    /// Opens Optuna RDBStorage (PostgreSQL/MySQL) via a URL and retrieves only the list of Studies.
    ScanRdb {
        url: RdbUrl,
        tx: SyncSender<AppMessage>,
    },
    /// Phase 2 / reselection: fully parses if not yet loaded, activates immediately if already loaded
    SelectStudy {
        meta: StudyMeta,
        tx: SyncSender<AppMessage>,
    },
    /// Loads another Study within the same file as a comparison target.
    /// If not yet loaded, parses only that Study from the cached byte buffer,
    /// and returns a DataFrame snapshot and HV history without changing the active Study.
    LoadComparisonStudy {
        meta: StudyMeta,
        /// Re-parses from storage even when a snapshot for this study_id is
        /// already in the shared store. Set by the toolbar Reload, whose whole
        /// point is that the stored snapshot is stale.
        force: bool,
        tx: SyncSender<AppMessage>,
    },
}

/// Local state of the worker thread
struct WorkerState {
    /// Raw byte buffer read in Phase 1. Cached to avoid re-reading the file in Phase 2
    journal_data: Option<Vec<u8>>,
    /// Path of the currently open Optuna SQLite storage. Unlike journal, the bytes aren't cached;
    /// Phase 2 re-queries directly from this path. Mutually exclusive with journal/CSV.
    sqlite_path: Option<PathBuf>,
    /// Connection URL of the currently open Optuna RDB (PostgreSQL/MySQL) storage.
    /// Like sqlite_path, bytes aren't cached; Phase 2 reconnects and re-queries from this URL every time.
    /// Mutually exclusive with journal/CSV/sqlite (wherever one is set, the others are always reset to None).
    rdb_url: Option<RdbUrl>,
    /// Set of study_ids whose DataFrame has been registered in the global store
    loaded_study_ids: HashSet<u32>,
    /// Artifacts derived from the `img` column when importing a flat CSV.
    /// `(artifacts_dir, trial_id -> entries)`. Kept so it can be resent every time a Study is
    /// selected (since StudySelected's `clear()` discards artifacts). Reset to None when a Journal is opened.
    csv_artifacts: Option<(PathBuf, HashMap<u32, Vec<ArtifactEntry>>)>,
}

fn worker_sender() -> &'static mpsc::Sender<StudyCommand> {
    static SENDER: OnceLock<mpsc::Sender<StudyCommand>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (cmd_tx, cmd_rx) = mpsc::channel::<StudyCommand>();
        std::thread::spawn(move || {
            let mut state = WorkerState {
                journal_data: None,
                sqlite_path: None,
                rdb_url: None,
                loaded_study_ids: HashSet::new(),
                csv_artifacts: None,
            };
            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    StudyCommand::ScanJournal { path, tx } => {
                        let (data, msg) = crate::io::journal::scan_journal_task(path);
                        if !matches!(msg, AppMessage::Error(_)) {
                            state.journal_data = Some(data);
                            state.sqlite_path = None;
                            state.rdb_url = None;
                            state.loaded_study_ids.clear();
                            state.csv_artifacts = None;
                        }
                        let _ = tx.send(msg);
                    }
                    StudyCommand::ScanCsv { path, tx } => {
                        match crate::io::flat_csv::load_csv(&path) {
                            Ok((meta, artifacts_dir, artifacts)) => {
                                // CSV immediately registers its single Study in the store. It
                                // doesn't use the Journal's streaming path, so it's treated as loaded.
                                state.journal_data = None;
                                state.sqlite_path = None;
                                state.rdb_url = None;
                                state.loaded_study_ids.clear();
                                state.loaded_study_ids.insert(meta.study_id);
                                state.csv_artifacts = Some((artifacts_dir, artifacts));
                                let _ = tx.send(AppMessage::JournalParsed {
                                    studies: vec![meta],
                                    path,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::Error(e));
                            }
                        }
                    }
                    StudyCommand::ScanSqlite { path, tx } => {
                        let msg = crate::io::sqlite::scan_sqlite_task(path.clone());
                        if !matches!(msg, AppMessage::Error(_)) {
                            state.journal_data = None;
                            state.sqlite_path = Some(path);
                            state.rdb_url = None;
                            state.loaded_study_ids.clear();
                            state.csv_artifacts = None;
                        }
                        let _ = tx.send(msg);
                    }
                    StudyCommand::ScanRdb { url, tx } => {
                        let msg = crate::io::rdb::scan_rdb_task(url.clone());
                        if !matches!(msg, AppMessage::Error(_)) {
                            state.journal_data = None;
                            state.sqlite_path = None;
                            state.rdb_url = Some(url);
                            state.loaded_study_ids.clear();
                            state.csv_artifacts = None;
                        }
                        let _ = tx.send(msg);
                    }
                    StudyCommand::SelectStudy { meta, tx } => {
                        let study_id = meta.study_id;
                        if state.loaded_study_ids.contains(&study_id) {
                            // The DataFrame is already in the store -> activate it directly (single immediate message)
                            let _ = tx.send(crate::io::study::select_study_task(meta));
                            // For CSV imports, StudySelected's clear() discards the artifacts,
                            // so resend them every time a selection happens.
                            if let Some((dir, artifacts)) = &state.csv_artifacts {
                                let _ = tx.send(AppMessage::ArtifactsDirScanned {
                                    trial_artifacts: artifacts.clone(),
                                    artifacts_dir: dir.clone(),
                                });
                            }
                        } else if let Some(ref data) = state.journal_data {
                            // Not yet loaded -> Phase 2: streaming parse. Sends completed Trials to
                            // tx incrementally as StudyChunkLoaded, every 1000 trials (multiple messages).
                            let ok = crate::io::journal::stream_single_study_task(data, meta, &tx);
                            if ok {
                                state.loaded_study_ids.insert(study_id);
                            }
                        } else if let Some(ref path) = state.sqlite_path {
                            // Not yet loaded -> loads all rows from SQLite as a single chunk (one message).
                            let ok = crate::io::sqlite::load_single_study_task(path, study_id, &tx);
                            if ok {
                                state.loaded_study_ids.insert(study_id);
                            }
                        } else if let Some(ref url) = state.rdb_url {
                            // Not yet loaded -> loads all rows from the RDB as a single chunk (one message).
                            let ok = crate::io::rdb::load_single_study_task(url, study_id, &tx);
                            if ok {
                                state.loaded_study_ids.insert(study_id);
                            }
                        } else {
                            let _ = tx.send(AppMessage::Error(
                                "No journal is loaded yet. Please open a journal first."
                                    .to_string(),
                            ));
                        }
                    }
                    StudyCommand::LoadComparisonStudy { meta, force, tx } => {
                        let study_id = meta.study_id;
                        // Secure the DataFrame: use it as-is if already in the store,
                        // otherwise parse only the target Study from the cached storage.
                        // `force` skips the store shortcut so a Reload re-reads the
                        // comparison study from storage instead of reusing the stale
                        // snapshot left over from before the reload.
                        // The old implementation swallowed parse failures with `Err(_) => None`,
                        // so the cause (corrupted data, DB error, etc.) never reached the user.
                        // Here the actual error string is kept and returned via `ComparisonStudyLoadFailed`.
                        let cached = (!force)
                            .then(|| tunny_core::dataframe::snapshot(study_id))
                            .flatten();
                        let df: Result<Arc<DataFrame>, String> = match cached {
                            Some(df) => Ok(df),
                            None => {
                                // Runs only the "parse" step per storage kind, normalizing the
                                // result to `Result<(DataFrame, StudyExtras), String>`.
                                // The "store registration" step shared across the three kinds is
                                // done together after a successful parse (consolidating the
                                // 8-line block that was copy-pasted 3 times in the old implementation
                                // into one place).
                                let parsed = if let Some(data) = state.journal_data.as_ref() {
                                    tunny_core::io::journal::parser::parse_single_study(
                                        data, study_id,
                                    )
                                    .map(|(_full_meta, df, extras)| (df, extras))
                                } else if let Some(path) = state.sqlite_path.as_ref() {
                                    tunny_core::sqlite::parse_single_study(path, study_id)
                                        .map(|(_full_meta, df, extras)| (df, extras))
                                } else if let Some(url) = state.rdb_url.as_ref() {
                                    tunny_core::rdb::parse_single_study_url(url, study_id)
                                        .map(|(_full_meta, df, extras)| (df, extras))
                                } else {
                                    Err("No storage is currently open.".to_string())
                                };
                                // Only on a successful parse does it perform the shared store registration and obtain the Arc.
                                parsed.map(|(df, extras)| {
                                    let arc = Arc::new(df);
                                    tunny_core::dataframe::swap_snapshot(study_id, arc.clone());
                                    tunny_core::dataframe::store_extras_for(study_id, extras);
                                    state.loaded_study_ids.insert(study_id);
                                    arc
                                })
                            }
                        };
                        let msg = match df {
                            Ok(df) => build_comparison_loaded(meta, &df),
                            Err(detail) => AppMessage::ComparisonStudyLoadFailed(format!(
                                "Failed to load study '{}' from the current storage: {detail}",
                                meta.name
                            )),
                        };
                        let _ = tx.send(msg);
                    }
                }
            }
        });
        cmd_tx
    })
}

pub fn dispatch_scan_journal(path: PathBuf, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanJournal { path, tx });
}

/// Loads a flat CSV and registers it as a single Study.
pub fn dispatch_scan_csv(path: PathBuf, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanCsv { path, tx });
}

/// Opens the Optuna SQLite storage and retrieves the list of Studies.
pub fn dispatch_scan_sqlite(path: PathBuf, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanSqlite { path, tx });
}

/// Opens Optuna RDBStorage (PostgreSQL/MySQL) via a URL and retrieves the list of Studies.
pub fn dispatch_scan_rdb(url: RdbUrl, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanRdb { url, tx });
}

pub fn dispatch_select_study(meta: StudyMeta, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::SelectStudy { meta, tx });
}

/// Loads another Study within the same file as a comparison target.
/// Reuses the cached byte buffer via the worker thread, and sends
/// `ComparisonStudyLoaded` without changing the active Study.
///
/// `force` re-parses from storage even when a snapshot is already registered —
/// used by the toolbar Reload, where the stored snapshot is exactly what has
/// gone stale.
pub fn dispatch_load_comparison_study(meta: StudyMeta, force: bool, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::LoadComparisonStudy { meta, force, tx });
}

/// Builds a `StudyContext` from a comparison Study's DataFrame snapshot.
/// Pareto rank isn't needed for this purpose, so it's initialized empty without computing it
/// (`StudyView::new` pads the empty vector with 0s for the row count).
/// Indicator-value computation is done together for base + all comparisons by `poll_chart`.
fn build_comparison_loaded(meta: StudyMeta, df: &Arc<DataFrame>) -> AppMessage {
    use crate::state::app_state::{StudyContext, StudyView};

    let view = StudyView::new(Arc::clone(df), Vec::new());
    AppMessage::ComparisonStudyLoaded {
        context: Box::new(StudyContext {
            meta,
            view,
            pareto_indices: Vec::new(),
        }),
    }
}
