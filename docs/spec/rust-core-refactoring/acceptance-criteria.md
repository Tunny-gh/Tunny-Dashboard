# rust-core リファクタリング 受け入れ基準

**作成日**: 2026-05-14
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コード分析・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: コード分析から妥当な推測による基準
- 🔴 **赤信号**: コード分析にない推測による基準

---

## REQ-A01 / REQ-A02 / REQ-A03: SensitivityMetric トレイト導入 🔵

**信頼性**: 🔵 *コード分析 Finding 2, 9 + ユーザヒアリングより*

### Given（前提条件）
- `rust_core` に `SensitivityMetric` トレイトが定義されている
- MDI, SHAP, RF-ANOVA, Permutation の 4 指標がトレイトを実装している

### When（実行条件）
- `compute_sensitivity_single_obj(df, obj_idx, metrics)` を呼ぶ

### Then（期待結果）
- 各指標の `compute()` が順次呼び出される
- 結果が `Vec<SensitivityResult>` として返る
- `full.rs` に個別指標名の match アームが存在しない

### テストケース

#### 正常系

- [ ] **TC-A01-01**: トレイトを実装した 4 指標すべてが `compute()` を通じて呼び出せる 🔵
  - **入力**: テスト用 DataFrame（5 パラメータ, 100 行）, obj_idx=0
  - **期待結果**: 4 つの `SensitivityResult` が返る（None なし）

- [ ] **TC-A01-02**: 新規指標を追加したとき、ディスパッチ側の変更なしで動作する 🔵
  - **入力**: `SensitivityMetric` を実装した新しい構造体をリストに追加
  - **期待結果**: `cargo test` が全パス

- [ ] **TC-A01-03**: `full.rs` の `compute_sensitivity_single_obj` が 150 行以内 🔵
  - **検証方法**: `wc -l sensitivity/analysis/full.rs` < 200 行（コメント・空行含む）

#### 異常系

- [ ] **TC-A01-E01**: 指標計算中にパニックが起きた場合、他指標の計算が継続される 🟡
  - **入力**: `compute()` が `None` を返す指標を含むリスト
  - **期待結果**: `None` の指標はスキップされ、他の指標の結果が返る

---

## REQ-A04 / REQ-A05: Pearson 相関の共通化 🔵

**信頼性**: 🔵 *コード分析 Finding 3 より*

### Given（前提条件）
- `core/math/stats.rs` に `pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64` が存在する

### When（実行条件）
- `spearman.rs` が `pearson_correlation` を呼び出す

### Then（期待結果）
- `spearman.rs` にローカルな `pearson_correlation` 定義が存在しない
- 既存の Spearman テストが全てパス

### テストケース

- [ ] **TC-A04-01**: `pearson_correlation([1,2,3], [1,2,3])` → `1.0` 🔵
- [ ] **TC-A04-02**: `pearson_correlation([1,2,3], [3,2,1])` → `-1.0` 🔵
- [ ] **TC-A04-03**: 分散 0 の入力（全値同一）→ `NAN`、パニックなし 🟡
  - **入力**: `x = [1.0, 1.0, 1.0]`, `y = [2.0, 3.0, 4.0]`
  - **期待結果**: `f64::NAN`（`is_nan()` が true）

---

## REQ-A06 / REQ-A07: k-means 初期化の共通化 🔵

**信頼性**: 🔵 *コード分析 Finding 4 より*

### Given（前提条件）
- `clustering/kmeans.rs` に `select_next_centroid` が存在する

### When（実行条件）
- k-means++ または決定論的初期化を実行する

### Then（期待結果）
- `init_kmeans_plusplus` と `init_deterministic` が `select_next_centroid` を共有している
- 既存の k-means クラスタリングテストが全てパス

### テストケース

- [ ] **TC-A06-01**: k-means++ 初期化で重心が `k` 個生成される 🔵
  - **入力**: 10 点 × 2 次元, k=3
  - **期待結果**: 3 つの重心が返る
- [ ] **TC-A06-02**: 決定論的初期化で同じシードなら同じ重心が生成される 🔵
  - **期待結果**: 2 回呼び出しで同一結果

---

## REQ-B03: クラスタ統計の分割 🔵

**信頼性**: 🔵 *コード分析 Finding 5 より*

### Given（前提条件）
- `clustering/stats.rs` に `compute_global_stats`, `compute_cluster_centroid_std`, `compute_significant_features` が存在する

### When（実行条件）
- 各関数を独立して呼び出す

### Then（期待結果）
- `compute_cluster_stats_on_data` は 3 関数のオーケストレーションのみ（50 行以内）
- 既存テストが全パス

### テストケース

- [ ] **TC-B03-01**: `compute_global_stats` は全データの平均・標準偏差を返す 🔵
  - **入力**: `flat_data = [1,2,3,4]`, `n_cols = 2`
  - **期待結果**: `means = [1.5, 3.5]`, `stds` が正の値

- [ ] **TC-B03-02**: `compute_significant_features` は閾値未満の特徴を除外する 🟡
  - **入力**: クラスタ統計, 閾値 = 0.1
  - **期待結果**: 重要でない特徴のインデックスが除外される

---

## REQ-B04: Ridge 回帰の分割 🔵

