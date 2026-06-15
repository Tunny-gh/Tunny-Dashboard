use std::sync::Arc;

use tunny_core::surrogate_opt::{SurrogateModelKind, TrainedSurrogate};

use crate::state::messages::ResponseSurfaceResult;
use crate::ui::widgets::scatter_3d::ArcballCamera;
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

// ── ResponseSurfacePlot 共有サロゲートキャッシュ ────────────────

/// 学習済みサロゲートの共有キャッシュキー。Auto は `model = None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurrogateKey {
    pub objective: String,
    /// None = Auto（CV で自動選択）。Some(kind) = 手動指定モデル。
    pub model: Option<SurrogateModelKind>,
    pub use_constraints: bool,
}

impl SurrogateKey {
    pub fn from_trained(t: &TrainedSurrogate) -> Self {
        Self {
            objective: t.objective_name.clone(),
            model: if t.model_selection.is_some() {
                None
            } else {
                Some(t.model_kind)
            },
            use_constraints: !t.constraint_names.is_empty(),
        }
    }
}

/// ResponseSurfacePlot と Optimizer が共有する学習済みサロゲートのキャッシュ。
/// どちらでフィットしても相互に再利用される（再フィット不要）。
/// `TrainedSurrogate` は `Arc` 越しに保持するため Clone は安価（キャンバス各アイテムへ伝播する）。
#[derive(Default, Clone)]
pub struct SurrogateCache {
    entries: Vec<(SurrogateKey, Arc<TrainedSurrogate>)>,
}

impl SurrogateCache {
    pub fn get(&self, key: &SurrogateKey) -> Option<Arc<TrainedSurrogate>> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, t)| Arc::clone(t))
    }

    pub fn insert(&mut self, key: SurrogateKey, trained: Arc<TrainedSurrogate>) {
        self.entries.retain(|(k, _)| k != &key);
        self.entries.push((key, trained));
        // 上限を超えたら古いものから捨てる。
        while self.entries.len() > 8 {
            self.entries.remove(0);
        }
    }
}

/// ResponseSurfacePlot のスライス生成リクエスト。
/// poll_chart が消費し、`key` で共有キャッシュを参照して曲面を生成する。
pub struct ResponseSurfaceSliceRequest {
    pub key: SurrogateKey,
    pub param_x: String,
    pub param_y: String,
    pub objective: String,
    pub model_label: String,
    pub minimize: bool,
    pub n_grid: usize,
}

// ── ResponseSurfacePlot UI 状態 ─────────────────────────────────
pub struct ResponseSurfacePlotState {
    pub selected_x: String,
    pub selected_y: String,
    pub selected_objective: usize,
    pub model: SurrogateModelKind,
    /// true のとき Auto（CV で自動選択）。
    pub auto_select: bool,
    pub use_constraints: bool,
    /// GP 系の事後標準偏差を 95% CI バンドとして重ねるか。
    pub show_uncertainty: bool,
    pub camera: ArcballCamera,
    /// サロゲート学習中。
    pub fitting: bool,
    /// 応答曲面スライス生成中。
    pub computing_slice: bool,
    pub result: Option<ResponseSurfaceResult>,
    pub error_message: Option<String>,
    pub pending_fit: Option<SurrogateFitComputeRequest>,
    pub pending_slice: Option<ResponseSurfaceSliceRequest>,
}

impl Default for ResponseSurfacePlotState {
    fn default() -> Self {
        Self {
            selected_x: String::new(),
            selected_y: String::new(),
            selected_objective: 0,
            model: SurrogateModelKind::Lgbm,
            auto_select: false,
            use_constraints: false,
            show_uncertainty: true,
            camera: ArcballCamera {
                rotation: [-0.2391, 0.3696, 0.0990, 0.8924],
                ..Default::default()
            },
            fitting: false,
            computing_slice: false,
            result: None,
            error_message: None,
            pending_fit: None,
            pending_slice: None,
        }
    }
}

impl ResponseSurfacePlotState {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む（最大化表示用）。
    /// 選択（X/Y/目的・モデル）は各アイテム側を維持する。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.fitting = src.fitting;
        self.computing_slice = src.computing_slice;
        self.result = src.result.clone();
        self.error_message = src.error_message.clone();
    }
}

