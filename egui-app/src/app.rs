use std::sync::mpsc;

use crate::state::app_state::{AppState, StudyContext};
use crate::state::layout_state::LayoutState;
use crate::state::messages::AppMessage;
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
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::sync_channel(32);
        Self {
            app_state: AppState::new(),
            layout: LayoutState::default(),
            widget_states: WidgetStates::default(),
            is_loading: false,
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
            match msg {
                AppMessage::JournalParsed { studies, path } => {
                    // 単一スタディの場合は load_journal_task 内で既に StudySelected が
                    // 返るため、ここに来るのは複数スタディの場合のみ
                    self.app_state.all_studies = studies;
                    self.app_state.journal_path = Some(path);
                    self.is_loading = false;
                }
                AppMessage::StudySelected {
                    meta,
                    trial_rows,
                    gpu_data,
                    pareto_indices,
                } => {
                    self.app_state.clear();
                    self.app_state.current_study = Some(StudyContext {
                        meta,
                        trial_rows,
                        gpu_data,
                        pareto_indices,
                    });
                    self.is_loading = false;
                }
                AppMessage::SensitivityDone(result) => {
                    self.app_state.sensitivity_result = Some(result);
                }
                AppMessage::SobolDone(result) => {
                    self.app_state.sobol_result = Some(result);
                }
                AppMessage::ClusteringDone(result) => {
                    self.app_state.cluster_result = Some(result);
                }
                AppMessage::TopsisDone(result) => {
                    self.app_state.topsis_result = Some(result);
                }
                AppMessage::DownsampleDone { key, indices } => {
                    use crate::state::messages::DownsampleKey;
                    match key {
                        DownsampleKey::Scatter => {
                            self.app_state.downsample_cache.scatter = Some(indices)
                        }
                        DownsampleKey::Pcp => self.app_state.downsample_cache.pcp = Some(indices),
                        DownsampleKey::Thumbnail => {
                            self.app_state.downsample_cache.thumbnail = Some(indices)
                        }
                        DownsampleKey::Hover => {
                            self.app_state.downsample_cache.hover = Some(indices)
                        }
                    }
                }
                AppMessage::HvHistoryDone {
                    trial_ids,
                    hv_values,
                } => {
                    use crate::state::app_state::HvHistory;
                    self.app_state.hv_history = Some(HvHistory {
                        trial_ids,
                        hv_values,
                    });
                }
                AppMessage::LiveUpdateDone {
                    new_trial_count: _,
                    pareto_updated: _,
                    new_indices: _,
                } => {
                    // TODO: TASK-2027で実装
                }
                AppMessage::PdpDone { .. } => {
                    // TODO: TASK-2025で実装
                }
                AppMessage::Pdp2dDone(result) => {
                    self.widget_states.pdp_2d.result = Some(result);
                    self.widget_states.pdp_2d.computing = false;
                }
                AppMessage::Error(e) => {
                    self.load_error = Some(e);
                    self.is_loading = false;
                }
                AppMessage::SensitivityError(_e) => {
                    self.widget_states.importance.computing = false;
                }
            }
            ctx.request_repaint();
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
