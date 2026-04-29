# RandomForest → LightGBM 置き換え 設計ヒアリング記録

**作成日**: 2026-04-27
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

コードベース調査で判明した設計上の選択肢（DLL 配置・SHAP/MDI の置き換え深度）について
ユーザーに確認し、設計方針を確定した。

---

## 質問と回答

### Q1: 設計規模

**カテゴリ**: 設計規模
**背景**: フル設計か軽量設計かによって出力ファイル数が変わる。

**回答**: フル設計（推奨）

**信頼性への影響**: architecture.md / dataflow.md / interfaces.rs / design-interview.md 全件作成。

---

### Q2: DLL 配置場所とリンク設定方法

**カテゴリ**: 技術制約
**背景**: `lib_lightgbm.dll` と `lib_lightgbm.dylib` がリポジトリルートに配置されていた。
`build.rs` でのリンクパス設定には最終的な配置場所が必要。
また `lib_lightgbm.dylib` の存在から macOS 対応も必要であることが判明。

**回答**: `libs/` サブディレクトリに移動（推奨選択）

**信頼性への影響**:
- `build.rs` の `manifest_dir.parent().join("libs")` パスが 🔵 に確定
- Windows と macOS 両対応の設計方針が確定（REQ-401 + macOS 拡張）
- `.cargo/config.toml` での `LIGHTGBM_LIB_DIR` 設定パターンも文書化

---

### Q3: SHAP の置き換え深度（重要）

**カテゴリ**: アーキテクチャ
**背景**: コードベース調査で重要な事実が判明:
- SHAP の TreeSHAP 計算は `ShapNode` 独自木構造を使って実装されている
- `RandomForest`（`train_rf_on_columns`）を使っているのは **R² 計算のみ** である
- 完全置き換えには `ShapNode` + `build_shap_tree` + `tree_shap_recurse` 全廃が必要
- LightGBM の `predict_contrib` で native TreeSHAP を使う方法が存在する

ユーザーに「R² のみ置き換え」か「SHAP 完全置き換え（LightGBM native SHAP）」かを確認。

**回答**: SHAP も完全置き換えに挑戦（LightGBM native SHAP）

**信頼性への影響**:
- `ShapNode`, `build_shap_tree`, `tree_shap_recurse` の完全削除が 🔵 に確定
- `lgbm_predict_contrib` 呼び出しフローが 🔵 に確定
- 削除されるコード量が大幅増加（設計上の大変更）
- MDI と同様に「計算結果が変わる可能性あり・許容済み」として扱う

---

### Q4: MDI の置き換え方針（要件定義フェーズからの継続確認）

**カテゴリ**: アーキテクチャ
**背景**: MDI も `MdiNode` 独自実装を使って Gain を集計している。
LightGBM の `feature_importance(Gain)` への置き換えを再確認。

**回答**: 要件定義フェーズで確定済み（許容）

**信頼性への影響**: REQ-103 が 🔵 のまま維持。`MdiNode` 完全削除が確定。

---

## ヒアリング結果サマリー

### 確認できた事項

- DLL は `libs/` ディレクトリに配置、`build.rs` で `manifest_dir.parent().join("libs")` を指定
- macOS (`lib_lightgbm.dylib`) も対象プラットフォームに含まれる
- SHAP は `ShapNode` を含む TreeSHAP 実装ごと LightGBM native SHAP に置き換える
- MDI は `MdiNode` + ゲイン集計を LightGBM `feature_importance(Gain)` に置き換える

### 設計方針の決定事項

1. **`libs/` 配置**: `lib_lightgbm.dll` (Windows) + `lib_lightgbm.dylib` (macOS) を `libs/` に移動
2. **`build.rs`**: ワークスペースルート相対で `libs/` を `rustc-link-search` に追加
3. **`core::lgbm` 新設**: 共有ラッパーとして `to_lgbm_dataset`, `train_lgbm_rf`, `lgbm_predict_contrib`, `lgbm_mse`, `lgbm_feature_importance` を提供
4. **SHAP 完全置き換え**: `ShapNode` 等の TreeSHAP 独自実装を全廃し LightGBM `predict_contrib` へ
5. **MDI 完全置き換え**: `MdiNode` 等を全廃し LightGBM `feature_importance(Gain)` へ
6. **`Lcg` 保持**: `random_forest/rng.rs` は残し `pub(crate) use rng::Lcg` でパスを維持

### 残課題

- `lightgbm` クレートのバージョン番号（ビルド確認後に `Cargo.toml` に固定）
- `predict_contrib` のバイアス列（最後の列）の処理実装詳細（既知仕様: スキップで OK）
- Windows `lib_lightgbm.dll` に対応するインポートライブラリ `.lib` の必要性確認（GNU toolchain なら不要の場合あり）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 8件
- 🟡 黄信号: 8件
- 🔴 赤信号: 4件

**ヒアリング後**:
- 🔵 青信号: 17件 (+9)
- 🟡 黄信号: 7件 (-1)
- 🔴 赤信号: 0件 (-4)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/random-forest-lightgbm-replacement/requirements.md)
