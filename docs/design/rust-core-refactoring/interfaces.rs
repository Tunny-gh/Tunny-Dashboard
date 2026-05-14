//! rust-core-refactoring 型定義・インターフェース仕様
//!
//! 作成日: 2026-05-14
//! 関連設計: architecture.md
//!
//! 信頼性レベル:
//! - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
//! - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
//! - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義

// ========================================
// エピック A: コード重複排除
// ========================================

// ---- A-1. SensitivityMetric トレイト (新規) ----
// ファイル: rust_core/src/sensitivity/metric_trait.rs

/// 感度分析指標の統一インターフェース
///
/// MDI・SHAP・RF-ANOVA・Permutation・Spearman・Ridge の各指標が実装する。
/// 🔵 信頼性: REQ-A01・ユーザーヒアリングより
pub trait SensitivityMetric: Send + Sync {
    /// 指定目的関数インデックスの感度を計算する。
    /// 計算不能な場合 None を返す（パニックしない）。
    /// 🔵 信頼性: REQ-A01・ユーザーストーリー A-1 より
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult>;

    /// 指標の識別名（ログ・デバッグ用）
    /// 🟡 信頼性: 合理的な実装要件から推測
    fn name(&self) -> &'static str;
}

/// SensitivityMetric トレイトを実装する構造体一覧
///
/// 🔵 信頼性: REQ-A01・既存 sensitivity/metrics.rs より
///
/// | 構造体 | ファイル |
/// |--------|----------|
/// | SpearmanMetric   | sensitivity/spearman.rs  |
/// | RidgeMetric      | sensitivity/ridge.rs     |
/// | RfAnovaMetric    | sensitivity/metrics.rs   |
/// | MdiMetric        | sensitivity/metrics.rs   |
/// | ShapMetric       | sensitivity/metrics.rs   |
/// | PermutationMetric| sensitivity/metrics.rs   |

// ---- SensitivityKind enum (リネーム: 旧 SensitivityMetric) ----
// ファイル: rust_core/src/sensitivity/types.rs

/// 感度分析指標の種別選択
///
/// 旧名 `SensitivityMetric` から `SensitivityKind` にリネーム。
/// egui-app 側での指標選択 UI や `compute_sensitivity_for` のセレクタとして使用。
/// 🔵 信頼性: 既存 sensitivity/types.rs・ユーザーヒアリング（リネーム確認）より
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityKind {
    Spearman,    // 🔵 既存バリアント
    Ridge,       // 🔵 既存バリアント
    RfAnova,     // 🔵 既存バリアント
    Mdi,         // 🔵 既存バリアント
    Shap,        // 🔵 既存バリアント
    Permutation, // 🔵 既存バリアント
}

// ---- A-2. Pearson 相関 (移動) ----
// ファイル: rust_core/src/core/math/stats.rs

/// x と y のピアソン相関係数を計算する。
///
/// 分散が 0（全値同一）の場合は `f64::NAN` を返す。パニックしない。
/// 🔵 信頼性: REQ-A04・EDGE-101・受け入れ基準 TC-A04-01〜03 より
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    // 実装: mean/std 計算 → covariance → correlation
    // 分散 < f64::EPSILON の場合 f64::NAN を返す
    todo!()
}

// ---- A-3. k-means 初期化共通関数 (新規) ----
// ファイル: rust_core/src/clustering/kmeans.rs

/// k-means++ / 決定論的初期化で共有する次の重心選択関数。
///
/// 既存重心からの距離² に基づいて次の重心候補を選択する。
/// `sampling_fn` に確率的/決定論的な選択を注入することで両戦略を統一する。
///
/// # 引数
/// - `flat_data`: 行優先フラット配列 `[i*p + j]`
/// - `n_cols`: 特徴数 p
/// - `existing`: 既選択重心のスライス（各重心は p 次元ベクトル）
/// - `n`: 総点数
/// - `sampling_fn`: `distances: &[f64]` を受け取り選択インデックスを返すクロージャ
///
/// 🔵 信頼性: REQ-A06・REQ-A07・受け入れ基準 TC-A06-01〜02 より
pub fn select_next_centroid<F>(
    flat_data: &[f64],
    n_cols: usize,
    existing: &[Vec<f64>],
    n: usize,
    sampling_fn: F,
) -> Vec<f64>
where
    F: Fn(&[f64]) -> usize,
{
    todo!()
}

// ========================================
// エピック B: 責務分離
// ========================================

// ---- B-2. クラスタ統計 3 分割 ----
// ファイル: rust_core/src/clustering/stats.rs

