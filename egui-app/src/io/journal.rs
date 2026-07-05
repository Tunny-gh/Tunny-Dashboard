use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;

use tunny_core::io::journal::parser::{self, OptimizationDirection};

/// rust_core の StudyMeta → egui-app の StudyMeta に変換
pub fn convert_study_meta(meta: parser::StudyMeta) -> StudyMeta {
    StudyMeta {
        study_id: meta.study_id,
        name: meta.name,
        directions: meta
            .directions
            .into_iter()
            .map(|d| match d {
                OptimizationDirection::Minimize => Direction::Minimize,
                OptimizationDirection::Maximize => Direction::Maximize,
            })
            .collect(),
        completed_trials: meta.completed_trials as usize,
        param_names: meta.param_names,
        objective_names: meta.objective_names,
        param_bounds: meta.param_bounds,
    }
}

/// Phase 1: ファイルを読み込んで op_code=0/3 のみスキャンし Study 一覧を返す。
/// 生バイト列も返すことで Phase 2 でのファイル再読み込みを不要にする。
pub fn scan_journal_task(path: PathBuf) -> (Vec<u8>, AppMessage) {
    match crate::io::file::read_journal_file(&path) {
        Ok(data) => match parser::scan_study_list(&data) {
            Ok(studies) => {
                let app_studies: Vec<StudyMeta> =
                    studies.into_iter().map(convert_study_meta).collect();
                let msg = AppMessage::JournalParsed {
                    studies: app_studies,
                    path,
                };
                (data, msg)
            }
            Err(e) => (vec![], AppMessage::Error(e)),
        },
        Err(e) => (vec![], AppMessage::Error(e)),
    }
}

/// Phase 2（ストリーミング）: キャッシュ済みバイト列から target study を前方 1 パスで解析し、
/// 完了 Trial を `BATCH_SIZE` 件ごとに `StudyChunkLoaded` として逐次送信する。
///
/// ファイル再読み込みは行わない。`tx` は bounded channel のため、UI が描画に追いつくまで
/// 自然にバックプレッシャーがかかる。成功時 `true`（呼び出し側が loaded 登録に使う）。
pub fn stream_single_study_task(data: &[u8], meta: StudyMeta, tx: &SyncSender<AppMessage>) -> bool {
    /// グラフへ反映する 1 バッチあたりの完了 Trial 数。
    const BATCH_SIZE: usize = 1000;

    let study_id = meta.study_id;
    let result = parser::parse_single_study_streaming(data, study_id, BATCH_SIZE, |batch| {
        let _ = tx.send(AppMessage::StudyChunkLoaded {
            study_id,
            meta: convert_study_meta(batch.meta),
            new_rows: batch.new_rows,
            param_names: batch.param_names,
            objective_names: batch.objective_names,
            user_attr_numeric_names: batch.user_attr_numeric_names,
            user_attr_string_names: batch.user_attr_string_names,
            max_constraints: batch.max_constraints,
            is_first: batch.is_first,
            is_final: batch.is_final,
        });
    });

    match result {
        Ok(()) => true,
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
    fn scan_journal_nonexistent_path_returns_error() {
        let path = PathBuf::from("/nonexistent/file.log");
        let (_data, msg) = scan_journal_task(path);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn convert_minimize_direction() {
        let core_meta = parser::StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize,
            ],
            completed_trials: 5,
            total_trials: 10,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: Default::default(),
        };
        let app_meta = convert_study_meta(core_meta);
        assert_eq!(app_meta.directions[0], Direction::Minimize);
        assert_eq!(app_meta.directions[1], Direction::Maximize);
        assert_eq!(app_meta.completed_trials, 5usize);
    }
}
