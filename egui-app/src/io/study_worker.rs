use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, OnceLock};

use crate::state::app_state::StudyMeta;
use crate::state::messages::AppMessage;

use tunny_core::dataframe::DataFrame;
use tunny_core::rdb::RdbUrl;

enum StudyCommand {
    /// A scan barrier behind legacy storage writes, not Journal authority.
    ScanJournal {
        path: PathBuf,
        source: tdashboard_study::SourceInstanceId,
        scan: u64,
        tx: SyncSender<AppMessage>,
    },
    /// Loads a flat CSV (1 row = 1 trial) format and registers it as a single Study.
    ScanCsv {
        path: PathBuf,
        source_generation: u64,
        tx: SyncSender<AppMessage>,
    },
    /// Opens Optuna SQLite (RDBStorage) and retrieves only the list of Studies.
    ScanSqlite {
        path: PathBuf,
        source_generation: u64,
        tx: SyncSender<AppMessage>,
    },
    /// Opens Optuna RDBStorage (PostgreSQL/MySQL) via a URL and retrieves only the list of Studies.
    ScanRdb {
        url: RdbUrl,
        source_generation: u64,
        tx: SyncSender<AppMessage>,
    },
    /// Phase 2 / reselection: fully parses if not yet loaded, activates immediately if already loaded
    SelectStudy {
        meta: StudyMeta,
        source_generation: u64,
        tx: SyncSender<AppMessage>,
    },
    /// Loads another Study within the same file as a comparison target.
    /// If not yet loaded, parses only that Study from the cached byte buffer,
    /// and returns a DataFrame snapshot and HV history without changing the active Study.
    LoadComparisonStudy {
        meta: StudyMeta,
        source_generation: u64,
        /// Re-parses from storage even when a snapshot for this study_id is
        /// already in the shared store. Set by the toolbar Reload, whose whole
        /// point is that the stored snapshot is stale.
        force: bool,
        tx: SyncSender<AppMessage>,
    },
}

/// Local state of the worker thread
struct WorkerState {
    /// Path of the currently open Optuna SQLite storage. Unlike journal, the bytes aren't cached;
    /// Phase 2 re-queries directly from this path. Mutually exclusive with journal/CSV.
    sqlite_path: Option<PathBuf>,
    /// Connection URL of the currently open Optuna RDB (PostgreSQL/MySQL) storage.
    /// Like sqlite_path, bytes aren't cached; Phase 2 reconnects and re-queries from this URL every time.
    /// Mutually exclusive with journal/CSV/sqlite (wherever one is set, the others are always reset to None).
    rdb_url: Option<RdbUrl>,
    /// Set of study_ids whose DataFrame has been registered in the global store
    loaded_study_ids: HashSet<u32>,
    /// Unresolved img paths and their source identity, resent on CSV activation.
    csv_artifacts: Option<CsvArtifacts>,
}

struct CsvArtifacts {
    source_path: PathBuf,
    source_generation: u64,
    images: Vec<(u32, String)>,
}

