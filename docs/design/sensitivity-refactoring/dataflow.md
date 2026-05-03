# sensitivity-refactoring データフロー図

**作成日**: 2026-05-04
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/sensitivity-refactoring/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・コード分析・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・コード分析・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・コード分析・ユーザヒアリングにない推測によるフロー

---

## 変更箇所の全体マップ 🔵

**信頼性**: 🔵 *コード分析・ユーザヒアリングより*

```mermaid
graph TB
    subgraph NEW["新規作成"]
        STATS["core/math/stats.rs\ncolumn_mean_std()"]
        CONSTS["sensitivity/constants.rs\nMAX_ROWS, シード定数"]
        METRICS["sensitivity/metrics.rs\nTreeMetric trait"]
    end

    subgraph MODIFIED["既存ファイルの変更"]
        RIDGE["sensitivity/ridge.rs\ntranspose_and_standardize()"]
        COMMON["sensitivity/analysis/common.rs\nbuild_standardized_param_columns()"]
        SOBOL["sensitivity/sobol.rs\ncolumn_mean_std() 削除"]
        PDPUTILS["pdp/utils.rs\ncol_mean_std() 委譲"]
        TYPES["sensitivity/types.rs\nNewtype 変更"]
        MDI["sensitivity/mdi.rs\nprepare_training_data 使用"]
        SHAP["sensitivity/shap.rs\nprepare_training_data 使用"]
        RFANOVA["sensitivity/rf_anova.rs\nconsts をインポート"]
        PFI["sensitivity/permutation.rs\nconsts をインポート"]
        FULL["sensitivity/analysis/full.rs\nTreeMetric 静的ディスパッチ"]
        SELECTED["sensitivity/analysis/selected.rs\nTreeMetric 静的ディスパッチ"]
    end

    STATS --> RIDGE
    STATS --> COMMON
    STATS --> SOBOL
    STATS --> PDPUTILS
    CONSTS --> MDI
    CONSTS --> SHAP
    CONSTS --> RFANOVA
    CONSTS --> PFI
    METRICS --> FULL
    METRICS --> SELECTED
    TYPES --> FULL
    TYPES --> SELECTED
```

---

## 1. `column_mean_std` の統一フロー 🔵

**信頼性**: 🔵 *REQ-001/002・コード分析（4箇所の重複）より*

```mermaid
sequenceDiagram
    participant R as ridge.rs
    participant C as analysis/common.rs
    participant S as sobol.rs
    participant P as pdp/utils.rs
    participant STATS as core::math::stats

    Note over R,P: リファクタリング前: 各ファイルにインライン実装

    Note over R: transpose_and_standardize() 内でインライン計算
    Note over C: build_standardized_param_columns() 内でインライン計算
    Note over S: ローカル関数 column_mean_std() で実装
    Note over P: ローカル関数 col_mean_std() で実装

    Note over R,P: リファクタリング後: core::math::stats に統一

    R->>STATS: column_mean_std(&col)
    C->>STATS: column_mean_std(&col)
    S->>STATS: column_mean_std(&col)
    P->>STATS: column_mean_std(&col)
    STATS-->>R: (mean, std_dev)
    STATS-->>C: (mean, std_dev)
    STATS-->>S: (mean, std_dev)
    STATS-->>P: (mean, std_dev)
```

### `column_mean_std` の入出力仕様 🔵

**信頼性**: 🔵 *既存4実装の動作を統合・EDGE-001より*

| 入力 | 出力 | 備考 |
|------|------|------|
| `[1.0, 2.0, 3.0]` | `(2.0, 0.8165...)` | 通常ケース |
| `[5.0, 5.0, 5.0]` | `(5.0, 1.0)` | std=0 → 1.0 固定 |
| `[]` (空) | `(0.0, 1.0)` | pdp の既存動作に統一 |
| `[3.0]` (単一要素) | `(3.0, 1.0)` | std=0 → 1.0 固定 |

---

## 2. Newtypeパターン導入のデータフロー 🔵

**信頼性**: 🔵 *REQ-003・ユーザヒアリングより*

