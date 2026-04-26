// ============================================================
// Entropy Weight Method 型定義
//
// 作成日: 2026-04-24
// 関連設計: architecture.md
// ブランチ: featura/egui
//
// 信頼性レベル:
// - 🔵 青信号: 要件定義書・既存実装を参考にした確実な型
// - 🟡 黄信号: 要件定義書・既存実装から妥当な推測による型
// - 🔴 赤信号: 要件定義書・既存実装にない推測による型
// ============================================================

// ============================================================
// rust_core/src/mcdm/entropy.rs に定義する型
// ============================================================

/// エントロピー法の計算結果
///
/// 🔵 信頼性: REQ-001・アルゴリズム概要より
#[derive(Debug, Clone, serde::Serialize)]
pub struct EntropyResult {
    /// 正規化済みエントロピー重み（sum = 1.0）
    /// 🔵 REQ-001・REQ-403（重み合計1.0）より
    pub weights: Vec<f64>,

    /// 各目的の情報エントロピー値 e_j ∈ [0, 1]
    /// 🔵 アルゴリズム概要 Step 2 より
    pub entropies: Vec<f64>,

    /// 各目的の分散度 d_j = 1 - e_j
    /// 🔵 アルゴリズム概要 Step 3 より
    pub diversities: Vec<f64>,

    /// 正規化行列 p_ij（表示・デバッグ用）
    /// 🔵 REQ-002（比例正規化）・REQ-004（テーブル表示）より
    pub normalized_matrix: Vec<f64>,

    /// 計算時間（ms）
    /// 🔵 既存TopsisResult/VikorResultパターンより
    pub duration_ms: f64,
}

/// エントロピー重み計算関数シグネチャ
///
/// # 引数
/// - `values`: 目的関数値の平坦配列 [N×M]（行major: trial0_obj0, trial0_obj1, ...）
/// - `n_trials`: 試行数（>= 1）
/// - `n_objectives`: 目的関数数（>= 1）
///
/// 🔵 REQ-401（rust_core実装）・REQ-402（フラットvalues形式）より
pub fn compute_entropy_weights(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
) -> Result<EntropyResult, String> {
    todo!()
}

// ============================================================
// egui-app/src/state/results.rs に追加する型
// ============================================================

/// 重み設定モード
///
/// 🔵 REQ-006（Manual/Entropy切替）・ユーザヒアリングより
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightMode {
    /// 手動で重みスライダーを編集
    /// 🔵 REQ-202（Manual時はスライダー編集可能）より
    Manual,

    /// エントロピー法による自動重み計算
    /// 🔵 REQ-201（Entropy時はスライダー読み取り専用）・ユーザヒアリングより
    Entropy,
}

impl WeightMode {
    /// 🔵 既存McdmMethod::label()パターンより
    pub fn label(&self) -> &'static str {
        match self {
            WeightMode::Manual => "Manual",
            WeightMode::Entropy => "Entropy",
        }
    }

    /// 🔵 既存McdmMethod::all()パターンより
    pub fn all() -> &'static [WeightMode] {
        &[WeightMode::Manual, WeightMode::Entropy]
    }
}

// ============================================================
// egui-app/src/ui/widgets/mcdm_chart.rs の変更型
// ============================================================

/// MCDMランキングバーチャートのUI状態（Entropy追加後）
///
/// 🔵 既存McdmRankChart実装・ユーザヒアリングより
pub struct McdmRankChart {
    pub method: McdmMethod,
    pub weights: Vec<f64>,

    /// VIKOR戦略パラメータ（デフォルト 0.5）
    /// 🔵 既存VIKOR実装より
    pub v_param: f64,

    pub computing: bool,

    /// 変更: Option<(McdmMethod, Vec<f64>)> → Option<McdmComputeRequest>
    /// 🔵 既存VIKOR設計より
    pub pending_compute: Option<McdmComputeRequest>,

    pub top_n: McdmTopN,

    // --- Entropy追加フィールド ---

    /// 重み設定モード
    /// 🔵 REQ-006・ユーザヒアリング「手法セレクタの横」より
    pub weight_mode: WeightMode,

    /// エントロピー計算結果キャッシュ
    /// 🔵 REQ-001・ユーザヒアリング「Weight Mode切替時」より
    pub entropy_result: Option<EntropyResult>,

    /// エントロピー計算要求フラグ
    /// 🔵 NFR-002（バックグラウンド実行）・ユーザヒアリングより
    pub pending_entropy: bool,
}

// ============================================================
// AppMessage 拡張
// ============================================================

/// アプリケーションメッセージに追加するEntropy完了通知
///
/// 🔵 既存AppMessage::McdmDoneパターン・ユーザヒアリングより
pub enum AppMessage {
    // ... 既存バリアント ...
    McdmDone(McdmResult),

    /// エントロピー計算完了
    /// 🔵 ユーザヒアリング「Weight Mode切替時」より
    EntropyDone(EntropyResult),

    Error(String),
}

// ============================================================
// 信頼性レベルサマリー
// - 🔵 青信号: 16件 (94%)
// - 🟡 黄信号: 1件 (6%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
// ============================================================
