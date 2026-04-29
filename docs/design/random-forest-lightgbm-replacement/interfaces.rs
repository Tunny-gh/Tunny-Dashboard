// ============================================================
// random-forest-lightgbm-replacement 型定義（Rust）
//
// 作成日: 2026-04-27
// 関連設計: architecture.md
//
// 信頼性レベル:
// - 🔵 青信号: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な型定義
// - 🟡 黄信号: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による型定義
// - 🔴 赤信号: EARS要件定義書・設計文書・ユーザヒアリングにない推測による型定義
// ============================================================

// ============================================================
// 1. core/lgbm.rs (新規: rust_core/src/core/lgbm.rs)
// ============================================================

/// LightGBM RandomForest モード用ハイパーパラメータ設定
///
/// 🔵 信頼性: REQ-001・NFR-101 (共有設定) + note.md ハイパーパラメータ表より
///
/// 使用例:
///   PDP 2D  → LgbmRfConfig { num_iterations: 100, max_depth: 10, .. }
///   MDI     → LgbmRfConfig { num_iterations: 64,  max_depth: 64, .. }
///   SHAP    → LgbmRfConfig { num_iterations: 64,  max_depth: 10, .. }
///   ANOVA   → LgbmRfConfig { num_iterations: 100, max_depth: 10, .. }
pub struct LgbmRfConfig {
    /// 木の本数 (旧 n_trees)
    /// 🔵 信頼性: note.md ハイパーパラメータ対応表より
    pub num_iterations: usize,

    /// 木の最大深度 (旧 max_depth)
    /// 🔵 信頼性: note.md ハイパーパラメータ対応表より
    pub max_depth: i32,

    /// リーフの最小サンプル数 (旧 min_samples_leaf)
    /// 🔵 信頼性: note.md ハイパーパラメータ対応表より
    pub min_data_in_leaf: i32,

    /// バギング率 (RF 有効化に必須、< 1.0)
    /// 🟡 信頼性: LightGBM RF モードの要件から妥当な推測
    pub bagging_fraction: f64,

    /// 特徴量サンプリング率
    /// 🟡 信頼性: LightGBM RF モードの要件から妥当な推測
    pub feature_fraction: f64,

    /// 乱数シード
    /// 🔵 信頼性: note.md ハイパーパラメータ対応表より (seed=42)
    pub seed: i32,
}

impl Default for LgbmRfConfig {
    /// SHAP/MDI 共通デフォルト (64木, 深度10)
    /// 🟡 信頼性: 既存実装の定数から妥当な推測
    fn default() -> Self;
}

/// &[Vec<f64>] と &[f64] から lightgbm::Dataset を作成する
///
/// 🔵 信頼性: lightgbm-rs API + REQ-001 より
///
/// # Returns
/// - `Ok(Dataset)` — 変換成功
/// - `Err(...)` — データが空または lightgbm エラー
pub fn to_lgbm_dataset(
    x: &[Vec<f64>],
    y: &[f64],
) -> Result<lightgbm::Dataset, lightgbm::Error>;

/// LightGBM RF モデルを訓練する
///
/// 🔵 信頼性: REQ-001 + lightgbm-rs API より
///
/// # Returns
/// - `Some(Booster)` — 訓練成功
/// - `None` — データ不足または LightGBM エラー
pub fn train_lgbm_rf(
    x: &[Vec<f64>],
    y: &[f64],
    config: &LgbmRfConfig,
) -> Option<lightgbm::Booster>;

/// LightGBM モデルで回帰予測する
///
/// 🔵 信頼性: REQ-101/REQ-104 + lightgbm-rs predict API より
///
/// # Returns
/// `Vec<f64>` — 長さ = x.len()
pub fn lgbm_predict(
    booster: &lightgbm::Booster,
    x: &[Vec<f64>],
) -> Vec<f64>;

/// LightGBM モデルで SHAP (TreeSHAP) 値を計算する
///
/// 🔵 信頼性: REQ-102 (SHAP完全置き換え) + lightgbm predict_contrib API より
///
/// # Returns
/// `Vec<Vec<f64>>` — shape: [n_samples][n_features + 1]
///   最後の列 (index n_features) はバイアス項（使用しない）
pub fn lgbm_predict_contrib(
    booster: &lightgbm::Booster,
    x: &[Vec<f64>],
) -> Vec<Vec<f64>>;

/// LightGBM モデルの MSE を評価データで計算する
///
/// 🔵 信頼性: REQ-102/REQ-103/REQ-104 + 既存 mse_on_dataset パターンより
///
/// # Returns
/// - `Some(mse)` — 計算成功
/// - `None` — データが空
pub fn lgbm_mse(
    booster: &lightgbm::Booster,
    x_eval: &[Vec<f64>],
    y_eval: &[f64],
) -> Option<f64>;

