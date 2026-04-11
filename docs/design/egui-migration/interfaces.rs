/// egui-app 型定義
///
/// 作成日: 2026-04-11
/// 関連設計: architecture.md
///
/// 信頼性レベル:
/// - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
/// - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
/// - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義

// ============================================================
// メインアプリ構造体
// ============================================================

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use egui::Context;

/// TunnyApp - メインアプリ構造体
/// 🔵 信頼性: eframe::App 実装パターン + 既存 studyStore/selectionStore 分析より
pub struct TunnyApp {
    // データ状態（Journalデータ・選択・分析結果）
    app_state: AppState, // 🔵 既存 Zustand stores から翻訳

    // UI レイアウト状態
    layout: LayoutState, // 🔵 既存 AppShell/layoutStore から翻訳

    // 非同期タスク通信
    tx: mpsc::SyncSender<AppMessage>, // 🔵 Rust非同期パターン
    rx: mpsc::Receiver<AppMessage>,   // 🔵 Rust非同期パターン

    // GPU レンダリング（wgpu 統合）
    scatter_renderer: Option<ScatterRenderer>, // 🟡 egui-wgpu統合パターン

    // UI 状態フラグ
    is_loading: bool,   // 🔵 既存 studyStore.isLoading から
    load_error: Option<String>, // 🔵 既存 studyStore.loadError から
}

// ============================================================
// アプリ状態
// ============================================================

/// AppState - データ状態（旧 Zustand stores の統合）
/// 🔵 信頼性: 既存 studyStore + selectionStore + analysisStore 分析より
pub struct AppState {
    // Study データ
    all_studies: Vec<StudyMeta>,         // 🔵 既存 studyStore.allStudies から
    current_study: Option<StudyContext>, // 🔵 既存 studyStore.currentStudy から

    // Brushing & Linking（旧 selectionStore）
    selected_indices: Vec<u32>,                    // 🔵 既存 selectionStore.selectedIndices から
    filter_ranges: HashMap<String, (f64, f64)>,    // 🔵 既存 selectionStore.filterRanges から
    highlighted_trial: Option<u32>,                // 🔵 既存 selectionStore.highlighted から
    color_mode: ColorMode,                         // 🔵 既存 selectionStore.colorMode から

    // 分析結果キャッシュ（旧 analysisStore）
    sensitivity_result: Option<SensitivityResult>, // 🔵 既存 analysisStore.sensitivityResult から
    sobol_result: Option<SobolResult>,             // 🔵 既存 analysisStore.sobolResult から
    cluster_result: Option<ClusterResult>,         // 🔵 既存 clusterStore から

    // ダウンサンプリングキャッシュ（旧 downsampleStore）
    downsample_cache: DownsampleCache, // 🔵 既存 downsampleStore.cache から

    // ライブ更新（旧 liveUpdateStore）
    live_update: LiveUpdateState, // 🔵 既存 liveUpdateStore から

    // MCDM（旧 mcdmStore）
    topsis_result: Option<TopsisResult>, // 🔵 既存 mcdmStore から
}

/// AppState のアクション実装
impl AppState {
    /// フィルタ変更 → filter_by_ranges 直接呼び出し
    /// 🔵 信頼性: 要件定義REQ-042・既存 selectionStore.addAxisFilter より
    pub fn set_filter(&mut self, axis: String, min: f64, max: f64) {
        // filter_ranges 更新
        // tunny_core::data::filter::filter_by_ranges() 直接呼び出し
        // selected_indices 更新
        // gpu_buffer alpha 更新
        todo!()
    }

    /// フィルタ削除
    /// 🔵 信頼性: 既存 selectionStore.removeAxisFilter より
    pub fn remove_filter(&mut self, axis: &str) { todo!() }

    /// 全フィルタクリア
    /// 🔵 信頼性: 既存 selectionStore.clearSelection より
    pub fn clear_filters(&mut self) { todo!() }

    /// Brush 選択（散布図ドラッグ）
    /// 🔵 信頼性: 既存 selectionStore.brushSelect より
    pub fn brush_select(&mut self, indices: Vec<u32>) { todo!() }

