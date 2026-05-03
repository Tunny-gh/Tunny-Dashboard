// ============================================================
// sensitivity-refactoring Rust型定義・インターフェース
//
// 作成日: 2026-05-04
// 関連設計: architecture.md
//
// 信頼性レベル:
// - 🔵 青信号: EARS要件定義書・コード分析・ユーザヒアリングを参考にした確実な型定義
// - 🟡 黄信号: EARS要件定義書・コード分析・ユーザヒアリングから妥当な推測による型定義
// - 🔴 赤信号: EARS要件定義書・コード分析・ユーザヒアリングにない推測による型定義
// ============================================================

// ============================================================
// rust_core/src/core/math/stats.rs  (新規)
// 🔵 信頼性: REQ-001・EDGE-001 より
// ============================================================

/// 列データの平均と標準偏差を返す。
/// - 空スライス: (0.0, 1.0)
/// - std < EPSILON: 1.0 に固定（ゼロ除算防止）
/// 🔵 REQ-001-1, REQ-001-2, EDGE-001 より
pub(crate) fn column_mean_std(vals: &[f64]) -> (f64, f64) {
    let n = vals.len();
    if n == 0 {
        return (0.0, 1.0); // 🔵 EDGE-001: 空スライスは (0.0, 1.0) を返す
    }
    let nf = n as f64;
    let mean = vals.iter().sum::<f64>() / nf;
    let var = vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf;
    let std_dev = var.sqrt();
    let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev }; // 🔵 REQ-001-2
    (mean, std_dev)
}

// ============================================================
// rust_core/src/sensitivity/constants.rs  (新規)
// 🔵 信頼性: REQ-005・ユーザヒアリング: constants.rs 新規作成 より
// ============================================================

// --- ツリーモデル共通 LightGBM 設定 ---
// 🔵 既存実装 mdi.rs/shap.rs/rf_anova.rs より抽出
pub(crate) const RF_TREES: usize = 64;
pub(crate) const RF_MAX_DEPTH: usize = 64;
pub(crate) const RF_MIN_SAMPLES_LEAF: usize = 2;

// --- ダウンサンプリング上限 ---
// LightGBM ゲイン計算は 1回の訓練コストが高いため 1000 に抑制
// 🔵 REQ-005-2: 値は変更しない
pub(crate) const MDI_MAX_ROWS: usize = 1_000;
// TreeSHAP はノード走査コストがあるため 1000 に抑制
// 🔵 REQ-005-2
pub(crate) const SHAP_MAX_ROWS: usize = 1_000;
// RF-ANOVA はゲインではなく分散分析のため 2000 まで許容
// 🔵 REQ-005-2
pub(crate) const RF_ANOVA_MAX_ROWS: usize = 2_000;
// PFI は 5 回リピートだが permutation 自体は軽量なため 2000 まで許容
// 🔵 REQ-005-2
pub(crate) const PFI_MAX_ROWS: usize = 2_000;

// --- 再現性のための固定シード ---
// 🔵 既存実装より抽出
pub(crate) const RF_SEED: u64 = 42;
pub(crate) const PFI_SEED_BASE: u64 = 1_000;
// PFI の反復回数（複数回繰り返してばらつきを安定化）
// 🔵 既存 permutation.rs より
pub(crate) const N_REPEATS: usize = 5;

// ============================================================
// rust_core/src/sensitivity/metrics.rs  (新規)
// 🔵 信頼性: REQ-004・ユーザヒアリング: TreeMetric Trait + 静的ディスパッチ
// ============================================================

use super::tree_common::PreparedData;

/// ツリーベースの感度分析メトリクス共通トレイト。
///
/// 各実装は `prepare_training_data` で前処理済みのデータを受け取り、
/// (feature_importances, r_squared) を返す。
/// importances の合計は 1.0 になるよう正規化すること（またはすべて 0.0）。
/// データ不足や計算失敗時は None を返す。
///
/// 🔵 REQ-004-1 より
pub(crate) trait TreeMetric {
    /// 前処理済みデータから (importances, r_squared) を計算する。
    /// 失敗時は None を返す。
    /// 🔵 REQ-004-1
    fn compute_importances(
        &self,
        data: &PreparedData,
    ) -> Option<(Vec<f64>, f64)>;

    /// ダウンサンプリング上限行数
    /// 🔵 REQ-005
    fn max_rows(&self) -> usize;

    /// データサンプリング用シード
    /// 🔵 既存実装より
    fn data_seed(&self) -> u64;

