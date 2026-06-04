use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, OnceLock};

use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;
use crate::state::results::HvHistory;

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
                                Some(data) => match tunny_core::io::journal::parser::parse_single_study(
                                    data, study_id,
                                ) {
                                    Ok((_full_meta, df)) => {
                                        let arc = Arc::new(df);
                                        tunny_core::dataframe::swap_snapshot(study_id, arc.clone());
                                        state.loaded_study_ids.insert(study_id);
                                        Some(arc)
                                    }
                                    Err(_) => None,
                                },
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

/// 比較 Study の DataFrame スナップショットから `StudyContext` と HV 履歴を構築する。
/// Pareto ランクはこの用途（HV 重ね描き）では不要なため計算せず 0 埋めする
/// （`StudyView::new` が空ベクタを行数分の 0 に補完する）。
fn build_comparison_loaded(meta: StudyMeta, study_idx: usize, df: &Arc<DataFrame>) -> AppMessage {
    use crate::state::app_state::{StudyContext, StudyView};

    let is_minimize = directions_to_is_minimize(&meta.directions, df.objective_col_names().len());
    let hv_history = compute_downsampled_hv(df, &is_minimize);

    let view = StudyView::new(Arc::clone(df), Vec::new());
    AppMessage::ComparisonStudyLoaded {
        study_idx,
        context: Box::new(StudyContext {
            meta,
            view,
            pareto_indices: Vec::new(),
        }),
        hv_history,
    }
}

/// `directions` を目的数 `n_obj` に合わせた `is_minimize` ベクタへ変換する。
/// 不足分は Minimize(true) で補い、超過分は切り詰める。
fn directions_to_is_minimize(directions: &[Direction], n_obj: usize) -> Vec<bool> {
    (0..n_obj)
        .map(|i| !matches!(directions.get(i), Some(Direction::Maximize)))
        .collect()
}

/// DataFrame からダウンサンプリング済みの Hypervolume 推移を計算する。
/// 基準 Study のチャート（`poll_chart`）と同じく最大 50 点までサンプリングする。
/// 目的が 0 件、または試行が 0 件のときは `None` を返す。
fn compute_downsampled_hv(df: &DataFrame, is_minimize: &[bool]) -> Option<HvHistory> {
    const TARGET_POINTS: usize = 50;
    let n = df.row_count();
    let obj_names = df.objective_col_names();
    if n == 0 || obj_names.is_empty() {
        return None;
    }
    let step = (n / TARGET_POINTS).max(1);
    let obj_cols: Vec<Option<&[f64]>> = obj_names
        .iter()
        .map(|name| df.get_numeric_column(name))
        .collect();
    let sampled_indices: Vec<usize> = (0..n).step_by(step).collect();
    let sampled_ids: Vec<u32> = sampled_indices
        .iter()
        .map(|&i| df.get_trial_id(i).unwrap_or(i as u32))
        .collect();
    let sampled_objs: Vec<Vec<f64>> = sampled_indices
        .iter()
        .map(|&i| {
            obj_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    let result =
        tunny_core::pareto::compute_hv_history_from_data(&sampled_ids, &sampled_objs, is_minimize);
    Some(HvHistory {
        trial_ids: result.trial_ids,
        hv_values: result.hv_values,
        sample_step: step,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_to_is_minimize_pads_and_maps() {
        let dirs = vec![Direction::Maximize];
        // 目的が 2 件: 1 件目は Maximize(false)、2 件目は不足分なので Minimize(true)
        let im = directions_to_is_minimize(&dirs, 2);
        assert_eq!(im, vec![false, true]);
    }

    #[test]
    fn directions_to_is_minimize_truncates() {
        let dirs = vec![Direction::Minimize, Direction::Maximize, Direction::Minimize];
        let im = directions_to_is_minimize(&dirs, 2);
        assert_eq!(im, vec![true, false]);
    }
}