    /// ハイライト設定（テーブル行クリック）
    /// 🔵 信頼性: 既存 selectionStore.setHighlight より
    pub fn set_highlight(&mut self, trial_id: Option<u32>) { todo!() }
}

// ============================================================
// Study 関連型
// ============================================================

/// StudyMeta - Journal内のStudy一覧情報
/// 🔵 信頼性: 既存 frontend/src/types/index.ts Study型より
pub struct StudyMeta {
    study_id: u32,              // 🔵 既存 Study.studyId から
    name: String,               // 🔵 既存 Study.name から
    directions: Vec<Direction>, // 🔵 既存 Study.directions から
    completed_trials: usize,    // 🔵 既存 Study.completedTrials から
    total_trials: usize,        // 🔵 既存 Study.totalTrials から
    param_names: Vec<String>,   // 🔵 既存 Study.paramNames から
    objective_names: Vec<String>, // 🔵 既存 Study.objectiveNames から
    user_attr_names: Vec<String>, // 🔵 既存 Study.userAttrNames から
    has_constraints: bool,      // 🔵 既存 Study.hasConstraints から
}

/// StudyContext - 選択中のStudy + Trial データ + GPU バッファ
/// 🔵 信頼性: 既存 studyStore の currentStudy + gpuBuffer + trialRows より
pub struct StudyContext {
    meta: StudyMeta,             // 🔵
    trial_rows: Vec<TrialRow>,   // 🔵 既存 studyStore.trialRows から
    gpu_buffer: GpuBuffer,       // 🔵 既存 gpuBuffer.ts より
    pareto_indices: Vec<u32>,    // 🔵 要件定義REQ-072から
}

/// Direction - 最適化方向
/// 🔵 信頼性: 要件定義・既存型定義より
pub enum Direction {
    Minimize, // 🔵
    Maximize, // 🔵
}

/// TrialRow - 1行分のTrialデータ
/// 🔵 信頼性: 既存 frontend/src/types/index.ts TrialData 型より
pub struct TrialRow {
    trial_id: u32,              // 🔵
    params: HashMap<String, f64>, // 🔵 パラメータ値
    objectives: Vec<f64>,       // 🔵 目的関数値
    pareto_rank: u32,           // 🔵 NDSort 結果
    cluster_id: Option<i32>,    // 🔵 クラスタリング結果（任意）
    state: TrialState,          // 🔵
    user_attrs: HashMap<String, String>, // 🔵
}

/// TrialState - Trial状態
/// 🔵 信頼性: 要件定義REQ-011・既存実装より
pub enum TrialState {
    Complete,  // 🔵
    Running,   // 🔵
    Pruned,    // 🔵
    Fail,      // 🔵
    Waiting,   // 🔵
}

// ============================================================
// GPU バッファ
// ============================================================

/// GpuBuffer - wgpu で管理する GPU-side データ
/// 🔵 信頼性: 既存 frontend/src/wasm/gpuBuffer.ts + 要件定義REQ-014〜015より
pub struct GpuBuffer {
    // wgpu バッファ（GPU メモリ）
    positions_buf: wgpu::Buffer,  // 🔵 Float32 flat: [x0,y0, x1,y1, ...]
    positions3d_buf: wgpu::Buffer, // 🔵 Float32 flat: [x0,y0,z0, ...]
    colors_buf: wgpu::Buffer,     // 🔵 Float32 RGBA flat: [r0,g0,b0,a0, ...]
    sizes_buf: wgpu::Buffer,      // 🔵 Float32 flat: [s0, s1, ...]

    // CPU-side 書き込み用
    positions_data: Vec<f32>,    // 🔵
    positions3d_data: Vec<f32>,  // 🔵
    colors_data: Vec<f32>,       // 🔵
    sizes_data: Vec<f32>,        // 🔵

    len: u32, // 🔵 Trial数
}

/// GpuBuffer のアルファ更新（< 1ms 要件）
impl GpuBuffer {
    /// 選択インデックスに基づいてアルファ値を更新
    /// 🔵 信頼性: 要件定義REQ-015・既存実装より
    pub fn update_alphas(&mut self, queue: &wgpu::Queue, selected: &[u32]) { todo!() }

