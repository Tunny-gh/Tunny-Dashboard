use std::path::PathBuf;

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
        total_trials: meta.total_trials as usize,
        param_names: meta.param_names,
        objective_names: meta.objective_names,
        user_attr_names: meta.user_attr_names,
        has_constraints: meta.has_constraints,
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

/// Phase 2: Phase 1 でキャッシュ済みのバイト列から target study のみ完全パースする。
/// ファイル再読み込みは行わない。
pub fn load_single_study_task(data: &[u8], meta: StudyMeta) -> AppMessage {
    let study_id = meta.study_id;
    match parser::parse_single_study(data, study_id) {
        Ok((full_meta_core, df)) => {
            tunny_core::dataframe::swap_snapshot(study_id, std::sync::Arc::new(df));
            let full_meta = convert_study_meta(full_meta_core);
            crate::io::study::select_study_task(full_meta)
        }
        Err(e) => AppMessage::Error(e),
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
        };
        let app_meta = convert_study_meta(core_meta);
        assert_eq!(app_meta.directions[0], Direction::Minimize);
        assert_eq!(app_meta.directions[1], Direction::Maximize);
        assert_eq!(app_meta.completed_trials, 5usize);
        assert_eq!(app_meta.total_trials, 10usize);
    }
}
