# rust-core リファクタリング ユーザストーリー

**作成日**: 2026-05-14
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コード分析・ユーザヒアリングを参考にした確実なストーリー
- 🟡 **黄信号**: コード分析から妥当な推測によるストーリー
- 🔴 **赤信号**: コード分析にない推測によるストーリー

---

## エピック A: コード重複排除

### ストーリー A-1: 木ベース感度指標の共通インターフェース 🔵

**信頼性**: 🔵 *コード分析 Finding 2 + ユーザヒアリングより*

**私は** rust_core の開発者として
**新しい感度指標（例: 新しい permutation 変種）を追加したい**
**そうすることで** `SensitivityMetric` トレイトを実装するだけで、ディスパッチ・結果収集・ベンチマークが自動的に対応される

**関連要件**: REQ-A01, REQ-A02, REQ-A03

**詳細シナリオ**:
1. `trait SensitivityMetric` を定義（`fn compute(df, obj_idx) -> Option<SensitivityResult>`）
2. `Mdi`, `Shap`, `RfAnova`, `Permutation` がそれぞれ実装
3. `compute_sensitivity_single_obj` は `Vec<Box<dyn SensitivityMetric>>` をループ
4. 新指標追加時はトレイト実装 + リストへの追加のみ

**前提条件**:
- `tree_common.rs` に共通ボイラープレートが集約済み
- 既存の `SensitivityResult` 型は維持

**優先度**: Must Have

---

### ストーリー A-2: Pearson 相関の共通化 🔵

**信頼性**: 🔵 *コード分析 Finding 3 より*

**私は** rust_core の開発者として
**`core/math/stats.rs` の公開 API から Pearson 相関を呼び出したい**
**そうすることで** `spearman.rs` と他モジュールで同じ計算を重複実装しなくて済む

**関連要件**: REQ-A04, REQ-A05

**詳細シナリオ**:
1. `core/math/stats.rs` に `pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64` を追加
2. `spearman.rs` のローカル定義を削除し、`use crate::core::math::stats::pearson_correlation;`

**優先度**: Must Have

---

### ストーリー A-3: k-means 初期化の共通化 🔵

**信頼性**: 🔵 *コード分析 Finding 4 より*

**私は** rust_core の開発者として
**k-means の初期化戦略を切り替えたい（k-means++ / 決定論的）**
**そうすることで** 重心選択ロジックを 1 箇所で管理でき、バグ修正が両戦略に即座に反映される

**関連要件**: REQ-A06, REQ-A07

**詳細シナリオ**:
1. `select_next_centroid(flat_data, n_cols, existing, sampling_fn)` を抽出
2. k-means++ はランダムな `sampling_fn`、決定論的は中央選択の `sampling_fn` を渡す

**優先度**: Must Have

---

## エピック B: 責務分離

### ストーリー B-1: 感度分析ディスパッチの簡略化 🔵

**信頼性**: 🔵 *コード分析 Finding 9 + ユーザヒアリングより*

**私は** rust_core の利用者（egui-app）として
**`compute_sensitivity` を呼ぶだけで全指標の結果を得たい**
**そうすることで** ディスパッチの詳細を知らなくてもよく、新指標追加時も再コンパイルだけで動作する

**関連要件**: REQ-B01, REQ-B02

**詳細シナリオ**:
1. `full.rs` の match アームを `SensitivityMetric` イテレーションに置き換え
2. 各指標モジュールが自身の `SensitivityResult` 構築を担当
3. `full.rs` は収集・エラー集約のみを行う（50 行以内）

**優先度**: Must Have

---

### ストーリー B-2: クラスタ統計の分割テスト 🔵

**信頼性**: 🔵 *コード分析 Finding 5 より*

**私は** rust_core の開発者として
**クラスタ統計の「有意特徴検出」を単体テストしたい**
**そうすることで** 89 行の複合関数をデバッグする代わりに、機能別に検証できる

**関連要件**: REQ-B03

**詳細シナリオ**:
1. `compute_global_stats` → 単独でテスト可能
2. `compute_cluster_centroid_std` → 単独でテスト可能
3. `compute_significant_features` → 単独でテスト可能
4. `compute_cluster_stats_on_data` は 3 関数のオーケストレーションのみ（20 行以内）