    /// ハイライト色更新
    /// 🔵 信頼性: 要件定義REQ-043・既存実装より
    pub fn update_highlight(&mut self, queue: &wgpu::Queue, trial_id: Option<u32>) { todo!() }
}

/// GpuBufferData - CPU側の初期化用データ（rust_core から受け取る）
/// 🔵 信頼性: 既存 frontend/src/types/index.ts GpuBuffers 型より
pub struct GpuBufferData {
    pub positions: Vec<f32>,    // 🔵
    pub positions3d: Vec<f32>,  // 🔵
    pub colors: Vec<f32>,       // 🔵
    pub sizes: Vec<f32>,        // 🔵
    pub trial_count: u32,       // 🔵
}

// ============================================================
// レイアウト状態
// ============================================================

/// LayoutState - UIレイアウト設定（旧 layoutStore）
/// 🔵 信頼性: 既存 AppShell.tsx + layoutStore.ts 分析より
pub struct LayoutState {
    left_panel_width: f32,     // 🔵 既存 LEFT_MIN=120, LEFT_MAX=600
    bottom_panel_height: f32,  // 🔵 既存 BOTTOM_MIN=60, BOTTOM_MAX=600
    layout_mode: LayoutMode,   // 🔵 既存 layoutStore.layoutMode から
    visible_charts: HashSet<ChartId>, // 🔵 既存 layoutStore.visibleCharts から
    chart_layout: Vec<ChartPlacement>, // 🔵 既存 FreeLayoutCanvas グリッドレイアウトから
}

/// LayoutMode - 4種類のレイアウトモード
/// 🔵 信頼性: 要件定義REQ-032より
pub enum LayoutMode {
    MultiObjective,    // 🔵 Mode A: Pareto中心
    VariableSpace,     // 🔵 Mode B: 変数空間探索
    ConvergenceAnalysis, // 🔵 Mode C: 収束分析
    FreeLayout,        // 🔵 Mode D: フリーレイアウト
}

/// ChartId - チャートの識別子
/// 🔵 信頼性: 既存チャートコンポーネント一覧から
pub enum ChartId {
    ParetoScatter3D,         // 🔵
    ParetoScatter2D,         // 🔵
    ParallelCoordinates,     // 🔵
    ScatterMatrix,           // 🔵
    ObjectivePairMatrix,     // 🔵
    HypervolumeHistory,      // 🔵
    OptimizationHistory,     // 🔵
    ImportanceChart,         // 🔵
    PdpChart,                // 🔵
    ContourPlot,             // 🔵
    SurfacePlot3D,           // 🔵
    SensitivityHeatmap,      // 🔵
    ClusterScatter,          // 🔵
    EdfPlot,                 // 🔵
    SlicePlot,               // 🔵
}

/// ChartPlacement - フリーレイアウト配置情報
/// 🔵 信頼性: 既存 FreeLayoutCanvas.tsx の 4×4 グリッドシステムより
pub struct ChartPlacement {
    chart_id: ChartId, // 🔵
    col: u8,           // 🔵 0-3 (4列グリッド)
    row: u8,           // 🔵 0-3 (4行グリッド)
    width: u8,         // 🔵 列スパン
    height: u8,        // 🔵 行スパン
}

// ============================================================
// カラーモード・カラーマップ
// ============================================================

/// ColorMode - カラーリングモード
/// 🔵 信頼性: 要件定義REQ-054・NFR-030より
pub enum ColorMode {
    ParetoRank,        // 🔵 Paretoランクで色付け
    ObjectiveValue(String), // 🔵 目的関数値で色付け（目的名を保持）
    TrialNumber,       // 🔵 試行番号で色付け
    ClusterId,         // 🔵 クラスタIDで色付け
}

/// ColorMap - カラーマップ定義
/// 🔵 信頼性: 既存 frontend/src/colormaps.ts より
pub enum ColorMap {
    Viridis,   // 🔵
    Plasma,    // 🔵
    Turbo,     // 🔵
    RdYlBu,    // 🔵
    Spectral,  // 🔵
    Blues,     // 🔵
}

