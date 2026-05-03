# sensitivity-refactoring 要件定義書

## 概要

`rust_core/src/sensitivity/` および `rust_core/src/pdp/` モジュールのリファクタリング。
保守性向上・コード重複排除・速度最適化を目的とする。
既存のパブリックAPIシグネチャと全テストケース（29件）は維持する。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コード分析・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: コード分析・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: コード分析・ユーザヒアリングにない推測による要件

---

### REQ-001: core::math::stats モジュールの新規作成

- REQ-001-1: システムは `rust_core/src/core/math/stats.rs` に列の平均・標準偏差計算関数 `column_mean_std(vals: &[f64]) -> (f64, f64)` を実装しなければならない 🔵 *コード分析: 4箇所の重複実装が判明*
- REQ-001-2: 標準偏差が `f64::EPSILON` 未満の場合、関数は `1.0` を返さなければならない（ゼロ除算防止） 🔵 *既存実装の共通動作より*
- REQ-001-3: `rust_core/src/core/math/mod.rs` はこのモジュールを `pub(crate)` として公開しなければならない 🔵 *ユーザヒアリング: core::math に昇格*

### REQ-002: 標準化処理の統一

- REQ-002-1: `sensitivity/ridge.rs` の `transpose_and_standardize` 関数は `core::math::stats::column_mean_std` を使用しなければならない 🔵 *コード分析より*
- REQ-002-2: `sensitivity/analysis/common.rs` の `build_standardized_param_columns` 関数は `core::math::stats::column_mean_std` を使用しなければならない 🔵 *コード分析より*
- REQ-002-3: `sensitivity/sobol.rs` のローカル関数 `column_mean_std` は削除し、`core::math::stats::column_mean_std` に置き換えなければならない 🔵 *コード分析より*
- REQ-002-4: `pdp/utils.rs` の `col_mean_std` 関数は削除し、`core::math::stats::column_mean_std` に置き換えなければならない 🔵 *ユーザヒアリング: pdpも包含*

### REQ-003: 型安全性の向上（Newtypeパターン）

- REQ-003-1: `RfAnovaResult`・`MdiResult`・`ShapResult`・`PermutationResult` は `TreeImportanceResult` の型エイリアスではなく、Newtypeラッパー `pub struct XxxResult(pub TreeImportanceResult)` に変更しなければならない 🔵 *ユーザヒアリング: Newtypeパターンに変更*
- REQ-003-2: Newtypeに変更した際、`SensitivityResult` 構造体のフィールド型も対応するNewtypeに更新しなければならない 🔵 *コード分析より*
- REQ-003-3: `pub use types::{...}` による再エクスポートは維持し、呼び出し側のコードを壊してはならない 🔵 *ユーザヒアリング: APIシグネチャ維持*

### REQ-004: ツリーメトリクスの Trait 抽象化

- REQ-004-1: `sensitivity` モジュール内に `TreeMetric` トレイトを定義しなければならない。トレイトは少なくとも `fn compute_importances(&self, x: &[Vec<f64>], y: &[f64]) -> Option<(Vec<f64>, f64)>` メソッドを持たなければならない 🔵 *ユーザヒアリング: Trait で抽象化*
- REQ-004-2: `RfAnova`・`Mdi`・`Shap`・`Permutation` の各メトリクスは `TreeMetric` トレイトを実装しなければならない 🔵 *ユーザヒアリングより*
- REQ-004-3: `analysis/full.rs` および `analysis/selected.rs` は `TreeMetric` トレイトオブジェクトまたはジェネリクスを通じてメトリクス計算を呼び出せなければならない 🟡 *トレイト抽象化から妥当な推測*
- REQ-004-4: 新しいメトリクスを追加する際は、`TreeMetric` トレイトを実装するだけで `analysis` 層に変更なく統合できなければならない 🟡 *拡張性要件として妥当な推測*

### REQ-005: 定数の集約

- REQ-005-1: ツリーメトリクスの `MAX_ROWS` 定数は `tree_common.rs` または専用の `constants.rs` に一か所で集約されなければならない 🔵 *ユーザヒアリング: 定数を集約*
- REQ-005-2: 各メトリクスの `MAX_ROWS` 値は変更してはならない（MDI/SHAP: 1000、RF-ANOVA/PFI: 2000） 🔵 *ユーザヒアリング: 値は変更しない*
- REQ-005-3: シード定数（`RF_SEED`、`PFI_SEED_BASE` 等）も同じ集約ファイルに含めなければならない 🟡 *一貫性から妥当な推測*

### REQ-006: 既存テストの維持

- REQ-006-1: リファクタリング後も既存の29件のテストケースは全てパスしなければならない 🔵 *品質要件として必須*
- REQ-006-2: パフォーマンステスト（P01-P03）の制約（Spearman ≤500ms、Ridge ≤300ms、Selected ≤50ms）を維持しなければならない 🔵 *既存テストより*
- REQ-006-3: `#[cfg(debug_assertions)]` による debug/release の分岐テストを維持しなければならない 🔵 *既存実装より*

### REQ-007: パブリックAPIの維持

- REQ-007-1: `sensitivity` モジュールの全ての `pub use` シンボルの名前とシグネチャを変更してはならない 🔵 *WASM公開API維持要件*
- REQ-007-2: `pdp` モジュールの全ての `pub use` シンボルの名前とシグネチャを変更してはならない 🔵 *WASM公開API維持要件*

---

## 非機能要件

### パフォーマンス

- NFR-001: リファクタリング後の全メトリクスの実行時間は、リファクタリング前と比較して5%以上劣化してはならない 🟡 *パフォーマンス要件として妥当*
- NFR-002: `core::math::stats::column_mean_std` は元の各実装と同等の計算量（O(n)）でなければならない 🔵 *既存実装から直接導出*

### 保守性

- NFR-003: 重複コードを排除した結果、`sensitivity/` + `pdp/` の合計行数は現状（約2,650行）より10%以上削減されなければならない 🟡 *重複排除の効果として妥当な推測*
- NFR-004: `TreeMetric` トレイトに新しいメトリクスを追加する場合、`analysis/` 層のコード変更は不要でなければならない 🟡 *拡張性要件として妥当な推測*

### コードの明確性

- NFR-005: 集約した定数には、値の根拠（例: なぜ MDI は 1000 行なのか）をコメントで記載しなければならない 🔵 *ユーザヒアリング: 問題箇所として指摘*

---

## Edge ケース

### エラー処理

- EDGE-001: `column_mean_std` は空スライス（`len() == 0`）を渡された場合、パニックせず `(0.0, 1.0)` を返さなければならない 🔵 *pdp/utils.rs の既存動作より*
- EDGE-002: Newtypeラッパーの `.0` フィールドアクセスパターンで既存コードが壊れる箇所は全てコンパイルエラーとして検出し修正しなければならない 🔵 *Newtypeパターン導入の影響*
- EDGE-003: `TreeMetric::compute_importances` が `None` を返した場合、`SensitivityResult` の対応フィールドは `None` のままでなければならない 🔵 *既存の Option<T> 動作より*