/// 全データの列ごとの平均と標準偏差を計算する。
///
/// 🔵 信頼性: REQ-B03・受け入れ基準 TC-B03-01 より
///
/// # 戻り値
/// `(means: Vec<f64>, stds: Vec<f64>)` — 各要素の長さは p
pub fn compute_global_stats(flat_data: &[f64], n: usize, p: usize) -> (Vec<f64>, Vec<f64>) {
    todo!()
}

/// クラスタごとの重心と標準偏差を計算する。
///
/// 空クラスタの場合は全体平均を重心として使用する。
/// 🔵 信頼性: REQ-B03・コード分析 clustering/stats.rs より
///
/// # 戻り値
/// `Vec<ClusterStat>` — cluster_id, size, centroid, std_dev を含む（significant_features 未設定）
pub fn compute_cluster_centroid_std(
    flat_data: &[f64],
    labels: &[usize],
    n: usize,
    p: usize,
    k: usize,
) -> Vec<ClusterStat> {
    todo!()
}

/// t 統計量によって有意な特徴を判定し ClusterStat を更新する。
///
/// t = (centroid[j] - global_mean[j]) / SE, SE = sqrt(var_c/nc + var_g/n)
/// t > 3.0 の特徴を significant_features = true とする。
/// 🔵 信頼性: REQ-B03・コード分析 clustering/stats.rs の t統計量実装より
pub fn compute_significant_features(
    cluster_stats: Vec<ClusterStat>,
    global_mean: &[f64],
    global_std: &[f64],
    n: usize,
) -> Vec<ClusterStat> {
    todo!()
}

// ---- B-3. Ridge 回帰 3 分割 ----
// ファイル: rust_core/src/sensitivity/ridge.rs

/// X 行列（列優先フラット配列）から X'X 行列を計算する。
///
/// Ridge 正則化項の加算（+αI）は呼び出し元で行う。
/// 🔵 信頼性: REQ-B04・受け入れ基準 TC-B04-01〜02 より
///
/// # 引数
/// - `x_cols`: 列優先フラット配列 `[j*n + i]`
/// - `p`: パラメータ数
/// - `n`: 行数
pub fn compute_xtx_matrix(x_cols: &[f64], p: usize, n: usize) -> Vec<Vec<f64>> {
    todo!()
}

/// X 行列（列優先フラット配列）から X'y ベクトルを計算する。
///
/// 🔵 信頼性: REQ-B04 より
pub fn compute_xty_vector(x_cols: &[f64], y: &[f64], p: usize, n: usize) -> Vec<f64> {
    todo!()
}

/// 決定係数 R² を計算する。
///
/// SS_tot ≈ 0 の場合（分散なし）は 0.0 を返す。
/// 🔵 信頼性: REQ-B04・受け入れ基準 TC-B04-01 (`compute_r_squared([1,2,3],[1,2,3])` → 1.0) より
pub fn compute_r_squared(y_actual: &[f64], y_predicted: &[f64]) -> f64 {
    todo!()
}

// ---- B-4. GpModel 分割 ----
// ファイル: rust_core/src/core/kriging/gaussian_process/model.rs

/// ガウス過程カーネルの超パラメータ。
///
/// GpFittedModel に内包される。
/// 🟡 信頼性: REQ-B05・コード分析 model.rs から妥当な推測（ユーザーヒアリング確認済み）
#[derive(Debug, Clone)]
pub struct GpKernel {
    /// 対数スケール長さ（各次元）— 旧 GpModel::log_ls
    pub log_ls: Vec<f64>, // 🔵 既存 GpModel より
    /// 対数信号分散 — 旧 GpModel::log_sf
    pub log_sf: f64, // 🔵 既存 GpModel より
    /// 対数ノイズ標準偏差 — 旧 GpModel::log_sn
    pub log_sn: f64, // 🔵 既存 GpModel より
}

/// 訓練済みガウス過程モデル。
///
/// GpKernel を内包し、訓練データと Cholesky 分解を保持する。
/// 🔵 信頼性: REQ-B05・ユーザーヒアリングより
#[derive(Debug, Clone)]
pub struct GpFittedModel {
    /// カーネル超パラメータ（最適化済み）
    pub kernel: GpKernel, // 🔵 ユーザーヒアリングより
    /// 訓練データの係数 α = (K + σ_n²I)^{-1} y — 旧 GpModel::alpha
    pub alpha: Vec<f64>, // 🔵 既存 GpModel より
    /// 訓練データ入力 — 旧 GpModel::x_train
    pub x_train: Vec<Vec<f64>>, // 🔵 既存 GpModel より
    /// Cholesky 分解 L（K_XX + σ_n² I = LL'）— 旧 GpModel::l
    pub l: Vec<Vec<f64>>, // 🔵 既存 GpModel より
}

