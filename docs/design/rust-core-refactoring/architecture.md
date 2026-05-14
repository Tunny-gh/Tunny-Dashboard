# rust-core-refactoring アーキテクチャ設計

**作成日**: 2026-05-14
**関連要件定義**: [requirements.md](../../spec/rust-core-refactoring/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書概要・ユーザーヒアリングより*

`rust_core/` クレート（`tunny-core`）を保守性・責務の分離・コードの重複排除・効率の 4 観点でリファクタリングする。
公開 API の破壊的変更を許容し、`egui-app` 側も合わせて修正する。

**クレート情報**:
- クレート名: `tunny-core`
- 言語: Rust 2021 edition
- 主要依存: `faer 0.24`, `serde/serde_json 1`, `rayon 1`, `criterion 0.5`
- 依存元: `egui-app/` (`tunny-desktop`) が `path` 依存で参照

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存コードベース構造・ユーザーヒアリングより*

- **パターン**: モジュール境界ベースの責務分離（Rust モジュールシステムによるレイヤード設計）
- **選択理由**: 既存のモジュール構成を維持しながら、各モジュール内の関数レベルの責務を分離する。破壊的変更許容のため、クリーンな API 再設計が可能。

---

## エピック別変更コンポーネント

### エピック A: コード重複排除

#### A-1. SensitivityMetric トレイト導入 🔵

**信頼性**: 🔵 *REQ-A01〜A03・ユーザーヒアリングより*

**変更内容**:
- **新規**: `rust_core/src/sensitivity/metric_trait.rs` — `SensitivityMetric` トレイト定義
- **リネーム**: `SensitivityMetric` enum → `SensitivityKind`（`sensitivity/types.rs` 内）
- **変更**: 既存 `TreeMetric` トレイト（`sensitivity/metrics.rs`）は内部実装として維持
- **変更**: `compute_sensitivity_single_obj` がトレイトオブジェクトのリストをイテレート

**新規 SensitivityMetric トレイト**:
```rust
// sensitivity/metric_trait.rs
pub trait SensitivityMetric: Send + Sync {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult>;
    fn name(&self) -> &'static str;
}
```

**実装者**:
| 構造体 | ファイル | 備考 |
|--------|----------|------|
| `SpearmanMetric` | `sensitivity/spearman.rs` | 既存関数をラップ |
| `RidgeMetric` | `sensitivity/ridge.rs` | 既存関数をラップ |
| `RfAnovaMetric` | `sensitivity/metrics.rs` | 既存 TreeMetric をラップ |
| `MdiMetric` | `sensitivity/metrics.rs` | 既存 TreeMetric をラップ |
| `ShapMetric` | `sensitivity/metrics.rs` | 既存 TreeMetric をラップ |
| `PermutationMetric` | `sensitivity/metrics.rs` | 既存 TreeMetric をラップ |

#### A-2. Pearson 相関の共通化 🔵

**信頼性**: 🔵 *REQ-A04・REQ-A05・コード分析 Finding 3 より*

- **移動先**: `core/math/stats.rs` に `pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64` を追加
- **削除**: `sensitivity/spearman.rs` のローカル `pearson_correlation` 定義を削除

#### A-3. k-means 初期化共通化 🔵

**信頼性**: 🔵 *REQ-A06・REQ-A07・コード分析 Finding 4 より*

- **新規**: `select_next_centroid(flat_data, n_cols, existing, n, sampling_fn)` を `clustering/kmeans.rs` に追加
- `init_kmeans_plusplus` と `init_deterministic` の両方が `select_next_centroid` を再利用

---

### エピック B: 責務分離

#### B-1. 感度分析ディスパッチ簡略化 🔵

**信頼性**: 🔵 *REQ-B01・REQ-B02・コード分析 Finding 9 より*

- **変更**: `sensitivity/analysis/full.rs` の `compute_sensitivity_single_obj` を 150 行以内に削減
- `match metric` の 6 アームを `Vec<Box<dyn SensitivityMetric>>` のイテレーションに置き換え

#### B-2. クラスタ統計の分割 🔵

**信頼性**: 🔵 *REQ-B03・コード分析 Finding 5 より*

`clustering/stats.rs` の `compute_cluster_stats_on_data`（130行）を 3 関数に分割:

| 関数名 | 役割 |
|--------|------|
| `compute_global_stats(flat_data, n, p) -> (Vec<f64>, Vec<f64>)` | 全体平均・標準偏差 |
| `compute_cluster_centroid_std(flat_data, labels, n, p, k) -> Vec<ClusterStat>` | クラスタ重心・標準偏差 |
| `compute_significant_features(cluster_stats, global_mean, global_std, n) -> Vec<ClusterStat>` | t 統計量による有意特徴選択 |

`compute_cluster_stats_on_data` はこれら 3 関数のオーケストレーションのみ（20 行以内）。

#### B-3. Ridge 回帰の分割 🔵

**信頼性**: 🔵 *REQ-B04・コード分析 Finding 7 より*

`sensitivity/ridge.rs` の `compute_ridge_from_standardized_columns`（55行）を 3 関数に分割:

| 関数名 | 役割 |
|--------|------|
| `compute_xtx_matrix(x_cols: &[f64], p: usize, n: usize) -> Vec<Vec<f64>>` | X'X 行列計算 |
| `compute_xty_vector(x_cols: &[f64], y: &[f64], p: usize, n: usize) -> Vec<f64>` | X'y ベクトル計算 |
| `compute_r_squared(y_actual: &[f64], y_predicted: &[f64]) -> f64` | R² 計算 |

#### B-4. GpModel の分離 🟡

**信頼性**: 🟡 *REQ-B05・コード分析 Finding 10 から妥当な推測*

`core/kriging/gaussian_process/model.rs` の `GpModel` を 2 構造体に分割:

```
GpModel (現在) → GpKernel + GpFittedModel
```

| 構造体 | フィールド | 役割 |
|--------|-----------|------|
| `GpKernel` | `log_ls: Vec<f64>`, `log_sf: f64`, `log_sn: f64` | カーネル超パラメータ |
| `GpFittedModel` | `kernel: GpKernel`, `alpha: Vec<f64>`, `x_train: Vec<Vec<f64>>`, `l: Vec<Vec<f64>>` | 訓練済みモデル |

---

### エピック C: 効率改善

#### C-1. SamplingContext による明示的状態管理 🔵

**信頼性**: 🔵 *REQ-C05〜C08・ユーザーヒアリング・コード分析 Finding 12 より*

**変更前**: `sampling/state.rs` — `thread_local! { static STATE: RefCell<SamplingState> }` でグローバル管理

**変更後**: `SamplingContext` 構造体（値型）を導入し、呼び出し側が明示的に保持

```rust
// rust_core/src/sampling/context.rs (新規)
pub struct SamplingContext {
    pub is_minimize: Vec<bool>,
    pub pareto_indices: Option<Vec<u32>>,
    pub all_ranks: Option<Vec<u32>>,
    pub cluster_labels: Option<Vec<i32>>,
}
```

**API 変更**:
- `init_sampling(is_minimize, pareto_indices, all_ranks) -> SamplingContext` (値を返す)
- `reset_sampling()` は削除（`None` への代入で代替）
- `downsample_smart(ctx: &SamplingContext, ...) -> Vec<u32>`
- `downsample_stratified_by_rank(ctx: &SamplingContext, ...) -> Vec<u32>`
- `downsample_by_cluster(ctx: &SamplingContext, ...) -> Vec<u32>`

**egui-app 変更**:
- `AppState` に `sampling_ctx: Option<SamplingContext>` フィールドを追加
- データロード完了時に `init_sampling` を呼び、結果を保存
- `set_cluster_labels` → `sampling_ctx.as_mut().map(|c| c.cluster_labels = Some(labels))`

#### C-2. TOPSIS 行列構築の効率化 🔵

**信頼性**: 🔵 *REQ-C03・コード分析 Finding 17 より*

`mcdm/topsis.rs` の `build_weighted_matrix` を単一アロケーションに変更:
- 変更前: `Vec::push` で行ごとに構築（O(n) 再アロケーションのリスク）
- 変更後: `vec![0.0_f64; n_valid * n_objectives]` の事前確保 + インデックス代入

#### C-3. k-means の不要クローン削減 🟡

**信頼性**: 🟡 *REQ-C01・C02・コード分析 Finding 13 から妥当な推測*

`clustering/kmeans.rs` の `to_vec()` → `Vec::with_capacity` + スライスコピーに変更

---

## ディレクトリ構造（変更対象のみ） 🔵

**信頼性**: 🔵 *既存プロジェクト構造・コード分析より*

```
rust_core/src/
├── sensitivity/
│   ├── types.rs               ← SensitivityMetric → SensitivityKind にリネーム
│   ├── metric_trait.rs        ← 新規: SensitivityMetric トレイト定義
│   ├── metrics.rs             ← 既存 TreeMetric + 新規 SensitivityMetric impl (4指標)
│   ├── spearman.rs            ← SpearmanMetric impl + pearson_correlation 削除
│   ├── ridge.rs               ← RidgeMetric impl + compute_xtx, compute_xty, compute_r_squared 追加
│   ├── analysis/
│   │   ├── full.rs            ← compute_sensitivity_single_obj 簡略化 (150行以内)
│   │   └── common.rs          ← 既存共通処理 (変更なし)
│   └── mod.rs                 ← pub use metric_trait::SensitivityMetric 追加
├── clustering/
│   ├── kmeans.rs              ← select_next_centroid 追加, クローン削減
│   └── stats.rs               ← compute_cluster_stats_on_data を 3 関数に分割
├── sampling/
│   ├── state.rs               ← thread_local 廃止 (ファイル削除または空にする)
│   ├── context.rs             ← 新規: SamplingContext 定義
│   └── mod.rs                 ← pub use context::SamplingContext 追加
├── mcdm/
│   └── topsis.rs              ← build_weighted_matrix 単一アロケーション化
└── core/
    ├── math/
    │   └── stats.rs           ← pearson_correlation 追加
    └── kriging/
        └── gaussian_process/
            └── model.rs       ← GpModel → GpKernel + GpFittedModel に分割

egui-app/src/
└── state/
    └── app_state.rs           ← sampling_ctx: Option<SamplingContext> フィールド追加
```

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-001・ベンチマークファイルの存在より*

- リファクタリング後も `cargo bench -p tunny-core` の全ベンチマークが既存比 +10% 以内
- `build_weighted_matrix` の単一アロケーション化でメモリ効率改善（NFR-002）
- ベンチマーク対象: `sampling_bench`, `sensitivity_bench`, `sobol_bench`, `rf_bench`, `permutation_bench`

### 正確性 🔵

**信頼性**: 🔵 *NFR-101・テストスイートの存在より*

- `cargo test -p tunny-core` 全テスト（数値計算結果含む）がパス
- `cargo test -p tunny-desktop` 全テストがパス（egui-app 修正後）
- 数値計算結果（感度指標・MCDM・クラスタリング・GP 予測）が浮動小数点許容誤差 `1e-10` 以内で一致（NFR-102）

### 保守性 🟡

**信頼性**: 🟡 *NFR-201・コード分析から妥当な推測*

- リファクタリング後の各関数は 50 行以内
- 新規追加する `SensitivityMetric` トレイト・`SamplingContext` 構造体は `///` ドキュメントコメントを持つ

---

## 技術的制約

### API 互換性 🔵

**信頼性**: 🔵 *note.md・ユーザーヒアリングより*

- 公開 API の破壊的変更は許容（egui-app 側も合わせて修正）
- WASM ビルド不要。ネイティブ API を自由に使用してよい

### 既存機能の維持 🔵

**信頼性**: 🔵 *要件定義・既存テストより*

- `compute_sensitivity_all(df)` は変更なし（後方互換）
- `SensitivityResult` 型は維持（フィールド追加は可）
- 既存の `ClusterStat` 型は維持

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/rust-core-refactoring/requirements.md)
- **ユーザストーリー**: [user-stories.md](../../spec/rust-core-refactoring/user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](../../spec/rust-core-refactoring/acceptance-criteria.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (82%)
- 🟡 黄信号: 3件 (18%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
