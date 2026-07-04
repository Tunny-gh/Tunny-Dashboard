use std::collections::HashMap;
use std::sync::mpsc;

use crate::io::live_update_poller::LiveUpdatePoller;
use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;
use crate::state::message_handler::MessageHandler;
use crate::state::messages::AppMessage;
use crate::ui::toolbar::ToolbarAction;
use crate::ui::widget_states::WidgetStates;
use tunny_core::io::journal::live_update::LiveUpdateContext;

/// 非同期計算の完了メッセージごとに、キャンバスの各アイテム（独立した WidgetStates）へ
/// どのウィジェットの完了状態を伝播するかを表す。
///
/// 計算の発行（`pending_compute`）はアイテム固有の WidgetStates から行われるが、
/// 完了メッセージ（`MessageHandler::handle`）はグローバルな `widget_states` のみを更新する。
/// そのため、伝播しないとキャンバスのアイテムは `computing` フラグが立ったままになり、
/// スピナーが消えず結果が描画されない（commit 73883d8 のアイテム別状態化に伴う回帰）。
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
            | AppMessage::SurrogateOptFailed(_)
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
            AppMessage::IndicatorHistoryDone { .. } => Some(Self::Convergence),
            AppMessage::SensitivityHeatmapDone { .. } => Some(Self::SensitivityHeatmap),
            _ => None,
        }
    }

    /// グローバル widget（処理済みの正状態）から、キャンバスの全アイテムへ完了状態を反映する。
    /// 各 `adopt_*` はアイテム固有の UI 選択（パラメータ・目的関数など）を維持し、
    /// 計算の出力・実行フラグのみを取り込む。
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
                Self::SurrogateFit => w.surrogate_opt.adopt_compute_state(&global.surrogate_opt),
                Self::SurrogateOpt => w.surrogate_opt.adopt_compute_state(&global.surrogate_opt),
                Self::SurrogateSuggest => {
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
            }
        }
    }
}

pub struct TunnyApp {
    pub app_state: AppState,
    pub layout: LayoutState,
    pub widget_states: WidgetStates,
    /// キャンバスビューの各アイテム（item.id 単位）に独立した UI 状態を保持する。
    /// 同じウィジェットを複数置いても設定が共有されないようにするため。
    pub canvas_widgets: HashMap<u64, WidgetStates>,
    pub is_loading: bool,
    pub load_error: Option<String>,
    tx: mpsc::SyncSender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
    poller: Option<LiveUpdatePoller>,
    /// 現在ウィンドウタイトルバーに設定済みの文字列。変化時のみ更新コマンドを送るために保持する。
    current_window_title: Option<String>,
}

