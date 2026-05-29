use std::sync::mpsc;

use crate::io::live_update_poller::LiveUpdatePoller;
use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;
use crate::state::message_handler::MessageHandler;
use crate::state::messages::AppMessage;
use crate::ui::toolbar::ToolbarAction;
use crate::ui::widget_states::WidgetStates;
use tunny_core::io::journal::live_update::LiveUpdateContext;

pub struct TunnyApp {
    pub app_state: AppState,
    pub layout: LayoutState,
    pub widget_states: WidgetStates,
    pub is_loading: bool,
    pub load_error: Option<String>,
    tx: mpsc::SyncSender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
    poller: Option<LiveUpdatePoller>,
}

impl TunnyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<std::path::PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(crate::theme::tunny_light_visuals());

        // Inter フォントを設定
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/Inter-VariableFont.ttf")).into(),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Inter".to_owned());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("Inter".to_owned());
        cc.egui_ctx.set_fonts(fonts);

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
            poller: None,
        }
    }

    /// バックグラウンドタスク起動用の Sender クローンを返す
    pub fn sender(&self) -> mpsc::SyncSender<AppMessage> {
        self.tx.clone()
    }

    /// ノンブロッキングにメッセージを処理し AppState を更新する
    pub fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            let is_journal_parsed = matches!(&msg, AppMessage::JournalParsed { .. });
            let is_live_error = matches!(&msg, AppMessage::LiveUpdateError(_));

            MessageHandler::handle(
                msg,
                &mut self.app_state,
                &mut self.widget_states,
                &mut self.is_loading,
                &mut self.load_error,
            );

            if is_journal_parsed && self.app_state.live_update.enabled {
                self.restart_poller();
            }
            if is_live_error {
                // poller stopped itself — drop the handle
                self.poller = None;
            }

            ctx.request_repaint();
        }
    }

    pub fn apply_toolbar_actions(&mut self, actions: Vec<ToolbarAction>) {
        for action in actions {
            match action {
                ToolbarAction::OpenJournal(path) => {
                    // Stop existing poller before loading new file
                    if let Some(mut p) = self.poller.take() {
                        p.stop();
                    }
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
                    self.app_state.live_update.enabled =
                        !self.app_state.live_update.enabled;
                    if self.app_state.live_update.enabled {
                        self.restart_poller();
                    } else {
                        if let Some(mut p) = self.poller.take() {
                            p.stop();
                        }
                        self.app_state.live_update.poller_active = false;
                    }
                }
                ToolbarAction::SetPollInterval(ms) => {
                    self.app_state.live_update.interval_ms = ms;
                    if let Some(ref poller) = self.poller {
                        poller.update_interval(ms);
                    }
                }
                ToolbarAction::GenerateHtmlReport => {
                    if let Some(ctx) = &self.app_state.current_study {
                        crate::io::html_report::build_and_send_report(
                            ctx,
                            &self.app_state.selected_indices,
                            self.sender(),
                        );
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
                        self.app_state.pinned_trials = snap.pinned_trials;
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
                    let mut snap = session::SessionSnapshot::new(
                        name,
                        self.app_state.filter_ranges.clone(),
                        self.app_state.selected_indices.clone(),
                    );
                    snap.pinned_trials = self.app_state.pinned_trials.clone();
                    session::save_session(&snap);
                }
                ToolbarAction::ClearLoadError => {
                    self.load_error = None;
                }
                ToolbarAction::ExportCsv(target) => {
                    if let Some(ctx) = &self.app_state.current_study {
                        let csv = crate::io::export::build_csv_string(
                            &crate::io::export::select_rows_for_export(
                                &ctx.trial_rows(),
                                &self.app_state.selected_indices,
                                &ctx.pareto_indices,
                                &target,
                            ),
                            &ctx.meta.param_names,
                            &ctx.meta.objective_names,
                        );
                        let _ = crate::io::export::save_csv_to_file(&csv);
                    }
                }
                ToolbarAction::AddComparisonStudy => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Journal", &["log"])
                        .pick_file()
                    {
                        let main_name = self
                            .app_state
                            .current_study
                            .as_ref()
                            .map(|c| c.meta.name.clone())
                            .unwrap_or_default();
                        let study_idx = self.app_state.comparison_studies.len();
                        self.app_state.comparison_mode = true;
                        // 同一ファイルなら再パース不要: 既存メタをそのまま渡す（option C）
                        let same_file_metas =
                            if self.app_state.journal_path.as_deref() == Some(path.as_path()) {
                                Some(self.app_state.all_studies.clone())
                            } else {
                                None
                            };
                        crate::io::study_worker::dispatch_load_comparison_study(
                            path,
                            main_name,
                            study_idx,
                            same_file_metas,
                            self.sender(),
                        );
                    }
                }
                ToolbarAction::RemoveComparisonStudy(idx) => {
                    if idx < self.app_state.comparison_studies.len() {
                        self.app_state.comparison_studies.remove(idx);
                        if idx < self.app_state.comparison_colors.len() {
                            self.app_state.comparison_colors.remove(idx);
                        }
                    }
                }
            }
        }
    }

    /// ポーラーを現在のファイルで（再）起動する
    fn restart_poller(&mut self) {
        // Stop any existing poller
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }

        let Some(ref file_path) = self.app_state.journal_path else {
            return;
        };

        let byte_offset = std::fs::metadata(file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let next_trial_id = self
            .app_state
            .current_study
            .as_ref()
            .map(|s| s.trial_rows().len() as u32)
            .unwrap_or_else(|| {
                self.app_state
                    .all_studies
                    .iter()
                    .map(|s| s.completed_trials as u32)
                    .sum()
            });

        let ctx = LiveUpdateContext {
            file_path: file_path.clone(),
            initial_byte_offset: byte_offset,
            next_trial_id,
            study_distributions: vec![],
            no_change_timeout_ms: 60_000,
        };

        let interval_ms = self.app_state.live_update.interval_ms;
        let poller = LiveUpdatePoller::start(ctx, self.tx.clone(), interval_ms);
        self.app_state.live_update.poller_active = true;
        self.poller = Some(poller);
    }
}