// ── Surrogate Optimizer 計算リクエスト（フィット段階） ──────────
pub struct SurrogateFitComputeRequest {
    pub objective: String,
    /// `auto_select = false` のときに使う具体的なモデル種別。Auto のときは無視される
    /// プレースホルダ（core 側が CV で選び直す）。
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
    /// true のとき core が `AUTO_CANDIDATES` を交差検証して最良モデルを自動選択する。
    pub auto_select: bool,
    /// 制約を使用するか（true のとき制約列を ConstraintData に詰めて渡す）。
    pub use_constraints: bool,
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

/// EHVI による多目的候補提案リクエスト。
pub struct SurrogateMultiSuggestComputeRequest {
    /// 提案する候補数。
    pub n_candidates: usize,
}

// ── Surrogate Optimizer UI 状態 ─────────────────────────────────
pub struct SurrogateOptState {
    pub selected_objective: usize,
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
    /// true のとき Model コンボで "Auto (cross-validated)" が選択されている。
    /// この場合 `model` はプレースホルダ扱いとなり、core が CV で最良モデルを選ぶ。
    pub auto_select: bool,
    pub optimizer: tunny_core::surrogate_opt::OptimizerKind,
    pub slice_x: String,
    pub slice_y: String,
    /// フィット段階のスピナーフラグ。
    pub fitting: bool,
    /// フィット中の進捗・キャンセル共有ハンドル（学習スレッドと共有）。
    /// `fitting` が true の間だけ `Some`。Cancel ボタンと進捗バーが参照する。
    pub fit_progress: Option<tunny_core::surrogate_opt::FitProgress>,
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
    /// 制約を使用するか（制約付き Study のみ UI に表示; true = 制約を渡す）。
    pub use_constraints: bool,
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
    /// 応答曲面スライスに予測標準偏差（±σ）を重ねて表示するか（GP 系のみ。既定 off）。
    pub show_slice_uncertainty: bool,
    // ── EHVI による多目的候補提案 ──────────────────────────────────
    /// 多目的提案の候補数（1〜10）。
    pub n_multi_suggest_candidates: usize,
    /// 多目的候補提案の計算中フラグ。
    pub multi_suggesting: bool,
    /// 多目的候補提案の未消化リクエスト。
    pub pending_multi_suggest: Option<SurrogateMultiSuggestComputeRequest>,
    /// 多目的候補提案の結果。
    pub multi_suggest_result: Option<crate::state::messages::SurrogateMultiSuggestUiResult>,
}

impl Default for SurrogateOptState {
    fn default() -> Self {
        Self {
            selected_objective: 0,
            model: tunny_core::surrogate_opt::SurrogateModelKind::GpFitc,
            auto_select: false,
            optimizer: tunny_core::surrogate_opt::OptimizerKind::MultiStartLbfgs,
            slice_x: String::new(),
            slice_y: String::new(),
            fitting: false,
            fit_progress: None,
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
            use_constraints: true,
            acq_kind: tunny_core::surrogate_opt::AcquisitionKind::ExpectedImprovement,
            n_suggest_candidates: 3,
            suggesting: false,
            pending_suggest: None,
            suggest_result: None,
            show_slice_uncertainty: false,
            n_multi_suggest_candidates: 3,
            multi_suggesting: false,
            pending_multi_suggest: None,
            multi_suggest_result: None,
        }
    }
}

impl SurrogateOptState {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む。
    /// キャンバスのアイテム別 WidgetStates へ完了状態を伝播するために使う
    /// （目的・モデル・最適化手法・スライス軸の選択は維持する）。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.fitting = src.fitting;
        self.fit_progress = src.fit_progress.clone();
        self.optimizing = src.optimizing;
        self.trained = src.trained.clone();
        self.result = src.result.clone();
        self.multi_trained = src.multi_trained.clone();
        self.multi_result = src.multi_result.clone();
        self.error_message = src.error_message.clone();
        self.suggesting = src.suggesting;
        self.suggest_result = src.suggest_result.clone();
        self.multi_suggesting = src.multi_suggesting;
        self.multi_suggest_result = src.multi_suggest_result.clone();
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
    /// ResponseSurfacePlot（学習済みサロゲートから生成する3D応答曲面）の UI 状態
    pub response_surface: ResponseSurfacePlotState,
    /// サロゲート最適化（応答曲面作成＋曲面上の最適化）の UI 状態
    pub surrogate_opt: SurrogateOptState,
    /// ResponseSurfacePlot と Optimizer が共有する学習済みサロゲートのキャッシュ
    pub surrogate_cache: SurrogateCache,
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
    fn widget_states_default_has_response_surface_and_capture_slots() {
        let ws = WidgetStates::default();
        assert!(ws.response_surface.result.is_none());
        assert!(!ws.response_surface.fitting);
        assert!(!ws.response_surface.computing_slice);
        assert!(ws.response_surface.error_message.is_none());
        assert!(ws.capture.last_error.is_none());
    }