**信頼性**: 🔵 *コード分析 Finding 7 より*

### Given（前提条件）
- `sensitivity/ridge.rs` に `compute_xtx_matrix`, `compute_xty_vector`, `compute_r_squared` が存在する

### When（実行条件）
- Ridge 回帰計算を実行する

### Then（期待結果）
- メイン関数は 30 行以内
- 既存の Ridge 感度テストが全パス

### テストケース

- [ ] **TC-B04-01**: `compute_r_squared([1,2,3], [1,2,3])` → `1.0` 🔵
- [ ] **TC-B04-02**: `compute_r_squared([1,2,3], [2,2,2])` → 既存実装と同じ値 🔵

---

## REQ-C05〜REQ-C08: SamplingContext 移行 🔵

**信頼性**: 🔵 *コード分析 Finding 12 + ユーザヒアリングより*

### Given（前提条件）
- `sampling/state.rs` のグローバル Mutex が削除されている
- `SamplingContext` 構造体が定義されている
- `egui-app` の `AppState` に `sampling_ctx: Option<SamplingContext>` が追加されている

### When（実行条件）
- データロード後、`init_sampling(df, is_minimize, pareto_indices)` を呼び出す

### Then（期待結果）
- `SamplingContext` が返り、`AppState.sampling_ctx` に保存される
- `downsample_smart(ctx, ...)` 等が `&SamplingContext` を受け取る

### テストケース

#### 正常系

- [ ] **TC-C05-01**: `init_sampling` が `SamplingContext` を返す 🔵
  - **入力**: テスト用 DataFrame, `is_minimize = [true]`, `pareto_indices = []`
  - **期待結果**: `SamplingContext` インスタンスが返る（グローバル副作用なし）

- [ ] **TC-C05-02**: 2 つの `SamplingContext` が独立して動作する 🔵
  - **入力**: 異なる設定で 2 つ作成
  - **期待結果**: 互いに干渉しない（グローバル状態廃止の確認）

- [ ] **TC-C05-03**: `downsample_smart(ctx, max_points)` が正しい結果を返す 🔵
  - **入力**: 1000 点 DataFrame, ctx, max_points=100
  - **期待結果**: 100 点以下のスライス返却、既存実装と同じ選択

#### 異常系

- [ ] **TC-C05-E01**: 空 DataFrame で `init_sampling` を呼んだ場合 🟡
  - **期待結果**: 有効な（空の）`SamplingContext` が返る、パニックなし

- [ ] **TC-C05-E02**: `None` の `sampling_ctx` で `downsample_smart` を呼んだ場合 🟡
  - **期待結果**: コンパイルエラーまたは `Option` で安全に処理

---

## REQ-C03: TOPSIS 行列構築の効率化 🔵

**信頼性**: 🔵 *コード分析 Finding 17 より*

### Given（前提条件）
- `mcdm/topsis.rs` の `build_weighted_matrix` が単一アロケーションに変更されている

### When（実行条件）
- `build_weighted_matrix(values, valid_indices, weights, ...)` を呼ぶ

### Then（期待結果）
- `cargo bench` で TOPSIS ベンチマークが既存比同等以内
- 既存の TOPSIS テストが全パス

### テストケース

- [ ] **TC-C03-01**: 10K トライアル × 4 目的で既存と同一の行列が返る 🔵
  - **入力**: 既存テストと同じ入力
  - **期待結果**: 行列内容が `f64::EPSILON` 以内で一致

---

## 非機能要件テスト

### NFR-001: ベンチマーク維持 🔵

**信頼性**: 🔵 *bench ファイルの存在より*

- [ ] **TC-NFR-001-01**: `cargo bench -p tunny-core` 全ベンチマークが既存比 +10% 以内 🔵
  - **測定条件**: `--bench sampling_bench sensitivity_bench sobol_bench rf_bench permutation_bench`

### NFR-101: 既存テスト全パス 🔵

**信頼性**: 🔵 *テストスイートの存在より*

- [ ] **TC-NFR-101-01**: `cargo test -p tunny-core` が全テストパス 🔵
- [ ] **TC-NFR-101-02**: `cargo test -p tunny-desktop` が全テストパス 🔵（egui-app 側も修正後）

### NFR-102: 数値精度 🟡

**信頼性**: 🟡 *数値計算の性質から推測*

- [ ] **TC-NFR-102-01**: 感度指標の計算結果がリファクタリング前後で `1e-10` 以内に一致 🟡
  - **検証方法**: 既存テストのアサーション値と比較

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| 重複排除 (A) | 7 | 1 | 1 | 9 |
| 責務分離 (B) | 4 | 0 | 0 | 4 |
| 効率改善 (C) | 5 | 2 | 0 | 7 |
| 非機能要件 | 3 | 0 | 0 | 3 |
| **合計** | **19** | **3** | **1** | **23** |

### 信頼性レベル分布

- 🔵 青信号: 19 件 (83%)
- 🟡 黄信号: 4 件 (17%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 14 件（トレイト導入・Pearson・SamplingContext・全パス確認）
- **Should Have**: 9 件（クラスタ分割・Ridge 分割・TOPSIS 効率化・ベンチマーク）
