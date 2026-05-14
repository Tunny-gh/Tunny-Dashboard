# rust-core-refactoring コンテキストノート

## プロジェクト概要

**対象クレート**: `rust_core/` (crate 名: `tunny-core`)
**役割**: Tunny Dashboard のコア解析ライブラリ。ジャーナルパース・DataFrame・感度分析・MCDM・クラスタリング・サロゲートモデル等を提供。
**依存元**: `egui-app/` (`tunny-desktop`) が `tunny-core` を `path` 依存で参照。

## 技術スタック

- **言語**: Rust 2021 edition
- **主要依存**:
  - `faer` 0.24 — 線形代数（行列演算）
  - `serde` / `serde_json` 1 — シリアライズ
  - `rayon` 1 — 並列処理
  - `criterion` 0.5 — ベンチマーク

## モジュール構成

```
rust_core/src/
├── lib.rs
├── lgbm_sys.rs          # LightGBM FFI（条件付きコンパイル）
├── convergence.rs
├── clustering/          # k-means, PCA, クラスタ統計
├── core/
│   ├── kriging/         # ガウス過程回帰（GP）, スパースFITC
│   ├── lgbm.rs          # LightGBM ラッパー
│   ├── math/            # 線形代数, 統計, グリッド
│   ├── optimization/    # L-BFGS, 直線探索
│   └── random_forest/   # ランダムフォレスト実装
├── data/
│   ├── dataframe.rs     # グローバル DataFrame 管理
│   └── filter.rs
├── io/
│   ├── export/          # CSV, HTML レポート
│   └── journal/         # Optuna ジャーナルパーサ
├── mcdm/                # TOPSIS, VIKOR, PROMETHEE, AHP, エントロピー重み
├── multi_objective/
│   └── pareto/          # パレートランク, ハイパーボリューム
├── pdp/                 # 偏依存プロット（Ridge/Kriging サロゲート）
├── sampling/            # ダウンサンプリング（スマート/層別/サムネイル）
└── sensitivity/         # Spearman, Ridge, MDI, SHAP, RF-ANOVA, Permutation, Sobol
```

## 現状の主要な問題点（コード分析より）

### コード重複
1. 木ベース感度指標（MDI, SHAP, RF-ANOVA, Permutation）が同一のボイラープレートを保持
2. k-means++ と決定論的初期化で 80% のコード重複
3. Pearson 相関が `spearman.rs` にローカル定義（`core/math/stats.rs` に移すべき）

### 責務分離
1. `compute_cluster_stats_on_data` が 89 行の単一関数（グローバル統計＋クラスタ統計＋有意性検定）
2. Ridge 回帰関数が 55 行で 5 つの異なるタスクを実行
3. `GpModel` がカーネル超パラメータと訓練データを混在
4. 感度分析ディスパッチ `compute_sensitivity_single_obj` が 150+ 行

### 効率
1. k-means 初期化での不要なクローン
2. Ridge 回帰での行列フォーマット変換が複数回発生
3. TOPSIS がフラット行列を行ごとに構築（大量 push）
4. Spearman ランクが呼び出しごとにランク配列を再アロケート

### グローバル状態
- `sampling/state.rs` がグローバル状態を管理（`init_sampling`, `reset_sampling`, `set_cluster_labels`）
- `data/dataframe/state.rs` もグローバル DataFrame 管理

## 開発ルール

- WASM ビルド不要。ネイティブ API を自由に使用してよい
- 公開 API の破壊的変更は許容（egui-app 側も合わせて修正する）
- `SensitivityMetric` トレイト導入に合意済み
- `SamplingContext` 構造体への移行に合意済み

## 関連ファイル（特に重要）

- `rust_core/src/sensitivity/analysis/full.rs` — 7種指標ディスパッチ（リファクタリング対象）
- `rust_core/src/sensitivity/tree_common.rs` — 木ベース共通処理
- `rust_core/src/mcdm/mod.rs` — 共通バリデーション（`validate_inputs`, `filter_valid_indices`）
- `rust_core/src/clustering/kmeans.rs` — k-means 実装
- `rust_core/src/sampling/state.rs` — グローバル状態
- `rust_core/src/core/math/stats.rs` — 統計ユーティリティ