impl TunnyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<std::path::PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(crate::theme::tunny_light_visuals());
        // artifact ギャラリーで file:// 画像を表示するためのローダを登録する。
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Inter + Noto Sans JP フォントを設定（日本語グリフは JP フォントにフォールバック）
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
            if crate::io::flat_csv::is_csv_path(&path) {
                crate::io::study_worker::dispatch_scan_csv(path, tx.clone());
            } else if crate::io::sqlite::is_sqlite_path(&path) {
                crate::io::study_worker::dispatch_scan_sqlite(path, tx.clone());
            } else {
                crate::io::study_worker::dispatch_scan_journal(path, tx.clone());
            }
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
            current_window_title: None,
        }
    }

    /// 開いているファイルのフルパスをウィンドウタイトルバーに反映する。
    /// ファイル未読み込み時は "Tunny Dashboard (Beta)"、読み込み時は "Tunny Dashboard (Beta) - <フルパス>"。
    fn sync_window_title(&mut self, ctx: &egui::Context) {
        const BASE_TITLE: &str = "Tunny Dashboard (Beta)";
        let title = match self
            .app_state
            .journal_path
            .as_ref()
            .map(|p| p.display().to_string())
        {
            Some(full_path) => format!("{BASE_TITLE} - {full_path}"),
            None => BASE_TITLE.to_owned(),
        };
        if self.current_window_title.as_deref() != Some(title.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.current_window_title = Some(title);
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
            // ストリーミングロードのバッチは 1 フレーム 1 件に絞り、各バッチの
            // DataFrame 再構築コストを 1 フレームに集中させない（描画フリーズ回避）。
            // 残りのバッチはチャネルに残し、次フレームで処理する。
            let is_study_chunk = matches!(&msg, AppMessage::StudyChunkLoaded { .. });
            // 非同期計算の完了/失敗メッセージはグローバルな widget_states のみ更新する。
            // キャンバスの各アイテムは独立した WidgetStates を持つため（commit 73883d8）、
            // 処理後に完了状態（computing/結果/キャッシュ）を各アイテムへ伝播する必要がある。
            // どのウィジェットへ伝播すべきかを msg 消費前に判定しておく。
            let sync = ComputeSyncKind::from_message(&msg);

            MessageHandler::handle(
                msg,
                &mut self.app_state,
                &mut self.widget_states,
                &mut self.is_loading,
                &mut self.load_error,
            );

            // 完了状態をキャンバスの全アイテムへ反映する（グローバル widget が処理済みの正状態）。
            if let Some(sync) = sync {
                sync.propagate(&self.widget_states, &mut self.canvas_widgets);
            }

            if is_journal_parsed {
                // フラット CSV は最適化方向・変数レンジの情報を持たないため、自動活性化せず
                // 確認ダイアログを開く。確定時に編集値を反映した meta で select_study を発行する。
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
                if is_csv || is_sqlite {
                    // Live Update は journal 専用。journal で有効化したまま CSV/SQLite を
                    // 開いた場合、poller が非 journal ファイルを追跡しないよう強制オフにする。
                    self.app_state.live_update.enabled = false;
                    self.app_state.live_update.poller_active = false;
                } else if self.app_state.live_update.enabled {
                    self.restart_poller();
                }
                if is_csv {
                    if let Some(meta) = self.app_state.all_studies.first() {
                        self.app_state.csv_import_settings =
                            Some(crate::state::app_state::CsvImportSettings::from_meta(meta));
                    }
                } else if self.app_state.all_studies.len() == 1 {
                    // Study が 1 件のみなら自動的に Phase 2 を開始する
                    self.is_loading = true;
                    let meta = self.app_state.all_studies[0].clone();
                    crate::io::study_worker::dispatch_select_study(meta, self.sender());
                }
            }
            if is_live_error {
                // poller stopped itself — drop the handle
                self.poller = None;
            }

            ctx.request_repaint();

            if is_study_chunk {
                break;
            }
        }

        // ストリーミングロード中は入力が無くても継続描画して次バッチを取り込む
        // （bounded channel への送信は UI 描画に追いつくまで自然にブロックされる）。
        if self.is_loading {
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
                    self.app_state.all_studies.clear();
                    self.app_state.current_study = None;
                    // 別ファイルを開くと study_id 空間が変わるため、
                    // 同一ファイル前提の比較セッションは破棄する。
                    self.app_state.reset_comparison_session();
                    if crate::io::flat_csv::is_csv_path(&path) {
                        crate::io::study_worker::dispatch_scan_csv(path, self.sender());
                    } else if crate::io::sqlite::is_sqlite_path(&path) {
                        crate::io::study_worker::dispatch_scan_sqlite(path, self.sender());
                    } else {
                        crate::io::study_worker::dispatch_scan_journal(path, self.sender());
                    }
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
                        let csv = crate::io::export::build_csv_string_from_view(
                            &ctx.view,
                            &crate::io::export::select_row_indices_for_export(
                                &ctx.view,
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
                ToolbarAction::AddComparisonStudy(meta) => {
                    // 同一ファイル内の別 Study を比較対象として追加する。
                    // 既に追加済み、または基準 Study 自身は無視する。
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
                        let study_idx = self.app_state.comparison_studies.len();
                        self.app_state.comparison_mode = true;
                        crate::io::study_worker::dispatch_load_comparison_study(
                            meta,
                            study_idx,
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
                            // データ（study / 比較セッション）はそのまま。レイアウトと
                            // 設定だけ差し替え、計算結果は次フレームの各ウィジェットの
                            // ポーリングで復元後の設定に基づいて再計算される。
                            self.layout = session.layout;
                            self.canvas_widgets = session.widgets;
                            session.view.apply(&mut self.app_state);
                        }
                        Err(e) => self.load_error = Some(e),
                    }
                }
            }
        }
    }

    /// CSV インポート確認ダイアログを描画し、確定時に編集値を Study へ反映して活性化する。
    fn show_csv_import_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::csv_import_modal::{self, CsvImportAction};

        let Some(mut settings) = self.app_state.csv_import_settings.take() else {
            return;
        };
        match csv_import_modal::show(ctx, &mut settings) {
            Some(CsvImportAction::Apply) => {
                // 編集値を all_studies のエントリへ反映してから select_study を発行する。
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
                // settings は drop してダイアログを閉じる。
            }
            None => {
                // 未確定。次フレームも表示を続ける。
                self.app_state.csv_import_settings = Some(settings);
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

        // Optuna は trial_id を全 study・全状態横断で op_code=4 の出現順に連番付与する。
        // ライブ更新の差分パーサは次に作る Trial へこの global trial_id を割り当て、
        // 続く op_code=5/6 を trial_id で照合する。したがって開始時の next_trial_id は
        // 「ファイル中の op_code=4 レコード総数」でなければならない。meta には全体総数が
        // 無い（Phase1 は total_trials=0、選択 study 以外も 0）ため、ファイルを 1 度読んで数える。
        // 同じバイト列から byte_offset も取り、metadata 取得との競合（読取り中の追記）を防ぐ。
        // per-study の作成数も同じバイト列から数え、各 Study の次の trial.number を seed する
        // （ライブ中に作られる Trial が Study 内で連続した番号を持つようにする）。
        let (byte_offset, next_trial_id, study_trial_number_seeds) = match std::fs::read(file_path)
        {
            Ok(bytes) => {
                let per_study =
                    tunny_core::io::journal::live_update::count_created_trials_per_study(&bytes);
                (
                    bytes.len() as u64,
                    tunny_core::io::journal::live_update::count_created_trials(&bytes),
                    per_study,
                )
            }
            Err(_) => (
                std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0),
                0,
                std::collections::HashMap::new(),
            ),
        };

        let ctx = LiveUpdateContext {
            file_path: file_path.clone(),
            initial_byte_offset: byte_offset,
            next_trial_id,
            study_trial_number_seeds,
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
    // logic() は描画を行わないフェーズ（メッセージポンプ・状態更新・スクリーンショット取得）を担当する。
    // egui 0.35 の eframe::App は update() が ui()/logic() に分割された。
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_messages(ctx);
        self.sync_window_title(ctx);

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
        crate::ui::widgets::license_modal::show(&ctx, &mut self.widget_states.license_modal);
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
        // 回帰防止: IndicatorHistoryDone が sync 対象から漏れると、計算完了後も
        // キャンバスアイテムの computing が下りず spinner が回り続ける。
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
        // 回帰防止: 多目的サロゲートの完了/失敗が sync 対象から漏れると、
        // キャンバスアイテムの fitting/optimizing が下りず spinner が回り続ける。
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
                minimize: vec![],
                front: vec![],
                r_squared: vec![],
                slices: vec![],
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
