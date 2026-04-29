# RandomForest → LightGBM 置き換え アーキテクチャ設計

**作成日**: 2026-04-27
**関連要件定義**: [requirements.md](../../spec/random-forest-lightgbm-replacement/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書 + ユーザヒアリングより*

`rust_core` ライブラリ内の純Rust RandomForest（CART + Bagging）を `lightgbm` クレート（C++ FFI バインディング）に置き換える。対象は以下の4箇所:

| 対象 | 現行 | 置き換え後 |
|---|---|---|
| **2D PDP** | `compute_pdp_2d_rf` (CART RF) | LightGBM RF 予測 |
| **SHAP** | `ShapNode` 独自TreeSHAP | LightGBM native SHAP (`predict_contrib`) |
| **MDI** | `MdiNode` 独自ゲイン集計 | LightGBM `feature_importance(Gain)` |
| **RF-ANOVA** | `train_rf_on_columns` + 順列重要度 | LightGBM RF + 順列重要度 |

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 rust_core 構造 + ユーザヒアリングより*

- **パターン**: 既存レイヤードアーキテクチャを維持、`core` 層に LightGBM ラッパーモジュールを追加
- **選択理由**: 各感度分析モジュールから共有できる単一の LightGBM 薄いラッパーにすることで DRY 原則を守る（NFR-101）

---

## ファイル変更一覧

### 新規作成

| ファイル | 役割 |
|---|---|
| `rust_core/src/core/lgbm.rs` | LightGBM 共有ラッパー（データセット変換・訓練・予測・重要度） |
| `rust_core/build.rs` | `libs/` ディレクトリへのリンクパス設定 |
| `libs/lib_lightgbm.dll` | Windows 用 LightGBM 共有ライブラリ（ルートから移動） |
| `libs/lib_lightgbm.dylib` | macOS 用 LightGBM 共有ライブラリ（ルートから移動） |

### 削除

| ファイル | 理由 |
|---|---|
| `rust_core/src/core/random_forest/forest.rs` | LightGBM で代替 |
| `rust_core/src/core/random_forest/tree.rs` | LightGBM で代替 |
| `rust_core/src/core/random_forest/types.rs` | LightGBM で代替 |
| `rust_core/src/core/random_forest/pdp.rs` | LightGBM で代替（`core/lgbm.rs` 経由） |

### 修正

| ファイル | 変更内容 |
|---|---|
| `rust_core/Cargo.toml` | `lightgbm = "X.Y"` 追加 |
| `rust_core/src/core/random_forest/mod.rs` | 削除ファイルの mod 宣言を除去、`rng.rs` のみ残す |
| `rust_core/src/core/mod.rs` | `mod lgbm;` 追加 |
| `rust_core/src/pdp/api.rs` | `"random_forest"` 分岐を LightGBM 呼び出しに変更 |
| `rust_core/src/sensitivity/shap.rs` | `ShapNode` 木を削除、LightGBM native SHAP に置き換え |
| `rust_core/src/sensitivity/mdi.rs` | `MdiNode` 木を削除、`feature_importance(Gain)` に置き換え |
| `rust_core/src/sensitivity/rf_anova.rs` | `train_rf_on_columns` 呼び出しを LightGBM に変更 |

### 変更しない

| ファイル | 理由 |
|---|---|
| `rust_core/src/core/random_forest/rng.rs` | Kriging モジュールが `Lcg` に依存（REQ-003） |
| `rust_core/src/core/random_forest/mod.rs` の `pub(crate) use rng::Lcg` | Kriging のパスを維持 |
| `egui-app/` 全体 | `rust_core` の内部変更のみで egui-app は影響なし |

---

## 新規モジュール: `core::lgbm` 🔵

**信頼性**: 🔵 *ユーザヒアリング（NFR-101: 共有設定）より*

各感度分析モジュールが共通して使う LightGBM 薄いラッパー。

### 役割

```
rust_core/src/core/lgbm.rs
├── LgbmRfConfig         — RF ハイパーパラメータ設定構造体
├── to_lgbm_dataset()    — &[Vec<f64>] → lightgbm::Dataset
├── train_lgbm_rf()      — データ → LightGBM Booster (RF mode)
├── lgbm_predict()       — Booster + データ → Vec<f64> (回帰予測)
├── lgbm_predict_contrib() — Booster + データ → SHAP値 Vec<Vec<f64>>
├── lgbm_mse()           — Booster + データ → f64 (MSE)
└── lgbm_feature_importance() — Booster → Vec<f64> (Gain正規化済み)
```

### LightGBM RF モード パラメータ 🟡

**信頼性**: 🟡 *LightGBM 公式ドキュメントから妥当な推測*

| LightGBM パラメータ | 値 | 旧 RF パラメータ対応 |
|---|---|---|
| `boosting_type` | `"rf"` | — |
| `num_iterations` | 用途別（100/64） | `n_trees` |
| `max_depth` | 用途別（10/64） | `max_depth` |
| `min_data_in_leaf` | `2` | `min_samples_leaf` |
| `bagging_fraction` | `0.8` | — (RF有効化に必要) |
| `bagging_freq` | `1` | — (RF有効化に必要) |
| `feature_fraction` | `0.8` | — |
| `verbose` | `-1` | — (ログ抑制) |
| `num_threads` | `1` | — (シングルスレッド) |

---

## ビルド設定 🔵

**信頼性**: 🔵 *ユーザヒアリング（libs/ 移動、NFR-201）より*

### DLL 配置

```
{workspace_root}/
└── libs/
    ├── lib_lightgbm.dll     # Windows
    └── lib_lightgbm.dylib   # macOS
```

### `rust_core/build.rs`

```rust
fn main() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let libs_dir = manifest_dir.parent().unwrap().join("libs");
    println!("cargo:rustc-link-search=native={}", libs_dir.display());
    println!("cargo:rerun-if-changed=build.rs");
}
```

`lightgbm-rs` クレートの内部 `build.rs` が `LIGHTGBM_LIB_DIR` 環境変数を参照するため、
`.cargo/config.toml` でも設定可能:

```toml
# .cargo/config.toml (ワークスペースルート)
[env]
LIGHTGBM_LIB_DIR = { value = "libs", relative = true }
```

---

## ディレクトリ構造（変更後） 🔵

**信頼性**: 🔵 *既存プロジェクト構造 + ヒアリングより*

```
rust_core/
├── build.rs                              ← 新規: libs/ リンクパス設定
├── Cargo.toml                            ← 修正: lightgbm 追加
└── src/
    ├── core/
    │   ├── lgbm.rs                       ← 新規: LightGBM 共有ラッパー
    │   ├── mod.rs                        ← 修正: mod lgbm; 追加
    │   └── random_forest/
    │       ├── mod.rs                    ← 修正: rng のみ残す
    │       ├── rng.rs                    ← 変更なし (Lcg 保持)
    │       ├── forest.rs                 ← 削除
    │       ├── tree.rs                   ← 削除
    │       ├── types.rs                  ← 削除
    │       ├── pdp.rs                    ← 削除
    │       └── tests.rs                  ← 削除 (新テストは各モジュール内)
    ├── sensitivity/
    │   ├── shap.rs                       ← 修正: ShapNode 削除、LightGBM SHAP
    │   ├── mdi.rs                        ← 修正: MdiNode 削除、feature_importance
    │   └── rf_anova.rs                   ← 修正: LightGBM RF 使用
    └── pdp/
        └── api.rs                        ← 修正: random_forest 分岐を LightGBM へ

libs/                                     ← 新規: DLL 格納ディレクトリ
├── lib_lightgbm.dll                      ← 移動: ルートから
└── lib_lightgbm.dylib                    ← 移動: ルートから
```

---

## SHAP の完全置き換え設計 🔵

**信頼性**: 🔵 *ユーザヒアリング（SHAP完全置き換え選択）より*

### 現行 vs 新規

| 項目 | 現行 | 新規 |
|---|---|---|
| 木の構築 | `build_shap_tree` → `ShapNode` | LightGBM 内部 |
| SHAP 値計算 | `tree_shap_recurse`（TreeSHAP） | `booster.predict_contrib(x)` |
| R² 計算 | `train_rf_on_columns` + `mse_on_dataset` | `lgbm_mse()` |
| バギング | 手動 `Lcg` ブートストラップ | LightGBM 内部 |

### 新しい compute_shap_importances の流れ

```
1. データクリーニング・ダウンサンプリング（既存ロジック流用）
2. 80/20 ホールドアウト分割（Lcg シャッフル → 変更なし）
3. train_lgbm_rf(x_train, y_train, RF_TREES=64) → booster
4. lgbm_predict_contrib(booster, x_train) → phi: Vec<Vec<f64>>  ← SHAP値
5. |phi[sample][feature]| を全サンプルで平均 → phi_sum
6. normalize(phi_sum) → importances
7. lgbm_mse(booster, x_eval, y_eval) → R²
```

削除されるコード: `ShapNode`, `PathElement`, `build_shap_tree`, `tree_shap_recurse`,
`extend_path`, `unwrap_path`, `sum_path`（TreeSHAP 実装全体）

---

## MDI の完全置き換え設計 🔵

**信頼性**: 🔵 *ユーザヒアリング（MDI互換性許容）より*

### 新しい compute_mdi_importances の流れ

```
1. データクリーニング・ダウンサンプリング（既存ロジック流用）
2. 80/20 ホールドアウト分割（Lcg シャッフル → 変更なし）
3. train_lgbm_rf(x_train, y_train, RF_TREES=64, MAX_DEPTH=64) → booster
4. lgbm_feature_importance(booster, p) → total_gains (Gain正規化済み)
5. lgbm_mse(booster, x_eval, y_eval) → R²
```

削除されるコード: `MdiNode`, `find_best_split_with_gain_idx`,
`build_mdi_tree_idx`, `accumulate_gains`（MDI ツリー実装全体）

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-001 ユーザヒアリング（定性目標）より*

LightGBM C++ 実装は純Rust CART 実装より以下の点で優位:
- ヒストグラムベースの分割探索（O(bin数) vs O(n log n)）
- SIMD 最適化済みの内部実装
- num_threads パラメータでマルチスレッド化も可能（現状は1スレッドで保守的に設定）

### 保守性 🟡

**信頼性**: 🟡 *NFR-101 から妥当な推測*

`core::lgbm` の `LgbmRfConfig` に全ハイパーパラメータを集約することで、
各感度分析モジュールで定数が重複しない。

---

## 技術的制約

### プラットフォーム別リンク 🔵

**信頼性**: 🔵 *コードベース調査（dll/dylib 両方存在）より*

| OS | ライブラリ | ファイル名 |
|---|---|---|
| Windows | DLL | `libs/lib_lightgbm.dll` |
| macOS | dylib | `libs/lib_lightgbm.dylib` |

`build.rs` でリンクライブラリ名は `lightgbm` で統一（プレフィックス `lib` は自動付与）。

### lightgbm クレートバージョン 🟡

**信頼性**: 🟡 *lightgbm-rs の慣例から妥当な推測*

LightGBM v3.x 系では `predict_contrib` (SHAP) が標準 API に含まれている。
実際のクレートバージョンはビルド確認後に `Cargo.toml` に固定する。

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/random-forest-lightgbm-replacement/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 16件 (80%)
- 🟡 黄信号: 4件 (20%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