```mermaid
sequenceDiagram
    participant F as analysis/full.rs
    participant M as mdi.rs / rf_anova.rs 等
    participant T as types.rs

    Note over F,T: リファクタリング前

    M->>F: (Vec<f64>, f64) タプル
    F->>F: transpose_to_tree_result() → TreeImportanceResult
    Note over F: type MdiResult = TreeImportanceResult なので<br/>誰がどのメトリクスか区別不可

    Note over F,T: リファクタリング後

    M->>F: (Vec<f64>, f64) タプル
    F->>F: transpose_to_tree_result() → TreeImportanceResult
    F->>T: MdiResult(tree_result) でラップ
    Note over F: pub struct MdiResult(pub TreeImportanceResult)<br/>コンパイル時に型の区別が可能
    F->>F: SensitivityResult { mdi: Some(MdiResult(...)), ... }
```

### Newtype導入後のアクセスパターン 🔵

**信頼性**: 🔵 *REQ-003-2・コード分析より*

```rust
// 旧アクセス（型エイリアスの場合）
let result: SensitivityResult = ...;
result.mdi.as_ref().map(|r| &r.importances);     // TreeImportanceResult に直接アクセス

// 新アクセス（Newtypeの場合）
result.mdi.as_ref().map(|r| &r.0.importances);   // .0 を経由してアクセス
result.mdi.as_ref().map(|r| &r.0.r_squared);
```

---

## 3. TreeMetric Trait によるメトリクス計算フロー 🔵

**信頼性**: 🔵 *REQ-004・ユーザヒアリング: 静的ディスパッチ*

```mermaid
sequenceDiagram
    participant FULL as analysis/full.rs
    participant HELPER as run_tree_metric_for_all_objectives<M: TreeMetric>
    participant PREP as tree_common::prepare_training_data
    participant M as M::compute_importances (静的ディスパッチ)
    participant LGBM as core::lgbm

    FULL->>HELPER: metric=MdiMetric, x_matrix, objectives
    loop 各目的変数 y
        HELPER->>PREP: x_matrix, y, metric.max_rows(), seeds
        PREP->>PREP: NaN/Inf フィルタリング
        PREP->>PREP: ダウンサンプリング (max_rows 以内)
        PREP->>PREP: Fisher-Yates シャッフル
        PREP->>PREP: 80/20 ホールドアウト分割
        PREP-->>HELPER: PreparedData { x_shuffled, y_shuffled, split_idx }
        HELPER->>M: compute_importances(x_shuffled, y_shuffled)
        M->>LGBM: train_lgbm_rf(x_train, y_train, config)
        LGBM-->>M: booster
        M->>LGBM: lgbm_feature_importance(booster, p)
        LGBM-->>M: importances: Vec<f64>
        M->>LGBM: mse_to_r_squared(lgbm_mse(...))
        LGBM-->>M: r_squared: f64
        M-->>HELPER: Some((importances, r_squared))
    end
    HELPER->>HELPER: transpose_to_tree_result()
    HELPER-->>FULL: TreeImportanceResult
    FULL->>FULL: MdiResult(tree_result) でラップ
```

### 静的ディスパッチの型パラメータフロー 🔵

**信頼性**: 🔵 *ユーザヒアリング: 静的ディスパッチ採用より*

```mermaid
graph LR
    FULL["analysis/full.rs\ncompute_sensitivity_impl()"]

    FULL -->|"run_tree_metric::<RfAnovaMetric>"| RFANOVA["RfAnovaMetric::compute_importances\n→ rf_anova.rs の計算"]
    FULL -->|"run_tree_metric::<MdiMetric>"| MDI["MdiMetric::compute_importances\n→ mdi.rs の計算"]
    FULL -->|"run_tree_metric::<ShapMetric>"| SHAP["ShapMetric::compute_importances\n→ shap.rs の計算"]
    FULL -->|"run_tree_metric::<PermutationMetric>"| PFI["PermutationMetric::compute_importances\n→ permutation.rs の計算"]
```

コンパイラが各 `M` に対してモノモーフィズムでコードを生成 → vtable オーバーヘッドなし

---

## 4. mdi.rs / shap.rs の前処理統一フロー 🔵

**信頼性**: 🔵 *ユーザヒアリング: prepare_training_data に統一*

```mermaid
sequenceDiagram
    participant METRICS as mdi.rs / shap.rs
    participant TC as tree_common::prepare_training_data
    participant DATA as data::sample_rows

    Note over METRICS,DATA: リファクタリング前 (mdi.rs の例)

    METRICS->>METRICS: valid_indices 計算 (NaN/Inf フィルタ)
    METRICS->>METRICS: フィルタ後データの clone
    METRICS->>DATA: sample_rows (ダウンサンプリング)
    METRICS->>METRICS: 80/20 split 計算 (インライン)
    METRICS->>METRICS: Fisher-Yates シャッフル (インライン)

    Note over METRICS,DATA: リファクタリング後

    METRICS->>TC: prepare_training_data(x, y, MDI_MAX_ROWS, seed1, seed2)
    TC-->>METRICS: Some(PreparedData) or None
    METRICS->>METRICS: booster 訓練 (変更なし)
```