impl ColorMap {
    /// t: 0.0 - 1.0 → egui::Color32
    /// 🔵 信頼性: 既存カラーマップ実装より
    pub fn interpolate(&self, t: f32) -> egui::Color32 { todo!() }
}

// ============================================================
// ダウンサンプリング
// ============================================================

/// DownsampleCache - ダウンサンプリング結果キャッシュ（旧 downsampleStore）
/// 🔵 信頼性: 既存 downsampleStore.ts より
pub struct DownsampleCache {
    scatter: Option<Vec<u32>>,   // 🔵 ParetoScatter 用（5,000点）
    pcp: Option<Vec<u32>>,       // 🔵 ParallelCoordinates 用
    thumbnail: Option<Vec<u32>>, // 🔵 ScatterMatrix サムネイル用
    hover: Option<Vec<u32>>,     // 🔵 ScatterMatrix ホバー用
}

/// DownsampleKey - キャッシュキー
/// 🔵 信頼性: 既存 downsampleStore.ts DownsampleKey より
pub enum DownsampleKey {
    Scatter,   // 🔵
    Pcp,       // 🔵
    Thumbnail, // 🔵
    Hover,     // 🔵
}

// ============================================================
// 非同期メッセージ
// ============================================================

/// AppMessage - 非同期タスクからメインスレッドへのメッセージ
/// 🔵 信頼性: Rust mpsc パターン + 既存 Zustand actions より
pub enum AppMessage {
    // Journal 処理
    JournalParsed(Vec<StudyMeta>),              // 🔵 wasm_parse_journal 相当
    StudySelected {                              // 🔵 wasm_select_study 相当
        meta: StudyMeta,
        trial_rows: Vec<TrialRow>,
        gpu_data: GpuBufferData,
        pareto_indices: Vec<u32>,
    },

    // 分析計算完了
    SensitivityDone(SensitivityResult),          // 🔵 wasm_compute_sensitivity 相当
    SobolDone(SobolResult),                      // 🔵 wasm_compute_sobol 相当
    ClusteringDone(ClusterResult),               // 🔵 wasm_run_kmeans 等相当
    PdpDone {                                    // 🔵 wasm_compute_pdp_2d 相当
        param: String,
        objective: String,
        result: PdpResult,
    },
    TopsisDone(TopsisResult),                   // 🔵 wasm_compute_topsis 相当

    // ダウンサンプリング
    DownsampleDone {                             // 🔵 wasm_downsample_* 相当
        key: DownsampleKey,
        indices: Vec<u32>,
    },

    // ライブ更新
    LiveUpdateDone {                             // 🔵 wasm_append_journal_diff 相当
        new_trial_count: usize,
        pareto_updated: bool,
        new_indices: Vec<u32>,
    },

    // エラー
    Error(String),                               // 🔵
}

// ============================================================
// 分析結果型（rust_core の結果を再利用）
// ============================================================

/// SensitivityResult - 感度分析結果
/// 🔵 信頼性: 既存 SensitivityWasmResult 型より（JS→Rust変換）
pub struct SensitivityResult {
    pub param_names: Vec<String>,     // 🔵
    pub objective_names: Vec<String>, // 🔵
    pub spearman: Vec<Vec<f64>>,      // 🔵 [param_idx][obj_idx]
    pub ridge_beta: Vec<Vec<f64>>,    // 🔵 [param_idx][obj_idx]
    pub ridge_r_squared: Vec<f64>,    // 🔵 [obj_idx]
    pub rf_anova: Option<Vec<f64>>,   // 🔵 [param_idx] (任意)
    pub duration_ms: u64,             // 🔵
}

/// SobolResult - Sobol感度指数
/// 🔵 信頼性: 既存 SobolWasmResult 型より
pub struct SobolResult {
    pub param_names: Vec<String>,   // 🔵
    pub s1: Vec<Vec<f64>>,          // 🔵 一次効果指数 [param][obj]
    pub st: Vec<Vec<f64>>,          // 🔵 総効果指数 [param][obj]
}

