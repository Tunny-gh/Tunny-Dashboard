# egui UI リファクタリング 設計ヒアリング記録

**作成日**: 2026-05-08
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

REQ-001〜004 の技術設計を確定するため、実装上の選択肢について確認を行いました。

---

## 質問と回答

### Q1: 設計規模

**質問日時**: 2026-05-08
**カテゴリ**: 設計方針
**背景**: 設計文書の詳細度を決めるため作業規模を確認。

**回答**: フル設計

**信頼性への影響**:
- アーキテクチャ・データフロー・型定義を含む全設計文書を作成 → 🔵

---

### Q2: 既存実装の詳細分析

**質問日時**: 2026-05-08
**カテゴリ**: コード分析
**背景**: `io/html_report.rs` と `rust_core/src/lib.rs` の詳細が未確認のため、追加分析の要否を確認。

**回答**: 必要

**信頼性への影響**:
- `html_report.rs` に `HtmlReportSnapshot` 等の型が既に存在することを確認 → REQ-003 設計が 🔴 → 🔵 に向上
- `rust_core/lib.rs` に `convergence` モジュールが未存在を確認 → REQ-002 の新規ファイル作成が確定 → 🔵

---

### Q3: normalize_weights の配置先

**質問日時**: 2026-05-08
**カテゴリ**: モジュール設計
**背景**: `normalize_weights` は多目的最適化の重み正規化関数。rust_core 内での配置先として `multi_objective` モジュール、`convergence.rs` との同居、独立した `tradeoff.rs` の 3 案があった。

**回答**: multi_objective モジュール内

**信頼性への影響**:
- `rust_core/src/multi_objective/weights.rs` 新規作成が確定 → 🔴 → 🔵 に向上
- `tunny_core::multi_objective::weights::normalize_weights` として公開

---

### Q4: render_chart.rs / poll_chart.rs の visibility

**質問日時**: 2026-05-08
**カテゴリ**: 技術選択
**背景**: 分割後の 2 ファイルの関数可視性として、`pub(crate)`（クレート内限定）と `pub`（完全公開）の選択が必要。

**回答**: pub(crate)

**信頼性への影響**:
- `chart_registry.rs` のみが `render_chart` / `poll_chart_work` を呼び出すことが確定 → 層境界が 🔵 で確実

---

## ヒアリング結果サマリー

### 確認できた事項

- `io/html_report.rs` に `HtmlReportSnapshot` 等の型が既に定義されている（REQ-003 実装が容易）
- `rust_core/lib.rs` に `convergence` モジュールは未存在（新規作成が必要）
- `multi_objective` モジュールは既存（weights.rs をサブモジュールとして追加）
- `render_chart` / `poll_chart_work` は `pub(crate)` で十分（外部公開不要）

### 設計方針の決定事項

| 決定事項 | 内容 |
|---|---|
| REQ-001 visibility | `render_chart` / `poll_chart_work` は `pub(crate)` |
| REQ-002 normalize_weights | `rust_core/src/multi_objective/weights.rs` に配置 |
| REQ-002 build_best_trial_history | `&[u32]` + `&[f64]` 引数で型依存を排除 |
| REQ-003 | `html_report.rs` に `build_and_send_report` を追加 |
| REQ-004 | `tradeoff_navigator.rs` / `convergence_card.rs` は `pub fn` |

### 残課題

- `normalize_weights` の `multi_objective/mod.rs` への `pub use` 追加が必要（lib.rs の re-export パス確定）
- `convergence.rs` の `lib.rs` への `pub mod convergence` 追加が必要

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 6件
- 🟡 黄信号: 2件
- 🔴 赤信号: 2件

**ヒアリング後**:
- 🔵 青信号: 10件 (+4)
- 🟡 黄信号: 0件 (-2)
- 🔴 赤信号: 0件 (-2)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/egui-ui-refactor/requirements.md)
