// ============================================================
// VIKOR 型定義
//
// 作成日: 2026-04-24
// 関連設計: architecture.md
// ブランチ: featura/egui
//
// 信頼性レベル:
// - 🔵 青信号: 要件定義書・既存実装を参考にした確実な型
// - 🟡 黄信号: 要件定義書・既存実装から妥当な推測による型
// - 🔴 赤信号: ヒアリングにない推測による型
// ============================================================

// ============================================================
// rust_core/src/mcdm/vikor.rs に定義する型
// ============================================================

/// VIKORアルゴリズムの計算結果
///
/// 🔵 信頼性: REQ-001・REQ-301・ユーザヒアリング（S/R/Q全出力）より
#[derive(Debug, Clone, serde::Serialize)]
pub struct VikorResult {
    /// utility measure（各試行のS値、低い = 良い）
    /// 🔵 ユーザヒアリング（S全出力）より
    pub s_values: Vec<f64>,

    /// regret measure（各試行のR値、低い = 良い）
    /// 🔵 ユーザヒアリング（R全出力）より
    pub r_values: Vec<f64>,

    /// 妥協スコア（各試行のQ値、0〜1、低い = 良い）
    /// 🔵 VIKORアルゴリズム仕様・REQ-001より
    pub q_values: Vec<f64>,

    /// バーチャート表示用スコア（= 1.0 - q_values、高い = 良い）
    /// McdmResult.primary_scores()と互換性を保つため格納
    /// 🔵 REQ-003・既存primary_scores()インターフェースより
    pub display_scores: Vec<f64>,

    /// Q昇順の試行インデックス（ranked_indices[0]が最良）
    /// 🔵 REQ-002・既存TopsisResult.ranked_indicesパターンより
    pub ranked_indices: Vec<u32>,

    /// 各目的の最良値 f*（目的数次元）
    /// 🔵 VIKORアルゴリズム仕様より
    pub best_values: Vec<f64>,

    /// 各目的の最悪値 f-（目的数次元）
    /// 🔵 VIKORアルゴリズム仕様より
    pub worst_values: Vec<f64>,

    /// 計算時間（ms）
    /// 🔵 既存TopsisResultパターンより
    pub duration_ms: f64,
}

/// VIKOR計算関数シグネチャ
///
/// # 引数
/// - `values`: 目的関数値の平坦配列 [N×M]（行major: trial0_obj0, trial0_obj1, ...）
/// - `n_trials`: 試行数（>= 1）
/// - `n_objectives`: 目的関数数（>= 1）
/// - `weights`: 各目的の重み（合計1.0に正規化済みを前提）len == n_objectives
/// - `is_minimize`: 各目的の最小化フラグ len == n_objectives
/// - `v`: 戦略重み [0.0, 1.0]（0=最小遺憾, 0.5=妥協, 1=最大多数合意）
///
/// 🔵 REQ-001・REQ-201・ユーザヒアリングより
pub fn compute_vikor(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
    v: f64,
) -> Result<VikorResult, String> {
    todo!()
}

// ============================================================
// egui-app/src/ui/widgets/mcdm_chart.rs に定義する型
// ============================================================

/// MCDMコンピュートリクエスト（pending_compute の型）
///
/// TOPSISとVIKORで共通のリクエスト型。
/// TOPSIS選択時は v フィールドを無視する。
///
/// 🔵 ユーザヒアリング（McdmComputeRequest構造体採用）より
pub struct McdmComputeRequest {
    /// 使用するMCDM手法
    pub method: McdmMethod,

    /// 正規化済み重み（合計1.0）
    /// 🔵 既存normalize_weights()パターンより
    pub weights: Vec<f64>,

    /// VIKOR戦略重み [0.0, 1.0]（TOPSIS時は無視）
    /// 🔵 REQ-007・ユーザヒアリングより
    pub v: f64,
}

/// MCDMランキングバーチャートのUI状態（変更後）
///
/// 🔵 既存McdmRankChart実装・ユーザヒアリングより
pub struct McdmRankChart {
    pub method: McdmMethod,
    pub weights: Vec<f64>,

    /// VIKOR戦略パラメータ（デフォルト 0.5）
    /// 🔵 ユーザヒアリング（vスライダー・デフォルト0.5）より
    pub v_param: f64,

    pub computing: bool,

    /// 変更: Option<(McdmMethod, Vec<f64>)> → Option<McdmComputeRequest>
    pub pending_compute: Option<McdmComputeRequest>,

    pub top_n: McdmTopN,
}

// ============================================================
// egui-app/src/state/results.rs の変更型
// ============================================================

/// MCDM手法の列挙（変更後）
///
/// 🔵 REQ-004・ユーザヒアリングより
pub enum McdmMethod {
    Topsis,
    Vikor,  // 追加
}

impl McdmMethod {
    pub fn label(&self) -> &'static str {
        match self {
            McdmMethod::Topsis => "TOPSIS",
            McdmMethod::Vikor => "VIKOR",   // 追加
        }
    }

    pub fn all() -> &'static [McdmMethod] {
        &[McdmMethod::Topsis, McdmMethod::Vikor]  // Vikor追加
    }
}

/// MCDM結果のenum（変更後）
///
/// 🔵 REQ-005・ユーザヒアリングより
pub enum McdmResult {
    Topsis(TopsisResult),
    Vikor(VikorResult),  // 追加
}

impl McdmResult {
    pub fn primary_scores(&self) -> &[f64] {
        match self {
            McdmResult::Topsis(r) => &r.scores,
            McdmResult::Vikor(r) => &r.display_scores,  // 1.0 - Q
        }
    }

    pub fn ranked_indices(&self) -> &[u32] {
        match self {
            McdmResult::Topsis(r) => &r.ranked_indices,
            McdmResult::Vikor(r) => &r.ranked_indices,  // Q昇順
        }
    }

    pub fn duration_ms(&self) -> f64 {
        match self {
            McdmResult::Topsis(r) => r.duration_ms,
            McdmResult::Vikor(r) => r.duration_ms,
        }
    }

    pub fn method(&self) -> McdmMethod {
        match self {
            McdmResult::Topsis(_) => McdmMethod::Topsis,
            McdmResult::Vikor(_) => McdmMethod::Vikor,
        }
    }

    pub fn method_label(&self) -> &'static str {
        self.method().label()
    }
}

// ============================================================
// 信頼性レベルサマリー
// - 🔵 青信号: 18件 (100%)
// - 🟡 黄信号: 0件 (0%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
// ============================================================
