# リファクタリング: Tree-based 感度分析の重複解消

**作成日**: 2026-05-02
**対象モジュール**: `rust_core/src/sensitivity`, `egui-app/src/ui`
**推定工数**: 4〜6時間

## 背景

Permutation Feature Importance (TASK-2156〜2160) の実装において、既存の RfAnova / MDI / SHAP と同一のコードパターンを踏襲した。その結果、`compute_sorted_importance` の match arm、`transpose_*_importances` 関数、`permute_single_column` / `normalize` ヘルパーがそれぞれ重複している。これらを統合し、今後の新メトリクス追加コストを下げる。

## 対象となる重複

### 1. 同一構造の Result 型（4型）

**ファイル**: `rust_core/src/sensitivity/types.rs`

`MdiResult`, `RfAnovaResult`, `ShapResult`, `PermutationResult` がすべて同一フィールド（`importances: Vec<Vec<f64>>`, `r_squared: Vec<f64>`）を持つ。

**efui-app 側**: `egui-app/src/state/results.rs` も同様に4つの同一構造体型を持つ。

**方針**: 単一の `TreeImportanceResult` 型を導入し、既存名を型エイリアスに変更。

```rust
// types.rs
#[derive(Debug, Clone)]
pub struct TreeImportanceResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

pub type MdiResult = TreeImportanceResult;
pub type RfAnovaResult = TreeImportanceResult;
pub type ShapResult = TreeImportanceResult;
pub type PermutationResult = TreeImportanceResult;
```

**影響範囲**: 全フィールドアクセス箇所は型エイリアス経由でそのまま動作。`mod.rs` の `pub use` は変更不要。

### 2. transpose 関数の重複（4関数）

**ファイル**: `rust_core/src/sensitivity/analysis/common.rs:136-198`

`transpose_mdi_importances`, `transpose_rf_anova_importances`, `transpose_shap_importances`, `transpose_permutation_importances` がすべて `transpose_importances_matrix` を呼んで同じ構造を返すのみ。

**方針**: 単一のジェネリック関数に統合。

```rust
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

**影響範囲**: `full.rs` の各メトリクス match arm が呼び出し先を変更。戻り値型が `TreeImportanceResult` になるため、`SensitivityResult` の各 `Option<TreeImportanceResult>` フィールドにそのまま代入可能。

### 3. permute_single_column / normalize の重複（2ファイル）

**ファイル**:
- `rust_core/src/sensitivity/rf_anova.rs:131-167`
- `rust_core/src/sensitivity/permutation.rs:126-155`

両ファイルに `permute_single_column` と `normalize` の同一実装が存在。

**方針**: 共有モジュール `rust_core/src/sensitivity/tree_common.rs`（新規）に移動。

```rust
// tree_common.rs
pub(crate) fn permute_single_column(
    x_matrix: &[Vec<f64>],
    feature_idx: usize,
    seed: u64,
) -> Option<Vec<Vec<f64>>> { /* ... */ }

pub(crate) fn normalize(values: &mut [f64]) { /* ... */ }
```

`rf_anova.rs` と `permutation.rs` からは `use super::tree_common::{permute_single_column, normalize};` で参照。

**影響範囲**: `rf_anova.rs`, `permutation.rs` の関数定義を削除して import に変更。`mod.rs` に `mod tree_common;` を追加。

### 4. compute_sorted_importance の match arm 重複（4 arm）

**ファイル**: `egui-app/src/ui/widgets/importance_chart.rs:297-332`

RfAnova, Mdi, Shap, Permutation の4 arm が「`Option<TreeImportanceResult>` から importances を抽出して obj_idx でスライス」という同一ロジックを実行。

**方針**: ヘルパー関数を抽出。

```rust
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

`compute_sorted_importance` 内の4 arm を `if let Some(scores) = extract_tree_importance(result, metric, obj_idx) { scores } else { return vec![]; }` に置換。

**影響範囲**: `importance_chart.rs` の `compute_sorted_importance` 関数のみ。

## 実装順序

各ステップは独立してコンパイルが通るよう、順番に実施する。

1. **Step 1**: `tree_common.rs` 新規作成 → `permute_single_column` / `normalize` を移動 → `rf_anova.rs` / `permutation.rs` を import に変更 → `cargo test`
2. **Step 2**: `types.rs` で `TreeImportanceResult` 導入 + 型エイリアス化 → `cargo build`
3. **Step 3**: `common.rs` で `transpose_to_tree_result` 統合 → 各 transpose 関数を削除 → `full.rs` の呼び出しを更新 → `cargo test`
4. **Step 4**: `egui-app/src/state/results.rs` で同様に `TreeImportanceResult` 導入 → `cargo build`
5. **Step 5**: `importance_chart.rs` で `extract_tree_importance` 導入 → 4 arm を統合 → `cargo test`
6. **Step 6**: `cargo clippy` で警告確認 → 全テスト再実行

## 注意事項

- 型エイリアスは `pub type` で公開するため、外部 crate からの参照先は変わらない
- `serde` 等の derive が必要な場合は `TreeImportanceResult` に付与する
- リファクタリング中は機能追加を行わず、既存テストがすべて通ることを各ステップで確認する
