# sensitivity-refactoring アーキテクチャ設計

**作成日**: 2026-05-04
**関連要件定義**: [requirements.md](../../spec/sensitivity-refactoring/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・コード分析・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・コード分析・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・コード分析・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義REQ-001〜007・コード分析より*

`rust_core/src/sensitivity/` および `rust_core/src/pdp/` モジュールのリファクタリング。外部公開APIを変更せず、内部構造を以下の4軸で改善する:
1. 標準化処理を `core::math::stats` へ昇格（重複4箇所 → 1箇所）
2. Newtypeパターンで Tree系結果型の型安全性を向上
3. `TreeMetric` トレイト + 静的ディスパッチで Tree系メトリクスを抽象化
4. 定数を `sensitivity/constants.rs` へ集約

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存プロジェクト構造・ユーザヒアリングより*

- **パターン**: レイヤードアーキテクチャ（既存パターンを維持・整理）
- **選択理由**: 既存の `analysis/` → 個別メトリクス → `core/` の依存方向を崩さず、重複部分のみを `core::math::stats` に昇格する最小変更で完結できる

---

## コンポーネント構成（変更後） 🔵

**信頼性**: 🔵 *コード分析・ユーザヒアリングより*

```
rust_core/src/
├── core/
│   ├── math/
│   │   ├── mod.rs          -- stats モジュールを追加 (pub(crate) mod stats)
│   │   ├── grid.rs         -- 変更なし
│   │   ├── linear_algebra.rs -- 変更なし
│   │   └── stats.rs        -- 【新規】column_mean_std を定義
│   ├── lgbm.rs             -- 変更なし
│   └── random_forest/      -- 変更なし
├── sensitivity/
│   ├── mod.rs              -- 変更なし（pub use は維持）
│   ├── types.rs            -- Newtype 変更: RfAnovaResult 等
│   ├── constants.rs        -- 【新規】MAX_ROWS, シード定数を集約
│   ├── tree_common.rs      -- 変更なし（prepare_training_data 等は既に統一済み）
│   ├── mdi.rs              -- prepare_training_data を使う形にリファクタリング
│   ├── shap.rs             -- prepare_training_data を使う形にリファクタリング
│   ├── rf_anova.rs         -- constants.rs から定数インポートに変更
│   ├── permutation.rs      -- constants.rs から定数インポートに変更
│   ├── ridge.rs            -- column_mean_std を core::math::stats から使用
│   ├── spearman.rs         -- 変更なし
│   ├── sobol.rs            -- column_mean_std の重複削除
│   ├── data.rs             -- 変更なし
│   ├── metrics.rs          -- 【新規】TreeMetric trait 定義
│   │                          RfAnova / Mdi / Shap / Permutation struct
│   ├── analysis/
│   │   ├── mod.rs          -- 変更なし
│   │   ├── common.rs       -- column_mean_std を core::math::stats から使用
│   │   ├── full.rs         -- TreeMetric 静的ディスパッチで compute_tree_metric_all 呼び出し
│   │   └── selected.rs     -- 同上
│   └── tests.rs            -- 変更なし（全テストパスを確認）
└── pdp/
    ├── utils.rs            -- col_mean_std を core::math::stats に委譲
    └── (その他変更なし)
```

---

## 新規コンポーネント詳細

### `core/math/stats.rs` 🔵

**信頼性**: 🔵 *REQ-001・コード分析（4箇所の重複）より*

```rust
/// 列データの平均と標準偏差を返す。
/// - 空スライス: (0.0, 1.0)
/// - 標準偏差 < EPSILON: std を 1.0 に固定（ゼロ除算防止）
pub(crate) fn column_mean_std(vals: &[f64]) -> (f64, f64) { ... }
```

既存4実装からの差分:
| 実装箇所 | 空スライス挙動 | 今後 |
|---------|-------------|-----|
| `sensitivity/ridge.rs` (inline) | パニック可能性 | 削除 |
| `sensitivity/analysis/common.rs` (inline) | 空チェックなし | 削除 |
| `sensitivity/sobol.rs::column_mean_std` | パニック可能性 | 削除 |
| `pdp/utils.rs::col_mean_std` | `(0.0, 1.0)` を返す ✅ | 削除・委譲 |

**統一された仕様**: 空スライスは `(0.0, 1.0)` を返す（pdp の既存動作に統一）

---

### `sensitivity/constants.rs` 🔵

**信頼性**: 🔵 *REQ-005・ユーザヒアリング: constants.rs を新規作成*

```rust
// ツリーモデルの LightGBM 共通設定
pub(crate) const RF_TREES: usize = 64;
pub(crate) const RF_MAX_DEPTH: usize = 64;
pub(crate) const RF_MIN_SAMPLES_LEAF: usize = 2;

// 各メトリクスのダウンサンプリング上限
// MDI/SHAP は LightGBM 訓練コストが高いため 1000 に抑制
pub(crate) const MDI_MAX_ROWS: usize = 1_000;
pub(crate) const SHAP_MAX_ROWS: usize = 1_000;
// RF-ANOVA と PFI は gain/permutation の追加コストが低いため 2000 まで許容
pub(crate) const RF_ANOVA_MAX_ROWS: usize = 2_000;
pub(crate) const PFI_MAX_ROWS: usize = 2_000;

// 再現性のための固定シード
pub(crate) const RF_SEED: u64 = 42;
pub(crate) const PFI_SEED_BASE: u64 = 1000;
pub(crate) const N_REPEATS: usize = 5;  // PFI の反復回数
```

---

### `sensitivity/metrics.rs` 🔵

**信頼性**: 🔵 *REQ-004・ユーザヒアリング: Trait で抽象化 + 静的ディスパッチ*

```rust
/// ツリーベースの感度分析メトリクス共通トレイト。
/// 実装者は prepare_training_data の結果を受け取り、
/// (importances, r_squared) を返す。
pub(crate) trait TreeMetric {
    fn compute_importances(
        &self,
        x: &[Vec<f64>],
        y: &[f64],
    ) -> Option<(Vec<f64>, f64)>;

    /// ダウンサンプリング上限行数（各メトリクスが定数を返す）
    fn max_rows(&self) -> usize;

    /// Fisher-Yates シャッフルに使用するシード
    fn data_seed(&self) -> u64;
    fn split_seed(&self) -> u64;
}

pub(crate) struct RfAnovaMetric;
pub(crate) struct MdiMetric;
pub(crate) struct ShapMetric;
pub(crate) struct PermutationMetric;
```

**静的ディスパッチによる呼び出し**:

```rust
// analysis/full.rs の利用例
fn run_tree_metric_for_all_objectives<M: TreeMetric>(
    metric: &M,
    x_matrix: &[Vec<f64>],
    objective_values: &[Vec<f64>],
) -> TreeImportanceResult {
    let importances_by_obj: Vec<(Vec<f64>, f64)> = objective_values
        .iter()
        .map(|y| {
            let prepared = prepare_training_data(
                x_matrix, y, metric.max_rows(),
                metric.data_seed(), metric.split_seed()
            );
            match prepared {
                Some(d) => metric.compute_importances(&d.x_shuffled, &d.y_shuffled)
                              .unwrap_or_else(|| (vec![0.0; x_matrix[0].len()], 0.0)),
                None => (vec![0.0; x_matrix[0].len()], 0.0),
            }
        })
        .collect();
    // transpose_to_tree_result に渡す
    ...
}
```

---

### `sensitivity/types.rs` の変更（Newtypeパターン） 🔵

**信頼性**: 🔵 *REQ-003・ユーザヒアリング: Newtypeパターンに変更*

**変更前**:
```rust
pub type RfAnovaResult = TreeImportanceResult;
pub type MdiResult = TreeImportanceResult;
pub type ShapResult = TreeImportanceResult;
pub type PermutationResult = TreeImportanceResult;
```

**変更後**:
```rust
#[derive(Debug, Clone)]
pub struct RfAnovaResult(pub TreeImportanceResult);
#[derive(Debug, Clone)]
pub struct MdiResult(pub TreeImportanceResult);
#[derive(Debug, Clone)]
pub struct ShapResult(pub TreeImportanceResult);
#[derive(Debug, Clone)]
pub struct PermutationResult(pub TreeImportanceResult);
```

`SensitivityResult` の各フィールドは `Option<RfAnovaResult>` 等に自動的に対応。  
既存の `pub use types::{RfAnovaResult, MdiResult, ShapResult, PermutationResult}` は維持。  
呼び出し側は `.0.importances` でアクセス可能。

---

## 依存関係マップ 🔵

**信頼性**: 🔵 *コード分析・ユーザヒアリングより*

```
analysis/full.rs
  └─ metrics.rs::TreeMetric<M> (静的ディスパッチ)
       ├─ metrics.rs::RfAnovaMetric → rf_anova.rs
       ├─ metrics.rs::MdiMetric     → mdi.rs
       ├─ metrics.rs::ShapMetric    → shap.rs
       └─ metrics.rs::PermutationMetric → permutation.rs
            ↓
        tree_common.rs::prepare_training_data
            ↓
        core::lgbm (LightGBM 訓練・推論)

ridge.rs, analysis/common.rs, sobol.rs, pdp/utils.rs
  └─ core::math::stats::column_mean_std  ← 【新規・共有】
```

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-001〜002・既存テストより*

- `column_mean_std`: O(n) の1パス計算。既存実装と同等
- 静的ディスパッチ: ゼロコストの型パラメータ。`dyn TreeMetric` と比べて vtable オーバーヘッドなし
- `prepare_training_data` への統一: mdi.rs/shap.rs で不要なクローンを削減する可能性あり
- パフォーマンステスト（TC-801-P01〜P03）で 5% 劣化なしを回帰テストとして確認

### 保守性 🔵

**信頼性**: 🔵 *NFR-003〜005・ユーザヒアリングより*

- 重複排除による行数削減: 約4実装×10〜15行 + mdi/shap 前処理 = 約120〜150行削減見込み（10%以上）
- 定数一元管理: 根拠コメントを必須化（NFR-005）
- 新メトリクス追加: `TreeMetric` を実装 + `SensitivityMetric` enum に追加 + `run_tree_metric_for_all_objectives` に呼び出し追加のみ

### パブリックAPI 🔵

**信頼性**: 🔵 *REQ-007より*

変更してはならないシンボル一覧:

```rust
// sensitivity/mod.rs の pub use
compute_sensitivity, compute_sensitivity_all, compute_sensitivity_selected,
compute_sensitivity_single_obj, compute_sensitivity_without_mdi,
compute_mdi_importances, compute_permutation_importances,
compute_rf_anova_importances, compute_ridge, compute_shap_importances,
compute_sobol, compute_sobol_from_df, compute_spearman,
MdiResult, PermutationResult, RfAnovaResult, RidgeResult,
SensitivityMetric, SensitivityResult, ShapResult, SobolResult, TreeImportanceResult
```

---

## 技術的制約

### Newtypeの破壊的変更への対処 🔵

**信頼性**: 🔵 *EDGE-002・コード分析より*

Newtype導入により `.importances` / `.r_squared` への直接アクセスが `.0.importances` / `.0.r_squared` に変わる。変更が必要な箇所の特定方法:

```bash
cargo build 2>&1 | grep "error\[E"
```

影響箇所の特定後、機械的に修正できる（一括置換可）。

### 後方互換性 🔵

**信頼性**: 🔵 *REQ-007より*

- `pub use types::RfAnovaResult` 等は維持するため、外部クレート（WASMバインディング）への影響なし
- Newtype は `pub struct XxxResult(pub TreeImportanceResult)` の `pub` フィールドにより、`.0` アクセスが可能

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義（Rust）**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/sensitivity-refactoring/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 12件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
