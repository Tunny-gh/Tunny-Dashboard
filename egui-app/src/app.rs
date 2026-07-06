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

/// ライブ更新ポーラーが「変化なし」と判断してから完了ヒント
/// (`AppMessage::LiveUpdateMaybeComplete`) を送るまでの無変化時間（ミリ秒）。
/// journal/sqlite/rdb の 3 ポーラー全てで共通の既定値として使う。
const LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS: u64 = 60_000;

/// アプリが現在起動しているライブ更新ポーラー。ストレージ種別（journal/sqlite/rdb）ごとに
/// 実装が異なる（journal: バイトオフセット差分、sqlite・rdb: フィンガープリント + 丸ごと再ロード）
/// ため、`TunnyApp` はどれか一方を保持できるようにこの enum で包む。
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
                // フィット・最適化・提案はいずれも同じ surrogate_opt 状態を共有する。
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
    /// キャンバスビューの各アイテム（item.id 単位）に独立した UI 状態を保持する。
    /// 同じウィジェットを複数置いても設定が共有されないようにするため。
    pub canvas_widgets: HashMap<u64, WidgetStates>,
    pub is_loading: bool,
    pub load_error: Option<String>,
    tx: mpsc::SyncSender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
    poller: Option<ActivePoller>,
    /// ライブ更新ポーラーの起動準備（H-1/H-2）に付与する世代カウンタ。
    /// `restart_poller` のたびに +1 し、準備完了メッセージ（`AppMessage::PollerReady`）
    /// が現在の世代と一致する場合のみポーラーを起動する。準備中にユーザーが
    /// トグル/Study 変更/別ファイルを開いた場合、古い準備結果を破棄するために使う。
    poller_generation: u64,
    /// 現在ウィンドウタイトルバーに設定済みの文字列。変化時のみ更新コマンドを送るために保持する。
    current_window_title: Option<String>,
}