**優先度**: Should Have

---

### ストーリー B-3: Ridge 回帰の段階的デバッグ 🔵

**信頼性**: 🔵 *コード分析 Finding 7 より*

**私は** rust_core の開発者として
**Ridge 回帰の計算ステップを個別に確認したい**
**そうすることで** X'X 計算・X'y 計算・R² 計算それぞれをテスト/ログで追跡できる

**関連要件**: REQ-B04

**詳細シナリオ**:
1. `compute_xtx_matrix` を独立関数として抽出
2. `compute_xty_vector` を独立関数として抽出
3. `compute_r_squared` を独立関数として抽出
4. メイン関数はオーケストレーションのみ

**優先度**: Should Have

---

## エピック C: 効率改善

### ストーリー C-1: SamplingContext による明示的状態管理 🔵

**信頼性**: 🔵 *コード分析 Finding 12 + ユーザヒアリングより*

**私は** egui-app の開発者として
**サンプリング状態を AppState のフィールドとして保持したい**
**そうすることで** グローバル状態への依存を排除し、テスト時に複数の状態を同時に持てる

**関連要件**: REQ-C05, REQ-C06, REQ-C07, REQ-C08

**詳細シナリオ**:
1. `SamplingContext { pareto_ranks, is_minimize, cluster_labels }` を定義
2. `init_sampling(df, is_minimize, pareto_indices) -> SamplingContext` に変更
3. `downsample_smart(ctx: &SamplingContext, ...)` に変更
4. `egui-app` の `AppState` に `sampling_ctx: Option<SamplingContext>` フィールドを追加
5. データロード時に `init_sampling` を呼び、結果を `AppState` に保存

**前提条件**:
- WASM ビルド不要（破壊的変更許容）

**制約事項**:
- グローバル Mutex を削除すること
- `reset_sampling()` は `sampling_ctx = None` 等の明示的リセットに置き換える

**優先度**: Must Have

---

### ストーリー C-2: TOPSIS のメモリ効率化 🔵

**信頼性**: 🔵 *コード分析 Finding 17 より*

**私は** egui-app の MCDM 機能ユーザーとして
**大量トライアル（10K+）で TOPSIS を実行した際にメモリスパイクを避けたい**
**そうすることで** 大規模最適化結果の分析時もアプリが快適に動作する

**関連要件**: REQ-C03

**詳細シナリオ**:
1. `build_weighted_matrix` を `vec![0.0; n_valid * n_objectives]` → インデックス代入に変更
2. ベンチマークで改善を確認（NFR-001）

**優先度**: Should Have

---

### ストーリー C-3: k-means クローン削減 🟡

**信頼性**: 🟡 *コード分析 Finding 13 から妥当な推測*

**私は** rust_core のパフォーマンスに関心のある開発者として
**k-means クラスタリングの初期化で不要なヒープアロケーションを削減したい**
**そうすることで** 多数のクラスタ数を試すハイパーパラメータ探索が高速になる

**関連要件**: REQ-C01, REQ-C02

**詳細シナリオ**:
1. `centroids.push(get_point(n/2).to_vec())` → `Vec::with_capacity` + スライスコピー
2. フォールバック時 `centroids[c].clone()` → 参照またはメモリ効率的な代替

**優先度**: Should Have

---

## ストーリーマップ

```
エピック A: コード重複排除
├── A-1 木ベース感度指標の共通インターフェース  (🔵 Must Have)
├── A-2 Pearson 相関の共通化                   (🔵 Must Have)
└── A-3 k-means 初期化の共通化                 (🔵 Must Have)

エピック B: 責務分離
├── B-1 感度分析ディスパッチの簡略化           (🔵 Must Have)
├── B-2 クラスタ統計の分割テスト               (🔵 Should Have)
└── B-3 Ridge 回帰の段階的デバッグ             (🔵 Should Have)

エピック C: 効率改善
├── C-1 SamplingContext による明示的状態管理   (🔵 Must Have)
├── C-2 TOPSIS のメモリ効率化                  (🔵 Should Have)
└── C-3 k-means クローン削減                   (🟡 Should Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 8 件 (89%)
- 🟡 黄信号: 1 件 (11%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: 高品質
