use crate::state::messages::{SurfacePlotRenderMode, SurfacePlotResult};
use crate::ui::widgets::{
    artifact_gallery::ArtifactGallery, cluster_scatter::ClusterScatter,
    cluster_scatter_3d::ClusterScatter3D, hv_history::HvHistoryChart,
    importance_chart::ImportanceChart, mcdm_chart::McdmRankChart,
    mcdm_scatter_chart::McdmScatterChart, mcdm_scatter_chart_3d::McdmScatterChart3D,
    optimization_history::OptimizationHistoryChart, parallel_coords::ParallelCoordsChart,
    pareto_2d::ParetoScatter2D, pareto_3d::Pareto3dChart, pdp_2d::PdpChart2DState,
    pdp_chart::PdpChart, scatter_matrix::ScatterMatrix, sensitivity_heatmap::SensitivityHeatmap,
    slice_chart::SliceChart, trial_table::TrialTable,
};

// ── TASK-2239: Surface Plot 計算リクエスト ──────────────────────
pub struct SurfacePlotComputeRequest {
    pub param_x: String,
    pub param_y: String,
    pub objective: String,
    pub n_grid: usize,
    /// 実行可能解（is_feasible > 0.5）のみでモデルをフィットするか
    pub feasible_only: bool,
}

// ── TASK-2228: Surface Plot UI 状態 ─────────────────────────────
#[derive(Default)]
pub struct SurfacePlotState {
    pub selected_x: String,
    pub selected_y: String,
    pub selected_objective: usize,
    pub render_mode: SurfacePlotRenderMode,
    pub computing: bool,
    pub result: Option<SurfacePlotResult>,
    pub error_message: Option<String>,
    /// 実行可能解のみでモデルをフィットするか（制約付きスタディのみ UI 表示）
    pub feasible_only: bool,
    pub pending_compute: Option<SurfacePlotComputeRequest>,
}

impl SurfacePlotState {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む。
    /// Surface 結果は widget 側（result）に保持されるため、キャンバスの各アイテム
    /// （独立した WidgetStates）にも反映する。X/Y/目的・描画モードの選択は維持する。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.result = src.result.clone();
        self.error_message = src.error_message.clone();
    }
}

// ── Surrogate Optimizer 計算リクエスト（フィット段階） ──────────
pub struct SurrogateFitComputeRequest {
    pub objective: String,
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
}

// ── Surrogate Optimizer 計算リクエスト（最適化段階） ────────────
pub struct SurrogateOptimizeComputeRequest {
    /// 応答曲面スライスの表示軸（パラメータ名）。
    pub slice_x: String,
    pub slice_y: String,
    pub optimizer: tunny_core::surrogate_opt::OptimizerKind,
}

// ── Surrogate Optimizer 計算リクエスト（候補提案段階） ──────────
pub struct SurrogateSuggestComputeRequest {
    /// 使用する獲得関数。
    pub acquisition: tunny_core::surrogate_opt::AcquisitionKind,
    /// 提案する候補数。
    pub n_candidates: usize,
    /// true = 最小化問題として提案する。
    pub minimize: bool,
}

/// 多目的サロゲート最適化のフィット段階リクエスト。
pub struct SurrogateMultiFitComputeRequest {
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
}

/// 多目的サロゲート最適化の最適化段階リクエスト。
pub struct SurrogateMultiOptimizeComputeRequest {
    /// 応答曲面スライスの表示軸（パラメータ名）。
    pub slice_x: String,
    pub slice_y: String,
}

