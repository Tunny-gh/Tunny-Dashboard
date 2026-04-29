# RandomForest → LightGBM 置き換え データフロー図

**作成日**: 2026-04-27
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/random-forest-lightgbm-replacement/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## 1. ビルドフロー 🔵

**信頼性**: 🔵 *ユーザヒアリング（libs/ 配置・build.rs）より*

```
cargo build
  │
  ├─ rust_core/build.rs 実行
  │    └─ cargo:rustc-link-search=native=<workspace>/libs/
  │
  ├─ lightgbm クレート内部 build.rs 実行
  │    └─ LIGHTGBM_LIB_DIR を参照して lib_lightgbm をリンク
  │
  └─ 成果物: tunny-core.rlib (lightgbm 動的リンク済み)
```

---

## 2. 共有ラッパー `core::lgbm` のデータフロー 🔵

**信頼性**: 🔵 *アーキテクチャ設計 + lightgbm-rs API より*

```
&[Vec<f64>] (x_matrix)
&[f64]      (y)
LgbmRfConfig
     │
     ▼
to_lgbm_dataset(x, y)
     │
     ▼ lightgbm::Dataset
     │
train_lgbm_rf(dataset, config)
     │
     ▼ lightgbm::Booster (RF モデル)
     │
     ├─── lgbm_predict(booster, x_new)
     │         └─ Vec<f64>  (回帰予測値)
     │
     ├─── lgbm_predict_contrib(booster, x_new)
     │         └─ Vec<Vec<f64>>  (SHAP 値: [sample][feature + 1])
     │
     ├─── lgbm_mse(booster, x_eval, y_eval)
     │         └─ f64  (MSE)
     │
     └─── lgbm_feature_importance(booster, n_features)
               └─ Vec<f64>  (Gain 正規化済み重要度)
```

---

## 3. 2D PDP データフロー 🔵

**信頼性**: 🔵 *要件 REQ-101 + コードベース調査より*

**呼び出し元**: `pdp/api.rs` の `compute_pdp_2d` (`model_type = "random_forest"`)

```
x_matrix: &[Vec<f64>]
y: &[f64]
param1_idx, param2_idx: usize
n_grid: usize
     │
     ▼
extract_columns(x_matrix, [param1_idx, param2_idx])
     │
     ▼ x2d: Vec<Vec<f64>>  (2列のみ)
     │
train_lgbm_rf(x2d, y, LgbmRfConfig { num_iterations: 100, max_depth: 10, .. })
     │
     ▼ booster
     │
linspace(min1, max1, n_grid)  →  grid1
linspace(min2, max2, n_grid)  →  grid2
     │
     ▼ n_grid × n_grid のグリッド点を Vec<Vec<f64>> に変換
     │
lgbm_predict(booster, grid_points)
     │
     ▼ z_values: Vec<Vec<f64>>  (n_grid × n_grid)
     │
lgbm_mse(booster, x2d, y)  →  R²計算 (現行と同じ ss_res/ss_tot)
     │
     ▼
PdpResult2d { x_values: grid1, y_values: grid2, z_values, r_squared, uncertainties: None }
```

---

## 4. SHAP データフロー（新規: LightGBM native SHAP） 🔵

**信頼性**: 🔵 *ユーザヒアリング（SHAP完全置き換え）より*

**変更点**: `ShapNode` 独自 TreeSHAP → `lgbm_predict_contrib` に全面置き換え

```
x_matrix: &[Vec<f64>]
y: &[f64]
     │
     ▼
① データクリーニング（非有限値除去）— 既存ロジック流用
     │
     ▼
② ダウンサンプリング（max 1,000 行）— 既存ロジック流用
     │
     ▼
③ Lcg でシャッフル → 80/20 ホールドアウト分割
   x_train / y_train  (80%)
   x_eval  / y_eval   (20%)
     │
     ▼
④ train_lgbm_rf(x_train, y_train, LgbmRfConfig { num_iterations: 64, max_depth: 10, .. })
     │
     ▼ booster
     │
     ├─ ⑤ lgbm_predict_contrib(booster, x_train)
     │       → phi: [[f64; p+1]; n_train]  ← 最後の列はバイアス項
     │
     │       各サンプルの |phi[i][j]| を j ごとに合計
     │       phi_sum[j] = Σ_i |phi[i][j]| / n_train
     │
     │       normalize(phi_sum)  → importances
     │
     └─ ⑥ lgbm_mse(booster, x_eval, y_eval)  → R²

戻り値: (importances: Vec<f64>, r_squared: f64)
```

