use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::OnceLock;

use crate::state::app_state::StudyMeta;
use crate::state::messages::AppMessage;

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
                        let msg = if state.loaded_study_ids.contains(&study_id) {
                            // DataFrame は既にストアにある → そのまま活性化
                            crate::io::study::select_study_task(meta)
                        } else if let Some(ref data) = state.journal_data {
                            // 未ロード → Phase 2: キャッシュ済みバイト列から対象 study を解析
                            let m = crate::io::journal::load_single_study_task(data, meta);
                            if matches!(m, AppMessage::StudySelected { .. }) {
                                state.loaded_study_ids.insert(study_id);
                            }
                            m
                        } else {
                            AppMessage::Error(
                                "No journal is loaded yet. Please open a journal first."
                                    .to_string(),
                            )
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

/// 比較 Study の study_idx を元に Journal からロードし `ComparisonStudyLoaded` を送信する。
/// 同名 Study がある場合はそれを優先し、なければ先頭 Study を採用する。
/// Study が存在しない場合は `ComparisonStudyLoadFailed` を送る。
pub fn dispatch_load_comparison_study(
    path: std::path::PathBuf,
    main_study_name: String,
    study_idx: usize,
    // 同一ファイルの場合のみ Some: 再パース不要な既存 StudyMeta リスト（option C）。
    same_file_metas: Option<Vec<StudyMeta>>,
    tx: SyncSender<AppMessage>,
) {
    std::thread::spawn(move || {
        let msg = load_comparison_study_task(&path, &main_study_name, study_idx, same_file_metas);
        let _ = tx.send(msg);
    });
}

/// Journal ファイルを解析して比較 Study を選択し `AppMessage` を返す内部関数。
///
/// `same_file_metas` が `Some` のとき、同一ファイルへの比較ロード：
///   - ファイル再読み込み・再パース・グローバルストア上書きを完全スキップ。
///   - 既存の StudyMeta を使って `snapshot` で Arc<DataFrame> を直接取得する。
fn load_comparison_study_task(
    path: &std::path::Path,
    main_study_name: &str,
    study_idx: usize,
    same_file_metas: Option<Vec<StudyMeta>>,
) -> AppMessage {
    // 同一ファイル最適化: 既存メタを使い再パースをスキップ
    let studies: Vec<StudyMeta> = if let Some(metas) = same_file_metas {
        metas
    } else {
        // クロスファイル: 通常通りパース（グローバルストアを上書きするが最大 4 件なので許容）
        let path_buf = path.to_path_buf();
        let data = match crate::io::file::read_journal_file(&path_buf) {
            Ok(d) => d,
            Err(e) => return AppMessage::ComparisonStudyLoadFailed(e),
        };
        let result = match tunny_core::io::journal::parser::parse_journal(&data) {
            Ok(r) => r,
            Err(e) => return AppMessage::ComparisonStudyLoadFailed(e),
        };
        result
            .studies
            .into_iter()
            .map(crate::io::journal::convert_study_meta)
            .collect()
    };

    let meta = match choose_comparison_study(&studies, main_study_name) {
        Some(m) => m.clone(),
        None => {
            return AppMessage::ComparisonStudyLoadFailed(
                "No studies found in the selected journal.".to_string(),
            )
        }
    };

    match crate::io::study::select_study_task(meta) {
        AppMessage::StudySelected {
            meta,
            study_id,
            pareto_rank,
            pareto_indices,
        } => {
            use crate::state::app_state::{StudyContext, StudyView};
            match tunny_core::dataframe::snapshot(study_id) {
                Some(df) => {
                    let view = StudyView::new(df, pareto_rank);
                    AppMessage::ComparisonStudyLoaded {
                        study_idx,
                        context: Box::new(StudyContext {
                            meta,
                            view,
                            pareto_indices,
                        }),
                    }
                }
                None => AppMessage::ComparisonStudyLoadFailed(format!(
                    "study_id {} not found in shared store",
                    study_id
                )),
            }
        }
        AppMessage::Error(e) => AppMessage::ComparisonStudyLoadFailed(e),
        other => {
            let _ = other;
            AppMessage::ComparisonStudyLoadFailed(
                "Unexpected response from study loader.".to_string(),
            )
        }
    }
}

/// 比較対象の Study を `main_study_name` と一致するものから選ぶ。
/// 一致がなければ先頭を返す。スタディがゼロ件のときは `None`。
pub fn choose_comparison_study<'a>(
    studies: &'a [StudyMeta],
    main_study_name: &str,
) -> Option<&'a StudyMeta> {
    if studies.is_empty() {
        return None;
    }
    studies
        .iter()
        .find(|s| s.name == main_study_name)
        .or_else(|| studies.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::Direction;

    fn make_meta(name: &str) -> StudyMeta {
        StudyMeta {
            study_id: 0,
            name: name.to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 0,
            total_trials: 0,
            param_names: vec![],
            objective_names: vec![],
            user_attr_names: vec![],
            has_constraints: false,
        }
    }

    #[test]
    fn choose_matching_study_if_name_exists() {
        let studies = vec![
            make_meta("study_a"),
            make_meta("study_b"),
            make_meta("study_c"),
        ];
        let chosen = choose_comparison_study(&studies, "study_b").unwrap();
        assert_eq!(chosen.name, "study_b");
    }

    #[test]
    fn fallback_to_first_study_when_no_name_match() {
        let studies = vec![make_meta("study_a"), make_meta("study_b")];
        let chosen = choose_comparison_study(&studies, "nonexistent").unwrap();
        assert_eq!(chosen.name, "study_a");
    }

    #[test]
    fn no_study_returns_none() {
        let studies: Vec<StudyMeta> = vec![];
        assert!(choose_comparison_study(&studies, "any").is_none());
    }
}