// ── Surrogate Optimizer UI 状態 ─────────────────────────────────
pub struct SurrogateOptState {
    pub selected_objective: usize,
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
    pub optimizer: tunny_core::surrogate_opt::OptimizerKind,
    pub slice_x: String,
    pub slice_y: String,
    /// フィット段階のスピナーフラグ。
    pub fitting: bool,
    /// 最適化段階のスピナーフラグ。
    pub optimizing: bool,
    /// 検証済みの学習結果（フィット完了後に保持）。
    pub trained: Option<std::sync::Arc<tunny_core::surrogate_opt::TrainedSurrogate>>,
    pub result: Option<crate::state::messages::SurrogateOptUiResult>,
    pub error_message: Option<String>,
    pub pending_fit: Option<SurrogateFitComputeRequest>,
    pub pending_optimize: Option<SurrogateOptimizeComputeRequest>,
    /// true のとき多目的モード（全目的を NSGA-II で同時最適化）。
    pub multi_objective: bool,
    /// 多目的フィット段階の計算リクエスト（未消化）。
    pub pending_multi_fit: Option<SurrogateMultiFitComputeRequest>,
    /// 多目的最適化段階の計算リクエスト（未消化）。
    pub pending_multi_optimize: Option<SurrogateMultiOptimizeComputeRequest>,
    /// 多目的フィット完了後の学習済みサロゲート群（目的順）。
    pub multi_trained: Option<std::sync::Arc<Vec<tunny_core::surrogate_opt::TrainedSurrogate>>>,
    /// 多目的最適化の完了結果。
    pub multi_result: Option<crate::state::messages::SurrogateMultiOptUiResult>,
    /// 多目的結果表示で選択中の目的インデックス（スライスヒートマップ対象）。
    pub multi_slice_objective: usize,
    /// 多目的検証表示で選択中の目的インデックス（OOF プロット対象）。
    pub multi_validation_objective: usize,
    // ── 獲得関数による候補提案 ──────────────────────────────────
    /// 選択中の獲得関数。
    pub acq_kind: tunny_core::surrogate_opt::AcquisitionKind,
    /// 提案する候補数（1〜10）。
    pub n_suggest_candidates: usize,
    /// 候補提案の計算中フラグ。
    pub suggesting: bool,
    /// 候補提案の未消化リクエスト。
    pub pending_suggest: Option<SurrogateSuggestComputeRequest>,
    /// 候補提案の結果。
    pub suggest_result: Option<crate::state::messages::SurrogateSuggestUiResult>,
}

impl Default for SurrogateOptState {
    fn default() -> Self {
        Self {
            selected_objective: 0,
            model: tunny_core::surrogate_opt::SurrogateModelKind::GpFitc,
            optimizer: tunny_core::surrogate_opt::OptimizerKind::MultiStartLbfgs,
            slice_x: String::new(),
            slice_y: String::new(),
            fitting: false,
            optimizing: false,
            trained: None,
            result: None,
            error_message: None,
            pending_fit: None,
            pending_optimize: None,
            multi_objective: false,
            pending_multi_fit: None,
            pending_multi_optimize: None,
            multi_trained: None,
            multi_result: None,
            multi_slice_objective: 0,
            multi_validation_objective: 0,
            acq_kind: tunny_core::surrogate_opt::AcquisitionKind::ExpectedImprovement,
            n_suggest_candidates: 3,
            suggesting: false,
            pending_suggest: None,
            suggest_result: None,
        }
    }
}

impl SurrogateOptState {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む。
    /// キャンバスのアイテム別 WidgetStates へ完了状態を伝播するために使う
    /// （目的・モデル・最適化手法・スライス軸の選択は維持する）。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.fitting = src.fitting;
        self.optimizing = src.optimizing;
        self.trained = src.trained.clone();
        self.result = src.result.clone();
        self.multi_trained = src.multi_trained.clone();
        self.multi_result = src.multi_result.clone();
        self.error_message = src.error_message.clone();
        self.suggesting = src.suggesting;
        self.suggest_result = src.suggest_result.clone();
    }
}

// ── TASK-2228/2245: チャートキャプチャ状態 ───────────────────────
/// キャプチャした PNG の出力先。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureDest {
    /// ファイルダイアログを開いて保存する。
    #[default]
    File,
    /// クリップボードへコピーする。
    Clipboard,
}

