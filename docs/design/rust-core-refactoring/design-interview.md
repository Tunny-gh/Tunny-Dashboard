# rust-core-refactoring 設計ヒアリング記録

**作成日**: 2026-05-14
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存コード分析・要件定義書・設計文書を確認し、設計上の不明点・選択肢を明確化するためのヒアリングを実施しました。

---

## Q1: 設計作業規模

**カテゴリ**: 設計方針
**背景**: フル設計（全ファイル作成）か軽量設計（最小限）かで出力物が変わる

**回答**: フル設計（推奨）— 包括的なアーキテクチャ設計、詳細なデータフロー、完全な型定義を含む

**信頼性への影響**:
- architecture.md, dataflow.md, interfaces.rs, design-interview.md の全4ファイルを作成

---

## Q2: 既存実装の詳細分析

**カテゴリ**: コード分析
**背景**: 既存実装を調査せずに設計すると、重要な実装詳細を見落とす可能性がある

**回答**: 必要 — Explore エージェントを使用して既存コードを網羅的に調査

**発見事項**（コード分析より）:
- `SensitivityMetric` は既に **enum** として実装済み（types.rs 53行）
  - バリアント: Spearman, Ridge, RfAnova, Mdi, Shap, Permutation
- `TreeMetric` **トレイト** が既に存在（sensitivity/metrics.rs 86行）
  - 実装者: RfAnovaMetric, MdiMetric, ShapMetric, PermutationMetric
- `sampling/state.rs` は `thread_local! { static STATE: RefCell<SamplingState> }` を使用（Mutex ではない）
- `GpModel` は既に全フィールドを持つ単一構造体（12行のシンプルな定義）
- `clustering/stats.rs` の `compute_cluster_stats_on_data` は 130行

**信頼性への影響**:
- 既存 TreeMetric トレイトとの関係を設計に明記することで 🔵 青信号項目が増加
- sampling/state.rs の `thread_local! + RefCell` パターンを正確に把握

---

## Q3: SensitivityMetric の名前衝突解消

**カテゴリ**: アーキテクチャ
**背景**: 
- `sensitivity-all-models` 設計では `SensitivityMetric` が「Spearman/Ridge/RfAnova を選択する列挙型」
- 今回の要件 REQ-A01 では `SensitivityMetric` を「MDI/SHAP/RF-ANOVA/Permutation が実装するトレイト」として定義
- 既存の enum と新規トレイトで同名が衝突する

**選択肢**:
1. トレイトとして定義（推奨）— 既存 enum を `SensitivityKind` にリネーム
2. 既存 enum を維持・拡張 — sensitivity-all-models の設計を優先

**回答**: **トレイトとして定義（推奨）** — 既存 enum は `SensitivityKind` にリネーム

**決定内容**:
- `trait SensitivityMetric` を新規作成（`sensitivity/metric_trait.rs`）
- 既存 `enum SensitivityMetric` → `enum SensitivityKind` にリネーム
- `sensitivity-all-models` 設計の `SensitivityMetric` 参照箇所も `SensitivityKind` に更新

**信頼性への影響**:
- `sensitivity/types.rs` の SensitivityMetric リネームが 🔵 青信号（ユーザー確認済み）
- `sensitivity-all-models` との整合性が 🔵 青信号に向上

---

## Q4: SamplingContext のフィールド構成

**カテゴリ**: データモデル
**背景**: 
- 既存 `SamplingState` は `is_minimize`, `pareto_indices`, `all_ranks`, `cluster_labels` の 4 フィールドを持つ
- 要件定義書（ユーザーストーリー C-1）では `{ pareto_ranks, is_minimize, cluster_labels }` の 3 フィールド記載
- `all_ranks` を含めるかどうかが不明

**選択肢**:
1. 全フィールドを引き継ぐ（推奨）— 既存 SamplingState と同等
2. 最小限フィールドのみ — cluster_labels は別途設定

**回答**: **全フィールドを引き継ぐ（推奨）** — SamplingContext { is_minimize, pareto_indices, all_ranks, cluster_labels }

**決定内容**:
```rust
pub struct SamplingContext {
    pub is_minimize: Vec<bool>,
    pub pareto_indices: Option<Vec<u32>>,
    pub all_ranks: Option<Vec<u32>>,
    pub cluster_labels: Option<Vec<i32>>,
}
```

**信頼性への影響**:
- SamplingContext の全フィールドが 🔵 青信号（既存実装と完全一致、ユーザー確認済み）

---

## Q5: GpModel 分割の構造

**カテゴリ**: アーキテクチャ
**背景**: 
- REQ-B05 は GpModel を GpKernel + GpFittedModel に分割することを要求
- GpFittedModel が GpKernel を所有するか、独立させるかで API が変わる

**選択肢**:
1. GpFittedModel が GpKernel を内包（推奨）— `GpFittedModel { kernel: GpKernel, alpha, x_train, l }`
2. 別々の独立構造体 — optimize 関数が GpKernel を返し、fit 関数が GpFittedModel を返す

**回答**: **GpFittedModel が GpKernel を内包（推奨）**

**決定内容**:
```rust
pub struct GpKernel {
    pub log_ls: Vec<f64>,
    pub log_sf: f64,
    pub log_sn: f64,
}

pub struct GpFittedModel {
    pub kernel: GpKernel,
    pub alpha: Vec<f64>,
    pub x_train: Vec<Vec<f64>>,
    pub l: Vec<Vec<f64>>,
}
```

**信頼性への影響**:
- GpFittedModel 構造は 🔵 青信号（ユーザー確認済み）
- GpKernel は 🟡 黄信号（REQ-B05 から推測。既存コードは 12 行の単一構造体）

---

## ヒアリング結果サマリー

### 確認できた事項

1. フル設計で全4ファイルを作成する
2. 既存実装に `TreeMetric` トレイトが既に存在し、`SensitivityMetric` トレイトの設計に活用できる
3. `SensitivityMetric` は新トレイト名、既存 enum は `SensitivityKind` にリネームする
4. `SamplingContext` は既存 `SamplingState` の全フィールドを継承する
5. `GpFittedModel` が `GpKernel` を内包する設計とする

### 設計方針の決定事項

| 項目 | 決定 |
|------|------|
| 作業規模 | フル設計（全ファイル作成） |
| SensitivityMetric 名前衝突 | トレイト優先・enum を SensitivityKind にリネーム |
| SamplingContext フィールド | 既存 SamplingState 4フィールドを全て継承 |
| GpModel 分割構造 | GpFittedModel { kernel: GpKernel } の内包形式 |
| 型定義ファイル形式 | Rust 言語のため interfaces.rs（.ts ではない） |
| DB スキーマ | 不要（Rust ライブラリクレートのため生成しない） |
| API 仕様 | 不要（内部ライブラリ API のため生成しない） |

### 残課題

- `SensitivityMetric` トレイトを全6指標（Spearman 含む）が実装するか、4 指標（木ベースのみ）が実装するかの詳細 → 設計では全 6 指標が実装するものとして記載（REQ-A03「7 つの match アームを削除」から推測）
- `select_next_centroid` の `sampling_fn` 型引数の詳細 → `impl Fn(&[f64]) -> usize` として定義（REQ-A07 から）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 9件
- 🟡 黄信号: 5件
- 🔴 赤信号: 2件

**ヒアリング後**:
- 🔵 青信号: 14件 (+5)
- 🟡 黄信号: 3件 (-2)
- 🔴 赤信号: 0件 (-2)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/rust-core-refactoring/requirements.md)
