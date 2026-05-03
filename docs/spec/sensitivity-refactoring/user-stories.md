# sensitivity-refactoring ユーザストーリー

**作成日**: 2026-05-04
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コード分析・ユーザヒアリングを参考にした確実なストーリー
- 🟡 **黄信号**: コード分析・ユーザヒアリングから妥当な推測によるストーリー
- 🔴 **赤信号**: コード分析・ユーザヒアリングにない推測によるストーリー

---

## エピック1: 標準化処理の統一

### ストーリー 1.1: 列の平均・標準偏差関数の一元化 🔵

**信頼性**: 🔵 *コード分析（4箇所の重複）・ユーザヒアリングより*

**私は** このモジュールを保守する開発者として
**`column_mean_std` を1箇所（`core::math::stats`）でのみ管理したい**
**そうすることで** バグ修正や動作変更を1箇所に行うだけで全てのメトリクスに反映できる

**関連要件**: REQ-001, REQ-002

**詳細シナリオ**:
1. `core/math/stats.rs` に `pub(crate) fn column_mean_std(vals: &[f64]) -> (f64, f64)` を実装する
2. `core/math/mod.rs` に `pub(crate) mod stats;` を追加する
3. `sensitivity/ridge.rs` の `transpose_and_standardize` 内のインライン計算を `core::math::stats::column_mean_std` に置き換える
4. `sensitivity/analysis/common.rs` の `build_standardized_param_columns` を同様に置き換える
5. `sensitivity/sobol.rs` のローカル関数 `column_mean_std` を削除し、`core::math::stats` のものを使う
6. `pdp/utils.rs` の `col_mean_std` を削除し、`core::math::stats::column_mean_std` に置き換える
7. 全テストがパスすることを確認する

**前提条件**:
- `rust_core/src/core/math/` ディレクトリが存在する（grid.rs, linear_algebra.rs が既にある）

**制約事項**:
- 空スライスの場合は `(0.0, 1.0)` を返す（pdp の既存動作と統一）
- 公開APIに変化なし

**優先度**: Must Have

---

### ストーリー 1.2: pdp/utils.rs の共通処理移行 🔵

**信頼性**: 🔵 *ユーザヒアリング: pdpも包含*

**私は** このモジュールを保守する開発者として
**pdp モジュールの統計ヘルパーも sensitivity と同じ共通関数を使いたい**
**そうすることで** sensitivity と pdp の計算結果が同じアルゴリズムで算出される保証が得られる

**関連要件**: REQ-002-4, REQ-007-2

**詳細シナリオ**:
1. `pdp/utils.rs` の `col_mean_std` を `core::math::stats::column_mean_std` への転送またはインポートに変更する
2. `pdp/ridge_core.rs` の参照先を更新する
3. `pdp` モジュールの既存テストがパスすることを確認する

**優先度**: Must Have

---

## エピック2: 型安全性の向上

### ストーリー 2.1: 型エイリアスからNewtypeへの移行 🔵

**信頼性**: 🔵 *ユーザヒアリング: Newtypeパターンに変更*

**私は** このモジュールのユーザーとして
**`RfAnovaResult` と `MdiResult` を誤って取り違えた場合にコンパイルエラーを得たい**
**そうすることで** ランタイムでのバグではなくコンパイル時に誤りを検出できる

**関連要件**: REQ-003

**詳細シナリオ**:
1. `types.rs` の型エイリアスをNewtypeに変更する：
   ```rust
   pub struct RfAnovaResult(pub TreeImportanceResult);
   pub struct MdiResult(pub TreeImportanceResult);
   pub struct ShapResult(pub TreeImportanceResult);
   pub struct PermutationResult(pub TreeImportanceResult);
   ```
2. 各メトリクスの戻り値箇所（`rf_anova.rs`、`mdi.rs`、`shap.rs`、`permutation.rs`）でNewtypeでラップする
3. `SensitivityResult` のフィールド参照箇所で `.0` アクセスが必要な箇所を修正する
4. `pub use types::{RfAnovaResult, MdiResult, ShapResult, PermutationResult, ...}` は維持する
5. `cargo build` でコンパイルエラーがなくなるまで修正する
6. 全テストがパスすることを確認する

**優先度**: Must Have

---

## エピック3: ツリーメトリクスのTrait抽象化

### ストーリー 3.1: TreeMetric トレイトの定義と実装 🔵

**信頼性**: 🔵 *ユーザヒアリング: Trait で抽象化*

**私は** このモジュールに新しいツリーベースのメトリクスを追加する開発者として
**`TreeMetric` トレイトを実装するだけで分析パイプラインに組み込みたい**
**そうすることで** `analysis/full.rs` などの上位層を変更せずに新機能を追加できる**

**関連要件**: REQ-004

**詳細シナリオ**:
1. `sensitivity/tree_common.rs` または新規 `sensitivity/metrics.rs` に `TreeMetric` トレイトを定義する
2. `RfAnova`・`Mdi`・`Shap`・`Permutation` の各メトリクスに対して構造体またはユニット型を定義する
3. 各型に `TreeMetric` を実装する
4. `analysis/full.rs` でトレイトを通じてメトリクスを呼び出す形に変更する
5. `SensitivityMetric` enum を `TreeMetric` impl へのディスパッチに活用する
6. 全テストがパスすることを確認する

**前提条件**:
- エピック2（Newtypeパターン）が完了していること

**優先度**: Must Have

---

## エピック4: 定数の集約と可読性向上

### ストーリー 4.1: MAX_ROWS とシード定数の一元管理 🔵

**信頼性**: 🔵 *ユーザヒアリング: 定数を集約*

**私は** このモジュールを保守する開発者として
**全てのツリーメトリクスの設定定数を1箇所で確認・変更したい**
**そうすることで** パラメータチューニング時に複数ファイルを横断する必要がなくなる**

**関連要件**: REQ-005

**詳細シナリオ**:
1. `tree_common.rs` に以下の定数を集約する（または `constants.rs` を新規作成）：
   - `MDI_MAX_ROWS = 1_000`
   - `SHAP_MAX_ROWS = 1_000`
   - `RF_ANOVA_MAX_ROWS = 2_000`
   - `PFI_MAX_ROWS = 2_000`
   - `RF_SEED: u64`
   - `PFI_SEED_BASE: u64`
2. 各メトリクスファイルのローカル定数定義を削除し、集約先からインポートする
3. 各定数に値の根拠をコメントで追記する（例: `// LightGBM 訓練コストを考慮した上限`）
4. 全テストがパスすることを確認する

**優先度**: Must Have

---

## ストーリーマップ

```
エピック1: 標準化処理の統一
├── ストーリー 1.1: column_mean_std の一元化 (🔵 Must Have)
└── ストーリー 1.2: pdp/utils.rs の共通処理移行 (🔵 Must Have)

エピック2: 型安全性の向上
└── ストーリー 2.1: 型エイリアス → Newtype (🔵 Must Have)

エピック3: ツリーメトリクスのTrait抽象化
└── ストーリー 3.1: TreeMetric トレイト定義と実装 (🔵 Must Have)

エピック4: 定数の集約
└── ストーリー 4.1: MAX_ROWS とシード定数の一元管理 (🔵 Must Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 5件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
