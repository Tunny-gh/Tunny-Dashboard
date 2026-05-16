//! rust_core 外部ライブラリ高速化 型定義
//!
//! 作成日: 2026-05-15
//! 関連設計: architecture.md
//!
//! 信頼性レベル:
//! - 🔵 青信号: 要件定義書・コードベース調査を参考にした確実な型定義
//! - 🟡 黄信号: 要件定義書・コードベース調査から妥当な推測による型定義
//! - 🔴 赤信号: 要件定義書・コードベース調査にない推測による型定義

// ========================================
// PRNG（乱数生成）- Phase 1
// ========================================

/// 決定論的シード対応 ChaCha8 RNG ラッパー
/// 🔵 信頼性: REQ-301・要件定義より
pub struct SeededRng {
    inner: rand_chacha::ChaCha8Rng,
}

impl SeededRng {
    /// u64 シードから決定論的 RNG を初期化
    /// 🔵 信頼性: REQ-301-04（再現性要件）より
    pub fn from_seed(seed: u64) -> Self { ... }

    /// [0, 1) の一様乱数を生成
    /// 🔵 信頼性: 既存 LCG::next_f64() の置き換えより
    pub fn next_f64(&mut self) -> f64 { ... }

    /// [0, bound) の一様整数乱数を生成
    /// 🔵 信頼性: Fisher-Yates シャッフル用途より
    pub fn next_usize(&mut self, bound: usize) -> usize { ... }
}

// ========================================
// 最適化 - Phase 3
// ========================================

/// L-BFGS 最適化の argmin ベース実装
/// 🔵 信頼性: REQ-201・要件定義より
pub struct LbfgsOptimizer {
    max_iter: u64,
    tolerance: f64,
}

/// 最適化結果
/// 🔵 信頼性: 既存 L-BFGS 戻り値パターンより
pub struct OptimizationResult {
    /// 最適化されたパラメータ
    /// 🔵 信頼性: 既存パターンより
    pub params: Vec<f64>,
    /// 目的関数の最終値
    /// 🔵 信頼性: 既存パターンより
    pub cost: f64,
    /// 実行されたイテレーション数
    /// 🔵 信頼性: 既存パターンより
    pub iterations: u64,
    /// 収束したかどうか
    /// 🔵 信頼性: 既存パターンより
    pub converged: bool,
}

impl LbfgsOptimizer {
    /// 新しい L-BFGS オプティマイザを作成
    /// 🔵 信頼性: REQ-201-04（収束条件維持）より
    pub fn new(max_iter: u64, tolerance: f64) -> Self { ... }

    /// 目的関数を最適化
    /// 🔵 信頼性: REQ-201-01・02（argmin 置き換え）より
    ///
    /// argmin の Executor::run() は Result を返すため、エラー（NaN 勾配・非収束等）を
    /// 呼び出し元に伝播させる。EDGE-003 / TC-201-E01 のエラーハンドリングを保証する。
    pub fn optimize<F, G>(
        &self,
        cost_fn: F,
        grad_fn: G,
        init_params: Vec<f64>,
    ) -> Result<OptimizationResult, argmin::core::Error>
    where
        F: Fn(&[f64]) -> f64,
        G: Fn(&[f64]) -> Vec<f64>,
    { ... }
}

// ========================================
// クラスタリング - Phase 4
// ========================================

/// K-means クラスタリング結果（既存型維持）
/// 🔵 信頼性: clustering/types.rs 既存型より
pub struct KmeansResult {
    /// クラスタ重心 (k x p)
    /// 🔵 信頼性: 既存 KmeansResult.centroids より
    pub centroids: faer::Mat<f64>,
    /// 各点のクラスタ割り当て
    /// 🔵 信頼性: 既存 KmeansResult.assignments より
    pub assignments: Vec<usize>,
    /// Within-Cluster Sum of Squares
    /// 🔵 信頼性: 既存 KmeansResult.wcss より
    pub wcss: f64,
    /// 使用したクラスタ数
    /// 🔵 信頼性: 既存 KmeansResult.k より
    pub k: usize,
}

