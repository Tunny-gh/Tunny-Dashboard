# Permutation Feature Importance コンテキストノート

**生成日**: 2026-05-02

## プロジェクト基本情報

- **リポジトリ**: c:\Users\hiroa\Desktop\Tunny-Dashboard
- **技術スタック**: Rust / egui / egui_plot / LightGBM FFI
- **テスト**: `cargo test`（rust_core / egui-app）

## 現在の ImportanceMetric 実装状況

| ImportanceMetric (egui-app) | SensitivityMetric (rust_core) | 実装ファイル | キャッシュID |
|---|---|---|---|
| Spearman | Spearman | spearman.rs | 0 |
| Ridge | Ridge | ridge.rs | 1 |
| RfAnova | RfAnova | rf_anova.rs | 2 |
| Mdi | Mdi | mdi.rs | 3 |
| SobolFirst | — | sobol.rs | 4 |
| SobolTotal | — | sobol.rs | 5 |
| Shap | Shap | shap.rs | 6 |
| **追加予定: Permutation** | **Permutation** | **permutation.rs（新規）** | **7** |

## RF-Anova との違い

現在の RF-Anova は1回のシャッフルのみ（n_repeats=1）でMSE増加量を計算する。  
新規 Permutation Feature Importance は **n_repeats=5** で繰り返しシャッフルし、平均MSE増加量を返すことで統計的安定性が向上する。

## 既存実装パターン（LightGBM ベースメトリクス共通）

```
1. NaN/Inf フィルタリング
2. ダウンサンプリング（最大行数）
3. 80/20 holdout 分割（Fisher-Yates シャッフル、seed=43）
4. LightGBM RF 学習（train_lgbm_rf）
5. eval セットで baseline_mse 計算（lgbm_mse）
6. 各特徴量をシャッフルして permuted_mse を計算
7. importance = max(permuted_mse - baseline_mse, 0.0) を正規化
8. R² = mse_to_r_squared(baseline_mse, y_eval) で返す
```

## 変更対象ファイル

### rust_core 側
- `rust_core/src/sensitivity/types.rs` — `SensitivityMetric::Permutation` 追加、`PermutationResult` 構造体追加、`SensitivityResult.permutation` フィールド追加
- `rust_core/src/sensitivity/mod.rs` — `pub use permutation::compute_permutation_importances` エクスポート追加
- `rust_core/src/sensitivity/analysis/full.rs` — `compute_sensitivity_single_obj` にPermutationケース追加
- `rust_core/src/sensitivity/permutation.rs`（**新規作成**） — n_repeats=5 実装

### egui-app 側
- `egui-app/src/ui/widgets/importance_chart.rs` — `ImportanceMetric::Permutation` 追加（cache_id=7）
- `egui-app/src/state/results.rs` — `PermutationResult` 構造体追加、`SensitivityResult.permutation` フィールド追加
- `egui-app/src/ui/chart_registry.rs` — Permutation のディスパッチケース追加

## 設定値方針（RF-Anova との比較）

| パラメータ | RF-Anova | Permutation |
|---|---|---|
| n_repeats | 1 | 5 |
| max_rows | 2,000 | 2,000 |
| num_trees | 100 | 100 |
| max_depth | 10 | 10 |
| min_data_in_leaf | 2 | 2 |
| seed_base | 42 | 42 |

## テスト方針

- `rust_core/src/sensitivity/tests.rs` に Permutation テストケースを追加
- 最小ケース（n=2, p=1）、通常ケース、NaN 混入ケース、大規模ケースを検証

## 関連ファイルパス

- `rust_core/src/sensitivity/types.rs`
- `rust_core/src/sensitivity/mod.rs`
- `rust_core/src/sensitivity/rf_anova.rs`（実装の参考）
- `rust_core/src/sensitivity/analysis/full.rs`
- `egui-app/src/ui/widgets/importance_chart.rs`
- `egui-app/src/state/results.rs`
- `egui-app/src/ui/chart_registry.rs`
