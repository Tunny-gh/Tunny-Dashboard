use std::collections::HashMap;
use std::sync::mpsc;

use crate::io::live_update_poller::{LiveUpdatePoller, RdbLivePoller, SqliteLivePoller};
use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;
use crate::state::message_handler::MessageHandler;
use crate::state::messages::AppMessage;
use crate::state::results::LiveUpdateStorageKind;
use crate::ui::toolbar::ToolbarAction;
use crate::ui::widget_states::WidgetStates;

mod dialogs;
mod files;
mod poller;
mod run;

#[cfg(test)]
mod tests;

/// No-change duration (milliseconds) between the live update poller deciding "no change"
/// and sending the completion hint (`AppMessage::LiveUpdateMaybeComplete`).
/// Used as the common default for all three pollers (journal/sqlite/rdb).
const LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS: u64 = 60_000;

/// The live update poller currently running in the app. Since the implementation differs
/// per storage kind (journal/sqlite/rdb) (journal: byte offset diffing, sqlite/rdb:
/// fingerprint + full reload), `TunnyApp` wraps them in this enum so it can hold either one.
enum ActivePoller {
    Journal(LiveUpdatePoller),
    Sqlite(SqliteLivePoller),
    Rdb(RdbLivePoller),
}

impl ActivePoller {
    fn stop(&mut self) {
        match self {
            ActivePoller::Journal(p) => p.stop(),
            ActivePoller::Sqlite(p) => p.stop(),
            ActivePoller::Rdb(p) => p.stop(),
        }
    }

    fn update_interval(&self, new_interval_ms: u64) {
        match self {
            ActivePoller::Journal(p) => p.update_interval(new_interval_ms),
            ActivePoller::Sqlite(p) => p.update_interval(new_interval_ms),
            ActivePoller::Rdb(p) => p.update_interval(new_interval_ms),
        }
    }
}

/// For each async-compute completion message, indicates which widget's completion state
/// should be propagated to each canvas item (an independent `WidgetStates`).
///
/// Compute is dispatched (`pending_compute`) from the item-specific `WidgetStates`, but
/// the completion message (`MessageHandler::handle`) only updates the global
/// `widget_states`. Without propagation, canvas items would be left with the `computing`
/// flag stuck on, so the spinner never stops and results never render (a regression
/// introduced by the per-item state split in commit 73883d8).
enum ComputeSyncKind {
    Cluster,
    Importance,
    Mcdm,
    Pdp,
    Pdp2d,
    ObservedContour,
    SurrogateFit,
    SurrogateOpt,
    SurrogateSuggest,
    RobustnessFit,
    ResponseSurfaceFit,
    Convergence,
    SensitivityHeatmap,
    SurrogateCompare,
}

impl ComputeSyncKind {
    fn from_message(msg: &AppMessage) -> Option<Self> {
        match msg {
            AppMessage::ClusteringDone { .. } | AppMessage::ClusterFailed { .. } => {
                Some(Self::Cluster)
            }
            AppMessage::SensitivityDone { .. }
            | AppMessage::SobolDone { .. }
            | AppMessage::SensitivityError(_) => Some(Self::Importance),
            AppMessage::McdmDone { .. }
            | AppMessage::McdmFailed { .. }
            | AppMessage::EntropyDone { .. } => Some(Self::Mcdm),
            AppMessage::PdpDone { .. } => Some(Self::Pdp),
            AppMessage::Pdp2dDone(_) => Some(Self::Pdp2d),
            AppMessage::ObservedContourDone(_) | AppMessage::ObservedContourFailed(_) => {
                Some(Self::ObservedContour)
            }
            AppMessage::SurrogateFitDone(_)
            | AppMessage::SurrogateFitFailed(_)
            | AppMessage::SurrogateFitCancelled
            | AppMessage::SurrogateMultiFitDone(_)
            | AppMessage::SurrogateMultiFitFailed(_)
            | AppMessage::SurrogateMultiFitCancelled => Some(Self::SurrogateFit),
            AppMessage::SurrogateOptDone(_)
            | AppMessage::SurrogateMultiOptDone(_)
            | AppMessage::SurrogateMultiOptFailed(_) => Some(Self::SurrogateOpt),
            AppMessage::SurrogateSuggestDone(_)
            | AppMessage::SurrogateSuggestFailed(_)
            | AppMessage::SurrogateMultiSuggestDone(_)
            | AppMessage::SurrogateMultiSuggestFailed(_) => Some(Self::SurrogateSuggest),
            AppMessage::RobustnessFitDone(_) | AppMessage::RobustnessFitFailed(_) => {
                Some(Self::RobustnessFit)
            }
            AppMessage::ResponseSurfaceFitDone(_) | AppMessage::ResponseSurfaceFitFailed(_) => {
                Some(Self::ResponseSurfaceFit)
            }
            AppMessage::SurrogateCompareDone(_) | AppMessage::SurrogateCompareFailed(_) => {
                Some(Self::SurrogateCompare)
            }
            AppMessage::IndicatorHistoryDone { .. } => Some(Self::Convergence),
            AppMessage::SensitivityHeatmapDone { .. } => Some(Self::SensitivityHeatmap),
            _ => None,
        }
    }