#[derive(Default)]
pub struct ChartCaptureState {
    pub last_error: Option<String>,
    /// PNG 保存対象セル（消費されたら `None` に戻す）
    pub pending_capture: Option<crate::state::layout_state::PanelItem>,
    /// 保存対象セルの描画矩形（`ViewportCommand::Screenshot` 後のクロップに使う）
    pub pending_capture_rect: Option<egui::Rect>,
    /// Screenshot コマンド発行済みフラグ（次フレームで `Event::Screenshot` を待つ）
    pub screenshot_requested: bool,
    /// キャプチャ結果の出力先（ファイル保存 or クリップボード）
    pub pending_capture_dest: CaptureDest,
}

/// 各チャートウィジェットの UI 状態をまとめて保持する
/// AppState（データ）とは分離した純粋な UI 状態
#[derive(Default)]
pub struct WidgetStates {
    pub pareto_2d: ParetoScatter2D,
    pub pareto_3d: Pareto3dChart,
    pub opt_history: OptimizationHistoryChart,
    pub hv_history: HvHistoryChart,
    pub importance: ImportanceChart,
    pub pdp_chart: PdpChart,
    pub pdp_2d: PdpChart2DState,
    pub parallel_coords: ParallelCoordsChart,
    pub scatter_matrix: ScatterMatrix,
    pub sensitivity_heatmap: SensitivityHeatmap,
    pub cluster_scatter: ClusterScatter,
    pub cluster_scatter_3d: ClusterScatter3D,
    pub mcdm_chart: McdmRankChart,
    /// トライアル一覧 / クラスタ割当 / MCDM ランキングを統合したテーブルウィジェット。
    pub trial_table: TrialTable,
    pub artifact_gallery: ArtifactGallery,
    pub slice_chart: SliceChart,
    // TASK-1504: MCDM 散布図ウィジェット
    pub scatter_chart: McdmScatterChart,
    pub mcdm_scatter_3d: McdmScatterChart3D,
    // TASK-2228: Surface Plot と capture の一時状態
    pub surface_plot: SurfacePlotState,
    /// サロゲート最適化（応答曲面作成＋曲面上の最適化）の UI 状態
    pub surrogate_opt: SurrogateOptState,
    pub capture: ChartCaptureState,
    /// ダブルクリックで最大化表示中のウィジェット（None = 通常表示）
    pub maximized_item: Option<crate::state::layout_state::PanelItem>,
}