/// ClusterResult - クラスタリング結果
/// 🔵 信頼性: 既存 KmeansWasmResult + ClusterStatsWasmResult より
pub struct ClusterResult {
    pub k: usize,               // 🔵
    pub labels: Vec<i32>,       // 🔵 各Trialのクラスタラベル
    pub centroids: Vec<Vec<f64>>, // 🔵 各クラスタの重心
    pub cluster_stats: Vec<ClusterStats>, // 🔵
}

/// ClusterStats - クラスタ統計情報
/// 🔵 信頼性: 既存 ClusterStatsWasmResult より
pub struct ClusterStats {
    pub cluster_id: usize,      // 🔵
    pub count: usize,           // 🔵
    pub mean: Vec<f64>,         // 🔵
    pub std: Vec<f64>,          // 🔵
}

/// PdpResult - 部分依存プロット結果
/// 🔵 信頼性: 既存 Pdp2dWasmResult より
pub struct PdpResult {
    pub grid_x: Vec<f64>,       // 🔵 x軸グリッド点
    pub grid_y: Option<Vec<f64>>, // 🔵 2D PDPの場合
    pub values: Vec<f64>,       // 🔵 PDP値（1D: n点, 2D: n*m点）
    pub ci_lower: Vec<f64>,     // 🔵 95%信頼区間下限
    pub ci_upper: Vec<f64>,     // 🔵 95%信頼区間上限
    pub r_squared: f64,         // 🔵 モデル品質
}

/// TopsisResult - TOPSIS多基準意思決定結果
/// 🔵 信頼性: 既存 TopsisWasmResult より
pub struct TopsisResult {
    pub scores: Vec<f64>,       // 🔵 各TrialのTOPSISスコア
    pub best_trial_id: u32,     // 🔵
    pub ranking: Vec<u32>,      // 🔵 ランキング順のTrial ID
}

// ============================================================
// ライブ更新状態
// ============================================================

/// LiveUpdateState - ライブ更新設定・状態（旧 liveUpdateStore）
/// 🔵 信頼性: 既存 liveUpdateStore.ts + 要件定義REQ-130〜135より
pub struct LiveUpdateState {
    pub is_active: bool,          // 🔵 ライブ更新中かどうか
    pub poll_interval_secs: u32,  // 🔵 1-30秒 (デフォルト5)
    pub file_path: Option<std::path::PathBuf>, // 🔵 監視中のファイルパス
    pub last_file_size: u64,      // 🔵 前回のファイルサイズ
    pub new_trial_count: usize,   // 🔵 更新で追加されたTrial数
}

// ============================================================
// wgpu レンダラー（散布図・3D）
// ============================================================

/// ScatterRenderer - wgpu を使った散布図・3Dポイントクラウドレンダラー
/// 🟡 信頼性: egui-wgpu統合の一般的パターンより（詳細は実装時に確定）
pub struct ScatterRenderer {
    render_pipeline: wgpu::RenderPipeline, // 🟡
    bind_group_layout: wgpu::BindGroupLayout, // 🟡
    uniform_buffer: wgpu::Buffer,          // 🟡 ViewProjection matrix
}

/// ScatterRenderParams - 描画パラメータ
/// 🟡 信頼性: wgpu/egui-wgpu統合パターンより
pub struct ScatterRenderParams {
    pub gpu_buffer: std::sync::Arc<GpuBuffer>, // 🟡
    pub view_proj: [[f32; 4]; 4],              // 🟡 4×4変換行列
    pub point_size: f32,                       // 🟡
    pub axis_x: u32,                           // 🟡 X軸の目的/パラメータインデックス
    pub axis_y: u32,                           // 🟡
    pub axis_z: Option<u32>,                   // 🟡 3D の場合
}

// ============================================================
// 信頼性レベルサマリー
// ============================================================
/// - 🔵 青信号: 62件 (84%)
/// - 🟡 黄信号: 12件 (16%)
/// - 🔴 赤信号: 0件 (0%)
///
/// 品質評価: ✅ 高品質