    /// Propagates the completion state from the global widget (the just-processed
    /// authoritative state) to every canvas item. Each `adopt_*` preserves the
    /// item-specific UI selections (parameters, objectives, etc.) and only pulls in
    /// the compute output and the running flag.
    fn propagate(self, global: &WidgetStates, canvas: &mut HashMap<u64, WidgetStates>) {
        for w in canvas.values_mut() {
            match self {
                Self::Cluster => {
                    w.cluster_scatter
                        .adopt_runtime_state(&global.cluster_scatter);
                    w.cluster_scatter_3d
                        .adopt_runtime_state(&global.cluster_scatter_3d);
                    w.trial_table
                        .cluster
                        .adopt_runtime_state(&global.trial_table.cluster);
                    w.artifact_gallery
                        .adopt_cluster_runtime(&global.artifact_gallery);
                }
                Self::Importance => w.importance.adopt_compute_state(&global.importance),
                Self::Mcdm => {
                    w.mcdm_chart.adopt_compute_state(&global.mcdm_chart);
                    w.scatter_chart.adopt_compute_state(&global.scatter_chart);
                    w.mcdm_scatter_3d
                        .adopt_compute_state(&global.mcdm_scatter_3d);
                    w.trial_table
                        .mcdm
                        .adopt_compute_state(&global.trial_table.mcdm);
                    w.artifact_gallery
                        .mcdm
                        .adopt_compute_state(&global.artifact_gallery.mcdm);
                }
                Self::Pdp => w.pdp_chart.adopt_compute_state(&global.pdp_chart),
                Self::Pdp2d => w.pdp_2d.adopt_compute_state(&global.pdp_2d),
                Self::ObservedContour => w
                    .observed_contour
                    .adopt_compute_state(&global.observed_contour),
                // Fit, optimize, and suggest all share the same surrogate_opt state.
                Self::SurrogateFit | Self::SurrogateOpt | Self::SurrogateSuggest => {
                    w.surrogate_opt.adopt_compute_state(&global.surrogate_opt)
                }
                Self::RobustnessFit => w.robustness.adopt_compute_state(&global.robustness),
                Self::ResponseSurfaceFit => w
                    .response_surface
                    .adopt_compute_state(&global.response_surface),
                Self::Convergence => w.convergence.adopt_compute_state(&global.convergence),
                Self::SensitivityHeatmap => w
                    .sensitivity_heatmap
                    .adopt_compute_state(&global.sensitivity_heatmap),
                Self::SurrogateCompare => w
                    .surrogate_compare
                    .adopt_compute_state(&global.surrogate_compare),
            }
        }
    }
}

