use std::sync::mpsc;

use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;
use crate::state::message_handler::MessageHandler;
use crate::state::messages::AppMessage;
use crate::ui::toolbar::ToolbarAction;
use crate::ui::widget_states::WidgetStates;

pub struct TunnyApp {
    pub app_state: AppState,
    pub layout: LayoutState,
    pub widget_states: WidgetStates,
    pub is_loading: bool,
    pub load_error: Option<String>,
    tx: mpsc::SyncSender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
}

impl TunnyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<std::path::PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(crate::theme::tunny_light_visuals());
        let (tx, rx) = mpsc::sync_channel(32);
        let is_loading = initial_path.is_some();
        if let Some(path) = initial_path {
            crate::io::study_worker::dispatch_load_journal(path, tx.clone());
        }
        Self {
            app_state: AppState::new(),
            layout: LayoutState::default(),
            widget_states: WidgetStates::default(),
            is_loading,
            load_error: None,
            tx,
            rx,
        }
    }

    /// バックグラウンドタスク起動用の Sender クローンを返す
    pub fn sender(&self) -> mpsc::SyncSender<AppMessage> {
        self.tx.clone()
    }

    /// ノンブロッキングにメッセージを処理し AppState を更新する
    pub fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            MessageHandler::handle(
                msg,
                &mut self.app_state,
                &mut self.widget_states,
                &mut self.is_loading,
                &mut self.load_error,
            );
            ctx.request_repaint();
        }
    }

    pub fn apply_toolbar_actions(&mut self, actions: Vec<ToolbarAction>) {
        for action in actions {
            match action {
                ToolbarAction::OpenJournal(path) => {
                    self.is_loading = true;
                    self.load_error = None;
                    crate::io::study_worker::dispatch_load_journal(path, self.sender());
                }
                ToolbarAction::SetLayoutMode(mode) => {
                    self.layout.layout_mode = mode;
                }
                ToolbarAction::SelectStudy(meta) => {
                    self.is_loading = true;
                    crate::io::study_worker::dispatch_select_study(meta, self.sender());
                }
                ToolbarAction::ToggleLiveUpdate => {
                    self.app_state.live_update.enabled = !self.app_state.live_update.enabled;
                }
                ToolbarAction::GenerateHtmlReport => {
                    if let Some(ctx) = &self.app_state.current_study {
                        use crate::io::html_report::{
                            generate_html_report_async, HtmlReportSnapshot, HtmlTrialRow,
                            TrialStatistics,
                        };
                        let snap = HtmlReportSnapshot {
                            study_name: ctx.meta.name.clone(),
                            objective_names: ctx.meta.objective_names.clone(),
                            param_names: ctx.meta.param_names.clone(),
                            total_trials: ctx.trial_rows.len(),
                            pareto_count: ctx.pareto_indices.len(),
                            selected_trials: self
                                .app_state
                                .selected_indices
                                .iter()
                                .filter_map(|&id| ctx.trial_rows.iter().find(|r| r.trial_id == id))
                                .map(|r| HtmlTrialRow {
                                    trial_id: r.trial_id,
                                    trial_number: r.trial_number,
                                    params: r.params.clone(),
                                    objectives: r.objectives.clone(),
                                    pareto_rank: r.pareto_rank,
                                })
                                .collect(),
                            statistics: TrialStatistics {
                                objective_means: vec![0.0; ctx.meta.objective_names.len()],
                                objective_variances: vec![0.0; ctx.meta.objective_names.len()],
                                pareto_count: ctx.pareto_indices.len(),
                            },
                        };
                        generate_html_report_async(snap, self.sender());
                    }
                }
                ToolbarAction::ScanArtifacts(base_dir) => {
                    crate::io::artifacts::scan_artifacts_dir(base_dir, self.sender());
                }
                ToolbarAction::LoadSession => {
                    use crate::io::session;
                    if let Some(snap) = session::load_session() {
                        self.app_state.filter_ranges = snap.filter_ranges;
                        self.app_state.selected_indices = snap.selected_indices;
                        self.app_state.tradeoff_weights = snap.tradeoff_weights;
                    }
                }
                ToolbarAction::SaveSession => {
                    use crate::io::session;
                    let name = self
                        .app_state
                        .current_study
                        .as_ref()
                        .map(|c| c.meta.name.clone())
                        .unwrap_or_default();
                    let snap = session::SessionSnapshot::new(
                        name,
                        self.app_state.filter_ranges.clone(),
                        self.app_state.selected_indices.clone(),
                    );
                    session::save_session(&snap);
                }
                ToolbarAction::ClearLoadError => {
                    self.load_error = None;
                }
            }
        }
    }
}

impl eframe::App for TunnyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_messages(ctx);
        crate::ui::layout::show_layout(self, ctx);
    }
}

/// バックグラウンドタスク起動ヘルパー
pub fn spawn_task<F>(tx: mpsc::SyncSender<AppMessage>, f: F)
where
    F: FnOnce() -> AppMessage + Send + 'static,
{
    std::thread::spawn(move || {
        let msg = f();
        let _ = tx.send(msg);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::StudyMeta;

    fn make_channel() -> (mpsc::SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
        mpsc::sync_channel(32)
    }

    #[test]
    fn channel_send_receive_journal_parsed() {
        let (tx, rx) = make_channel();
        let studies = vec![StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![],
            completed_trials: 5,
            total_trials: 5,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
        }];
        tx.send(AppMessage::JournalParsed {
            studies,
            path: std::path::PathBuf::from("test.log"),
        })
        .unwrap();
        match rx.recv().unwrap() {
            AppMessage::JournalParsed { studies: s, .. } => assert_eq!(s.len(), 1),
            _ => panic!("Expected JournalParsed"),
        }
    }

    #[test]
    fn channel_try_recv_empty_returns_error() {
        let (_tx, rx) = make_channel();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn spawn_task_sends_message() {
        let (tx, rx) = make_channel();
        spawn_task(tx, || AppMessage::Error("from thread".to_string()));
        let msg = rx.recv().unwrap();
        match msg {
            AppMessage::Error(e) => assert_eq!(e, "from thread"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn spawn_task_multiple_messages() {
        let (tx, rx) = make_channel();
        let tx2 = tx.clone();
        spawn_task(tx, || AppMessage::Error("msg1".to_string()));
        spawn_task(tx2, || AppMessage::Error("msg2".to_string()));
        let mut received = vec![];
        for _ in 0..2 {
            match rx.recv().unwrap() {
                AppMessage::Error(e) => received.push(e),
                _ => panic!("Expected Error"),
            }
        }
        assert_eq!(received.len(), 2);
    }
}