impl Drop for TunnyApp {
    fn drop(&mut self) {
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }
    }
}

impl eframe::App for TunnyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_messages(ctx);
        crate::ui::layout::show_layout(self, ctx);

        // PNG capture flow: request screenshot on next frame, consume event when it arrives
        let cap = &mut self.widget_states.capture;
        if cap.pending_capture.is_some() && !cap.screenshot_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            cap.screenshot_requested = true;
        }

        if cap.screenshot_requested {
            let scale = ctx.pixels_per_point();
            let crop_rect = cap.pending_capture_rect;
            let event = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });

            if let Some(image) = event {
                cap.screenshot_requested = false;
                cap.pending_capture = None;
                cap.pending_capture_rect = None;

                let result = (|| -> Result<Option<()>, String> {
                    let rect = crop_rect.ok_or_else(|| "No capture rect".to_string())?;
                    let cropped = crate::io::chart_capture::crop_image(&image, rect, scale)
                        .ok_or_else(|| "Crop rect outside image bounds".to_string())?;
                    let png_bytes = crate::io::chart_capture::encode_png(cropped)?;
                    crate::io::chart_capture::save_png_to_file(&png_bytes)
                })();

                match result {
                    Ok(None) => {} // user cancelled — no-op
                    Ok(Some(())) => {}
                    Err(e) => {
                        self.widget_states.capture.last_error = Some(e);
                    }
                }
            }
        }
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

    #[test]
    fn toggle_live_update_updates_state() {
        let mut app_state = AppState::new();
        assert!(!app_state.live_update.enabled);
        app_state.live_update.enabled = true;
        assert!(app_state.live_update.enabled);
        app_state.live_update.enabled = false;
        assert!(!app_state.live_update.enabled);
    }

    #[test]
    fn set_poll_interval_updates_state() {
        let mut app_state = AppState::new();
        assert_eq!(app_state.live_update.interval_ms, 2000);
        app_state.live_update.interval_ms = 5000;
        assert_eq!(app_state.live_update.interval_ms, 5000);
    }
}
