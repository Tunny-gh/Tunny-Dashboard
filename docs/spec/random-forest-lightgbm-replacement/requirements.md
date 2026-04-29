# RandomForest → LightGBM 置き換え 要件定義書

## 概要

`rust_core` に純Rustで実装された Random Forest（CART + Bagging）を、
`lightgbm` クレート（LightGBM C++ FFI バインディング）に置き換える。
置き換え対象は 2D PDP サーフェス計算・SHAP 感度分析・MDI 感度分析・RF-ANOVA 感度分析の4箇所。
ターゲットは egui-app（ネイティブデスクトップ）のみ。WASMは廃止。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **準備タスク**: [🔧 prep.md](prep.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: ヒアリング・実装から妥当な推測による要件
- 🔴 **赤信号**: ヒアリング・実装にない推測による要件

### 通常要件

- REQ-001: システムは `lightgbm` クレートを `rust_core/Cargo.toml` に追加し、LightGBM RandomForest モード (`boosting_type = "rf"`) で回帰モデルをトレーニングしなければならない 🔵 *ユーザヒアリング（スコープ・実装策略）より*
- REQ-002: システムは `rust_core/src/core/random_forest/forest.rs`, `tree.rs`, `types.rs` の純Rust実装（`RandomForest`, `DecisionTree`, `TreeNode`）を削除しなければならない 🔵 *ユーザヒアリング（WASM廃止・完全置き換え）より*
- REQ-003: システムは `rust_core/src/core/random_forest/rng.rs` の `Lcg` を削除してはならない（Kriging モジュールが依存しているため） 🔵 *コードベース調査（sparse_fitc.rs, training.rs より)*

### 条件付き要件

- REQ-101: 2D PDP 計算で `model_type = "random_forest"` が指定された場合、システムは LightGBM RF モードでフィッティングしたモデルで 2D サーフェスグリッドを予測しなければならない 🔵 *ユーザヒアリング（スコープ①）より*
- REQ-102: SHAP 感度分析が実行される場合、システムは LightGBM RF モデルを baseline 回帰モデルとして使用し、R² スコアを計算しなければならない 🔵 *ユーザヒアリング（スコープ②）より*
- REQ-103: MDI 感度分析が実行される場合、システムは LightGBM の `feature_importance(ImportanceType::Gain)` を使って各パラメータの重要度スコアを計算しなければならない 🔵 *ユーザヒアリング（スコープ③・MDI互換性許容）より*
- REQ-104: RF-ANOVA 感度分析が実行される場合、システムは LightGBM RF モデルを使って MSE ベースの ANOVA スコアを計算しなければならない 🔵 *ユーザヒアリング（スコープ④）より*

### 状態要件

- REQ-201: WASMターゲットが廃止された状態において、システムは `#[cfg(target_arch = "wasm32")]` の条件コンパイルブロックが `rust_core` 内に残存する場合、これを削除しなければならない 🔵 *ユーザヒアリング（WASM廃止）より*

### オプション要件

- REQ-301: システムは LightGBM モデルのハイパーパラメータ（`num_iterations`, `max_depth`, `min_data_in_leaf`, `bagging_fraction`, `feature_fraction`）を現行の純Rust実装のデフォルト値に相当する値で設定してもよい 🟡 *note.md のハイパーパラメータ対応表から妥当な推測*

### 制約要件

- REQ-401: システムは CMake や LightGBM のソースビルドを行わず、ユーザーが指定フォルダに配置した `lib_lightgbm.dll`（および対応するインポートライブラリ）を動的リンクして使用しなければならない 🔵 *ユーザヒアリング（DLL配置方針）より*
- REQ-402: システムは `rust_core/src/core/random_forest/rng.rs` の `Lcg` を `Lcg` としてアクセス可能な状態に維持しなければならない（既存の Kriging コードとの互換性） 🔵 *コードベース調査より*
- REQ-403: システムは `rust_core` 内の公開・内部 API（`train_rf_on_columns`, `mse_on_dataset`, `compute_pdp_2d_rf` 等）のシグネチャを変更してもよい 🔵 *ユーザヒアリング（API互換性）より*

## 非機能要件

### パフォーマンス

- NFR-001: LightGBM への置き換えにより、純Rust実装よりも高速に RandomForest のトレーニングおよび推論が動作しなければならない（数値目標は未設定、体感での改善を確認） 🔵 *ユーザヒアリング（パフォーマンス目標）より*

### 保守性

- NFR-101: LightGBM 固有のモデル設定（ハイパーパラメータ群）は定数または設定構造体としてまとめ、各感度分析モジュールから共有可能な形にしなければならない 🟡 *実装から妥当な推測（DRY原則）*

### ビルド

- NFR-201: `lib_lightgbm.dll` のリンクパスは `build.rs` または `.cargo/config.toml` で設定し、パスをハードコードしてはならない 🟡 *lightgbm-rs の慣例から妥当な推測*

## Edgeケース

### エラー処理

- EDGE-001: LightGBM モデルのトレーニングが失敗した場合（データ不足、設定不正等）、システムは `None` を返すか適切なエラーを伝播しなければならない 🟡 *既存 RF 実装の`Option`パターンから妥当な推測*
- EDGE-002: `feature_importance` の結果が空（全特徴量のスコアが0）の場合、システムはパニックせずゼロ初期化されたスコアを返さなければならない 🟡 *MDI の実装パターンから妥当な推測*

### 境界値

- EDGE-101: サンプル数が `min_data_in_leaf` の2倍未満の場合、LightGBM は木を構築できない可能性があるため、システムはフォールバック処理（Ridge等）または `None` を返さなければならない 🟡 *現行 RF の `n < 2 * min_samples_leaf` チェックから妥当な推測*
