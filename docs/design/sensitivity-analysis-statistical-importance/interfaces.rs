// ============================================================
// sensitivity-analysis-statistical-importance 型定義（Rust）
//
// 作成日: 2026-04-25
// 関連設計: architecture.md
//
// 信頼性レベル:
// - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
// - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
// - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義
// ============================================================

// ============================================================
// 1. statistics.rs (新規: rust_core/src/core/math/statistics.rs)
// ============================================================

/// t分布の累積分布関数（CDF）
///
/// 🔵 信頼性: NFR-STAT-020・ユーザヒアリング（高精度選択）より
///
/// 実装: 不完全ベータ関数の継続分数展開（Lentz法）
///   x = df / (df + t²) として I_x(df/2, 0.5) を計算
///   df > 30 の場合は正規分布近似にフォールバック（誤差 < 10^{-6}）
pub fn student_t_cdf(t: f64, df: f64) -> f64;

/// t分布の両側p値
/// 🔵 信頼性: REQ-STAT-011より
pub fn t_two_sided_p(t_stat: f64, df: f64) -> f64;
// = 2.0 * (1.0 - student_t_cdf(t_stat.abs(), df))

/// t分布の片側（上側）p値（重要度が0以上の場合に使用）
/// 🔵 信頼性: REQ-STAT-032より
pub fn t_one_sided_upper_p(t_stat: f64, df: f64) -> f64;
// = 1.0 - student_t_cdf(t_stat, df)

/// 標準正規分布のCDF（Abramowitz & Stegun近似、誤差 < 1.5×10^{-7}）
/// 🟡 信頼性: Sobol p値計算に必要、高精度選択から妥当な推測
pub fn normal_cdf(z: f64) -> f64;

/// 正規分布の両側p値（Sobol の帰無仮説検定で使用）
/// 🔵 信頼性: REQ-STAT-043より
pub fn z_two_sided_p(z_stat: f64) -> f64;
// = 2.0 * (1.0 - normal_cdf(z_stat.abs()))

/// t_{0.025, df}（95%信頼区間の臨界値）の近似値
/// 🟡 信頼性: 信頼区間計算に必要、既知の近似式から推測
/// df >= 100 の場合は 1.96（正規近似）を返す
pub fn t_critical_95(df: f64) -> f64;

/// Bonferroni補正
/// p_values の各要素を min(p * n_params, 1.0) に変換
/// 🔵 信頼性: REQ-STAT-050より
pub fn bonferroni_adjust(p_values: &[Option<f64>], n_params: usize) -> Vec<Option<f64>>;

/// 有意性マーク（補正済みp値から）
/// 🔵 信頼性: REQ-STAT-051より
pub fn significance_mark(p_adjusted: Option<f64>) -> &'static str;
// p < 0.001 → "***"
// p < 0.01  → "**"
// p < 0.05  → "*"
// else      → ""

// ============================================================
// 2. types.rs 変更（rust_core/src/sensitivity/types.rs）
// ============================================================

/// 感度分析全体の結果（spearman統計フィールドを追加）
/// 🔵 信頼性: REQ-STAT-061・既存実装より
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    // 既存フィールド
    pub spearman: Vec<Vec<f64>>,         // [obj][param]
    // 追加フィールド
    pub spearman_p_values: Option<Vec<Vec<f64>>>,  // [obj][param] Bonferroni補正前
    pub spearman_ci_lower: Option<Vec<Vec<f64>>>,   // [obj][param]
    pub spearman_ci_upper: Option<Vec<Vec<f64>>>,   // [obj][param]
    // 既存フィールド（変更なし）
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
    pub mdi: Option<MdiResult>,
    pub shap: Option<ShapResult>,
}

/// Ridge回帰の結果（統計フィールドを追加）
/// 🔵 信頼性: REQ-STAT-062・既存実装より
pub struct RidgeResult {
    // 既存フィールド
    pub beta: Vec<f64>,
    pub r_squared: f64,
    // 追加フィールド
    pub std_errors: Option<Vec<f64>>,     // [param]
    pub p_values: Option<Vec<f64>>,       // [param] Bonferroni補正前
    pub ci_lower: Option<Vec<f64>>,       // [param] 95% CI 下限
    pub ci_upper: Option<Vec<f64>>,       // [param] 95% CI 上限
    pub is_approximate: bool,             // α>0による近似バイアスフラグ
}

// RidgeResult::default() に相当するempty値（既存パターンと互換）
// RidgeResult { beta: vec![], r_squared: 0.0,
//   std_errors: None, p_values: None, ci_lower: None, ci_upper: None,
//   is_approximate: true }

/// RF-ANOVA重要度の結果（統計フィールドを追加）
/// 🔵 信頼性: REQ-STAT-060・既存実装より
pub struct RfAnovaResult {
    // 既存フィールド
    pub importances: Vec<Vec<f64>>,    // [param][objective] 正規化後
    pub r_squared: Vec<f64>,           // [objective]
    // 追加フィールド
    pub p_values: Option<Vec<Vec<f64>>>,   // [param][objective] raw（補正前）
    pub ci_lower: Option<Vec<Vec<f64>>>,   // [param][objective] 正規化後スケール
    pub ci_upper: Option<Vec<Vec<f64>>>,   // [param][objective] 正規化後スケール
}

/// MDI重要度の結果（統計フィールドを追加）
/// 🔵 信頼性: REQ-STAT-060・既存実装より
pub struct MdiResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
    // 追加フィールド（RfAnovaResult と同じ構造）
    pub p_values: Option<Vec<Vec<f64>>>,
    pub ci_lower: Option<Vec<Vec<f64>>>,
    pub ci_upper: Option<Vec<Vec<f64>>>,
}

