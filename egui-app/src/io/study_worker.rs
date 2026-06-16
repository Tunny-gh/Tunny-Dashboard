use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, OnceLock};

use crate::state::app_state::StudyMeta;
use crate::state::messages::AppMessage;

use tunny_core::dataframe::DataFrame;

enum StudyCommand {
    /// Phase 1: ファイルをスキャンして Study 一覧のみ取得する
    ScanJournal {
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
}

fn worker_sender() -> &'static mpsc::Sender<StudyCommand> {
    static SENDER: OnceLock<mpsc::Sender<StudyCommand>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (cmd_tx, cmd_rx) = mpsc::channel::<StudyCommand>();
        std::thread::spawn(move || {
            let mut state = WorkerState {
                journal_data: None,
                loaded_study_ids: HashSet::new(),
            };
            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    StudyCommand::ScanJournal { path, tx } => {
                        let (data, msg) = crate::io::journal::scan_journal_task(path);
                        if !matches!(msg, AppMessage::Error(_)) {
                            state.journal_data = Some(data);
                            state.loaded_study_ids.clear();
                        }
                        let _ = tx.send(msg);
                    }
                    StudyCommand::SelectStudy { meta, tx } => {
                        let study_id = meta.study_id;
                        if state.loaded_study_ids.contains(&study_id) {
                            // DataFrame は既にストアにある → そのまま活性化（即時 1 通）
                            let _ = tx.send(crate::io::study::select_study_task(meta));
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
