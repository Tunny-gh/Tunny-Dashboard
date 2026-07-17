use std::collections::HashMap;
use std::sync::mpsc;

use crate::io::live_update_poller::{
    LiveUpdatePoller, RdbLivePoller, RdbLiveUpdateContext, SqliteLivePoller,
    SqliteLiveUpdateContext,
};
use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;
use crate::state::message_handler::MessageHandler;
use crate::state::messages::{AppMessage, PollerPrep};
use crate::state::results::LiveUpdateStorageKind;
use crate::ui::toolbar::ToolbarAction;
use crate::ui::widget_states::WidgetStates;
use tunny_core::io::journal::live_update::LiveUpdateContext;

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
        Self {
            app_state: AppState::new(),
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
            }
        }
    }

    /// Renders the "Report…" modal, and on Export confirmation collects a snapshot of
    /// the study and delegates report generation to a background thread (called every
    /// frame from here after `ToolbarAction::OpenReportDialog` starts
    /// `app_state.report_dialog`).
    fn show_report_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::report_modal::{self, ReportModalAction};

        let Some(mut dialog) = self.app_state.report_dialog.take() else {
            return;
        };
        let study_name = self
            .app_state
            .current_study
            .as_ref()
            .map(|s| s.meta.name.clone());

        let action = report_modal::show(ctx, &mut dialog, study_name.as_deref());
        let can_start_export = !dialog.generating && dialog.success_paths.is_none();

        match action {
            Some(ReportModalAction::Close) => {
                // OK to close without waiting even while generating (the background job
                // continues fire-and-forget; without the dialog, completion/failure just
                // won't be reported).
            }
            Some(ReportModalAction::Export) if can_start_export => {
                match dialog.selected_formats() {
                    Err(e) => dialog.error = Some(e.to_string()),
                    Ok(formats) => {
                        dialog.error = None;
                        let default_name = report_modal::default_file_name_for(
                            study_name.as_deref().unwrap_or("study"),
                            &formats,
                        );
                        let chosen = rfd::FileDialog::new()
                            .set_file_name(&default_name)
                            .add_filter("Report", &["html", "md", "json"])
                            .save_file();
                        if let Some(base_path) = chosen {
                            if let Some(ctx_study) = &self.app_state.current_study {
                                let meta = ctx_study.meta.clone();
                                let df = ctx_study.view.df.clone();
                                let extras = tunny_core::dataframe::active_extras_snapshot();
                                let storage_display = crate::io::report_export::storage_display(
                                    self.app_state.journal_path.as_deref(),
                                );
                                dialog.generating = true;
                                crate::io::report_export::spawn_report_export(
                                    meta,
                                    df,
                                    extras,
                                    storage_display,
                                    dialog.lang,
                                    dialog.top_n,
                                    formats,
                                    base_path,
                                    self.sender(),
                                );
                            }
                        }
                    }
                }
                self.app_state.report_dialog = Some(dialog);
            }
            _ => {
                self.app_state.report_dialog = Some(dialog);
            }
        }
    }

    /// Opens the given path (journal / CSV / SQLite / RDB URL — any of them).
    /// Shared handling called both from `ToolbarAction::OpenJournal` and the Open button
    /// of the "Open URL…" dialog (a URL is passed as
    /// `PathBuf::from(normalized url string)`).
    fn open_path(&mut self, path: std::path::PathBuf) {
        // .ghx is separate from the existing journal/CSV/SQLite/RDB scan path (it's an
        // optimization problem definition, not a result store). Route it through the
        // same handling as D&D (`handle_dropped_files`), and open the optimization setup
        // modal once extraction succeeds.
        if crate::io::file::is_ghx_path(&path) {
            self.open_ghx_path(path);
            return;
        }
        // Stop existing poller before loading new file
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }
        self.is_loading = true;
        self.load_error = None;
        self.app_state.all_studies.clear();
        self.app_state.current_study = None;
        // Opening a different file (a different URL) changes the study_id space, so
        // discard any comparison session that assumed the same file.
        self.app_state.reset_comparison_session();
        // Invalidate the pending poller prep task before opening a different file
        // (H-1/H-2).
        self.invalidate_pending_poller();
        dispatch_scan(path, self.sender());
    }

    /// Renders the "Open URL…" dialog and, on Open confirmation, feeds the normalized
    /// URL string into `open_path` (the same path as `ToolbarAction::OpenJournal`).
    fn show_db_url_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::rdb_url_modal::{self, RdbUrlDialogAction};

        let Some(mut input) = self.app_state.db_url_dialog.take() else {
            return;
        };
        match rdb_url_modal::show(ctx, &mut input) {
            Some(RdbUrlDialogAction::Open(normalized_url)) => {
                self.open_path(std::path::PathBuf::from(normalized_url));
            }
            Some(RdbUrlDialogAction::Cancel) => {
                // Drop input to close the dialog.
            }
            None => {
                // Not confirmed yet. Keep showing it on the next frame.
                self.app_state.db_url_dialog = Some(input);
            }
        }
    }

    /// Renders the CSV import confirmation dialog and, on confirmation, applies the
    /// edited values to the Study and activates it.
    fn show_csv_import_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::csv_import_modal::{self, CsvImportAction};

        let Some(mut settings) = self.app_state.csv_import_settings.take() else {
            return;
        };
        match csv_import_modal::show(ctx, &mut settings) {
            Some(CsvImportAction::Apply) => {
                // Apply the edited values to the all_studies entry before dispatching
                // select_study.
                if let Some(slot) = self
                    .app_state
                    .all_studies
                    .iter_mut()
                    .find(|s| s.study_id == settings.study_id)
                {
                    settings.apply_to(slot);
                }
                if let Some(meta) = self
                    .app_state
                    .all_studies
                    .iter()
                    .find(|s| s.study_id == settings.study_id)
                    .cloned()
                {
                    self.is_loading = true;
                    crate::io::study_worker::dispatch_select_study(meta, self.tx.clone());
                }
                // Drop settings to close the dialog.
            }
            None => {
                // Not confirmed yet. Keep showing it on the next frame.
                self.app_state.csv_import_settings = Some(settings);
            }
        }
    }

    // ── File D&D (.ghx -> optimization setup modal, storage -> open) ────

    /// Accepts drag & drop of files. Works on every screen, including the
    /// startup guidance screen (drops are read from the raw input every frame).
    ///
    /// - A `.ghx` file opens the Grasshopper optimization setup modal
    ///   (if several files are dropped, the first `.ghx` wins).
    /// - Otherwise, the first recognized result storage file
    ///   (journal / SQLite / CSV) is routed to the normal open flow.
    /// - Anything else surfaces an error explaining the supported types
    ///   (in particular, binary `.gh` must be re-saved as `.ghx`) — a silent
    ///   no-op here would look like the drop simply didn't work.
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let paths: Vec<std::path::PathBuf> = dropped.into_iter().filter_map(|f| f.path).collect();
        if let Some(path) = paths.iter().find(|p| crate::io::file::is_ghx_path(p)) {
            self.open_ghx_path(path.clone());
            return;
        }
        if let Some(path) = paths.iter().find(|p| crate::io::file::is_storage_path(p)) {
            self.open_path(path.clone());
            return;
        }
        self.load_error = Some(unsupported_drop_message(&paths));
    }

    /// While files are being dragged over the window, dims the screen and shows
    /// what will happen on drop (Grasshopper optimization for .ghx, normal open
    /// for storage files, or an unsupported-type notice). This makes the
    /// always-available drop target visible.
    fn show_drop_hover_overlay(&self, ctx: &egui::Context) {
        let hovered: Vec<_> = ctx.input(|i| i.raw.hovered_files.clone());
        if hovered.is_empty() {
            return;
        }
        let paths: Vec<std::path::PathBuf> = hovered.into_iter().filter_map(|f| f.path).collect();
        let text = if paths.iter().any(|p| crate::io::file::is_ghx_path(p)) {
            "Drop to set up Grasshopper optimization"
        } else if paths.iter().any(|p| crate::io::file::is_storage_path(p)) {
            "Drop to open"
        } else if paths.iter().any(|p| crate::io::file::is_gh_binary_path(p)) {
            ".gh is not supported — in Grasshopper, save as .ghx (Grasshopper XML) and drop that"
        } else if paths.is_empty() {
            // Some platforms don't expose the path while hovering.
            "Drop files to open"
        } else {
            "Unsupported file type (.log / .db / .sqlite / .csv / .ghx are supported)"
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("file_drop_overlay"),
        ));
        let rect = ctx.content_rect();
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(120));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
        );
    }

    /// Loads a .ghx, and if problem extraction (synchronous, fast) succeeds, opens the
    /// optimization setup modal. Shared handling called both from D&D
    /// (`handle_dropped_files`) and the .ghx path in `open_path`.
    fn open_ghx_path(&mut self, path: std::path::PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => match tunny_core::gh::extract_problem(&text) {
                Ok(problem) => {
                    self.app_state.gh_opt_dialog = Some(
                        crate::state::app_state::GhOptDialogState::new(path, text, problem),
                    );
                }
                Err(e) => self.load_error = Some(e),
            },
            Err(e) => self.load_error = Some(format!("{}: {e}", path.display())),
        }
    }

    /// Renders the .ghx optimization setup modal. On Run confirmation, wires into
    /// `start_ghx_run`; setup errors (failures from `build_compute_definition` /
    /// `prepare_gh_run`) are sent back to the dialog, which stays open.
    fn show_ghx_opt_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::ghx_opt_modal::{self, GhxOptAction};

        let Some(mut dialog) = self.app_state.gh_opt_dialog.take() else {
            return;
        };
        match ghx_opt_modal::show(ctx, &mut dialog) {
            Some(GhxOptAction::Run) => {
                self.start_ghx_run(dialog);
                // If start_ghx_run fails due to a setup error, it puts the dialog back
                // into gh_opt_dialog itself.
            }
            Some(GhxOptAction::Cancel) => {
                // Drop dialog to close it.
            }
            None => {
                // Not confirmed yet. Keep showing it on the next frame.
                self.app_state.gh_opt_dialog = Some(dialog);
            }
        }
    }

    /// Upper bound in seconds to wait for rhino.compute to start locally. The first
    /// startup can take tens of seconds while Rhino loads, so give it some margin.
    const COMPUTE_STARTUP_TIMEOUT_SECS: u64 = 180;

    /// On Run confirmation, assembles the Rhino.Compute evaluator, journal, and progress
    /// handle, and starts the optimization loop (`run_prepared`) on a background thread.
    /// If `build_compute_definition` / `prepare_gh_run` fails, the error is put in
    /// `dialog.error` and sent back to `gh_opt_dialog`, keeping the modal open.
    ///
    /// The Compute target is either a URL (an existing server) or a rhino.compute EXE
    /// path. For the EXE case, starting the process takes time, so process launch and
    /// waiting also happen on the background task side (the progress overlay shows
    /// "Starting…").
    fn start_ghx_run(&mut self, mut dialog: crate::state::app_state::GhOptDialogState) {
        use tunny_core::gh::{
            build_compute_definition, classify_compute_input, prepare_gh_run, run_prepared,
            start_compute_server_tracked, ComputeConfig, ComputeEvaluator, ComputeTarget,
            GhRunConfig, GhSampler,
        };
        use tunny_core::io::journal::parser::OptimizationDirection;
        use tunny_core::surrogate_opt::FitProgress;

        let directions: Vec<OptimizationDirection> = dialog
            .maximize
            .iter()
            .map(|&is_max| {
                if is_max {
                    OptimizationDirection::Maximize
                } else {
                    OptimizationDirection::Minimize
                }
            })
            .collect();
        let run_cfg = GhRunConfig {
            study_name: dialog.study_name.clone(),
            directions,
            sampler: if dialog.sampler_is_random {
                GhSampler::Random
            } else {
                GhSampler::Nsga2
            },
            n_trials: dialog.n_trials,
            population_size: dialog.population_size,
            generations: dialog.generations,
            seed: dialog.seed,
        };
        // In EXE mode the path comes from the dedicated field; in URL mode a
        // pasted EXE path is still tolerated via classification.
        let target = if dialog.compute_use_exe {
            ComputeTarget::Exe(std::path::PathBuf::from(dialog.compute_exe_path.trim()))
        } else {
            classify_compute_input(&dialog.compute_url)
        };
        let compute_port = dialog.compute_port;
        let api_key = if dialog.api_key.trim().is_empty() {
            None
        } else {
            Some(dialog.api_key.clone())
        };
        let max_parallel = dialog.max_parallel;

        let def = match build_compute_definition(&dialog.ghx_text, &dialog.problem) {
            Ok(def) => def,
            Err(e) => {
                dialog.error = Some(e);
                self.app_state.gh_opt_dialog = Some(dialog);
                return;
            }
        };

        let journal_path = std::path::PathBuf::from(&dialog.journal_path);
        // Persist the injected definition next to the journal (best-effort).
        // It can be opened in Grasshopper to inspect the exact definition sent
        // to Compute, and in EXE mode its absolute path doubles as the request
        // `pointer` (compute then loads and caches the definition from the file
        // instead of re-parsing the base64 payload on every request).
        let compute_ghx_path = journal_path.with_extension("compute.ghx");
        let compute_ghx_abs =
            std::path::absolute(&compute_ghx_path).unwrap_or_else(|_| compute_ghx_path.clone());
        let definition_pointer = match std::fs::write(&compute_ghx_abs, &def.ghx) {
            Ok(()) => Some(compute_ghx_abs.to_string_lossy().into_owned()),
            Err(_) => None,
        };
        let prep = match prepare_gh_run(&journal_path, &dialog.problem, &run_cfg) {
            Ok(prep) => prep,
            Err(e) => {
                dialog.error = Some(e);
                self.app_state.gh_opt_dialog = Some(dialog);
                return;
            }
        };

        let progress = FitProgress::new();
        self.app_state.gh_opt_run = Some(crate::state::app_state::GhOptRunState {
            progress: progress.clone(),
            journal_path: journal_path.clone(),
            study_name: dialog.study_name.clone(),
            finished: None,
        });
        // The study is already created in the journal, so opening it now will show it in
        // the study list. Start the poller so subsequent trials stream into the live
        // view.
        self.app_state.live_update.enabled = true;

        let problem = dialog.problem.clone();
        spawn_task(self.sender(), move || {
            let result = (|| {
                // If an EXE was specified, start the process here to obtain the URL.
                // Keep the handle in scope until the optimization loop finishes; it
                // stops on Drop.
                let _server;
                // The definition-file pointer is only valid when compute runs on
                // this machine, which the EXE launch mode guarantees.
                let mut use_pointer = false;
                let server_url = match target {
                    ComputeTarget::Url(url) => url,
                    ComputeTarget::Exe(path) => {
                        let handle = start_compute_server_tracked(
                            &path,
                            compute_port,
                            Self::COMPUTE_STARTUP_TIMEOUT_SECS,
                            &progress,
                        )?;
                        let url = handle.url().to_string();
                        _server = handle;
                        use_pointer = true;
                        url
                    }
                };
                let compute_cfg = ComputeConfig {
                    server_url,
                    api_key,
                    max_parallel,
                    ..ComputeConfig::default()
                };
                let mut evaluator = ComputeEvaluator::new(&compute_cfg, &def);
                if use_pointer {
                    if let Some(pointer) = definition_pointer {
                        evaluator = evaluator.with_definition_pointer(pointer);
                    }
                }
                run_prepared(&prep, &problem, &evaluator, &run_cfg, &progress)
            })();
            AppMessage::GhOptFinished { result }
        });

        // The study is already written to the journal, so opening it shows it in the
        // study list (if there's only one, poll_messages auto-selects it and live update
        // streams the trials in).
        self.open_path(journal_path);
        // Drop dialog to close the modal (don't put it back as None).
    }

    /// Displays a running (or just-finished) .ghx optimization in a non-modal progress
    /// overlay. Shows a progress bar + Cancel while running, and a result message +
    /// Close once finished.
    fn show_ghx_opt_overlay(&mut self, ctx: &egui::Context) {
        let Some(run) = self.app_state.gh_opt_run.as_ref() else {
            return;
        };

        let mut cancel_clicked = false;
        let mut close_clicked = false;

        egui::Window::new("Grasshopper Optimization")
            .id(egui::Id::new("ghx_opt_progress_window"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .show(ctx, |ui| {
                ui.set_min_width(260.0);
                match &run.finished {
                    None => {
                        // Request a repaint at a fixed interval to smoothly update the
                        // progress.
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(250));
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new(&run.study_name).strong());
                        });
                        let snapshot = run.progress.snapshot();
                        if snapshot.total > 0 {
                            let frac =
                                (snapshot.done as f32 / snapshot.total as f32).clamp(0.0, 1.0);
                            ui.add(
                                egui::ProgressBar::new(frac)
                                    .show_percentage()
                                    .desired_width(240.0),
                            );
                        }
                        if !snapshot.stage.is_empty() {
                            ui.label(
                                egui::RichText::new(&snapshot.stage)
                                    .color(crate::theme::TEXT_SECONDARY()),
                            );
                        }
                        let cancelling = run.progress.is_cancelled();
                        let label = if cancelling {
                            "Cancelling…"
                        } else {
                            "Cancel"
                        };
                        if ui
                            .add_enabled(!cancelling, egui::Button::new(label))
                            .clicked()
                        {
                            cancel_clicked = true;
                        }
                    }
                    Some(result) => {
                        match result {
                            Ok(msg) => {
                                ui.label(msg);
                            }
                            Err(err) => {
                                ui.colored_label(crate::theme::ERROR_COLOR(), err);
                            }
                        }
                        if ui.button("Close").clicked() {
                            close_clicked = true;
                        }
                    }
                }
            });

        if cancel_clicked {
            run.progress.request_cancel();
        }
        if close_clicked {
            self.app_state.gh_opt_run = None;
        }
    }

    /// Invalidates the pending (starting) poller (H-1/H-2).
    /// Advancing the generation causes the `AppMessage::PollerReady` sent by an in-flight
    /// prep task to be discarded on receipt. Call on toggle-off, opening a different
    /// file, or a live error.
    fn invalidate_pending_poller(&mut self) {
        self.poller_generation = self.poller_generation.wrapping_add(1);
    }

    /// (Re)starts the poller for the current file.
    ///
    /// Obtaining the fingerprint (DB connection + query) or reading the whole journal
    /// plus counting trials involves I/O that would freeze the UI thread (H-1/H-2), so
    /// this only spawns a prep task in the background. Once prep completes,
    /// `AppMessage::PollerReady` arrives and `start_prepared_poller` actually starts the
    /// poller. What prep does differs by storage kind (journal / sqlite / rdb).
    fn restart_poller(&mut self) {
        // Stop any existing poller
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }

        // Take it out by value (since invalidate_pending_poller below takes &mut self, we
        // can't hold a borrow of self.app_state across that call).
        let Some(file_path) = self.app_state.journal_path.clone() else {
            return;
        };

        // Advance the generation and assign the now-current generation only to the prep
        // task spawned by this call. If restart_poller is called again later due to a
        // toggle/Study change, the generation advances further and this task's result
        // gets discarded in start_prepared_poller.
        self.invalidate_pending_poller();
        let generation = self.poller_generation;
        let tx = self.tx.clone();

        match self.app_state.live_update.storage_kind {
            LiveUpdateStorageKind::Sqlite => {
                // SQLite fingerprints can only be obtained per study, so start nothing
                // if there's no active Study (it gets called again via
                // is_study_activated once Study selection completes).
                let Some(study_id) = self
                    .app_state
                    .current_study
                    .as_ref()
                    .map(|s| s.meta.study_id)
                else {
                    return;
                };
                let file_path = file_path.clone();
                spawn_task(tx, move || {
                    // Even if the initial fingerprint fetch fails (e.g. a read
                    // conflict), start with a default value. If it later mismatches the
                    // real value on the next poll, it just causes one extra reload —
                    // fails safe.
                    let initial_fingerprint =
                        tunny_core::sqlite::study_fingerprint(&file_path, study_id)
                            .unwrap_or_default();
                    let ctx = SqliteLiveUpdateContext {
                        file_path,
                        study_id,
                        initial_fingerprint,
                        no_change_timeout_ms: LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS,
                    };
                    AppMessage::PollerReady {
                        generation,
                        prep: PollerPrep::Sqlite(ctx),
                    }
                });
            }
            LiveUpdateStorageKind::Rdb => {
                // RDB fingerprints can also only be obtained per study, so like SQLite,
                // start nothing if there's no active Study.
                let Some(study_id) = self
                    .app_state
                    .current_study
                    .as_ref()
                    .map(|s| s.meta.study_id)
                else {
                    return;
                };
                // journal_path holds the URL string directly (Phase C design). This
                // should normally always be Some here; if it unexpectedly isn't, do
                // nothing as a safe fallback.
                let Some(url) = crate::io::rdb::path_as_rdb_url(&file_path) else {
                    return;
                };
                spawn_task(tx, move || {
                    // The DB connection + query happens here (in the background). Even
                    // if it's slow or unreachable, the UI thread isn't blocked (H-1). On
                    // fetch failure, start with a default value (same fallback policy as
                    // SQLite).
                    let initial_fingerprint =
                        tunny_core::rdb::study_fingerprint_url(&url, study_id).unwrap_or_default();
                    let ctx = RdbLiveUpdateContext {
                        url,
                        study_id,
                        initial_fingerprint,
                        no_change_timeout_ms: LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS,
                    };
                    AppMessage::PollerReady {
                        generation,
                        prep: PollerPrep::Rdb(ctx),
                    }
                });
            }
            LiveUpdateStorageKind::Journal => {
                let file_path = file_path.clone();
                spawn_task(tx, move || {
                    // Optuna assigns trial_id sequentially, in op_code=4 appearance
                    // order, across all studies and states. The live update diff parser
                    // assigns this global trial_id to the next Trial it creates, and
                    // matches subsequent op_code=5/6 records by trial_id. So the
                    // starting next_trial_id must equal "the total count of op_code=4
                    // records in the file." meta doesn't hold the overall total (Phase1
                    // has total_trials=0, and so do non-selected studies), so read the
                    // file once and count. Also grab byte_offset from the same byte
                    // buffer to avoid a race with metadata fetching (appends happening
                    // during the read). Count the per-study creation counts from the same
                    // buffer too, to seed each Study's next trial.number (so Trials
                    // created during live update get consecutive numbers within their
                    // Study). Reading and counting the whole hundred-MB-scale journal
                    // also happens here (in the background) (H-2).
                    let (byte_offset, next_trial_id, study_trial_number_seeds) =
                        match std::fs::read(&file_path) {
                            Ok(bytes) => {
                                let per_study =
                            tunny_core::io::journal::live_update::count_created_trials_per_study(
                                &bytes,
                            );
                                (
                                    bytes.len() as u64,
                                    tunny_core::io::journal::live_update::count_created_trials(
                                        &bytes,
                                    ),
                                    per_study,
                                )
                            }
                            Err(_) => (
                                std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0),
                                0,
                                std::collections::HashMap::new(),
                            ),
                        };

                    let ctx = LiveUpdateContext {
                        file_path,
                        initial_byte_offset: byte_offset,
                        next_trial_id,
                        study_trial_number_seeds,
                        study_distributions: vec![],
                        no_change_timeout_ms: LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS,
                    };
                    AppMessage::PollerReady {
                        generation,
                        prep: PollerPrep::Journal(ctx),
                    }
                });
            }
        }

        // Treat this as "starting" as soon as the prep task is spawned (for UI display).
        // The actual poller start happens in start_prepared_poller.
        self.app_state.live_update.poller_active = true;
    }

    /// Receives the `PollerReady` that arrives once the background prep task (H-1/H-2)
    /// completes, and starts the poller if the generation is still current (i.e. no
    /// toggle/Study change happened while it was preparing).
    fn start_prepared_poller(&mut self, generation: u64, prep: PollerPrep) {
        // Discard it if the generation has advanced due to a toggle/Study
        // change/opening a different file while preparing.
        if generation != self.poller_generation {
            return;
        }
        // restart_poller normally already stopped it, but stop any existing poller just
        // in case.
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }
        // The interval may have changed while preparing, so use its latest value at
        // startup time.
        let interval_ms = self.app_state.live_update.interval_ms;
        let tx = self.tx.clone();
        let poller = match prep {
            PollerPrep::Journal(ctx) => {
                ActivePoller::Journal(LiveUpdatePoller::start(ctx, tx, interval_ms))
            }
            PollerPrep::Sqlite(ctx) => {
                ActivePoller::Sqlite(SqliteLivePoller::start(ctx, tx, interval_ms))
            }
            PollerPrep::Rdb(ctx) => ActivePoller::Rdb(RdbLivePoller::start(ctx, tx, interval_ms)),
        };
        self.poller = Some(poller);
        self.app_state.live_update.poller_active = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::StudyMeta;

    fn make_channel() -> (mpsc::SyncSender<AppMessage>, mpsc::Receiver<AppMessage>) {
        mpsc::sync_channel(32)
    }

    #[test]
    fn unsupported_drop_message_guides_gh_binary_to_ghx() {
        let msg = unsupported_drop_message(&[std::path::PathBuf::from("/a/model.gh")]);
        assert!(msg.contains("model.gh"), "{msg}");
        assert!(msg.contains(".ghx"), "{msg}");
        assert!(msg.contains("Save As"), "{msg}");
    }

    #[test]
    fn unsupported_drop_message_lists_supported_types() {
        let msg = unsupported_drop_message(&[std::path::PathBuf::from("notes.txt")]);
        assert!(msg.contains("notes.txt"), "{msg}");
        assert!(msg.contains("unsupported file type"), "{msg}");
        assert!(msg.contains(".ghx"), "{msg}");
    }

    #[test]
    fn unsupported_drop_message_handles_missing_paths() {
        let msg = unsupported_drop_message(&[]);
        assert!(msg.contains("(unknown)"), "{msg}");
    }

    #[test]
    fn channel_send_receive_journal_parsed() {
        let (tx, rx) = make_channel();
        let studies = vec![StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![],
            completed_trials: 5,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
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
    fn convergence_done_maps_to_compute_sync() {
        // Regression guard: if IndicatorHistoryDone falls out of the sync targets, the
        // canvas item's computing flag never drops after compute finishes and the
        // spinner keeps spinning.
        use crate::state::app_state::ConvergenceHistory;
        let msg = AppMessage::IndicatorHistoryDone {
            indicator: tunny_core::indicators::MoIndicator::Hypervolume,
            base: ConvergenceHistory {
                trial_ids: vec![],
                values: vec![],
                sample_step: 1,
                ref_point: vec![],
            },
            comparisons: vec![],
        };
        assert!(matches!(
            ComputeSyncKind::from_message(&msg),
            Some(ComputeSyncKind::Convergence)
        ));
    }

    #[test]
    fn surrogate_multi_messages_map_to_compute_sync() {
        // Regression guard: if multi-objective surrogate completion/failure falls out of
        // the sync targets, the canvas item's fitting/optimizing flag never drops and the
        // spinner keeps spinning.
        assert!(matches!(
            ComputeSyncKind::from_message(&AppMessage::SurrogateMultiFitFailed("e".into())),
            Some(ComputeSyncKind::SurrogateFit)
        ));
        assert!(matches!(
            ComputeSyncKind::from_message(&AppMessage::SurrogateMultiOptFailed("e".into())),
            Some(ComputeSyncKind::SurrogateOpt)
        ));
        let done =
            AppMessage::SurrogateMultiOptDone(crate::state::messages::SurrogateMultiOptUiResult {
                param_names: vec![],
                objective_names: vec![],
                front: vec![],
                r_squared: vec![],
            });
        assert!(matches!(
            ComputeSyncKind::from_message(&done),
            Some(ComputeSyncKind::SurrogateOpt)
        ));
        let fit_done = AppMessage::SurrogateMultiFitDone(std::sync::Arc::new(vec![]));
        assert!(matches!(
            ComputeSyncKind::from_message(&fit_done),
            Some(ComputeSyncKind::SurrogateFit)
        ));
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
    fn spawn_task_captures_panic() {
        // M-4: a panic inside a worker is reported as TaskPanicked, preventing an
        // infinite spinner.
        let (tx, rx) = make_channel();
        spawn_task(tx, || panic!("boom in worker"));
        match rx.recv().unwrap() {
            AppMessage::TaskPanicked(detail) => assert!(detail.contains("boom in worker")),
            _ => panic!("Expected TaskPanicked"),
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

    // ── Phase C: window title password masking ─────────────────

    #[test]
    fn compute_window_title_no_path_returns_base_title() {
        assert_eq!(
            TunnyApp::compute_window_title(None),
            "Tunny Dashboard (Beta)"
        );
    }

    #[test]
    fn compute_window_title_local_path_shows_full_path() {
        let path = std::path::PathBuf::from("/home/user/study.log");
        assert_eq!(
            TunnyApp::compute_window_title(Some(&path)),
            "Tunny Dashboard (Beta) - /home/user/study.log"
        );
    }

    #[test]
    fn compute_window_title_rdb_url_masks_password() {
        let path =
            std::path::PathBuf::from("postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test");
        assert_eq!(
            TunnyApp::compute_window_title(Some(&path)),
            "Tunny Dashboard (Beta) - postgresql://tunny:***@127.0.0.1:5432/tunny_test"
        );
    }

    #[test]
    fn compute_window_title_rdb_url_without_password_unchanged() {
        let path = std::path::PathBuf::from("mysql://tunny@127.0.0.1:3306/tunny_test");
        assert_eq!(
            TunnyApp::compute_window_title(Some(&path)),
            "Tunny Dashboard (Beta) - mysql://tunny@127.0.0.1:3306/tunny_test"
        );
    }
}