**削減されるコード量（推定）**: mdi.rs と shap.rs それぞれ約50〜60行 → `prepare_training_data` 呼び出し1行へ

---

## 5. 定数フローの変更 🔵

**信頼性**: 🔵 *REQ-005・ユーザヒアリング: constants.rs 新規作成*

```mermaid
graph TD
    subgraph BEFORE["変更前: 各ファイルにローカル定数"]
        MDI_C["mdi.rs\nconst MDI_MAX_ROWS = 1_000\nconst RF_SEED = 42"]
        SHAP_C["shap.rs\nconst SHAP_MAX_ROWS = 1_000\nconst RF_SEED = 42"]
        RFANOVA_C["rf_anova.rs\nconst RF_ANOVA_MAX_ROWS = 2_000\nconst RF_SEED = 42"]
        PFI_C["permutation.rs\nconst PFI_MAX_ROWS = 2_000\nconst PFI_SEED_BASE = 1000"]
    end

    subgraph AFTER["変更後: constants.rs に集約"]
        CONSTS_FILE["constants.rs\nMDI_MAX_ROWS = 1_000   // LightGBM 訓練コストのため\nSHAP_MAX_ROWS = 1_000  // TreeSHAP コストのため\nRF_ANOVA_MAX_ROWS = 2_000 // gain 計算のため\nPFI_MAX_ROWS = 2_000    // 5回リピートのため\nRF_SEED = 42\nPFI_SEED_BASE = 1000"]
        MDI_I["mdi.rs\nuse crate::sensitivity::constants::*"]
        SHAP_I["shap.rs\nuse crate::sensitivity::constants::*"]
        RFANOVA_I["rf_anova.rs\nuse crate::sensitivity::constants::*"]
        PFI_I["permutation.rs\nuse crate::sensitivity::constants::*"]
    end

    CONSTS_FILE --> MDI_I
    CONSTS_FILE --> SHAP_I
    CONSTS_FILE --> RFANOVA_I
    CONSTS_FILE --> PFI_I
```

---

## 6. pdp/utils.rs の変更フロー 🔵

**信頼性**: 🔵 *REQ-002-4・ユーザヒアリング: pdpも包含*

```mermaid
sequenceDiagram
    participant PDPRIDGE as pdp/ridge_core.rs
    participant PDPUTILS as pdp/utils.rs
    participant STATS as core::math::stats

    Note over PDPRIDGE,STATS: 変更前

    PDPRIDGE->>PDPUTILS: col_mean_std(&param_col)
    PDPUTILS->>PDPUTILS: ローカル実装で計算
    PDPUTILS-->>PDPRIDGE: (mean, std_dev)

    Note over PDPRIDGE,STATS: 変更後

    PDPRIDGE->>PDPUTILS: col_mean_std(&param_col)
    PDPUTILS->>STATS: column_mean_std(data)
    STATS-->>PDPUTILS: (mean, std_dev)
    PDPUTILS-->>PDPRIDGE: (mean, std_dev)
```

`pdp/utils.rs` の `col_mean_std` は `pub(super)` として残し中身を委譲、または `core::math::stats` を直接呼び出すように変更。`pdp/ridge_core.rs` の呼び出し箇所は変更不要。

---

## エラーハンドリングフロー 🔵

**信頼性**: 🔵 *EDGE-001〜003・既存実装の Option<T> パターンより*

```mermaid
flowchart TD
    A[TreeMetric::compute_importances 呼び出し] --> B{PreparedData が None?}
    B -->|はい: データ不足| C["(vec![0.0; p], 0.0) を返す"]
    B -->|いいえ| D{LightGBM 訓練成功?}
    D -->|いいえ: Noneを返す| E["(vec![0.0; p], 0.0) を返す"]
    D -->|はい| F[importance + R² 計算]
    F --> G{importances の sum が 0?}
    G -->|はい| H["全て 0.0 のまま返す"]
    G -->|いいえ| I[normalize: 合計が 1.0 になるよう正規化]
    I --> J["Some((importances, r_squared))"]
    C --> K[SensitivityResult の対応フィールド = None に設定]
    E --> K
    J --> L[SensitivityResult の対応フィールド = Some(XxxResult(result)) に設定]
```

---

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