    #[test]
    fn response_surface_placeholder_without_result() {
        // Verify default state with no result does not panic when accessed.
        let state = ResponseSurfacePlotState::default();
        assert!(state.result.is_none());
        assert!(!state.fitting);
        assert!(!state.computing_slice);
        assert!(state.selected_x.is_empty());
        assert!(state.selected_y.is_empty());
        assert!(state.show_uncertainty);
    }

    #[test]
    fn surrogate_cache_empty_get_and_key_distinguishes_auto() {
        let cache = SurrogateCache::default();
        let key = SurrogateKey {
            objective: "f".into(),
            model: Some(SurrogateModelKind::Ridge),
            use_constraints: false,
        };
        assert!(cache.get(&key).is_none());
        // Auto キー（model=None）は手動指定キーと別物として扱われる。
        let auto_key = SurrogateKey {
            model: None,
            ..key.clone()
        };
        assert_ne!(key, auto_key);
        // 制約フラグもキーの一部。
        let constrained = SurrogateKey {
            use_constraints: true,
            ..key.clone()
        };
        assert_ne!(key, constrained);
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

    // ── ResponseSurfacePlot 状態遷移の回帰テスト ──────────────────

    // フィット → スライス生成 → 結果/エラーの状態遷移を確認する。
    #[test]
    fn response_surface_state_transitions_are_covered() {
        // フィット開始: fitting = true, pending_fit set
        let mut state = ResponseSurfacePlotState {
            fitting: true,
            pending_fit: Some(SurrogateFitComputeRequest {
                objective: "f".into(),
                model: SurrogateModelKind::Lgbm,
                auto_select: false,
                use_constraints: false,
            }),
            ..Default::default()
        };
        assert!(state.fitting);
        assert!(state.pending_fit.is_some());

        // フィット完了 → スライス生成へ
        state.fitting = false;
        state.pending_fit = None;
        state.computing_slice = true;
        state.pending_slice = Some(ResponseSurfaceSliceRequest {
            key: SurrogateKey {
                objective: "f".into(),
                model: Some(SurrogateModelKind::Lgbm),
                use_constraints: false,
            },
            param_x: "x".into(),
            param_y: "y".into(),
            objective: "f".into(),
            model_label: "LightGBM".into(),
            minimize: true,
            n_grid: 24,
        });
        assert!(state.computing_slice);

        // スライス完了: result arrives, spinner off
        state.computing_slice = false;
        state.pending_slice = None;
        state.result = Some(crate::state::messages::ResponseSurfaceResult {
            slice: tunny_core::surrogate_opt::SurfaceSlice {
                param_x_idx: 0,
                param_y_idx: 1,
                x_values: vec![0.0, 1.0],
                y_values: vec![0.0, 1.0],
                z_values: vec![vec![0.0, 1.0], vec![1.0, 2.0]],
                z_std: None,
            },
            param_x_name: "x".into(),
            param_y_name: "y".into(),
            objective_name: "f".into(),
            model_label: "LightGBM".into(),
            cv_r2_mean: 0.9,
        });
        assert!(!state.computing_slice);
        assert!(state.result.is_some());

        // 失敗: error message set, spinner off
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