pub struct TunnyApp {
    pub app_state: AppState,
    pub layout: LayoutState,
    pub widget_states: WidgetStates,
    /// Holds independent UI state per canvas view item (keyed by item.id), so placing the
    /// same widget more than once doesn't share its settings.
    pub canvas_widgets: HashMap<u64, WidgetStates>,
    pub is_loading: bool,
    pub load_error: Option<String>,
    tx: mpsc::SyncSender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
    poller: Option<ActivePoller>,
    /// Generation counter attached to live update poller startup prep (H-1/H-2).
    /// Incremented by 1 on every `restart_poller` call; the poller is only started when
    /// the ready message (`AppMessage::PollerReady`) matches the current generation.
    /// Used to discard stale prep results if the user toggles / switches Study / opens
    /// a different file while prep is still in flight.
    poller_generation: u64,
    /// The string currently set on the window title bar. Kept so an update command is only
    /// sent when it actually changes.
    current_window_title: Option<String>,
}

impl TunnyApp {
    /// eframe storage key for [`crate::state::app_state::GhComputePrefs`].
    const GH_COMPUTE_PREFS_KEY: &'static str = "gh_compute_prefs";

    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<std::path::PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(crate::theme::tunny_visuals(false));
        // Register a loader so the artifact gallery can display file:// images.
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Set up Inter + Noto Sans JP fonts (Japanese glyphs fall back to the JP font).
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/Inter-VariableFont.ttf")).into(),
        );
        fonts.font_data.insert(
            "NotoSansJP".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../assets/NotoSansJP-VariableFont_wght.ttf"
            ))
            .into(),
        );
        let proportional = fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap();
        proportional.insert(0, "Inter".to_owned());
        proportional.push("NotoSansJP".to_owned());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("Inter".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let (tx, rx) = mpsc::sync_channel(32);
        let is_loading = initial_path.is_some();
        if let Some(path) = initial_path {
            dispatch_scan(path, tx.clone());
        }
        let mut app_state = AppState::new();
        // Restore persisted preferences (currently the .ghx Compute/sampler
        // settings). Absent or unreadable storage falls back to defaults.
        if let Some(storage) = cc.storage {
            if let Some(prefs) = eframe::get_value::<crate::state::app_state::GhComputePrefs>(
                storage,
                Self::GH_COMPUTE_PREFS_KEY,
            ) {
                app_state.gh_compute_prefs = prefs;
            }
        }
        Self {
            app_state,
            layout: LayoutState::default(),
            widget_states: WidgetStates::default(),
            canvas_widgets: HashMap::new(),
            is_loading,
            load_error: None,
            tx,
            rx,
            poller: None,
            poller_generation: 0,
            current_window_title: None,
        }
    }

    /// Reflects the full path of the opened file in the window title bar.
    /// Shows "Tunny Dashboard (Beta)" when no file is loaded, and
    /// "Tunny Dashboard (Beta) - <full path>" when one is loaded.
    /// Since an RDB URL may contain a password, the actual computation is split out into
    /// `compute_window_title`, which shows the password masked via `RdbUrl::masked()`
    /// when the path is a URL.
    fn sync_window_title(&mut self, ctx: &egui::Context) {
        let title = Self::compute_window_title(self.app_state.journal_path.as_deref());
        if self.current_window_title.as_deref() != Some(title.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.current_window_title = Some(title);
        }
    }

    /// Pure function that computes the window title string (split out from
    /// `sync_window_title` so it's testable). If `journal_path` can be interpreted as an
    /// RDB connection URL, shows the password-masked `RdbUrl::masked()`; otherwise falls
    /// back to the usual `Path::display()`.
    fn compute_window_title(journal_path: Option<&std::path::Path>) -> String {
        const BASE_TITLE: &str = "Tunny Dashboard (Beta)";
        match journal_path {
            Some(path) => {
                let shown = match crate::io::rdb::path_as_rdb_url(path) {
                    Some(url) => url.masked(),
                    None => path.display().to_string(),
                };
                format!("{BASE_TITLE} - {shown}")
            }
            None => BASE_TITLE.to_owned(),
        }
    }

    /// Returns a Sender clone for launching background tasks.
    pub fn sender(&self) -> mpsc::SyncSender<AppMessage> {
        self.tx.clone()
    }

    /// Processes messages non-blockingly and updates AppState.
    pub fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            // H-1/H-2: poller prep completion needs tx/poller, so intercept it here
            // (rather than in the tx-less MessageHandler) and only start it when the
            // generation still matches.
            let msg = match msg {
                AppMessage::PollerReady { generation, prep } => {
                    self.start_prepared_poller(generation, prep);
                    ctx.request_repaint();
                    continue;
                }
                other => other,
            };
            let is_journal_parsed = matches!(&msg, AppMessage::JournalParsed { .. });
            let is_live_error = matches!(&msg, AppMessage::LiveUpdateError(_));
            // Both the .ghx and the process-integration runs use the same
            // `gh_opt_run` overlay state, so both trigger the post-run refresh.
            let is_gh_opt_finished = matches!(
                &msg,
                AppMessage::GhOptFinished { .. } | AppMessage::ProcessOptFinished { .. }
            );
            // Limit streaming-load batches to one per frame so each batch's DataFrame
            // rebuild cost isn't concentrated into a single frame (avoids render stalls).
            // Remaining batches stay in the channel and are processed on the next frame.
            let is_study_chunk = matches!(&msg, AppMessage::StudyChunkLoaded { .. });
            // SQLite live update: once a study becomes active (selection completed), if
            // live update is enabled, restart the poller so it tracks the new study_id
            // (unlike journal, the fingerprint can only be obtained per study).
            let is_study_activated = matches!(&msg, AppMessage::StudySelected { .. })
                || matches!(&msg, AppMessage::StudyChunkLoaded { is_final: true, .. });
            // SQLite live update: once a fingerprint change is detected, ask the worker
            // to reload. The actual re-parse needs tx, so dispatch it here (in app.rs,
            // which holds tx) rather than in the tx-less MessageHandler.
            let sqlite_reload_study_id = MessageHandler::sqlite_reload_study_id(&msg);
            // Async compute completion/failure messages only update the global
            // widget_states. Since each canvas item holds independent WidgetStates
            // (commit 73883d8), the completion state (computing/result/cache) must be
            // propagated to every item after processing. Decide which widget to
            // propagate to before msg is consumed.
            let sync = ComputeSyncKind::from_message(&msg);

            MessageHandler::handle(
                msg,
                &mut self.app_state,
                &mut self.widget_states,
                &mut self.is_loading,
                &mut self.load_error,
            );

            // Reflect the completion state into every canvas item (the global widget is
            // now the just-processed authoritative state).
            if let Some(sync) = sync {
                sync.propagate(&self.widget_states, &mut self.canvas_widgets);
            }

            if is_journal_parsed {
                // Flat CSV has no info on optimization direction / variable range, so
                // don't auto-activate — open the confirmation dialog instead. On
                // confirmation, dispatch select_study with meta reflecting the edited
                // values.
                let is_csv = self
                    .app_state
                    .journal_path
                    .as_deref()
                    .is_some_and(crate::io::flat_csv::is_csv_path);
                let is_sqlite = self
                    .app_state
                    .journal_path
                    .as_deref()
                    .is_some_and(crate::io::sqlite::is_sqlite_path);
                let is_rdb = self
                    .app_state
                    .journal_path
                    .as_deref()
                    .is_some_and(|p| crate::io::rdb::path_as_rdb_url(p).is_some());
                if is_csv {
                    // CSV is a flat import (one-time ingestion) with no concept of
                    // streaming appends, so force Live Update off and keep it out of
                    // scope.
                    self.app_state.live_update.enabled = false;
                    self.app_state.live_update.poller_active = false;
                } else {
                    self.app_state.live_update.storage_kind = if is_sqlite {
                        LiveUpdateStorageKind::Sqlite
                    } else if is_rdb {
                        LiveUpdateStorageKind::Rdb
                    } else {
                        LiveUpdateStorageKind::Journal
                    };
                    if is_sqlite || is_rdb {
                        // SQLite/RDB fingerprinting needs study_id, so don't start the
                        // poller before a Study is selected (it starts via
                        // is_study_activated once Study selection completes).
                        self.app_state.live_update.poller_active = false;
                    } else if self.app_state.live_update.enabled {
                        self.restart_poller();
                    }
                }
                if is_csv {
                    if let Some(meta) = self.app_state.all_studies.first() {
                        self.app_state.csv_import_settings =
                            Some(crate::state::app_state::CsvImportSettings::from_meta(meta));
                    }
                } else if self.app_state.all_studies.len() == 1 {
                    // If there's only one Study, automatically start Phase 2.
                    self.is_loading = true;
                    let meta = self.app_state.all_studies[0].clone();
                    crate::io::study_worker::dispatch_select_study(meta, self.sender());
                }
            }
            if is_live_error {
                // poller stopped itself — drop the handle
                self.poller = None;
                // Invalidate any pending prep task so an error doesn't cause it to
                // restart the poller on its own.
                self.invalidate_pending_poller();
            }

            if is_gh_opt_finished {
                self.refresh_after_gh_opt();
            }

            if let Some(study_id) = sqlite_reload_study_id {
                // SqliteLiveChanged is a signal message reused by both SQLite and RDB
                // live update, so dispatch the actual reload based on the current
                // storage_kind.
                if self.app_state.live_update.storage_kind == LiveUpdateStorageKind::Rdb {
                    crate::io::study_worker::dispatch_reload_rdb_study(study_id, self.sender());
                } else {
                    crate::io::study_worker::dispatch_reload_sqlite_study(study_id, self.sender());
                }
            }

            // SQLite/RDB live update can only get a fingerprint per study, so when the
            // displayed study switches, restart the poller with the new study_id
            // (journal tracks the whole file, so no restart is needed on study switch).
            if is_study_activated
                && self.app_state.live_update.enabled
                && matches!(
                    self.app_state.live_update.storage_kind,
                    LiveUpdateStorageKind::Sqlite | LiveUpdateStorageKind::Rdb
                )
            {
                self.restart_poller();
            }

            ctx.request_repaint();

            if is_study_chunk {
                break;
            }
        }

        // Keep repainting during streaming load even without input, to keep pulling in
        // the next batch (sends to the bounded channel naturally block until UI
        // rendering catches up).
        if self.is_loading {
            ctx.request_repaint();
        }
    }

    pub fn apply_toolbar_actions(&mut self, actions: Vec<ToolbarAction>) {
        for action in actions {
            match action {
                ToolbarAction::OpenJournal(path) => self.open_path(path),
                ToolbarAction::OpenDbUrlDialog => {
                    self.app_state.db_url_dialog = Some(String::new());
                }
                ToolbarAction::SelectStudy(meta) => {
                    self.is_loading = true;
                    crate::io::study_worker::dispatch_select_study(meta, self.sender());
                }
                ToolbarAction::ToggleLiveUpdate => {
                    self.app_state.live_update.enabled = !self.app_state.live_update.enabled;
                    if self.app_state.live_update.enabled {
                        self.restart_poller();
                    } else {
                        if let Some(mut p) = self.poller.take() {
                            p.stop();
                        }
                        // Invalidate any pending prep task (H-1/H-2).
                        self.invalidate_pending_poller();
                        self.app_state.live_update.poller_active = false;
                    }
                }
                ToolbarAction::SetPollInterval(ms) => {
                    self.app_state.live_update.interval_ms = ms;
                    if let Some(ref poller) = self.poller {
                        poller.update_interval(ms);
                    }
                }
                ToolbarAction::ScanArtifacts(base_dir) => {
                    crate::io::artifacts::scan_artifacts_dir(
                        base_dir,
                        self.app_state.journal_path.clone(),
                        self.sender(),
                    );
                }
                ToolbarAction::ClearLoadError => {
                    self.load_error = None;
                }
                ToolbarAction::ExportCsv(target) => {
                    if let Some(ctx) = &self.app_state.current_study {
                        // The save dialog (rfd) runs first on the UI thread to pin down
                        // the path.
                        if let Some(path) = crate::io::export::pick_csv_save_path("export.csv") {
                            // Resolve the row selection on the UI thread only, then clone
                            // the StudyView snapshot and column names to hand off to the
                            // worker (CSV building + writing happens in the background,
                            // so the UI doesn't freeze even for huge Studies).
                            let row_indices = crate::io::export::select_row_indices_for_export(
                                &ctx.view,
                                &self.app_state.selected_indices,
                                &ctx.pareto_indices,
                                &target,
                            );
                            // Don't swallow save failures — reflect them into load_error
                            // (via CsvExportFailed).
                            crate::io::export::spawn_view_csv_export(
                                ctx.view.clone(),
                                row_indices,
                                ctx.meta.param_names.clone(),
                                ctx.meta.objective_names.clone(),
                                path,
                                self.sender(),
                            );
                        }
                    }
                }
                ToolbarAction::AddComparisonStudy(meta) => {
                    // Add another Study from the same file as a comparison target.
                    // Ignore it if already added, or if it's the base Study itself.
                    let base_id = self
                        .app_state
                        .current_study
                        .as_ref()
                        .map(|c| c.meta.study_id);
                    let already = self
                        .app_state
                        .comparison_studies
                        .iter()
                        .any(|s| s.meta.study_id == meta.study_id);
                    if base_id != Some(meta.study_id) && !already {
                        self.app_state.comparison_mode = true;
                        crate::io::study_worker::dispatch_load_comparison_study(
                            meta,
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
                        if idx < self.app_state.comparison_convergence_histories.len() {
                            self.app_state.comparison_convergence_histories.remove(idx);
                        }
                    }
                }
                ToolbarAction::SaveSession => {
                    let view = crate::io::session::ViewSettings::capture(&self.app_state);
                    if let Err(e) = crate::io::session::save_session_dialog(
                        &self.layout,
                        &self.canvas_widgets,
                        &view,
                    ) {
                        self.load_error = Some(e);
                    }
                }
                ToolbarAction::LoadSession(path) => {
                    match crate::io::session::read_session_from_path(&path) {
                        Ok(session) => {
                            // The data (study / comparison session) stays as-is. Only
                            // swap the layout and settings; compute results get
                            // recomputed on the next frame by each widget's polling,
                            // based on the restored settings.
                            self.layout = session.layout;
                            self.canvas_widgets = session.widgets;
                            session.view.apply(&mut self.app_state);
                        }
                        Err(e) => self.load_error = Some(e),
                    }
                }
                ToolbarAction::OpenReportDialog => {
                    self.app_state.report_dialog =
                        Some(crate::ui::widgets::report_modal::ReportDialogState::default());
                }
                ToolbarAction::OpenProcessDefinition(path) => {
                    self.open_process_definition(path);
                }
                ToolbarAction::NewProcessDefinition => {
                    self.app_state.process_def_builder =
                        Some(crate::state::app_state::ProcessDefBuilderState::new());
                }
            }
        }
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
    /// Persists user preferences (called periodically and on shutdown by eframe).
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            Self::GH_COMPUTE_PREFS_KEY,
            &self.app_state.gh_compute_prefs,
        );
    }

    /// Keep egui's own memory (window positions, collapsing states) out of the
    /// storage — only explicitly chosen preferences are persisted.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    // logic() handles the non-rendering phase (message pump, state updates, screenshot
    // capture). In egui 0.35, eframe::App's update() was split into ui()/logic().
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Theme sync: regardless of what changed dark_mode (toolbar toggle, session
        // restore), keep the global flag and Visuals following it consistently from
        // here.
        if self.app_state.dark_mode != crate::theme::is_dark_mode() {
            ctx.set_visuals(crate::theme::tunny_visuals(self.app_state.dark_mode));
        }

        self.poll_messages(ctx);
        self.sync_window_title(ctx);
        self.handle_dropped_files(ctx);

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
                use crate::ui::widget_states::CaptureDest;
                cap.screenshot_requested = false;
                cap.pending_capture = None;
                cap.pending_capture_rect = None;
                let dest = cap.pending_capture_dest;

                let result = (|| -> Result<Option<()>, String> {
                    let rect = crop_rect.ok_or_else(|| "No capture rect".to_string())?;
                    let cropped = crate::io::chart_capture::crop_image(&image, rect, scale)
                        .ok_or_else(|| "Crop rect outside image bounds".to_string())?;
                    match dest {
                        CaptureDest::File => {
                            let png_bytes = crate::io::chart_capture::encode_png(cropped)?;
                            crate::io::chart_capture::save_png_to_file(&png_bytes)
                        }
                        CaptureDest::Clipboard => {
                            crate::io::chart_capture::copy_image_to_clipboard(cropped)?;
                            Ok(Some(()))
                        }
                    }
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

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        crate::ui::layout::show_layout(self, ui);
        self.show_csv_import_dialog(&ctx);
        self.show_db_url_dialog(&ctx);
        self.show_report_dialog(&ctx);
        self.show_ghx_opt_dialog(&ctx);
        self.show_process_def_builder(&ctx);
        self.show_process_opt_dialog(&ctx);
        self.show_ghx_opt_overlay(&ctx);
        self.show_drop_hover_overlay(&ctx);
        crate::ui::widgets::license_modal::show(&ctx, &mut self.widget_states.license_modal);
    }
}

/// Determines the kind of path being opened (RDB URL / flat CSV / SQLite / journal) and
/// dispatches the corresponding scan to a worker thread. Shared handling (D-12) between
/// `TunnyApp::new` (initial path) and `open_path` (toolbar / URL dialog).
fn dispatch_scan(path: std::path::PathBuf, tx: mpsc::SyncSender<AppMessage>) {
    if let Some(url) = crate::io::rdb::path_as_rdb_url(&path) {
        crate::io::study_worker::dispatch_scan_rdb(url, tx);
    } else if crate::io::flat_csv::is_csv_path(&path) {
        crate::io::study_worker::dispatch_scan_csv(path, tx);
    } else if crate::io::sqlite::is_sqlite_path(&path) {
        crate::io::study_worker::dispatch_scan_sqlite(path, tx);
    } else {
        crate::io::study_worker::dispatch_scan_journal(path, tx);
    }
}

/// Background task launch helper.
///
/// Catches worker panics with `catch_unwind` and reports them to the UI as
/// `AppMessage::TaskPanicked` (M-4). Without catching them, a panic would mean the
/// completion message never arrives, leaving the relevant widget's `computing`/`fitting`
/// flag stuck on and the spinner spinning forever.
/// Builds the error message shown when a drop contained no openable file.
/// Special-cases binary `.gh`, the most likely mistake for the .ghx flow,
/// with instructions on how to re-save the definition as `.ghx`.
fn unsupported_drop_message(paths: &[std::path::PathBuf]) -> String {
    let names: Vec<String> = paths
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let names = if names.is_empty() {
        "(unknown)".to_string()
    } else {
        names.join(", ")
    };
    if paths.iter().any(|p| crate::io::file::is_gh_binary_path(p)) {
        format!(
            "{names}: .gh is a binary Grasshopper file and cannot be read directly. \
             In Grasshopper, use File > Save As and choose the \"Grasshopper XML (*.ghx)\" \
             file type, then drop the .ghx here."
        )
    } else {
        format!(
            "{names}: unsupported file type. Supported: .log / .journal (Optuna journal), \
             .db / .sqlite / .sqlite3 (Optuna SQLite), .csv (DesignExplorer), \
             .ghx (Grasshopper XML)."
        )
    }
}

pub fn spawn_task<F>(tx: mpsc::SyncSender<AppMessage>, f: F)
where
    F: FnOnce() -> AppMessage + Send + 'static,
{
    std::thread::spawn(move || {
        let msg = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(msg) => msg,
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_string());
                AppMessage::TaskPanicked(detail)
            }
        };
        let _ = tx.send(msg);
    });
}