impl TunnyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<std::path::PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(crate::theme::tunny_visuals(false));
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

    /// 開いているファイルのフルパスをウィンドウタイトルバーに反映する。
    /// ファイル未読み込み時は "Tunny Dashboard (Beta)"、読み込み時は "Tunny Dashboard (Beta) - <フルパス>"。
    /// RDB URL はパスワードを含みうるため、実際の計算は `compute_window_title` へ切り出し、
    /// URL の場合は `RdbUrl::masked()` でパスワードを隠したものを表示する。
    fn sync_window_title(&mut self, ctx: &egui::Context) {
        let title = Self::compute_window_title(self.app_state.journal_path.as_deref());
        if self.current_window_title.as_deref() != Some(title.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.current_window_title = Some(title);
        }
    }

    /// ウィンドウタイトル文字列を計算する純関数（`sync_window_title` から分離してテスト可能にする）。
    /// `journal_path` が RDB 接続 URL として解釈できる場合はパスワードをマスクした
    /// `RdbUrl::masked()` を表示し、それ以外は従来どおり `Path::display()` を使う。
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

    /// バックグラウンドタスク起動用の Sender クローンを返す
    pub fn sender(&self) -> mpsc::SyncSender<AppMessage> {
        self.tx.clone()
    }

    /// ノンブロッキングにメッセージを処理し AppState を更新する
    pub fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            // H-1/H-2: ポーラー起動準備の完了は tx/poller を要するため、tx を持たない
            // MessageHandler ではなくここで横取りして処理する（世代一致時のみ起動）。
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
            // ストリーミングロードのバッチは 1 フレーム 1 件に絞り、各バッチの
            // DataFrame 再構築コストを 1 フレームに集中させない（描画フリーズ回避）。
            // 残りのバッチはチャネルに残し、次フレームで処理する。
            let is_study_chunk = matches!(&msg, AppMessage::StudyChunkLoaded { .. });
            // SQLite ライブ更新: study がアクティブ化された（選択完了）タイミングで、
            // ライブ更新が有効なら新しい study_id を追跡するようポーラーを再起動する
            // （journal と異なりフィンガープリントは study 単位でしか取れないため）。
            let is_study_activated = matches!(&msg, AppMessage::StudySelected { .. })
                || matches!(&msg, AppMessage::StudyChunkLoaded { is_final: true, .. });
            // SQLite ライブ更新: フィンガープリント変化を検出したら再ロードをワーカーへ依頼する。
            // 実際の再パースは tx を必要とするため、tx を持たない MessageHandler ではなく
            // ここ（tx を持つ app.rs）で dispatch する。
            let sqlite_reload_study_id = MessageHandler::sqlite_reload_study_id(&msg);
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
                let is_rdb = self
                    .app_state
                    .journal_path
                    .as_deref()
                    .is_some_and(|p| crate::io::rdb::path_as_rdb_url(p).is_some());
                if is_csv {
                    // CSV はフラットインポート（1 回きりの取り込み）でストリーミング追記の
                    // 概念が無いため、Live Update 対象外のまま強制オフにする。
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
                        // SQLite/RDB はフィンガープリントに study_id が要るため、Study 選択前は
                        // ポーラーを起動しない（Study 選択完了時に is_study_activated 経由で起動する）。
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
                    // Study が 1 件のみなら自動的に Phase 2 を開始する
                    self.is_loading = true;
                    let meta = self.app_state.all_studies[0].clone();
                    crate::io::study_worker::dispatch_select_study(meta, self.sender());
                }
            }
            if is_live_error {
                // poller stopped itself — drop the handle
                self.poller = None;
                // 起動待ちの準備タスクがあれば陳腐化させ、エラー後に勝手に再起動させない。
                self.invalidate_pending_poller();
            }

            if let Some(study_id) = sqlite_reload_study_id {
                // SqliteLiveChanged は SQLite/RDB 両方のライブ更新が流用するシグナルメッセージ
                // なので、実際の再ロード先は現在の storage_kind で振り分ける。
                if self.app_state.live_update.storage_kind == LiveUpdateStorageKind::Rdb {
                    crate::io::study_worker::dispatch_reload_rdb_study(study_id, self.sender());
                } else {
                    crate::io::study_worker::dispatch_reload_sqlite_study(study_id, self.sender());
                }
            }

            // SQLite/RDB ライブ更新は study 単位でしかフィンガープリントを取れないため、
            // 表示中の study が切り替わったらポーラーを新しい study_id で再起動する
            // （journal はファイル全体を追跡するため study 切り替えでの再起動は不要）。
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

        // ストリーミングロード中は入力が無くても継続描画して次バッチを取り込む
        // （bounded channel への送信は UI 描画に追いつくまで自然にブロックされる）。
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
                        // 起動待ちの準備タスクがあれば陳腐化させる（H-1/H-2）。
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
                        // 保存失敗は握り潰さず load_error に反映する（SaveSession と同方針）。
                        if let Err(e) = crate::io::export::save_csv_to_file(&csv) {
                            self.load_error = Some(e);
                        }
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
                ToolbarAction::OpenReportDialog => {
                    self.app_state.report_dialog =
                        Some(crate::ui::widgets::report_modal::ReportDialogState::default());
                }
            }
        }
    }

    /// 「Report…」モーダルを描画し、Export 確定時に study のスナップショットを集めて
    /// バックグラウンドスレッドへレポート生成を委譲する（`ToolbarAction::OpenReportDialog`
    /// が `app_state.report_dialog` を開始した後、毎フレームここから呼ばれる）。
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
                // 生成中でも待たずに閉じてよい（バックグラウンドジョブは fire-and-forget
                // で継続し、ダイアログが無ければ完了/失敗は通知されない）。
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

    /// 指定パスを開く（journal / CSV / SQLite / RDB URL いずれか）。
    /// `ToolbarAction::OpenJournal` と「Open URL…」ダイアログの Open ボタンの
    /// 両方から呼ばれる共通処理（URL は `PathBuf::from(正規化済み url 文字列)` として渡される）。
    fn open_path(&mut self, path: std::path::PathBuf) {
        // Stop existing poller before loading new file
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }
        self.is_loading = true;
        self.load_error = None;
        self.app_state.all_studies.clear();
        self.app_state.current_study = None;
        // 別ファイル（別 URL）を開くと study_id 空間が変わるため、
        // 同一ファイル前提の比較セッションは破棄する。
        self.app_state.reset_comparison_session();
        // 別ファイルを開く前に、起動待ちのポーラー準備タスクを陳腐化させる（H-1/H-2）。
        self.invalidate_pending_poller();
        dispatch_scan(path, self.sender());
    }

    /// 「Open URL…」ダイアログを描画し、Open 確定時に正規化済み URL 文字列を
    /// `open_path` へ流す（`ToolbarAction::OpenJournal` と同じ経路）。
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
                // input は drop してダイアログを閉じる。
            }
            None => {
                // 未確定。次フレームも表示を続ける。
                self.app_state.db_url_dialog = Some(input);
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

    /// 起動待ち（準備中）のポーラーを陳腐化させる（H-1/H-2）。
    /// 世代を進めることで、進行中の準備タスクが送る `AppMessage::PollerReady` を
    /// 受信時に破棄させる。トグルオフ・別ファイルオープン・ライブエラー時に呼ぶ。
    fn invalidate_pending_poller(&mut self) {
        self.poller_generation = self.poller_generation.wrapping_add(1);
    }

    /// ポーラーを現在のファイルで（再）起動する。
    ///
    /// フィンガープリント取得（DB 接続 + クエリ）やジャーナル全読込 + trial 数
    /// カウントは I/O を伴い UI スレッドをフリーズさせるため（H-1/H-2）、ここでは
    /// 準備タスクをバックグラウンドへ spawn するだけに留める。準備完了後に
    /// `AppMessage::PollerReady` が届き、`start_prepared_poller` が実際にポーラーを
    /// 起動する。ストレージ種別（journal / sqlite / rdb）で準備内容が異なる。
    fn restart_poller(&mut self) {
        // Stop any existing poller
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }

        // 所有権付きで取り出す（この後 invalidate_pending_poller が &mut self を取るため、
        // self.app_state を借用したまま跨げない）。
        let Some(file_path) = self.app_state.journal_path.clone() else {
            return;
        };

        // 世代を進め、この呼び出しで spawn する準備タスクにだけ有効な世代を割り当てる。
        // 以降にトグル/Study 変更で restart_poller が再度呼ばれると世代が進み、
        // 本タスクの結果は start_prepared_poller で破棄される。
        self.invalidate_pending_poller();
        let generation = self.poller_generation;
        let tx = self.tx.clone();

        match self.app_state.live_update.storage_kind {
            LiveUpdateStorageKind::Sqlite => {
                // SQLite のフィンガープリントは study 単位でしか取れないため、
                // アクティブ Study が無ければ何も起動しない
                // （Study 選択完了時に is_study_activated 経由で改めて呼ばれる）。
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
                    // 初期フィンガープリント取得に失敗しても（読み取り競合等）デフォルト値で
                    // 起動する。次回ポーリングで実値と食い違えば単に 1 回余分に再ロードされる
                    // だけで、安全側に倒れる。
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
                // RDB のフィンガープリントも study 単位でしか取れないため、SQLite と同様に
                // アクティブ Study が無ければ何も起動しない。
                let Some(study_id) = self
                    .app_state
                    .current_study
                    .as_ref()
                    .map(|s| s.meta.study_id)
                else {
                    return;
                };
                // journal_path には URL 文字列がそのまま格納されている（Phase C 設計）。
                // 通常はここで必ず Some になるが、想定外に外れていれば安全側で何もしない。
                let Some(url) = crate::io::rdb::path_as_rdb_url(&file_path) else {
                    return;
                };
                spawn_task(tx, move || {
                    // DB 接続 + クエリはここ（バックグラウンド）で行う。低速・到達不能でも
                    // UI スレッドはブロックされない（H-1）。取得失敗時はデフォルト値で
                    // 起動する（SQLite と同じフォールバック方針）。
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
                    // Optuna は trial_id を全 study・全状態横断で op_code=4 の出現順に連番付与する。
                    // ライブ更新の差分パーサは次に作る Trial へこの global trial_id を割り当て、
                    // 続く op_code=5/6 を trial_id で照合する。したがって開始時の next_trial_id は
                    // 「ファイル中の op_code=4 レコード総数」でなければならない。meta には全体総数が
                    // 無い（Phase1 は total_trials=0、選択 study 以外も 0）ため、ファイルを 1 度読んで数える。
                    // 同じバイト列から byte_offset も取り、metadata 取得との競合（読取り中の追記）を防ぐ。
                    // per-study の作成数も同じバイト列から数え、各 Study の次の trial.number を seed する
                    // （ライブ中に作られる Trial が Study 内で連続した番号を持つようにする）。
                    // 数百 MB 級ジャーナルの全読込 + カウントもここ（バックグラウンド）で行う（H-2）。
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

        // 準備タスクを spawn した時点で「起動処理中」として扱う（UI 表示のため）。
        // 実際のポーラー起動は start_prepared_poller が行う。
        self.app_state.live_update.poller_active = true;
    }

    /// バックグラウンド準備タスク（H-1/H-2）が完了して届いた `PollerReady` を受け、
    /// 世代が最新（準備中にトグル/Study 変更が起きていない）ならポーラーを起動する。
    fn start_prepared_poller(&mut self, generation: u64, prep: PollerPrep) {
        // 準備中にトグル/Study 変更/別ファイルオープンで世代が進んでいれば破棄する。
        if generation != self.poller_generation {
            return;
        }
        // 通常は restart_poller が停止済みだが、念のため既存ポーラーを止める。
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }
        // 準備中に間隔が変わっている可能性があるため、起動時点の最新値を使う。
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
    // logic() は描画を行わないフェーズ（メッセージポンプ・状態更新・スクリーンショット取得）を担当する。
    // egui 0.35 の eframe::App は update() が ui()/logic() に分割された。
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // テーマ同期: dark_mode の変更源（ツールバートグル・セッション復元）に
        // よらず、グローバルフラグと Visuals をここで一元的に追従させる。
        if self.app_state.dark_mode != crate::theme::is_dark_mode() {
            ctx.set_visuals(crate::theme::tunny_visuals(self.app_state.dark_mode));
        }

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
        self.show_db_url_dialog(&ctx);
        self.show_report_dialog(&ctx);
        crate::ui::widgets::license_modal::show(&ctx, &mut self.widget_states.license_modal);
    }
}

/// 開くパスの種別（RDB URL / フラット CSV / SQLite / journal）を判定し、対応する
/// スキャンをワーカースレッドへ発行する。`TunnyApp::new`（初期パス）と `open_path`
/// （ツールバー・URL ダイアログ）の共通処理（D-12）。
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

/// バックグラウンドタスク起動ヘルパー。
///
/// ワーカーの panic を `catch_unwind` で捕捉し、`AppMessage::TaskPanicked` として
/// UI へ通知する（M-4）。捕捉しないと panic 時に完了メッセージが届かず、該当
/// ウィジェットの `computing`/`fitting` フラグが立ちっぱなしでスピナーが永久に回る。
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
                    .unwrap_or_else(|| "不明な panic".to_string());
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
        // M-4: ワーカー内 panic は TaskPanicked として通知され、無限スピナー化を防ぐ。
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

    // ── Phase C: ウィンドウタイトルのパスワードマスク ─────────────────

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
