# sensitivity-refactoring コンテキストノート

## 技術スタック

- 言語: Rust (rust_core クレート)
- WASMビルド: wasm-pack + wasm-bindgen
- 外部依存: LightGBM (`crate::core::lgbm`)
- 乱数: `crate::core::random_forest::Lcg` (LCG法)
- テストフレームワーク: Rust 標準 `#[cfg(test)]`

## 対象モジュール

### 主対象
- `rust_core/src/sensitivity/` (16ファイル、2,356行)
  - `mod.rs`, `types.rs`, `data.rs`, `spearman.rs`, `ridge.rs`
  - `rf_anova.rs`, `mdi.rs`, `shap.rs`, `permutation.rs`, `sobol.rs`
  - `tree_common.rs`, `analysis/mod.rs`, `analysis/full.rs`, `analysis/selected.rs`, `analysis/common.rs`
  - `tests.rs`

### 波及対象
- `rust_core/src/pdp/` (7ファイル) — ridge.rs と utils.rs を共通化対象に含む
- `rust_core/src/core/math/` (既存: grid.rs, linear_algebra.rs) — stats.rs を新規追加

## 現在の重複実装

| 重複内容 | 重複箇所 |
|---------|---------|
| 列ごとの平均・標準偏差計算 | `sensitivity/ridge.rs:transpose_and_standardize`、`sensitivity/analysis/common.rs:build_standardized_param_columns`、`sensitivity/sobol.rs:column_mean_std`、`pdp/utils.rs:col_mean_std` |
| NaN/Inf フィルタリング | `tree_common.rs:prepare_training_data`（統一済み）、旧 `mdi.rs`/`shap.rs` に残留 |
| Tree系 共通前処理フロー | MDI/SHAP/RF-ANOVA/PFI の各ファイルで類似パターン |

## 既存の共通化済み処理（変更対象外）

- `tree_common.rs::prepare_training_data` — NaN/Inf フィルタリング + ダウンサンプリング + ホールドアウト分割（統一済み）
- `tree_common.rs::permute_column_inplace` — Fisher-Yates インプレースシャッフル
- `tree_common.rs::normalize` — 重要度の総和正規化

## 既存テスト（回帰なし要件）

| テスト数 | 内容 |
|---------|------|
| 29件 | Spearman, Ridge, 統合, パフォーマンス, Sobol, PFI |
| パフォーマンス制約 | Spearman: 50k×30×4 ≤500ms, Ridge: 50k×30×4 ≤300ms, Selected: 50k ≤50ms (release) |

## 追加ルール

- WASM出力の公開APIシグネチャを変更しない（`lib.rs` / `wasm.rs` から呼ばれる関数）
- `pub use` で再エクスポートしているシンボルの名前を変えない
- `#[cfg(debug_assertions)]` による debug/release 分岐テストを維持する
