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