/// LightGBM モデルの Gain ベース特徴重要度を返す（正規化済み、合計=1）
///
/// 🔵 信頼性: REQ-103 (MDI) + lightgbm feature_importance API より
///
/// # Returns
/// `Vec<f64>` — 長さ = n_features、合計 = 1.0（全ゼロの場合はゼロのまま）
pub fn lgbm_feature_importance(
    booster: &lightgbm::Booster,
    n_features: usize,
) -> Vec<f64>;

// ============================================================
// 2. core/random_forest/mod.rs (変更後: rng のみ残す)
// ============================================================

// 変更前:
//   pub(crate) mod forest;
//   pub(crate) mod pdp;
//   pub(crate) mod rng;
//   pub(crate) mod tree;
//   pub(crate) mod types;
//   pub(crate) use forest::{extract_columns, mse_on_dataset, train_rf_on_columns};
//   pub(crate) use pdp::compute_pdp_2d_rf;
//   pub(crate) use rng::Lcg;
//
// 変更後:
//   pub(crate) mod rng;              // ← Lcg 保持 (REQ-003, REQ-402)
//   pub(crate) use rng::Lcg;
//
// 🔵 信頼性: REQ-003 コードベース調査より（Kriging 依存）

// ============================================================
// 3. rust_core/build.rs (新規)
// ============================================================

// /// libs/ ディレクトリへのリンクパスを設定する
// ///
// /// 🔵 信頼性: REQ-401・NFR-201 + ユーザヒアリング（libs/ 配置）より
// fn main() {
//     let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//     // rust_core/Cargo.toml からワークスペースルートの libs/ を指す
//     let libs_dir = manifest_dir.parent().unwrap().join("libs");
//     println!("cargo:rustc-link-search=native={}", libs_dir.display());
//     println!("cargo:rerun-if-changed=build.rs");
// }

// ============================================================
// 4. sensitivity/shap.rs (変更後のシグネチャ)
// ============================================================

/// SHAP 重要度を計算する（LightGBM native SHAP 使用）
///
/// 🔵 信頼性: REQ-102 + ユーザヒアリング（SHAP完全置き換え）より
///
/// # 変更点
/// - `ShapNode`, `PathElement` を削除
/// - `build_shap_tree`, `tree_shap_recurse` 等の TreeSHAP 実装を削除
/// - `train_lgbm_rf` + `lgbm_predict_contrib` で置き換え
///
/// # Returns
/// `(importances: Vec<f64>, r_squared: f64)`
///   - importances: 正規化済み絶対 SHAP 値（合計=1）
///   - r_squared: LightGBM RF の R²（ホールドアウト評価）
pub fn compute_shap_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64);

// ============================================================
// 5. sensitivity/mdi.rs (変更後のシグネチャ)
// ============================================================

/// MDI 重要度を計算する（LightGBM feature_importance(Gain) 使用）
///
/// 🔵 信頼性: REQ-103 + ユーザヒアリング（MDI互換性許容）より
///
/// # 変更点
/// - `MdiNode` enum を削除
/// - `build_mdi_tree_idx`, `accumulate_gains` を削除
/// - `train_lgbm_rf` + `lgbm_feature_importance(Gain)` で置き換え
///
/// # Returns
/// `(importances: Vec<f64>, r_squared: f64)`
///   - importances: Gain ベース特徴重要度（正規化済み）
///   - r_squared: LightGBM RF の R²（ホールドアウト評価）
pub fn compute_mdi_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64);

// ============================================================
// 6. sensitivity/rf_anova.rs (変更後のシグネチャ)
// ============================================================

/// RF-ANOVA 順列重要度を計算する（LightGBM RF 使用）
///
/// 🔵 信頼性: REQ-104 + コードベース調査より
///
/// # 変更点
/// - `train_rf_on_columns` → `train_lgbm_rf`
/// - `mse_on_dataset` → `lgbm_mse`
/// - 順列重要度計算ロジック自体は変更なし
///
/// # Returns
/// `(importances: Vec<f64>, r_squared: f64)`
pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64);

// ============================================================
// 7. pdp/api.rs の変更箇所
// ============================================================

// 変更前:
//   "random_forest" => {
//       let (x_values, y_values, z_values, r_squared) =
//           random_forest::compute_pdp_2d_rf(&x_matrix, &y, p1_idx, p2_idx, n_grid)?;
//       ...
//   }
//
// 変更後:
//   "random_forest" => {
//       // core::lgbm を使った 2D PDP（compute_pdp_2d_lgbm または直接インライン）
//       let result = crate::core::lgbm::compute_pdp_2d_lgbm(
//           &x_matrix, &y, p1_idx, p2_idx, n_grid
//       )?;
//       Some(PdpResult2d { ..result, uncertainties: None })
//   }
//
// 🔵 信頼性: REQ-101 より

// ============================================================
// 信頼性レベルサマリー
// ============================================================
// - 🔵 青信号: 16件 (84%)
// - 🟡 黄信号: 3件 (16%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
