# tree-based-dedup アーキテクチャ設計

**作成日**: 2026-05-02
**関連要件定義**: [requirements.md](../../spec/tree-based-dedup/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・既存実装を参考にした確実な設計
- 🟡 **黄信号**: 要件定義書・既存実装から妥当な推測による設計
- 🔴 **赤信号**: 要件定義書・既存実装にない推測による設計

---

## 変更概要 🔵

**信頼性**: 🔵 *要件定義書 Epic 1〜5 より*

Tree-based 感度分析（RfAnova / MDI / SHAP / Permutation）の4メトリクスにわたる重複コードを解消する。既存の 2 層コア/UI 分離アーキテクチャを維持し、各ステップが独立してコンパイル可能な段階的リファクタリングを行う。

---

## 変更ファイル一覧 🔵

**信頼性**: 🔵 *要件定義書・既存コードベース調査より*

| ファイル | 変更種別 | 変更内容 |
|---|---|---|
| `rust_core/src/sensitivity/tree_common.rs` | **新規** | `permute_single_column`, `normalize` を集約 |
| `rust_core/src/sensitivity/types.rs` | 修正 | `TreeImportanceResult` 導入、4型をエイリアス化 |
| `rust_core/src/sensitivity/mod.rs` | 修正 | `mod tree_common;` 追加、`TreeImportanceResult` エクスポート |
| `rust_core/src/sensitivity/rf_anova.rs` | 修正 | ローカル関数削除 → `tree_common` から import、R² 計算を `mse_to_r_squared()` に統一 |
| `rust_core/src/sensitivity/permutation.rs` | 修正 | ローカル関数削除 → `tree_common` から import |
| `rust_core/src/sensitivity/analysis/common.rs` | 修正 | 4関数を `transpose_to_tree_result` に統合 |
| `rust_core/src/sensitivity/analysis/full.rs` | 修正 | 呼び出しを `transpose_to_tree_result` に更新 |
| `egui-app/src/state/results.rs` | 修正 | `TreeImportanceResult` 導入、4型をエイリアス化 |
| `egui-app/src/ui/widgets/importance_chart.rs` | 修正 | `extract_tree_importance` 導入、4 arm を統合 |

---

## 詳細設計

### 1. tree_common.rs の新規作成 🔵

**信頼性**: 🔵 *要件 REQ-401〜406・既存 rf_anova.rs/permutation.rs 実装より*

**関連要件**: REQ-401, REQ-402, REQ-403, REQ-404

```rust
// rust_core/src/sensitivity/tree_common.rs

use crate::core::random_forest::Lcg;

/// 指定列を Fisher-Yates シャッフルで並び替えた行列を返す
pub(crate) fn permute_single_column(
    x_matrix: &[Vec<f64>],
    feature_idx: usize,
    seed: u64,
) -> Option<Vec<Vec<f64>>> {
    let n = x_matrix.len();
    if n == 0 {
        return None;
    }

    let mut column_values: Vec<f64> = x_matrix.iter().map(|row| row[feature_idx]).collect();

    let mut rng = Lcg::new(seed);
    for i in (1..n).rev() {
        let j = rng.next_usize(i + 1);
        column_values.swap(i, j);
    }

    let mut permuted = x_matrix.to_vec();
    for (i, row) in permuted.iter_mut().enumerate() {
        row[feature_idx] = column_values[i];
    }
    Some(permuted)
}

/// 値を合計で正規化する。合計が 0 以下の場合は全要素を 0.0 にする
pub(crate) fn normalize(values: &mut [f64]) {
    let sum = values.iter().sum::<f64>();
    if sum < f64::EPSILON {
        for v in values.iter_mut() {
            *v = 0.0;
        }
        return;
    }
    for v in values.iter_mut() {
        *v /= sum;
    }
}
```

**設計決定**:
- `pub(crate)` 可視性: crate 内の `rf_anova.rs`, `permutation.rs` からアクセス可能とする 🔵
- `normalize` は for ループスタイルを採用: ヒアリング確認済み 🔵
- EDGE-001, EDGE-002 のエッジケース動作を維持 🔵

### 2. types.rs の型統合 🔵

**信頼性**: 🔵 *要件 REQ-001〜004・既存 types.rs 実装より*

**関連要件**: REQ-001, REQ-002, REQ-003, REQ-004, REQ-101, REQ-102

```rust
// rust_core/src/sensitivity/types.rs

/// Tree-based 感度分析の共通結果型
#[derive(Debug, Clone)]
pub struct TreeImportanceResult {
    pub importances: Vec<Vec<f64>>, // [param][objective]
    pub r_squared: Vec<f64>,        // [objective]
}

pub type MdiResult = TreeImportanceResult;
pub type RfAnovaResult = TreeImportanceResult;
pub type ShapResult = TreeImportanceResult;
pub type PermutationResult = TreeImportanceResult;
```

**設計決定**:
- `serde` derive は現状不要（既存4型にも付与されていない）。将来必要になった場合は `TreeImportanceResult` に付与 🟡
- `SensitivityResult` のフィールド型は `Option<RfAnovaResult>` 等のまま（型エイリアス経由で同一動作） 🔵

### 3. mod.rs の更新 🔵

**信頼性**: 🔵 *既存 mod.rs 構造・要件 REQ-406 より*

```rust
// rust_core/src/sensitivity/mod.rs に追加
mod tree_common;

// pub use 行に TreeImportanceResult を追加
pub use types::{
    MdiResult, PermutationResult, RfAnovaResult, RidgeResult, SensitivityMetric,
    SensitivityResult, ShapResult, SobolResult, TreeImportanceResult,
};
```

### 4. rf_anova.rs の修正 🔵

**信頼性**: 🔵 *要件 REQ-405, REQ-501, REQ-502 より*

```diff
  use super::data::sample_rows;
+ use super::tree_common::{normalize, permute_single_column};
  use crate::core::lgbm::{lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};
  use crate::core::random_forest::Lcg;

- // 行131-167: permute_single_column と normalize を削除
+ // tree_common から import に変更済み
```

R² 計算の変更:

```diff
- // 独自 R² 計算（行107-113）
- let ss_res: f64 = y.iter().zip(y_pred.iter())
-     .map(|(a, b)| (a - b).powi(2)).sum();
- let ss_tot: f64 = y.iter()
-     .map(|v| (v - y_mean).powi(2)).sum();
- let r_squared = if ss_tot < f64::EPSILON { 0.0 } else { 1.0 - ss_res / ss_tot };

+ let r_squared = mse_to_r_squared(baseline_mse, y);
```

### 5. permutation.rs の修正 🔵

**信頼性**: 🔵 *要件 REQ-405 より*

```diff
  use super::data::sample_rows;
+ use super::tree_common::{normalize, permute_single_column};
  use crate::core::lgbm::{lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};
  use crate::core::random_forest::Lcg;

- // 行126-155: permute_single_column と normalize を削除
+ // tree_common から import に変更済み
```

### 6. common.rs の transpose 統合 🔵

**信頼性**: 🔵 *要件 REQ-201〜203・既存 common.rs 実装より*

4つの `transpose_*_importances` 関数を `transpose_to_tree_result` に統合:

```rust
// rust_core/src/sensitivity/analysis/common.rs

use super::super::types::TreeImportanceResult;

pub(super) fn transpose_to_tree_result(
    importances_by_objective: &[Vec<f64>],
    r_squared: Vec<f64>,
    param_count: usize,
    objective_count: usize,
) -> TreeImportanceResult {
    TreeImportanceResult {
        importances: transpose_importances_matrix(
            importances_by_objective,
            param_count,
            objective_count,
        ),
        r_squared,
    }
}
```

**削除対象**:
- `transpose_mdi_importances`（行136-150）
- `transpose_rf_anova_importances`（行152-166）
- `transpose_shap_importances`（行168-182）
- `transpose_permutation_importances`（行184-198）

### 7. full.rs の呼び出し更新 🔵

**信頼性**: 🔵 *要件 REQ-301・既存 full.rs 呼び出し箇所より*

```diff
  use super::common::{
      build_standardized_param_columns, compute_ridge_from_standardized_columns, empty_result,
-     transpose_mdi_importances, transpose_permutation_importances,
-     transpose_rf_anova_importances, transpose_shap_importances,
+     transpose_to_tree_result,
  };
```

呼び出し箇所の置換（例: single_obj の RfAnova）:

```diff
- let result = transpose_rf_anova_importances(&[imp], vec![r2], p, 1);
+ let result = transpose_to_tree_result(&[imp], vec![r2], p, 1);
```

全8箇所（single_obj 4箇所 + all 4箇所）を同様に置換。

### 8. egui-app results.rs の型統合 🔵

**信頼性**: 🔵 *要件 REQ-701, REQ-702 より*

```rust
// egui-app/src/state/results.rs

#[derive(Debug, Clone)]
pub struct TreeImportanceResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

pub type MdiResult = TreeImportanceResult;
pub type RfAnovaResult = TreeImportanceResult;
pub type ShapResult = TreeImportanceResult;
pub type PermutationResult = TreeImportanceResult;

#[derive(Debug, Clone)]
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<TreeImportanceResult>,
    pub mdi: Option<TreeImportanceResult>,
    pub shap: Option<TreeImportanceResult>,
    pub permutation: Option<TreeImportanceResult>,
}
```

### 9. importance_chart.rs の match arm 統合 🔵

**信頼性**: 🔵 *要件 REQ-601〜603 より*

```rust
// egui-app/src/ui/widgets/importance_chart.rs

fn extract_tree_importance(
    result: &SensitivityResult,
    metric: &ImportanceMetric,
    obj_idx: usize,
) -> Option<Vec<f64>> {
    let importances = match metric {
        ImportanceMetric::RfAnova => result.rf_anova.as_ref()?.importances.clone(),
        ImportanceMetric::Mdi => result.mdi.as_ref()?.importances.clone(),
        ImportanceMetric::Shap => result.shap.as_ref()?.importances.clone(),
        ImportanceMetric::Permutation => result.permutation.as_ref()?.importances.clone(),
        _ => return None,
    };
    Some(
        importances
            .iter()
            .map(|param_imp| param_imp.get(obj_idx).copied().unwrap_or(0.0).abs())
            .collect(),
    )
}
```

`compute_sorted_importance` 内の置換:

```diff
  match metric {
      ImportanceMetric::Spearman => { /* 変更なし */ }
      ImportanceMetric::Ridge => { /* 変更なし */ }
-     ImportanceMetric::RfAnova => {
-         let Some(ref rf) = result.rf_anova else { return vec![]; };
-         rf.importances.iter()
-             .map(|param_imp| param_imp.get(obj_idx).copied().unwrap_or(0.0).abs())
-             .collect()
-     }
-     ImportanceMetric::Mdi => { /* ... */ }
-     ImportanceMetric::Shap => { /* ... */ }
-     ImportanceMetric::Permutation => { /* ... */ }
+     ImportanceMetric::RfAnova
+     | ImportanceMetric::Mdi
+     | ImportanceMetric::Shap
+     | ImportanceMetric::Permutation => {
+         extract_tree_importance(result, metric, obj_idx).unwrap_or_default()
+     }
      ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => return vec![],
  };
```

---

## 段階的実装順序 🔵

**信頼性**: 🔵 *要件 NFR-102・タスクファイル実装順序より*

各 Step は独立してコンパイルが通る:

| Step | 変更内容 | 検証コマンド |
|------|---------|-------------|
| 1 | `tree_common.rs` 新規 → `rf_anova.rs`/`permutation.rs` を import に変更 | `cargo test` |
| 2 | `types.rs` で `TreeImportanceResult` 導入 + 型エイリアス化 | `cargo build` |
| 3 | `common.rs` で `transpose_to_tree_result` 統合 → `full.rs` 呼び出し更新 | `cargo test` |
| 4 | `egui-app/results.rs` で `TreeImportanceResult` 導入 | `cargo build` |
| 5 | `importance_chart.rs` で `extract_tree_importance` 導入 | `cargo test` |
| 6 | `cargo clippy` で警告確認 → 全テスト再実行 | `cargo clippy && cargo test` |

---

## 技術的制約

### 循環依存の回避 🔵

**信頼性**: 🔵 *モジュール構造 mod.rs より*

- `tree_common` は `core::random_forest::Lcg` に依存（既存方向: sensitivity → core）
- `rf_anova` / `permutation` → `tree_common` は `super::tree_common`（同一モジュール内）
- 新たな循環依存は発生しない

### 可視性 🔵

**信頼性**: 🔵 *既存 sensitivity モジュールの可視性パターンより*

- `tree_common::permute_single_column`: `pub(crate)` — crate 内の `rf_anova.rs`, `permutation.rs` からアクセス可能
- `tree_common::normalize`: `pub(crate)` — 同上
- `TreeImportanceResult`: `pub` — 外部クレートからエクスポート
- 型エイリアス: `pub type` — 外部クレートから透過的にアクセス可能

### 後方互換性 🔵

**信頼性**: 🔵 *要件 NFR-201 より*

- `MdiResult`, `RfAnovaResult` 等の型名は `pub type` で維持
- フィールドアクセス（`.importances`, `.r_squared`）は変更なし
- `mod.rs` の `pub use` に既存型名を含め続ける

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/tree-based-dedup/requirements.md)
- **コンテキストノート**: [note.md](../../spec/tree-based-dedup/note.md)
- **類似設計**: [pdp-linspace-dedup architecture](../pdp-linspace-dedup/architecture.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 18件 (95%)
- 🟡 黄信号: 1件 (5%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質 — 全設計項目が要件定義書・既存実装・ヒアリング確認に基づく