fn worker_sender() -> &'static mpsc::Sender<StudyCommand> {
    static SENDER: OnceLock<mpsc::Sender<StudyCommand>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (cmd_tx, cmd_rx) = mpsc::channel::<StudyCommand>();
        std::thread::spawn(move || {
            let mut state = WorkerState {
                sqlite_path: None,
                rdb_url: None,
                loaded_study_ids: HashSet::new(),
                csv_artifacts: None,
            };
            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    StudyCommand::ScanJournal {
                        path,
                        source,
                        scan,
                        tx,
                    } => {
                        let result = crate::io::file::read_journal_file(&path).and_then(|bytes| {
                            let studies = tunny_core::journal_parser::scan_study_list(&bytes)?
                                .into_iter()
                                .map(crate::io::journal::convert_study_meta)
                                .collect();
                            Ok((bytes, studies))
                        });
                        let _ = tx.send(AppMessage::JournalScanned {
                            source,
                            scan,
                            path,
                            result,
                        });
                    }
                    StudyCommand::ScanCsv {
                        path,
                        source_generation,
                        tx,
                    } => {
                        match crate::io::flat_csv::load_csv(&path) {
                            Ok((meta, images)) => {
                                // CSV immediately registers its single Study in the store. It
                                // doesn't use the Journal's streaming path, so it's treated as loaded.
                                state.sqlite_path = None;
                                state.rdb_url = None;
                                state.loaded_study_ids.clear();
                                state.loaded_study_ids.insert(meta.study_id);
                                state.csv_artifacts = Some(CsvArtifacts {
                                    source_path: path.clone(),
                                    source_generation,
                                    images,
                                });
                                send_result(
                                    &tx,
                                    source_generation,
                                    AppMessage::JournalParsed {
                                        studies: vec![meta],
                                        path,
                                    },
                                );
                            }
                            Err(e) => {
                                send_result(&tx, source_generation, AppMessage::Error(e));
                            }
                        }
                    }
                    StudyCommand::ScanSqlite {
                        path,
                        source_generation,
                        tx,
                    } => {
                        let msg = crate::io::sqlite::scan_sqlite_task(path.clone());
                        if !matches!(msg, AppMessage::Error(_)) {
                            state.sqlite_path = Some(path);
                            state.rdb_url = None;
                            state.loaded_study_ids.clear();
                            state.csv_artifacts = None;
                        }
                        send_result(&tx, source_generation, msg);
                    }
                    StudyCommand::ScanRdb {
                        url,
                        source_generation,
                        tx,
                    } => {
                        let msg = crate::io::rdb::scan_rdb_task(url.clone());
                        if !matches!(msg, AppMessage::Error(_)) {
                            state.sqlite_path = None;
                            state.rdb_url = Some(url);
                            state.loaded_study_ids.clear();
                            state.csv_artifacts = None;
                        }
                        send_result(&tx, source_generation, msg);
                    }
                    StudyCommand::SelectStudy {
                        meta,
                        source_generation,
                        tx,
                    } => {
                        let study_id = meta.study_id;
                        if state.loaded_study_ids.contains(&study_id) {
                            // The DataFrame is already in the store -> activate it directly (single immediate message)
                            send_result(
                                &tx,
                                source_generation,
                                crate::io::study::select_study_task(meta),
                            );
                            // Supply source-identified CSV img associations on activation,
                            // including the initial import.
                            if let Some(artifacts) = &state.csv_artifacts {
                                send_result(
                                    &tx,
                                    source_generation,
                                    AppMessage::CsvArtifacts {
                                        source_path: artifacts.source_path.clone(),
                                        source_generation: artifacts.source_generation,
                                        images: artifacts.images.clone(),
                                    },
                                );
                            }
                        } else if let Some(ref path) = state.sqlite_path {
                            // Not yet loaded -> loads all rows from SQLite as a single chunk (one message).
                            // Capture that one result so every storage callback,
                            // including its error, receives the dispatch identity.
                            let (local_tx, local_rx) = mpsc::sync_channel(1);
                            let ok = crate::io::sqlite::load_single_study_task(
                                path, study_id, &local_tx,
                            );
                            if let Ok(result) = local_rx.try_recv() {
                                send_result(&tx, source_generation, result);
                            }
                            if ok {
                                state.loaded_study_ids.insert(study_id);
                            }
                        } else if let Some(ref url) = state.rdb_url {
                            // Not yet loaded -> loads all rows from the RDB as a single chunk (one message).
                            let (local_tx, local_rx) = mpsc::sync_channel(1);
                            let ok =
                                crate::io::rdb::load_single_study_task(url, study_id, &local_tx);
                            if let Ok(result) = local_rx.try_recv() {
                                send_result(&tx, source_generation, result);
                            }
                            if ok {
                                state.loaded_study_ids.insert(study_id);
                            }
                        } else {
                            send_result(
                                &tx,
                                source_generation,
                                AppMessage::Error(
                                    "No journal is loaded yet. Please open a journal first."
                                        .to_string(),
                                ),
                            );
                        }
                    }
                    StudyCommand::LoadComparisonStudy {
                        meta,
                        force,
                        source_generation,
                        tx,
                    } => {
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
                                let parsed = if let Some(path) = state.sqlite_path.as_ref() {
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
                        send_result(&tx, source_generation, msg);
                    }
                }
            }
        });
        cmd_tx
    })
}

/// Loads a flat CSV and registers it as a single Study.
pub fn dispatch_scan_csv(path: PathBuf, source_generation: u64, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanCsv {
        path,
        source_generation,
        tx,
    });
}

/// Opens the Optuna SQLite storage and retrieves the list of Studies.
pub fn dispatch_scan_sqlite(path: PathBuf, source_generation: u64, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanSqlite {
        path,
        source_generation,
        tx,
    });
}

/// Opens Optuna RDBStorage (PostgreSQL/MySQL) via a URL and retrieves the list of Studies.
pub fn dispatch_scan_rdb(url: RdbUrl, source_generation: u64, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanRdb {
        url,
        source_generation,
        tx,
    });
}

pub fn dispatch_select_study(meta: StudyMeta, source_generation: u64, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::SelectStudy {
        meta,
        source_generation,
        tx,
    });
}

/// Loads another Study within the same file as a comparison target.
/// Reuses the cached byte buffer via the worker thread, and sends
/// `ComparisonStudyLoaded` without changing the active Study.
///
/// `force` re-parses from storage even when a snapshot is already registered —
/// used by the toolbar Reload, where the stored snapshot is exactly what has
/// gone stale.
pub fn dispatch_load_comparison_study(
    meta: StudyMeta,
    force: bool,
    source_generation: u64,
    tx: SyncSender<AppMessage>,
) {
    let _ = worker_sender().send(StudyCommand::LoadComparisonStudy {
        meta,
        force,
        source_generation,
        tx,
    });
}

fn send_result(tx: &SyncSender<AppMessage>, source_generation: u64, result: AppMessage) {
    let _ = tx.send(AppMessage::StorageResult {
        source_generation,
        result: Box::new(result),
    });
}

pub fn dispatch_scan_journal(
    path: PathBuf,
    source: tdashboard_study::SourceInstanceId,
    scan: u64,
    tx: SyncSender<AppMessage>,
) {
    let _ = worker_sender().send(StudyCommand::ScanJournal {
        path,
        source,
        scan,
        tx,
    });
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
