// AHP 型定義
//
// 作成日: 2026-04-29
// 関連設計: architecture.md
//
// 信頼性レベル:
// 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
// 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
// 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義

// ============================================================
// rust_core/src/mcdm/ahp.rs — アルゴリズム層の型
// ============================================================

/// AHP 整合性指標 ランダム整合性指標テーブル
/// 🔵 REQ-AHP-006・note.md AHP アルゴリズム概要より
const RI_TABLE: [f64; 6] = [0.0, 0.0, 0.58, 0.90, 1.12, 1.24];
//                           n=1  n=2  n=3   n=4   n=5   n≥6(近似)

/// AHP 計算結果
/// 🔵 REQ-AHP-002・要件定義書 REQ-AHP-005〜008 より
#[derive(Debug, Clone, serde::Serialize)]
pub struct AhpResult {
    /// 優先度ベクトル (重み): len = n_objectives, Σ ≈ 1.0
    /// 🔵 REQ-AHP-005 より
    pub priority_vector: Vec<f64>,

    /// AHP スコア: len = n_trials, [0.0, 1.0], NaN 試行は 0.0
    /// 🔵 REQ-AHP-007 より
    pub scores: Vec<f64>,

    /// ランキング: scores 降順、NaN 試行は末尾, len = n_trials
    /// 🔵 REQ-AHP-008 より
    pub ranked_indices: Vec<u32>,

    /// 最大固有値近似値 λmax
    /// 🔵 REQ-AHP-006 より
    pub lambda_max: f64,

    /// 整合性指標 CI = (λmax - n) / (n - 1)
    /// 🔵 REQ-AHP-006 より
    pub ci: f64,

    /// ランダム整合性指標 RI（Saaty テーブル参照）
    /// 🔵 REQ-AHP-006 より
    pub ri: f64,

    /// 整合性比率 CR = CI / RI
    /// 🔵 REQ-AHP-006 より
    pub cr: f64,

    /// 整合性判定: CR <= 0.10 なら true
    /// 🔵 REQ-AHP-006・ユーザヒアリングより
    pub is_consistent: bool,

    /// 計算時間 (ms)
    /// 🔵 既存 TopsisResult / VikorResult / PrometheeResult パターンより
    pub duration_ms: f64,
}

/// compute_ahp の公開シグネチャ
/// 🔵 REQ-AHP-001 より
///
/// # Arguments
/// - `values`: フラット行優先 Vec<f64>, len = n_trials × n_objectives
/// - `n_trials`: 試行数
/// - `n_objectives`: 目的関数数 (≥ 1)
/// - `pairwise_matrix`: 上三角のみ, row-major, len = n_objectives*(n_objectives-1)/2
///                      n=1 の場合は空スライス
/// - `is_minimize`: 最小化フラグ, len = n_objectives
///
/// # Returns
/// - `Ok(AhpResult)` 計算成功
/// - `Err(String)` バリデーションエラー
pub fn compute_ahp(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    pairwise_matrix: &[f64],
    is_minimize: &[bool],
) -> Result<AhpResult, String> {
    todo!("実装: rust_core/src/mcdm/ahp.rs")
}

// ============================================================
// egui-app/src/state/results.rs — 型・状態管理層の変更
// ============================================================

/// AHP 計算結果 (egui-app 側コピー)
/// 🔵 REQ-AHP-010・既存 TopsisResult/VikorResult/PrometheeResult パターンより
///
/// rust_core 側 AhpResult と同一フィールド構成。
/// chart_registry.rs でフィールドコピーして生成する。
#[derive(Debug, Clone)]
pub struct AhpResult {
    pub priority_vector: Vec<f64>,
    pub scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub lambda_max: f64,
    pub ci: f64,
    pub ri: f64,
    pub cr: f64,
    pub is_consistent: bool,
    pub duration_ms: f64,
}

// ============================================================
// egui-app/src/state/messages.rs — メッセージ型の変更
// ============================================================

/// AppMessage への追加バリアント（McdmDone とは完全独立）
/// 🔵 REQ-AHP-013・note.md より
///
/// 既存の AppMessage enum に以下を追加:
///
/// ```rust
/// AppMessage::AhpDone(AhpResult),
/// ```

// ============================================================
// egui-app/src/state/app_state.rs — AppState の変更
// ============================================================

/// AppState への追加フィールド
/// 🔵 REQ-AHP-012・既存 mcdm_result パターンより
///
/// 既存の AppState struct に以下を追加:
///
/// ```rust
/// pub ahp_result: Option<AhpResult>,
/// ```

// ============================================================
// egui-app/src/state/layout_state.rs — ChartId の変更
// ============================================================

/// ChartId への追加バリアント
/// 🔵 REQ-AHP-014・ユーザヒアリングより
///
/// 既存の ChartId enum に以下を追加:
///
/// ```rust
/// /// AHP: 一対比較行列入力 + CR 表示 + 優先度ベクトルバーチャート
/// ChartId::AhpRankChart,
///
/// /// AHP: ランキングテーブル (Top5/10/20)
/// ChartId::AhpTable,
/// ```

