# rust-core リファクタリング 要件定義書

## 概要

`rust_core/` クレート（`tunny-core`）を保守性・責務の分離・コードの重複排除・効率の 4 観点でリファクタリングする。
公開 API の破壊的変更を許容し、`egui-app` 側も合わせて修正する。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

---

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コード分析・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: コード分析から妥当な推測による要件
- 🔴 **赤信号**: コード分析にない推測による要件

---

### エピック A: コード重複排除

#### A-1: 木ベース感度指標の共通処理抽出

- REQ-A01: システムは `SensitivityMetric` トレイトを定義し、MDI・SHAP・RF-ANOVA・Permutation の 4 指標が共通インターフェース `compute()` を実装しなければならない 🔵 *コード分析 Finding 2 + ユーザヒアリングより*
- REQ-A02: `tree_common.rs` は `prepare_training_data()` → `train_model()` のボイラープレートを 1 箇所に集約し、各指標実装からこれを再利用しなければならない 🔵 *コード分析 Finding 2 より*
- REQ-A03: 感度分析ディスパッチ `compute_sensitivity_single_obj` は `SensitivityMetric` 実装者のリストをイテレートする形に変更し、7 つの match アームを削除しなければならない 🔵 *コード分析 Finding 9 + ユーザヒアリングより*

#### A-2: Pearson 相関の共通化

- REQ-A04: `core/math/stats.rs` は `pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64` を公開しなければならない 🔵 *コード分析 Finding 3 より*
- REQ-A05: `sensitivity/spearman.rs` 内のローカル `pearson_correlation` を削除し、REQ-A04 の共通実装を使用しなければならない 🔵 *コード分析 Finding 3 より*

#### A-3: k-means 初期化の共通化

- REQ-A06: `clustering/kmeans.rs` は `init_kmeans_plusplus()` と `init_deterministic()` の共通重心選択ロジックを `select_next_centroid(flat_data, existing_centroids, sampling_fn)` として抽出しなければならない 🔵 *コード分析 Finding 4 より*
- REQ-A07: `select_next_centroid` は `sampling_fn: impl Fn(&[f64]) -> usize` パラメータにより確率的/決定論的を切り替え可能でなければならない 🟡 *コード分析から妥当な推測*

---

### エピック B: 責務分離

#### B-1: 感度分析ディスパッチの分割

- REQ-B01: `sensitivity/analysis/full.rs` の `compute_sensitivity_single_obj` は 150 行以内に収め、各指標計算への委譲のみを行いそれ以外のビジネスロジックを含まないものとしなければならない 🔵 *コード分析 Finding 9 より*
- REQ-B02: `SensitivityResult` の構築ロジックは各指標モジュール内、またはファクトリ関数として分離しなければならない 🟡 *コード分析 Finding 9 から妥当な推測*

#### B-2: クラスタ統計関数の分割

- REQ-B03: `clustering/stats.rs` の `compute_cluster_stats_on_data` を以下の 3 関数に分割しなければならない 🔵 *コード分析 Finding 5 より*
  - `compute_global_stats(flat_data, n_cols) -> (Vec<f64>, Vec<f64>)` — 全体平均・標準偏差
  - `compute_cluster_centroid_std(flat_data, labels, n_cols) -> Vec<ClusterStat>` — クラスタ統計
  - `compute_significant_features(cluster_stats, global_stats, threshold) -> Vec<usize>` — 有意特徴選択

#### B-3: Ridge 回帰関数の分割

- REQ-B04: `sensitivity/ridge.rs` の `compute_ridge_from_standardized_columns` を以下に分割しなければならない 🔵 *コード分析 Finding 7 より*
  - `compute_xtx_matrix(x_cols: &[&[f64]]) -> Vec<Vec<f64>>` — X'X 計算
  - `compute_xty_vector(x_cols: &[&[f64]], y: &[f64]) -> Vec<f64>` — X'y 計算
  - `compute_r_squared(y_actual: &[f64], y_predicted: &[f64]) -> f64` — R² 計算

#### B-4: GpModel の分離

- REQ-B05: `core/kriging/gaussian_process/model.rs` の `GpModel` を以下に分割しなければならない 🟡 *コード分析 Finding 10 から妥当な推測*
  - `GpKernel` — 超パラメータ（`log_ls`, `log_sf`, `log_sn`）
  - `GpFittedModel` — 訓練済みモデル（`x_train`, `alpha`, `l`, kernel）