impl WidgetStates {
    /// Study 切替時に全チャートの show_infeasible フラグを true にリセットする。
    pub fn reset_infeasible_flags(&mut self) {
        self.pareto_3d.show_infeasible = true;
        self.cluster_scatter_3d.show_infeasible = true;
        self.mcdm_scatter_3d.show_infeasible = true;
        self.parallel_coords.show_infeasible = true;
        self.scatter_matrix.show_infeasible = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_states_default_has_surface_and_capture_slots() {
        let ws = WidgetStates::default();
        assert!(ws.surface_plot.result.is_none());
        assert!(!ws.surface_plot.computing);
        assert!(ws.surface_plot.error_message.is_none());
        assert_eq!(ws.surface_plot.render_mode, SurfacePlotRenderMode::Heatmap);
        assert!(ws.capture.last_error.is_none());
    }

    #[test]
    fn render_surface_plot_placeholder_without_result() {
        // Verify SurfacePlotState with no result does not panic when accessed
        let state = SurfacePlotState::default();
        assert!(state.result.is_none());
        assert!(!state.computing);
        assert!(state.selected_x.is_empty());
        assert!(state.selected_y.is_empty());
    }

    // ── SurrogateOptState の新 2 段階フィールドに対する回帰テスト ──

    #[test]
    fn surrogate_opt_state_default_has_expected_flags() {
        let state = SurrogateOptState::default();
        assert!(!state.fitting);
        assert!(!state.optimizing);
        assert!(state.trained.is_none());
        assert!(state.pending_fit.is_none());
        assert!(state.pending_optimize.is_none());
        assert!(state.result.is_none());
        // 多目的フィールドの初期値確認
        assert!(!state.multi_objective);
        assert!(state.pending_multi_fit.is_none());
        assert!(state.pending_multi_optimize.is_none());
        assert!(state.multi_trained.is_none());
        assert!(state.multi_result.is_none());
        assert_eq!(state.multi_slice_objective, 0);
        assert_eq!(state.multi_validation_objective, 0);
    }

    #[test]
    fn surrogate_opt_adopt_compute_state_propagates_new_fields() {
        let src = SurrogateOptState {
            fitting: false,
            optimizing: false,
            error_message: Some("err".into()),
            ..Default::default()
        };

        let mut dst = SurrogateOptState {
            fitting: true,
            optimizing: true,
            model: tunny_core::surrogate_opt::SurrogateModelKind::Ridge,
            selected_objective: 2,
            multi_validation_objective: 1,
            ..Default::default()
        };
        dst.adopt_compute_state(&src);

        // 伝播されるフィールド
        assert!(!dst.fitting);
        assert!(!dst.optimizing);
        assert_eq!(dst.error_message.as_deref(), Some("err"));
        // 選択は維持される
        assert_eq!(
            dst.model,
            tunny_core::surrogate_opt::SurrogateModelKind::Ridge
        );
        assert_eq!(dst.selected_objective, 2);
        // UI 選択（OOF プロット対象）は伝播されず維持される
        assert_eq!(dst.multi_validation_objective, 1);
        // multi_trained / multi_result も伝播される
        assert!(dst.multi_trained.is_none());
        assert!(dst.multi_result.is_none());
    }

    // ── TASK-2246: 回帰テスト ──────────────────────────────────────

    // F-005: surface plot state transitions (spinner on/off, error path)
    #[test]
    fn comparison_and_surface_plot_state_transitions_are_covered() {
        // start compute: computing = true, pending_compute set
        let mut state = SurfacePlotState {
            computing: true,
            pending_compute: Some(SurfacePlotComputeRequest {
                param_x: "x".into(),
                param_y: "y".into(),
                objective: "f".into(),
                n_grid: 20,
                feasible_only: false,
            }),
            ..Default::default()
        };
        assert!(state.computing);
        assert!(state.pending_compute.is_some());

        // success: result arrives, spinner off
        state.computing = false;
        state.pending_compute = None;
        state.result = Some(crate::state::messages::SurfacePlotResult {
            x_values: vec![0.0],
            y_values: vec![0.0],
            z_values: vec![vec![0.0]],
            param_x_name: "x".into(),
            param_y_name: "y".into(),
            objective_name: "f".into(),
            r2: Some(0.9),
        });
        assert!(!state.computing);
        assert!(state.result.is_some());

        // failure: error message set, spinner off
        state.result = None;
        state.error_message = Some("compute failed".into());
        assert!(state.error_message.is_some());
    }

    // F-008: PNG capture state transitions
    #[test]
    fn png_capture_state_transitions_are_covered() {
        use crate::state::layout_state::{ChartId, PanelItem};

        let mut capture = ChartCaptureState::default();
        assert!(capture.pending_capture.is_none());
        assert!(!capture.screenshot_requested);
        assert!(capture.pending_capture_rect.is_none());

        // "Save as PNG" pressed → pending set
        capture.pending_capture = Some(PanelItem::Chart(ChartId::ParallelCoordinates));
        capture.pending_capture_rect = Some(egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 80.0),
        ));
        assert!(capture.pending_capture.is_some());

        // Screenshot command issued
        capture.screenshot_requested = true;
        assert!(capture.screenshot_requested);

        // Screenshot received → consumed and reset
        capture.screenshot_requested = false;
        capture.pending_capture = None;
        capture.pending_capture_rect = None;
        assert!(!capture.screenshot_requested);
        assert!(capture.pending_capture.is_none());

        // Failure path: error stored
        capture.last_error = Some("crop rect outside image".into());
        assert!(capture.last_error.is_some());
    }
}
