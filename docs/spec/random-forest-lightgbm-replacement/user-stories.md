# RandomForest → LightGBM 置き換え ユーザストーリー

**作成日**: 2026-04-27
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリング・コードベース調査を参考にした確実なストーリー
- 🟡 **黄信号**: ヒアリング・実装から妥当な推測によるストーリー
- 🔴 **赤信号**: ヒアリング・実装にない推測によるストーリー

---

## エピック1: LightGBM 依存関係の導入

### ストーリー 1.1: rust_core への lightgbm クレート追加 🔵

**信頼性**: 🔵 *ユーザヒアリング（実装策略: rust_core に直接追加）より*

**私は** `rust_core` の開発者 **として**
**`lightgbm` クレートを `rust_core/Cargo.toml` に追加したい**
**そうすることで** 既存の RandomForest を LightGBM に置き換えるための基盤を用意できる

**関連要件**: REQ-001, REQ-401

**詳細シナリオ**:
1. `rust_core/Cargo.toml` の `[dependencies]` に `lightgbm = "X.Y"` を追加する
2. `build.rs`（または `.cargo/config.toml`）に `lib_lightgbm.dll` へのリンクパスを設定する
3. `cargo build` でビルドが成功することを確認する

**前提条件**:
- `lib_lightgbm.dll` および対応するインポートライブラリが所定のフォルダに配置済み
- LightGBM ライブラリのバージョンが `lightgbm` クレートと互換性がある

**優先度**: Must Have

---

### ストーリー 1.2: WASM ビルド設定の削除 🔵

**信頼性**: 🔵 *ユーザヒアリング（WASM廃止）より*

**私は** メンテナー **として**
**`rust_core` 内に残存する WASM 条件コンパイルブロックを削除したい**
**そうすることで** `lightgbm`（WASM非対応）の追加によるビルドエラーを防ぎ、コードを整理できる

**関連要件**: REQ-201

**詳細シナリオ**:
1. `rust_core` 内で `#[cfg(target_arch = "wasm32")]` を検索する
2. 該当する条件コンパイルブロックをすべて削除する
3. `cargo build` でエラーがないことを確認する

**前提条件**:
- WASMターゲットが完全に廃止されていること（ユーザー確認済み）

**優先度**: Must Have

---

## エピック2: 2D PDP への LightGBM 適用

### ストーリー 2.1: compute_pdp_2d_rf の LightGBM 置き換え 🔵

**信頼性**: 🔵 *ユーザヒアリング（スコープ①）より*

**私は** 最適化結果を分析するエンジニア **として**
**2D PDP チャートで `"random_forest"` モデルを選択したとき LightGBM RF で計算してほしい**
**そうすることで** 現行より高速に 2D パラメータ間の相互作用を可視化できる

**関連要件**: REQ-101, REQ-001

**詳細シナリオ**:
1. `rust_core/src/core/random_forest/pdp.rs` の `compute_pdp_2d_rf` を LightGBM を使って再実装する
2. `pdp/api.rs` の `"random_forest"` 分岐から新しい関数を呼び出す
3. 2D グリッド（n_grid × n_grid）の予測値と R² が返ってくることを確認する

**前提条件**:
- LightGBM が正常にリンクされている
- 入力データ（x_matrix, y）が有効

**優先度**: Must Have

---

## エピック3: 感度分析への LightGBM 適用

### ストーリー 3.1: SHAP baseline モデルの LightGBM 置き換え 🔵

**信頼性**: 🔵 *ユーザヒアリング（スコープ②）より*

**私は** 感度分析を実行するユーザー **として**
**SHAP 分析の baseline RF を LightGBM RF に置き換えてほしい**
**そうすることで** SHAP スコア算出が高速化される

**関連要件**: REQ-102

**詳細シナリオ**:
1. `sensitivity/shap.rs` の `train_rf_on_columns` / `mse_on_dataset` 呼び出しを LightGBM に置き換える
2. R² スコアの計算ロジックを LightGBM モデルの予測値を使って更新する
3. SHAP スコア自体の計算ロジックは変更しない（baseline モデルの置き換えのみ）

**優先度**: Must Have

---

### ストーリー 3.2: MDI の LightGBM feature_importance への切り替え 🔵

**信頼性**: 🔵 *ユーザヒアリング（スコープ③・MDI互換性許容）より*

**私は** 感度分析を実行するユーザー **として**
**MDI 感度分析を LightGBM の feature_importance(gain) で計算してほしい**
**そうすることで** MDI の計算が高速化される（結果の変化は許容）**

**関連要件**: REQ-103

**詳細シナリオ**:
1. `sensitivity/mdi.rs` の `MdiNode` 独自実装および関連するツリー走査コードを削除する
2. LightGBM RF モデルをトレーニングし、`feature_importance(ImportanceType::Gain)` で重要度を取得する
3. 取得した重要度スコアを正規化し、既存の出力フォーマット（Vec\<f64>）に変換する

**前提条件**: MDI の計算結果が変わることをユーザーが許容している

**優先度**: Must Have

---

### ストーリー 3.3: RF-ANOVA の LightGBM 置き換え 🔵

**信頼性**: 🔵 *ユーザヒアリング（スコープ④）より*

**私は** 感度分析を実行するユーザー **として**
**RF-ANOVA の RF を LightGBM に置き換えてほしい**
**そうすることで** ANOVA ベースの感度分析が高速化される

**関連要件**: REQ-104

**詳細シナリオ**:
1. `sensitivity/rf_anova.rs` の `train_rf_on_columns` / `mse_on_dataset` / `extract_columns` 呼び出しを LightGBM に置き換える
2. MSE ベースの ANOVA スコア計算を LightGBM モデルの予測値で更新する
3. 既存の出力フォーマットを維持する

**優先度**: Must Have

---

## エピック4: Lcg の保持とクリーンアップ

### ストーリー 4.1: Lcg (LCG乱数生成器) の保持 🔵

**信頼性**: 🔵 *コードベース調査（kriging/sparse_fitc.rs, kriging/gaussian_process/training.rs）より*

**私は** メンテナー **として**
**RandomForest 実装を削除した後も `Lcg` 構造体にアクセスできるようにしたい**
**そうすることで** Kriging モジュールのビルドが壊れない

**関連要件**: REQ-003, REQ-402

**詳細シナリオ**:
1. RF 削除後に `crate::core::random_forest::Lcg` のパスが有効か確認する
2. 必要に応じて `Lcg` を別モジュール（`core::rng` 等）に移動し、Kriging からのパスを更新する
3. `cargo build` でコンパイルエラーがないことを確認する

**優先度**: Must Have

---

## ストーリーマップ

```
エピック1: LightGBM 依存関係の導入
├── ストーリー 1.1 (🔵 Must Have) — lightgbm クレート追加
└── ストーリー 1.2 (🔵 Must Have) — WASM 設定削除

エピック2: 2D PDP への LightGBM 適用
└── ストーリー 2.1 (🔵 Must Have) — compute_pdp_2d_rf 置き換え

エピック3: 感度分析への LightGBM 適用
├── ストーリー 3.1 (🔵 Must Have) — SHAP baseline モデル置き換え
├── ストーリー 3.2 (🔵 Must Have) — MDI → LightGBM feature_importance
└── ストーリー 3.3 (🔵 Must Have) — RF-ANOVA 置き換え

エピック4: Lcg の保持とクリーンアップ
└── ストーリー 4.1 (🔵 Must Have) — Lcg 保持
```

## 信頼性レベルサマリー

- 🔵 青信号: 7件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