**削除されるコード**:
- `ShapNode`, `PathElement` 構造体
- `build_shap_tree`, `tree_shap_recurse`, `extend_path`, `unwrap_path`, `sum_path` 関数

---

## 5. MDI データフロー（新規: LightGBM feature_importance） 🔵

**信頼性**: 🔵 *要件 REQ-103 + ユーザヒアリング（MDI互換性許容）より*

**変更点**: `MdiNode` 独自ゲイン集計 → `lgbm_feature_importance(Gain)` に全面置き換え

```
x_matrix: &[Vec<f64>]
y: &[f64]
     │
     ▼
① データクリーニング（非有限値除去）— 既存ロジック流用
     │
     ▼
② ダウンサンプリング（max 1,000 行）— 既存ロジック流用
     │
     ▼
③ Lcg でシャッフル → 80/20 ホールドアウト分割
   x_train / y_train  (80%)
   x_eval  / y_eval   (20%)
     │
     ▼
④ train_lgbm_rf(x_train, y_train, LgbmRfConfig { num_iterations: 64, max_depth: 64, .. })
     │
     ▼ booster
     │
     ├─ ⑤ lgbm_feature_importance(booster, p)
     │       → Vec<f64>  (Gain, 正規化済み)  = MDI importances
     │
     └─ ⑥ lgbm_mse(booster, x_eval, y_eval)  → R²

戻り値: (importances: Vec<f64>, r_squared: f64)
```

**削除されるコード**:
- `MdiNode` enum
- `find_best_split_with_gain_idx`, `build_mdi_tree_idx`, `accumulate_gains` 関数

---

## 6. RF-ANOVA データフロー（LightGBM RF + 順列重要度） 🔵

**信頼性**: 🔵 *要件 REQ-104 + コードベース調査より*

**変更点**: `train_rf_on_columns` / `mse_on_dataset` → LightGBM 呼び出し

```
x_matrix: &[Vec<f64>]
y: &[f64]
     │
     ▼
① データクリーニング・ダウンサンプリング（max 2,000 行）— 既存ロジック流用
     │
     ▼
② Lcg でシャッフル → 80/20 ホールドアウト分割
   x_train / y_train  (80%)
   x_eval  / y_eval   (20%)
     │
     ▼
③ train_lgbm_rf(x_train, y_train, LgbmRfConfig { num_iterations: 100, max_depth: 10, .. })
     │
     ▼ booster
     │
④ lgbm_mse(booster, x_eval, y_eval)
     → baseline_mse, R²
     │
     ▼
⑤ for feature_idx in 0..p:
     permute_single_column(x_eval, feature_idx, seed)  — 既存ロジック流用
     lgbm_mse(booster, x_permuted, y_eval)  → permuted_mse
     importances[feature_idx] = (permuted_mse - baseline_mse).max(0.0)
     │
     ▼
normalize(importances)

戻り値: (importances: Vec<f64>, r_squared: f64)
```

---

## 7. Lcg の役割（置き換え後） 🔵

**信頼性**: 🔵 *コードベース調査（REQ-003）より*

置き換え後、`Lcg` は以下の用途で引き続き使用される（削除禁止）:

| 使用箇所 | 用途 |
|---|---|
| `sensitivity/shap.rs` | 80/20 分割用シャッフル |
| `sensitivity/mdi.rs` | 80/20 分割用シャッフル |
| `sensitivity/rf_anova.rs` | 80/20 分割用シャッフル + `sample_rows` |
| `core/kriging/sparse_fitc.rs` | Kriging 内部（変更なし） |
| `core/kriging/gaussian_process/training.rs` | Kriging 内部（変更なし） |

`Lcg` へのパス `crate::core::random_forest::Lcg` は変更後も維持する。

---

## 8. エラーハンドリングフロー 🟡

**信頼性**: 🟡 *既存 Option パターン + EDGE-001 から妥当な推測*

```
train_lgbm_rf() が Err を返した場合
     │
     ▼
Result<Booster, LgbmError>
     │
     ├─ PDP: None を返す (既存の Option<PdpResult2d> パターンと一致)
     │
     ├─ SHAP: (vec![0.0; p], 0.0) を返す (既存のフォールバックと一致)
     │
     ├─ MDI: (vec![0.0; p], 0.0) を返す (既存のフォールバックと一致)
     │
     └─ RF-ANOVA: (vec![0.0; p], 0.0) を返す (既存のフォールバックと一致)
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (89%)
- 🟡 黄信号: 1件 (11%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
