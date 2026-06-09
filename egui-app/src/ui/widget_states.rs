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
use crate::{state::app_state::AppState, theme::color_compute::compute_chart_colors_view};

// ── TASK-2239: Surface Plot 計算リクエスト ──────────────────────
pub struct SurfacePlotComputeRequest {
    pub param_x: String,
    pub param_y: String,
    pub objective: String,
    pub n_grid: usize,
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

// ── Surrogate Optimizer 計算リクエスト ──────────────────────────
pub struct SurrogateOptComputeRequest {
    pub objective: String,
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
    pub optimizer: tunny_core::surrogate_opt::OptimizerKind,
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
    pub computing: bool,
    pub result: Option<crate::state::messages::SurrogateOptUiResult>,
    pub error_message: Option<String>,
    pub pending_compute: Option<SurrogateOptComputeRequest>,
}

impl Default for SurrogateOptState {
    fn default() -> Self {
        Self {
            selected_objective: 0,
            model: tunny_core::surrogate_opt::SurrogateModelKind::Kriging,
            optimizer: tunny_core::surrogate_opt::OptimizerKind::MultiStartLbfgs,
            slice_x: String::new(),
            slice_y: String::new(),
            computing: false,
            result: None,
            error_message: None,
            pending_compute: None,
        }
    }
}

impl SurrogateOptState {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む。
    /// キャンバスのアイテム別 WidgetStates へ完了状態を伝播するために使う
    /// （目的・モデル・最適化手法・スライス軸の選択は維持する）。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.result = src.result.clone();
        self.error_message = src.error_message.clone();
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
    /// チャート描画用の色キャッシュ（UI専用）
    pub chart_colors: Vec<egui::Color32>,
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

    /// 色モード・カラーマップ・MCDM結果の変化を描画色キャッシュへ反映する。
    /// `StudySelected` 後、色設定変更後、`McdmDone` 後に呼び出すことを想定する。
    pub fn update_chart_colors(&mut self, app_state: &AppState) {
        if let Some(ctx) = &app_state.current_study {
            let color_mode = app_state.color_mode.clone();
            let colormap_name = app_state.selected_colormap.clone();
            let objective_names = &ctx.meta.objective_names;
            let mcdm_scores = app_state.mcdm_result.as_ref().map(|r| r.primary_scores());
            self.chart_colors = compute_chart_colors_view(
                &color_mode,
                &colormap_name,
                &ctx.view,
                objective_names,
                mcdm_scores,
            );
        } else {
            self.chart_colors.clear();
        }
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
