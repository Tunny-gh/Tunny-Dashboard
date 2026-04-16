//! 責務分離リファクタリング 型定義
//!
//! 作成日: 2026-04-15
//! 関連設計: architecture.md
//!
//! 信頼性レベル:
//! - 🔵 青信号: 既存コード・ユーザヒアリングを参考にした確実な型定義
//! - 🟡 黄信号: 既存コード・ユーザヒアリングから妥当な推測による型定義
//! - 🔴 赤信号: 既存コード・ユーザヒアリングにない推測による型定義

// ========================================
// state/types.rs — データモデル型
// ========================================

/// 最適化方向 🔵 信頼性: 既存 app_state.rs:8-11 より
#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Minimize,
    Maximize,
}

/// Trial の状態 🔵 信頼性: 既存 app_state.rs:13-20 より
#[derive(Debug, Clone, PartialEq)]
pub enum TrialState {
    Complete,
    Running,
    Pruned,
    Fail,
    Waiting,
}

/// Study のメタ情報 🔵 信頼性: 既存 app_state.rs:23-33 より
#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<Direction>,
    pub completed_trials: usize,
    pub total_trials: usize,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
}

/// 1つの Trial の行データ 🔵 信頼性: 既存 app_state.rs:36-44 より
#[derive(Debug, Clone)]
pub struct TrialRow {
    pub trial_id: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,
    pub cluster_id: Option<i32>,
    pub state: TrialState,
    pub user_attrs: HashMap<String, String>,
}

/// GPU バッファデータ 🔵 信頼性: 既存 app_state.rs:47-53 より
#[derive(Debug, Clone)]
pub struct GpuBufferData {
    pub positions: Vec<f32>,
    pub positions3d: Vec<f32>,
    pub colors: Vec<f32>,
    pub sizes: Vec<f32>,
    pub trial_count: u32,
}

/// 現在選択中の Study コンテキスト 🔵 信頼性: 既存 app_state.rs:56-61 より
#[derive(Debug, Clone)]
pub struct StudyContext {
    pub meta: StudyMeta,
    pub trial_rows: Vec<TrialRow>,
    pub gpu_data: GpuBufferData,
    pub pareto_indices: Vec<u32>,
}

impl StudyContext {
    /// パラメータのデータ範囲 [min, max] を返す 🔵 信頼性: 既存 app_state.rs:65-82 より
    pub fn param_range(&self, param_name: &str) -> (f64, f64) {
        let values: Vec<f64> = self
            .trial_rows
            .iter()
            .filter_map(|r| r.params.get(param_name).copied())
            .collect();
        if values.is_empty() {
            return (0.0, 1.0);
        }
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        }
    }
}

/// 色分けモード 🔵 信頼性: 既存 app_state.rs:84-101 より
#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    ParetoRank,
    ObjectiveValue(String),
    TrialNumber,
    ClusterId,
}

impl ColorMode {
    pub fn label(&self) -> &str {
        match self {
            ColorMode::ParetoRank => "Pareto Rank",
            ColorMode::ObjectiveValue(_) => "Objective",
            ColorMode::TrialNumber => "Trial Number",
            ColorMode::ClusterId => "Cluster ID",
        }
    }
}

// ========================================
// state/filter.rs — フィルタリング関連型
// ========================================

/// ダウンサンプリングキャッシュ 🔵 信頼性: 既存 app_state.rs:152-166 より
#[derive(Debug, Clone, Default)]
pub struct DownsampleCache {
    pub scatter: Option<Vec<u32>>,
    pub pcp: Option<Vec<u32>>,
    pub thumbnail: Option<Vec<u32>>,
    pub hover: Option<Vec<u32>>,
}

impl DownsampleCache {
    pub fn clear(&mut self) {
        self.scatter = None;
        self.pcp = None;
        self.thumbnail = None;
        self.hover = None;
    }
}

/// 選択率の変化が再サンプリングをトリガーすべきか判定する
/// 🔵 信頼性: 既存 app_state.rs:169-171 より
pub fn should_resample(current_rate: f64, last_rate: f64) -> bool {
    (current_rate - last_rate).abs() > 0.20
}

// ========================================
// state/results.rs — 分析結果型
// ========================================

/// 感度分析結果 🔵 信頼性: 既存 app_state.rs:108-114 より
#[derive(Debug, Clone)]
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
}

/// Ridge回帰結果 🔵 信頼性: 既存 app_state.rs:117-120 より
#[derive(Debug, Clone)]
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

/// Random Forest ANOVA結果 🔵 信頼性: 既存 app_state.rs:123-125 より
#[derive(Debug, Clone)]
pub struct RfAnovaResult {
    pub importances: Vec<Vec<f64>>,
}

/// Sobol 感度指標 🔵 信頼性: 既存 app_state.rs:128-133 より
#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
}

/// クラスタリング結果 🔵 信頼性: 既存 app_state.rs:136-139 より
#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub labels: Vec<i32>,
    pub n_clusters: usize,
}

/// TOPSIS 分析結果 🔵 信頼性: 既存 app_state.rs:142-145 より
#[derive(Debug, Clone)]
pub struct TopsisResult {
    pub scores: Vec<f64>,
    pub ranking: Vec<usize>,
}

/// Hypervolume 推移データ 🔵 信頼性: 既存 app_state.rs:201-205 より
#[derive(Debug, Clone)]
pub struct HvHistory {
    pub trial_ids: Vec<u32>,
    pub hv_values: Vec<f64>,
}

/// ライブ更新状態 🔵 信頼性: 既存 app_state.rs:178-194 より
#[derive(Debug, Clone)]
pub struct LiveUpdateState {
    pub enabled: bool,
    pub file_path: Option<String>,
    pub last_byte_offset: u64,
    pub interval_ms: u64,
}

impl Default for LiveUpdateState {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: None,
            last_byte_offset: 0,
            interval_ms: 2000,
        }
    }
}

// ========================================
// state/message_handler.rs — メッセージ処理
// ========================================

/// メッセージ処理ハンドラー
/// 🔵 信頼性: ユーザーヒアリング（MessageHandler抽出）・既存 app.rs:38-124 より
pub struct MessageHandler;

impl MessageHandler {
    /// 単一メッセージを処理し AppState と WidgetStates を更新する
    /// 🔵 信頼性: 既存 app.rs:poll_messages の抽出より
    pub fn handle(
        msg: AppMessage,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
        load_error: &mut Option<String>,
    ) {
        // 実装は architecture.md Phase 3 を参照
    }
}

// ========================================
// ui/chart_registry.rs — チャートディスパッチ
// ========================================

/// ChartId に対応するチャートを描画するレジストリ関数
/// 🔵 信頼性: ユーザーヒアリング（レジストリパターン）・既存 grid_canvas.rs:257-371 より
pub fn show_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    // 実装は architecture.md Phase 2 を参照
}

/// タイトル付きチャートセル描画
/// 🔵 信頼性: 既存 grid_canvas.rs:258-268 より
pub fn show_cell_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    ui.label(egui::RichText::new(chart_id.label()).strong());
    ui.separator();
    show_chart(ui, app_state, widgets, chart_id, tx);
}

// ========================================
// 信頼性レベルサマリー
// ========================================
//
// - 🔵 青信号: 22件 (100%)
// - 🟡 黄信号: 0件 (0%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
// 全型定義は既存コードから機械的に抽出するため、推測なし