    /// ホールドアウト分割シャッフル用シード
    /// 🔵 既存実装より
    fn split_seed(&self) -> u64;
}

/// RF-ANOVA メトリクス（ランダムフォレスト分散分析）
/// 🔵 REQ-004-2
pub(crate) struct RfAnovaMetric;

/// MDI メトリクス（Mean Decrease Impurity / LightGBM ゲイン）
/// 🔵 REQ-004-2
pub(crate) struct MdiMetric;

/// SHAP メトリクス（LightGBM ネイティブ TreeSHAP）
/// 🔵 REQ-004-2
pub(crate) struct ShapMetric;

/// Permutation Feature Importance メトリクス（5回リピート）
/// 🔵 REQ-004-2
pub(crate) struct PermutationMetric;

// impl TreeMetric for RfAnovaMetric {
//     fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
//         // rf_anova.rs の内部ロジックを呼ぶ
//         ...
//     }
//     fn max_rows(&self) -> usize { RF_ANOVA_MAX_ROWS }
//     fn data_seed(&self) -> u64 { RF_SEED }
//     fn split_seed(&self) -> u64 { RF_SEED.wrapping_add(1) }
// }
// 他3メトリクスも同様に実装

// ============================================================
// rust_core/src/sensitivity/types.rs  (変更)
// 🔵 信頼性: REQ-003・ユーザヒアリング: Newtypeパターン
// ============================================================

/// ツリー系メトリクス共通の計算結果
/// 各フィールドのインデックス: importances[param_idx][obj_idx]
/// 🔵 既存実装より（構造体は変更なし）
#[derive(Debug, Clone)]
pub struct TreeImportanceResult {
    pub importances: Vec<Vec<f64>>, // [param][objective]
    pub r_squared: Vec<f64>,        // [objective]
}

/// RF-ANOVA の結果（Newtype）
/// 🔵 REQ-003-1: 型エイリアスから Newtype に変更
#[derive(Debug, Clone)]
pub struct RfAnovaResult(pub TreeImportanceResult);

/// MDI の結果（Newtype）
/// 🔵 REQ-003-1
#[derive(Debug, Clone)]
pub struct MdiResult(pub TreeImportanceResult);

/// SHAP の結果（Newtype）
/// 🔵 REQ-003-1
#[derive(Debug, Clone)]
pub struct ShapResult(pub TreeImportanceResult);

/// Permutation Feature Importance の結果（Newtype）
/// 🔵 REQ-003-1
#[derive(Debug, Clone)]
pub struct PermutationResult(pub TreeImportanceResult);

/// 全メトリクスの感度分析結果
/// 🔵 既存実装より（フィールド型のみ変更）
#[derive(Debug, Clone)]
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,              // [param][objective]
    pub ridge: Vec<RidgeResult>,              // [objective]
    pub rf_anova: Option<RfAnovaResult>,      // 🔵 Newtype に変更
    pub mdi: Option<MdiResult>,               // 🔵 Newtype に変更
    pub shap: Option<ShapResult>,             // 🔵 Newtype に変更
    pub permutation: Option<PermutationResult>, // 🔵 Newtype に変更
}

/// Ridge 回帰の結果
/// 🔵 変更なし
#[derive(Debug, Clone)]
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

/// Sobol 指標の結果
/// 🔵 変更なし
#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
    pub n_samples: usize,
}

/// メトリクス種別
/// 🔵 変更なし
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityMetric {
    Spearman,
    Ridge,
    RfAnova,
    Mdi,
    Shap,
    Permutation,
}

// ============================================================
// analysis/full.rs の変更パターン（代表的な呼び出し箇所）
// 🔵 信頼性: REQ-004-3 より
// ============================================================

// 変更前:
// let (imp, r2) = compute_mdi_importances(&x_matrix, &y);
// let mdi = Some(transpose_to_tree_result(&[imp], vec![r2], n_params, 1));

// 変更後:
// let mdi = run_tree_metric_for_all_objectives(
//     &MdiMetric,
//     &x_matrix,
//     &[y],
// ).map(MdiResult);

// ヘルパー関数:
// fn run_tree_metric_for_all_objectives<M: TreeMetric>(
//     metric: &M,
//     x_matrix: &[Vec<f64>],
//     objectives: &[Vec<f64>],
// ) -> Option<TreeImportanceResult> { ... }

// ============================================================
// 信頼性レベルサマリー
// - 🔵 青信号: 全項目 (100%)
// - 🟡 黄信号: 0件
// - 🔴 赤信号: 0件
//
// 品質評価: ✅ 高品質
// ============================================================