---

### エピック C: 効率改善

#### C-1: k-means の不要クローン削減

- REQ-C01: `clustering/kmeans.rs` の重心初期化時に `to_vec()` による再アロケートを `Vec::with_capacity` + スライスコピーに置き換えなければならない 🔵 *コード分析 Finding 13 より*
- REQ-C02: k-means フォールバック時の `centroids[c].clone()` を参照ベースの代入または `swap` に変更しなければならない 🟡 *コード分析 Finding 13 から妥当な推測*

#### C-2: TOPSIS 行列構築の効率化

- REQ-C03: `mcdm/topsis.rs` の `build_weighted_matrix` は `vec![0.0; n_valid * n_objectives]` の単一アロケーション後にインデックス代入する実装に変更しなければならない 🔵 *コード分析 Finding 17 より*

#### C-3: Ridge 行列フォーマット変換の削減

- REQ-C04: `sensitivity/ridge.rs` の行列計算において column-major → `Vec<Vec<f64>>` 変換を排除し、faer のデータ構造を一貫して使用するか単一レイアウトに統一しなければならない 🔵 *コード分析 Finding 15 より*

#### C-4: サンプリングのグローバル状態廃止

- REQ-C05: `sampling/state.rs` のグローバル状態を廃止し、`SamplingContext` 構造体を定義しなければならない 🔵 *コード分析 Finding 12 + ユーザヒアリングより*
- REQ-C06: `init_sampling()` は `SamplingContext` を返し、呼び出し側が明示的に保持しなければならない 🔵 *ユーザヒアリングより*
- REQ-C07: `downsample_smart`, `downsample_stratified_by_rank`, `downsample_by_cluster` は `&SamplingContext` を引数として受け取るよう変更しなければならない 🔵 *ユーザヒアリングより*
- REQ-C08: `egui-app` 側の `SamplingContext` 利用箇所（AppState または WidgetStates）を更新し、`SamplingContext` をフィールドとして保持しなければならない 🟡 *コード分析から妥当な推測*

---

### 非機能要件

#### パフォーマンス

- NFR-001: 既存のベンチマーク（`sampling_bench`, `sensitivity_bench`, `sobol_bench`, `rf_bench`, `permutation_bench`）は全て refactoring 後も同等以上のスコアを維持しなければならない 🔵 *bench ファイルの存在より*
- NFR-002: TOPSIS の `build_weighted_matrix` はリファクタリング後、同入力に対して既存実装と同等以内のメモリを使用しなければならない 🟡 *コード分析 Finding 17 から推測*

#### 正確性

- NFR-101: 全ての既存テスト（`cargo test -p tunny-core`）は refactoring 後も全てパスしなければならない 🔵 *テストスイートの存在より*
- NFR-102: 数値計算結果（感度指標・MCDM スコア・クラスタリング・GP 予測）は浮動小数点許容誤差 `1e-10` 以内で一致しなければならない 🟡 *数値計算の性質から妥当な推測*

#### 保守性

- NFR-201: リファクタリング後の各関数は 50 行以内とし、単一責務を持たなければならない 🟡 *コード分析 Finding 5, 7, 9 から推測*
- NFR-202: 新規追加するトレイト・構造体はドキュメントコメント（`///`）を持たなければならない 🔴 *推測による品質要件*

---

## エッジケース

### エラー処理

- EDGE-001: `SensitivityMetric::compute()` は計算に失敗した場合、`None` または `Err` を返し、パニックしてはならない 🟡 *既存実装パターンから推測*
- EDGE-002: `SamplingContext` が空の DataFrame で初期化された場合、ダウンサンプリング関数は空のスライスを返さなければならない 🟡 *既存実装から推測*
- EDGE-003: `select_next_centroid` で既存重心が 0 個の場合、最初の重心をランダムに選択しなければならない 🔵 *k-means アルゴリズムの性質より*

### 境界値

- EDGE-101: `pearson_correlation` において入力が全て同一値の場合（分散 0）、`f64::NAN` を返し、パニックしないようにしなければならない 🟡 *数値計算の性質から推測*
- EDGE-102: `GpFittedModel` において `x_train` が空の場合、予測関数はエラーを返さなければならない 🟡 *既存実装から推測*
