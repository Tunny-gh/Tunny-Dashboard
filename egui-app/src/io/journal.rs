use std::path::PathBuf;

use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;

use tunny_core::io::journal::parser::{self, OptimizationDirection};

/// rust_core の StudyMeta → egui-app の StudyMeta に変換
fn convert_study_meta(meta: parser::StudyMeta) -> StudyMeta {
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

/// ファイルを読み込んで parse_journal を呼び出し AppMessage を返す。
/// スタディが1件のみの場合は同スレッドでスタディ選択まで完結させる
/// （thread_local の GLOBAL_STATE はスレッドをまたいで共有されないため）。
pub fn load_journal_task(path: PathBuf) -> AppMessage {
    match crate::io::file::read_journal_file(&path) {
        Ok(data) => match tunny_core::io::journal::parser::parse_journal(&data) {
            Ok(result) => {
                let studies: Vec<StudyMeta> =
                    result.studies.into_iter().map(convert_study_meta).collect();
                if studies.len() == 1 {
                    // parse 済みの thread_local データを同スレッドで即選択する
                    crate::io::study::select_study_task(studies[0].clone())
                } else {
                    AppMessage::JournalParsed {
                        studies,
                        path,
                    }
                }
            }
            Err(e) => AppMessage::Error(e),
        },
        Err(e) => AppMessage::Error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_journal_nonexistent_path_returns_error() {
        let path = PathBuf::from("/nonexistent/file.log");
        let msg = load_journal_task(path);
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
            directions: vec![OptimizationDirection::Minimize, OptimizationDirection::Maximize],
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
