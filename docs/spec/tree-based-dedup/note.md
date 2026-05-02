---
name: tree-based-dedup コンテキストノート
description: Tree-based 感度分析の重複解消リファクタリングに関する技術スタック・実装状況・注意事項
type: project
---

# tree-based-dedup コンテキストノート

**生成日**: 2026-05-02
**要件**: Tree-based 感度分析の重複解消（`refactor-tree-based-dedup.md`）

---

## 技術スタック

| レイヤー | 技術 |
|---------|------|
| コアライブラリ | Rust（`rust_core` クレート） |
| UI フレームワーク | eframe + egui |
| 機械学習 | linfa, 内製 LightGBM ラッパー（`core/lgbm`） |
| RNG | 内製 Lcg（`core/random_forest::Lcg`）|
| ビルド | cargo（ワークスペース構成）|
| テスト | cargo test |

---

## 対象ファイルと現状

### rust_core 側

| ファイル | 役割 | 重複箇所 |
|---------|------|---------|
| `rust_core/src/sensitivity/types.rs` | Result 型定義 | `RfAnovaResult`, `MdiResult`, `ShapResult`, `PermutationResult`（全て同一フィールド）|
| `rust_core/src/sensitivity/rf_anova.rs` | RF-ANOVA 計算 | `permute_single_column`（行131-154）, `normalize`（行156-167）|
| `rust_core/src/sensitivity/permutation.rs` | PFI 計算 | `permute_single_column`（行126-146）, `normalize`（行148-155）|
| `rust_core/src/sensitivity/analysis/common.rs` | 共通ヘルパー | `transpose_mdi_importances`（行136）, `transpose_rf_anova_importances`（行152）, `transpose_shap_importances`（行168）, `transpose_permutation_importances`（行184）|
| `rust_core/src/sensitivity/mod.rs` | エクスポート | `mod tree_common;` の追加が必要 |

### egui-app 側

| ファイル | 役割 | 重複箇所 |
|---------|------|---------|
| `egui-app/src/state/results.rs` | UI 用 Result 型定義 | `RfAnovaResult`, `MdiResult`, `ShapResult`, `PermutationResult`（全て同一フィールド）|
| `egui-app/src/ui/widgets/importance_chart.rs` | 重要度チャート | `compute_sorted_importance` の match arm 4 箇所（行297-332）|

---

## 公開 API（変更しない）

```rust
// rust_core 側（mod.rs から re-export）
pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64)
pub fn compute_permutation_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64)
pub fn compute_mdi_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64)
pub fn compute_shap_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64)

// 型（型エイリアス化後も同一名でアクセス可能）
pub type MdiResult = TreeImportanceResult;
pub type RfAnovaResult = TreeImportanceResult;
pub type ShapResult = TreeImportanceResult;
pub type PermutationResult = TreeImportanceResult;
```

---

## 実装済みテストケース（既存テスト保護対象）

`rust_core/src/sensitivity/tests.rs` より：

### RF-ANOVA 関連
- `tc_801_14_rf_anova_importances_sum_to_one_per_objective`
- `tc_801_15_rf_anova_small_dataset_non_zero`

### PFI 関連
- `tc_pfi_001_01_normal_case`（合計≈1.0 検証）
- `tc_pfi_001_02_single_feature`
- `tc_pfi_001_e03_nan_filtering`
- `tc_pfi_001_b01_min_valid_rows`
- `tc_pfi_001_e02_empty_input`
- `tc_pfi_int_01_single_obj_returns_some`
- `tc_pfi_int_02_result_shape`

---

## 実装の差分・注意事項

### normalize の実装スタイル差異
- `rf_anova.rs`（行156-167）: `for` ループ使用
- `permutation.rs`（行148-155）: `iter_mut().for_each()` 使用
- → 機能的に同一。統合時にどちらかのスタイルへ統一

### R² 計算の差異
- `rf_anova.rs`: 独自実装（行107-113）
- `permutation.rs`: `mse_to_r_squared()` 関数を使用
- → 統合時は `mse_to_r_squared()` 呼び出しに統一することが望ましい

### permutation.rs のみの特徴
- `N_REPEATS: 5` による反復計算（行107-117）
- `mse_to_r_squared` のインポートが追加

### rf_anova.rs のみの特徴
- `RF_ANOVA_MAX_ROWS: 2000`, `permutation.rs` は `PFI_MAX_ROWS: 2000`（数値は同一）
- 定数名が異なる

### タスクファイルに明記されていない潜在的な重複
- NaN/Inf フィルタリング: `rf_anova.rs`（行23-31）と `permutation.rs`（行27-34）
- 80/20 holdout split: 両ファイルに類似実装
- → タスクファイルのスコープ外。別 issue として検討

---

## 影響範囲

1. `mod.rs` の `pub use` は変更不要（型エイリアスにより外部参照先が維持される）
2. `full.rs` の呼び出し側が `transpose_to_tree_result` に切り替わる
3. `results.rs`（egui-app 側）は独立した型定義を持つため、別途同様の変更が必要
4. `importance_chart.rs` の `compute_sorted_importance` のみが egui-app 側の変更対象

---

## 開発規則

- リファクタリング中は機能追加を行わない
- 各ステップで `cargo test` が全て通ることを確認
- 型エイリアスは `pub type` で公開し、外部クレートの互換性を維持
- `serde` derive が必要な場合は `TreeImportanceResult` に付与する
