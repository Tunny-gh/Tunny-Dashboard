use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, OnceLock};

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::StudyMeta;
use crate::state::messages::AppMessage;

use tunny_core::dataframe::DataFrame;

enum StudyCommand {
    /// Phase 1: ファイルをスキャンして Study 一覧のみ取得する
    ScanJournal {
        path: PathBuf,
        tx: SyncSender<AppMessage>,
    },
    /// フラット CSV（1 行 = 1 トライアル）形式を読み込み、単一 Study を登録する。
    ScanCsv {
        path: PathBuf,
        tx: SyncSender<AppMessage>,
    },
    /// Phase 2 兼再選択: 未ロードなら完全パース、ロード済みなら即活性化
    SelectStudy {
        meta: StudyMeta,
        tx: SyncSender<AppMessage>,
    },
    /// 同一ファイル内の別 Study を比較対象としてロードする。
    /// 未ロードならキャッシュ済みバイト列から該当 Study のみパースし、
    /// アクティブ Study は変更せずに DataFrame スナップショットと HV 履歴を返す。
    LoadComparisonStudy {
        meta: StudyMeta,
        study_idx: usize,
        tx: SyncSender<AppMessage>,
    },
}

/// ワーカースレッドのローカル状態
struct WorkerState {
    /// Phase 1 で読み込んだ生バイト列。Phase 2 でファイル再読み込みを避けるためキャッシュする
    journal_data: Option<Vec<u8>>,
    /// DataFrame をグローバルストアに登録済みの study_id セット
    loaded_study_ids: HashSet<u32>,
    /// フラット CSV インポート時の `img` 列由来アーティファクト。
    /// `(artifacts_dir, trial_id → entries)`。Study 選択（StudySelected が `clear()` で
    /// アーティファクトを破棄する）後に毎回再送するため保持する。Journal を開くと None に戻す。
    csv_artifacts: Option<(PathBuf, HashMap<u32, Vec<ArtifactEntry>>)>,
}

fn worker_sender() -> &'static mpsc::Sender<StudyCommand> {
    static SENDER: OnceLock<mpsc::Sender<StudyCommand>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (cmd_tx, cmd_rx) = mpsc::channel::<StudyCommand>();
        std::thread::spawn(move || {
            let mut state = WorkerState {
                journal_data: None,
                loaded_study_ids: HashSet::new(),
                csv_artifacts: None,
            };
            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    StudyCommand::ScanJournal { path, tx } => {
                        let (data, msg) = crate::io::journal::scan_journal_task(path);
                        if !matches!(msg, AppMessage::Error(_)) {
                            state.journal_data = Some(data);
                            state.loaded_study_ids.clear();
                            state.csv_artifacts = None;
                        }
                        let _ = tx.send(msg);
                    }
                    StudyCommand::ScanCsv { path, tx } => {
                        match crate::io::flat_csv::load_csv(&path) {
                            Ok((meta, artifacts_dir, artifacts)) => {
                                // CSV は単一 Study を即時ストア登録済み。Journal の
                                // ストリーミング経路は使わず、loaded 扱いにする。
                                state.journal_data = None;
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
                    StudyCommand::SelectStudy { meta, tx } => {
                        let study_id = meta.study_id;
                        if state.loaded_study_ids.contains(&study_id) {
                            // DataFrame は既にストアにある → そのまま活性化（即時 1 通）
                            let _ = tx.send(crate::io::study::select_study_task(meta));
                            // CSV インポート時は StudySelected の clear() でアーティファクトが
                            // 破棄されるため、選択のたびに再送する。
                            if let Some((dir, artifacts)) = &state.csv_artifacts {
                                let _ = tx.send(AppMessage::ArtifactsDirScanned {
                                    trial_artifacts: artifacts.clone(),
                                    artifacts_dir: dir.clone(),
                                });
                            }
                        } else if let Some(ref data) = state.journal_data {
                            // 未ロード → Phase 2: ストリーミング解析。完了 Trial を 1000 件ごとに
                            // StudyChunkLoaded として tx へ逐次送信する（複数通）。
                            let ok = crate::io::journal::stream_single_study_task(data, meta, &tx);
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
                    StudyCommand::LoadComparisonStudy {
                        meta,
                        study_idx,
                        tx,
                    } => {
                        let study_id = meta.study_id;
                        // DataFrame を確保する: 既にストアにあればそのまま、
                        // 未ロードならキャッシュ済みバイト列から該当 Study のみパースする。
                        let df = match tunny_core::dataframe::snapshot(study_id) {
                            Some(df) => Some(df),
                            None => match state.journal_data.as_ref() {
                                Some(data) => {
                                    match tunny_core::io::journal::parser::parse_single_study(
                                        data, study_id,
                                    ) {
                                        Ok((_full_meta, df)) => {
                                            let arc = Arc::new(df);
                                            tunny_core::dataframe::swap_snapshot(
                                                study_id,
                                                arc.clone(),
                                            );
                                            state.loaded_study_ids.insert(study_id);
                                            Some(arc)
                                        }
                                        Err(_) => None,
                                    }
                                }
                                None => None,
                            },
                        };
                        let msg = match df {
                            Some(df) => build_comparison_loaded(meta, study_idx, &df),
                            None => AppMessage::ComparisonStudyLoadFailed(format!(
                                "Failed to load study '{}' from the current journal.",
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

/// フラット CSV を読み込み単一 Study として登録する。
pub fn dispatch_scan_csv(path: PathBuf, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::ScanCsv { path, tx });
}

pub fn dispatch_select_study(meta: StudyMeta, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::SelectStudy { meta, tx });
}

/// 同一ファイル内の別 Study を比較対象としてロードする。
/// ワーカースレッド経由でキャッシュ済みバイト列を再利用し、
/// アクティブ Study を変更せずに `ComparisonStudyLoaded` を送信する。
pub fn dispatch_load_comparison_study(
    meta: StudyMeta,
    study_idx: usize,
    tx: SyncSender<AppMessage>,
) {
    let _ = worker_sender().send(StudyCommand::LoadComparisonStudy {
        meta,
        study_idx,
        tx,
    });
}

/// 比較 Study の DataFrame スナップショットから `StudyContext` を構築する。
/// Pareto ランクはこの用途では不要なため計算せず空で初期化する
/// （`StudyView::new` が空ベクタを行数分の 0 に補完する）。
/// 指標値の計算は `poll_chart` が base+全比較を一括で行う。
fn build_comparison_loaded(meta: StudyMeta, study_idx: usize, df: &Arc<DataFrame>) -> AppMessage {
    use crate::state::app_state::{StudyContext, StudyView};

    let view = StudyView::new(Arc::clone(df), Vec::new());
    AppMessage::ComparisonStudyLoaded {
        study_idx,
        context: Box::new(StudyContext {
            meta,
            view,
            pareto_indices: Vec::new(),
        }),
    }
}
