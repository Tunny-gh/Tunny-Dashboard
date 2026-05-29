use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::OnceLock;

use crate::state::app_state::StudyMeta;
use crate::state::messages::AppMessage;

enum StudyCommand {
    LoadJournal {
        path: PathBuf,
        tx: SyncSender<AppMessage>,
    },
    SelectStudy {
        meta: StudyMeta,
        tx: SyncSender<AppMessage>,
    },
}

fn worker_sender() -> &'static mpsc::Sender<StudyCommand> {
    static SENDER: OnceLock<mpsc::Sender<StudyCommand>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (cmd_tx, cmd_rx) = mpsc::channel::<StudyCommand>();
        std::thread::spawn(move || {
            let mut has_loaded_journal = false;
            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    StudyCommand::LoadJournal { path, tx } => {
                        let msg = crate::io::journal::load_journal_task(path);
                        has_loaded_journal = !matches!(msg, AppMessage::Error(_));
                        let _ = tx.send(msg);
                    }
                    StudyCommand::SelectStudy { meta, tx } => {
                        let msg = if has_loaded_journal {
                            crate::io::study::select_study_task(meta)
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

pub fn dispatch_load_journal(path: PathBuf, tx: SyncSender<AppMessage>) {
    let _ = worker_sender().send(StudyCommand::LoadJournal { path, tx });
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
    tx: SyncSender<AppMessage>,
) {
    std::thread::spawn(move || {
        let msg = load_comparison_study_task(&path, &main_study_name, study_idx);
        let _ = tx.send(msg);
    });
}

/// Journal ファイルを解析して比較 Study を選択し `AppMessage` を返す内部関数。
fn load_comparison_study_task(
    path: &std::path::Path,
    main_study_name: &str,
    study_idx: usize,
) -> AppMessage {
    let path_buf = path.to_path_buf();
    let data = match crate::io::file::read_journal_file(&path_buf) {
        Ok(d) => d,
        Err(e) => return AppMessage::ComparisonStudyLoadFailed(e),
    };

    let result = match tunny_core::io::journal::parser::parse_journal(&data) {
        Ok(r) => r,
        Err(e) => return AppMessage::ComparisonStudyLoadFailed(e),
    };

    let studies: Vec<StudyMeta> = result
        .studies
        .into_iter()
        .map(crate::io::journal::convert_study_meta)
        .collect();

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
            AppMessage::ComparisonStudyLoadFailed("Unexpected response from study loader.".to_string())
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
        let studies = vec![make_meta("study_a"), make_meta("study_b"), make_meta("study_c")];
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