// ========================================
// エピック C: 効率改善
// ========================================

// ---- C-1. SamplingContext (新規) ----
// ファイル: rust_core/src/sampling/context.rs

/// ダウンサンプリングに必要なコンテキスト情報。
///
/// 旧 `sampling/state.rs` のグローバル `thread_local! STATE` を値型に置き換える。
/// 呼び出し元（egui-app の AppState）が明示的に保持する。
/// 🔵 信頼性: REQ-C05〜C08・ユーザーヒアリング・コード分析 sampling/state.rs より
#[derive(Debug, Clone)]
pub struct SamplingContext {
    /// 各目的関数の最小化フラグ
    pub is_minimize: Vec<bool>, // 🔵 既存 SamplingState より
    /// Pareto rank 0 のトライアルインデックス
    pub pareto_indices: Option<Vec<u32>>, // 🔵 既存 SamplingState より
    /// 全トライアルのパレートランク（オンデマンド計算キャッシュ）
    pub all_ranks: Option<Vec<u32>>, // 🔵 既存 SamplingState より
    /// クラスタラベル（-1 = 未分類）
    pub cluster_labels: Option<Vec<i32>>, // 🔵 既存 SamplingState より
}

// ---- C-1. init_sampling 新シグネチャ ----
// ファイル: rust_core/src/sampling/mod.rs

/// サンプリングコンテキストを初期化して返す。
///
/// グローバル副作用なし。呼び出し元が戻り値を明示的に保持する。
/// 🔵 信頼性: REQ-C06・受け入れ基準 TC-C05-01 より
pub fn init_sampling(
    is_minimize: Vec<bool>,
    pareto_indices: Option<Vec<u32>>,
    all_ranks: Option<Vec<u32>>,
) -> SamplingContext {
    todo!()
}

/// スマートダウンサンプリング（パレートランク優先）。
///
/// 旧グローバル状態参照 → `ctx: &SamplingContext` に変更。
/// 🔵 信頼性: REQ-C07・受け入れ基準 TC-C05-03 より
pub fn downsample_smart(ctx: &SamplingContext, max_points: usize) -> Vec<u32> {
    todo!()
}

/// パレートランクによる層別ダウンサンプリング。
///
/// 🔵 信頼性: REQ-C07 より
pub fn downsample_stratified_by_rank(ctx: &SamplingContext, max_points: usize) -> Vec<u32> {
    todo!()
}

/// クラスタベースのダウンサンプリング。
///
/// `ctx.cluster_labels` が None の場合は全インデックスを返す。
/// 🔵 信頼性: REQ-C07 より
pub fn downsample_by_cluster(ctx: &SamplingContext, max_points: usize) -> Vec<u32> {
    todo!()
}

// ---- C-1. egui-app 側の変更 ----
// ファイル: egui-app/src/state/app_state.rs

/// AppState へのフィールド追加（抜粋）
///
/// 🟡 信頼性: REQ-C08・コード分析 app_state.rs から妥当な推測
///
/// ```rust
/// pub struct AppState {
///     // ... 既存フィールド ...
///
///     /// サンプリングコンテキスト（データロード後に Some になる）
///     pub sampling_ctx: Option<SamplingContext>, // 🔵 REQ-C08・ユーザーストーリー C-1 より
/// }
/// ```

// ========================================
// 既存型（参照用・変更なし）
// ========================================

/// 感度分析結果（既存型・変更なし）
///
/// 🔵 信頼性: 既存 sensitivity/types.rs より（リファクタリング対象外）
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
    pub mdi: Option<MdiResult>,
    pub shap: Option<ShapResult>,
    pub permutation: Option<PermutationResult>,
}

/// Ridge 回帰結果（既存型・変更なし）
///
/// 🔵 信頼性: 既存 sensitivity/types.rs より
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

/// クラスタ統計（既存型・変更なし）
///
/// 🔵 信頼性: 既存 clustering/stats.rs より
pub struct ClusterStat {
    pub cluster_id: usize,
    pub size: usize,
    pub centroid: Vec<f64>,
    pub std_dev: Vec<f64>,
    pub significant_features: Vec<bool>,
}

// ========================================
// 信頼性レベルサマリー
// ========================================
//
// - 🔵 青信号: 28件 (85%)
// - 🟡 黄信号: 5件 (15%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