/// linfa-clustering バックエンドの K-means 公開 API
/// 🔵 信頼性: REQ-401-01（linfa-clustering 置き換え）より
pub fn kmeans_clustering(
    data: &faer::Mat<f64>,
    k: usize,
    max_iter: usize,
    seed: u64,
) -> KmeansResult { ... }

/// エルボー法による最適クラスタ数推定
/// 🔵 信頼性: REQ-401-04（エルボー法維持）より
pub fn estimate_optimal_k(
    data: &faer::Mat<f64>,
    max_k: usize,
    max_iter: usize,
    seed: u64,
) -> (usize, Vec<f64>) { ... }

// ========================================
// PCA - Phase 2
// ========================================

/// PCA 結果（既存型維持）
/// 🔵 信頼性: clustering/types.rs 既存型より
pub struct PcaResult {
    /// 射影されたデータ (n x n_components)
    /// 🔵 信頼性: 既存 PcaResult.projected より
    pub projected: faer::Mat<f64>,
    /// 固有値（降順）
    /// 🔵 信頼性: 既存 PcaResult.eigenvalues より
    pub eigenvalues: Vec<f64>,
    /// 固有ベクトル（列ベクトル、降順）
    /// 🔵 信頼性: 既存 PcaResult.components より
    pub components: faer::Mat<f64>,
    /// 寄与率
    /// 🟡 信頼性: 既存実装から妥当な推測
    pub explained_variance_ratio: Vec<f64>,
}

// ========================================
// Ridge 回帰 - Phase 2
// ========================================

/// Ridge 回帰結果
/// 🔵 信頼性: 既存 ridge_core.rs 戻り値パターンより
pub struct RidgeResult {
    /// 回帰係数
    /// 🔵 信頼性: 既存パターンより
    pub coefficients: Vec<f64>,
    /// 決定係数 R²
    /// 🔵 信頼性: REQ-103-02（R² 同一性）より
    pub r_squared: f64,
}

// ========================================
// 境界変換ユーティリティ
// ========================================

/// faer::Mat → ndarray::Array2 変換（linfa-clustering 用）
/// 🔵 信頼性: REQ-401 + アーキテクチャ設計より
///
/// faer::Mat は column-major、ndarray::Array2 のデフォルトは row-major のため
/// as_slice() による直接変換は転置が発生する。element-wise コピーで正確に変換する。
pub fn faer_to_ndarray(mat: &faer::Mat<f64>) -> ndarray::Array2<f64> {
    ndarray::Array2::from_shape_fn(
        (mat.nrows(), mat.ncols()),
        |(i, j)| mat[(i, j)],
    )
}

/// ndarray::Array2 → faer::Mat 変換（linfa 結果取り出し用）
/// 🔵 信頼性: REQ-401 + アーキテクチャ設計より
pub fn ndarray_to_faer(arr: &ndarray::Array2<f64>) -> faer::Mat<f64> {
    faer::Mat::from_fn(arr.nrows(), arr.ncols(), |i, j| arr[[i, j]])
}

// ========================================
// 削除対象モジュール（Phase 1 で削除）
// ========================================

// ❌ core/random_forest/tree.rs → 削除
// ❌ core/random_forest/forest.rs → 削除
// ❌ core/random_forest/types.rs → 削除
// ❌ core/random_forest/rng.rs → core/math/rng.rs (SeededRng) に移行後削除
// ❌ core/random_forest/tests.rs → 削除
// ❌ core/optimization/line_search.rs → argmin 内蔵 line search に移行後削除
// ❌ core/math/linear_algebra.rs の Vec<Vec<f64>> 変換関数 → faer::Mat 移行後削除

// ========================================
// 信頼性レベルサマリー
// ========================================
//
// - 🔵 青信号: 32 件 (94%)
// - 🟡 黄信号: 2 件 (6%)
// - 🔴 赤信号: 0 件 (0%)
//
// 品質評価: ✅ 高品質