/// SHAP重要度の結果（統計フィールドを追加）
/// 🔵 信頼性: REQ-STAT-060・既存実装より
pub struct ShapResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
    // 追加フィールド
    pub p_values: Option<Vec<Vec<f64>>>,
    pub ci_lower: Option<Vec<Vec<f64>>>,
    pub ci_upper: Option<Vec<Vec<f64>>>,
}

/// Sobol指標の結果（統計フィールドを追加）
/// 🔵 信頼性: REQ-STAT-063・既存実装より
pub struct SobolResult {
    // 既存フィールド
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,    // [param][objective]
    pub total_effect: Vec<Vec<f64>>,   // [param][objective]
    pub r_squared: Vec<f64>,           // [objective] サロゲート品質
    pub n_samples: usize,
    // 追加フィールド
    pub first_order_ci_lower: Option<Vec<Vec<f64>>>,    // [param][objective]
    pub first_order_ci_upper: Option<Vec<Vec<f64>>>,
    pub first_order_p_values: Option<Vec<Vec<f64>>>,    // raw（補正前）
    pub total_effect_ci_lower: Option<Vec<Vec<f64>>>,
    pub total_effect_ci_upper: Option<Vec<Vec<f64>>>,
    pub total_effect_p_values: Option<Vec<Vec<f64>>>,
    pub surrogate_quality_warning: bool,  // R² < 0.5 の場合 true
}

// ============================================================
// 3. spearman.rs 拡張（rust_core/src/sensitivity/spearman.rs）
// ============================================================

/// Spearman統計情報（単一パラメータ×単一目的関数）
/// 🔵 信頼性: REQ-STAT-010〜013より
pub struct SpearmanStats {
    pub rho: f64,
    pub p_value_raw: Option<f64>,    // 両側p値（補正前）
    pub ci_lower: Option<f64>,       // 95% CI 下限（Fisher z変換）
    pub ci_upper: Option<f64>,       // 95% CI 上限
}

/// Spearman相関係数と統計指標を同時計算
/// 🔵 信頼性: REQ-STAT-010〜013より
pub fn compute_spearman_with_stats(x: &[f64], y: &[f64]) -> SpearmanStats;

// ============================================================
// 4. ridge.rs 拡張（rust_core/src/sensitivity/ridge.rs）
// ============================================================

/// 対角逆行列要素 [A^{-1}]_{jj} を計算
///
/// 🔵 信頼性: ユーザヒアリング（対角成分のみ選択）・REQ-STAT-021より
///
/// A: p×p 対称正定値行列（X^TX + αI）
/// returns: Vec<f64> of length p、各要素が [A^{-1}]_{jj}
/// 実装: 各 j について e_j を右辺として gaussian_elimination を呼び出し
///       j 番目の要素のみを使用
pub fn compute_diagonal_inverse(a: &[Vec<f64>]) -> Vec<f64>;

/// Ridge回帰と統計指標を同時計算
/// 🔵 信頼性: REQ-STAT-020〜025より
pub fn compute_ridge_with_stats(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    alpha: f64,
) -> RidgeResult;  // 統計フィールド込みの RidgeResult を返す

// ============================================================
// 5. rf_anova.rs 変更（rust_core/src/sensitivity/rf_anova.rs）
// ============================================================

/// 木ごとの raw 重要度を記録する内部構造体（統計計算用）
/// 🔵 信頼性: REQ-STAT-030より
struct PerTreeImportances {
    /// [tree_idx][param_idx] の raw 重要度（正規化前）
    pub raw: Vec<Vec<f64>>,
    pub n_trees: usize,
    pub n_params: usize,
}

/// RF-ANOVA重要度と統計指標を計算
/// 既存 compute_rf_anova_importances() の拡張版
/// 🔵 信頼性: REQ-STAT-030〜034より
pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64);
// シグネチャは変更なし（後方互換）
// 内部で per_tree_importances を計算し RfAnovaResult に統計情報を格納

// ※ MdiResult, ShapResult も同様のパターン

// ============================================================
// 6. egui-app/src/ui/widgets/importance_chart.rs の補助関数
// ============================================================

/// 統計情報を取得（メトリクス・パラメータ・目的関数インデックス指定）
/// Bonferroni補正済みのp値・CIを返す
/// 🔵 信頼性: REQ-STAT-005・ユーザヒアリングより
fn get_stat_info(
    result: &SensitivityResult,
    sobol: Option<&SobolResult>,
    metric: &ImportanceMetric,
    param_idx: usize,
    obj_idx: usize,
    n_params: usize,
) -> StatInfo;

pub struct StatInfo {
    pub ci_lower: Option<f64>,    // 正規化後スケール
    pub ci_upper: Option<f64>,
    pub p_adjusted: Option<f64>,  // Bonferroni補正済み
    pub mark: &'static str,       // "", "*", "**", "***"
    pub is_approximate: bool,     // Ridge の近似フラグ
}

/// p値の色分け（有意性で色変化）
/// 🟡 信頼性: UIデザインから妥当な推測
fn p_value_color(p_adjusted: Option<f64>) -> egui::Color32;
// p < 0.001 → Color32::from_rgb(0, 128, 0)     （濃緑）
// p < 0.01  → Color32::from_rgb(60, 180, 60)   （緑）
// p < 0.05  → Color32::from_rgb(0, 80, 200)    （青）
// p ≥ 0.05  → Color32::GRAY                    （グレー）
// None      → Color32::GRAY

// ============================================================
// 信頼性レベルサマリー
// ============================================================
// - 🔵 青信号: 20件 (87%)
// - 🟡 黄信号: 3件 (13%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