// ============================================================
// egui-app/src/ui/widgets/ahp_chart.rs — UI 層の新規型
// ============================================================

/// AHP 計算リクエスト
/// 🔵 REQ-AHP-013・既存 McdmComputeRequest パターンより
pub struct AhpComputeRequest {
    /// フラット行優先, len = n_trials × n_objectives
    pub objectives: Vec<f64>,
    pub n_trials: usize,
    pub n_objectives: usize,
    /// 上三角のみ (row-major), len = n_objectives*(n_objectives-1)/2
    pub pairwise_matrix: Vec<f64>,
    pub is_minimize: Vec<bool>,
}

/// Top N 選択
/// 🔵 REQ-AHP-025・ユーザストーリー 2.1 より
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AhpTopN {
    #[default]
    Top5,
    Top10,
    Top20,
}

impl AhpTopN {
    pub fn count(&self) -> usize {
        match self {
            AhpTopN::Top5 => 5,
            AhpTopN::Top10 => 10,
            AhpTopN::Top20 => 20,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AhpTopN::Top5 => "Top 5",
            AhpTopN::Top10 => "Top 10",
            AhpTopN::Top20 => "Top 20",
        }
    }

    /// 🔵 UI コンボボックス用
    pub fn all() -> &'static [AhpTopN] {
        &[AhpTopN::Top5, AhpTopN::Top10, AhpTopN::Top20]
    }
}

/// AHP チャートウィジェット（AhpRankChart + AhpTable 両方を担当）
/// 🔵 REQ-AHP-020・ユーザヒアリング（同一ウィジェット集約）より
#[derive(Debug, Default)]
pub struct AhpChart {
    /// 一対比較行列 上三角 (row-major)
    /// n=1: len=0, n=2: len=1, n=3: len=3, n=4: len=6
    /// 🔵 REQ-AHP-021 より
    pub pairwise: Vec<f64>,

    /// 計算中フラグ (Run ボタン disabled 制御)
    /// 🔵 既存 McdmRankChart.computing パターンより
    pub computing: bool,

    /// 計算リクエストキュー（chart_registry が取り出す）
    /// 🔵 既存 pending_compute パターンより
    pub pending_compute: Option<AhpComputeRequest>,

    /// テーブル表示件数選択
    /// 🔵 REQ-AHP-025 より
    pub top_n: AhpTopN,
}

impl AhpChart {
    /// Study が n_objectives 個の目的関数を持つ場合の初期化
    /// 🔵 REQ-AHP-027・ユーザストーリー 3.1 より
    pub fn reset_for_objectives(n_objectives: usize) -> Self {
        let upper_len = n_objectives * n_objectives.saturating_sub(1) / 2;
        Self {
            pairwise: vec![1.0; upper_len],
            computing: false,
            pending_compute: None,
            top_n: AhpTopN::default(),
        }
    }

    /// 上三角インデックス変換: (row i, col j) where i < j → pairwise index
    /// 🔵 REQ-AHP-021 より
    pub fn upper_tri_index(n: usize, i: usize, j: usize) -> usize {
        debug_assert!(i < j && j < n);
        i * (2 * n - i - 1) / 2 + (j - i - 1)
    }
}

// ============================================================
// egui-app/src/ui/widget_states.rs — WidgetStates の変更
// ============================================================

/// WidgetStates への追加フィールド
/// 🔵 REQ-AHP-020・既存 mcdm_chart パターンより
///
/// 既存の WidgetStates struct に以下を追加:
///
/// ```rust
/// pub ahp_chart: AhpChart,
/// ```

// ============================================================
// UI 定数
// ============================================================

/// AHP UI 定数
/// 🔵 REQ-AHP-022, REQ-AHP-022-A・ユーザヒアリングより
pub mod ahp_ui {
    /// CR ≤ 0.10: 整合性あり表示色
    pub const CONSISTENT_COLOR: egui::Color32 = egui::Color32::GREEN;

    /// CR > 0.10: 不整合警告表示色
    pub const INCONSISTENT_COLOR: egui::Color32 = egui::Color32::RED;

    /// 一対比較 DragValue の最小値
    pub const PAIRWISE_MIN: f64 = 1.0;

    /// 一対比較 DragValue の最大値 (Saaty スケール)
    /// 🔵 REQ-AHP-021-C・ユーザヒアリングより
    pub const PAIRWISE_MAX: f64 = 9.0;

    /// 一対比較 DragValue のステップ
    /// 🟡 実装容易性から妥当な推測
    pub const PAIRWISE_STEP: f64 = 0.5;

    /// 整合性あり ラベル
    pub const CONSISTENT_LABEL: &str = "✓ Consistent";

    /// 不整合 警告ラベル
    pub const INCONSISTENT_LABEL: &str = "⚠ Inconsistent (CR > 0.10)";
}
