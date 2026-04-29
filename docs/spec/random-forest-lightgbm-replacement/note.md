---
name: RandomForest LightGBM置き換えコンテキストノート
description: RandomForest → LightGBM置き換えの技術コンテキスト、実装詳細、注意事項
type: project
---

# RandomForest → LightGBM 置き換え コンテキストノート

## プロジェクト概要

Tunny Dashboard (egui-app) の `rust_core` ライブラリ内にある純Rust実装の Random Forest を、
`lightgbm` クレート（LightGBM C++ バインディング）に置き換える。

## 技術スタック

- **言語**: Rust (エディション2021)
- **ターゲット**: ネイティブデスクトップ（Windows / macOS / Linux）。**WASMは廃止済み**
- **ワークスペース**: `rust_core` + `egui-app`（Cargo workspace）
- **新規依存**: `lightgbm` クレート（lightgbm-rs: LightGBM C++ FFI バインディング）
- **ダイナミックリンク**: ユーザー提供の `lib_lightgbm.dll` を指定フォルダに配置して利用

## 現行実装の構造

### RandomForest モジュール (`rust_core/src/core/random_forest/`)

| ファイル | 役割 |
|---|---|
| `forest.rs` | `RandomForest::train`, `predict`, `mse_on_dataset`, `train_rf_on_columns` |
| `tree.rs` | `build_tree`, `find_best_split`, `predict_one` |
| `pdp.rs` | `compute_pdp_2d_rf`: 2D PDP サーフェス計算 |
| `types.rs` | `TreeNode`, `DecisionTree`, `RandomForest` 構造体 |
| `rng.rs` | `Lcg`: LCG 乱数生成器（他モジュールからも使用） |
| `tests.rs` | ユニットテスト群 |

### RandomForest の利用箇所 (4箇所)

| ファイル | 用途 | 置き換え方針 |
|---|---|---|
| `rust_core/src/pdp/api.rs:168` | `"random_forest"` モデルタイプの 2D PDP | LightGBM RF モード |
| `rust_core/src/sensitivity/shap.rs` | SHAP の baseline 回帰モデル | LightGBM RF モード |
| `rust_core/src/sensitivity/mdi.rs` | MDI 感度分析（MdiNode + 内部ツリー走査） | LightGBM `feature_importance(gain)` |
| `rust_core/src/sensitivity/rf_anova.rs` | RF-ANOVA 感度分析 | LightGBM RF モード |

### Lcg (LCG 乱数生成器) の利用箇所 ⚠️ 削除禁止

`Lcg` は `random_forest` モジュール内で定義されているが、RF 以外からも使われている:

- `rust_core/src/core/kriging/sparse_fitc.rs:38`
- `rust_core/src/core/kriging/gaussian_process/training.rs:14`

→ RF 削除後も `Lcg` は `core::random_forest` （またはリファクタリングして別モジュール）に保持すること。

## LightGBM クレートの仕様

- クレート名: `lightgbm` (crates.io: `lightgbm-rs`)
- LightGBM の RandomForest モード: `boosting_type = "rf"` + `bagging_fraction < 1.0` + `feature_fraction < 1.0`
- 対応する feature importance: `Booster::feature_importance(ImportanceType::Gain)` → MDI 代替
- システム依存: `lib_lightgbm.dll` (Windows) — ユーザーが指定フォルダに配置

## DLL リンク設定

`lib_lightgbm.dll` の配置フォルダを `build.rs` または `.cargo/config.toml` の `rustflags` で指定する。
ユーザーが DLL パスを決定次第、リンク設定を実装する。

## MDI の設計変更点

**現行**: `MdiNode` enum で独自ツリー構造を走査し `weighted_gain` を集計  
**新規**: LightGBM の `feature_importance(ImportanceType::Gain)` を利用  
→ 計算結果が変わる可能性あり。ユーザーにより **許容済み**。

## WASM 廃止

現時点で `rust_core/Cargo.toml` に WASM 固有の設定は確認されていないが、
`#[cfg(target_arch = "wasm32")]` の条件コンパイルブロックが残存する場合は削除する。

## 現行ハイパーパラメータ（参考値）

各利用箇所で使われている RF のデフォルト値:

| 利用箇所 | n_trees | max_depth | min_samples_leaf | seed |
|---|---|---|---|---|
| PDP 2D | 100 | 10 | 2 | 42 |
| MDI | 64 | 64 | 2 | 42 |
| SHAP/RF-ANOVA | コードで確認要 | — | — | — |

LightGBM への変換時は `num_iterations` (= n_trees), `max_depth`, `min_data_in_leaf` にマッピングする。
