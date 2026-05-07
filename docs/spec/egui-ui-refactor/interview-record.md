# egui UI リファクタリング ヒアリング記録

**作成日**: 2026-05-08
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

Phase 1-3 リファクタリング完了後の追加責務分離について、残存課題を特定し方針を確認するためのヒアリングを実施しました。

---

## 質問と回答

### Q1: 作業規模

**質問日時**: 2026-05-08
**カテゴリ**: 優先順位
**背景**: リファクタリング要件定義の詳細度を決めるため作業規模を確認。

**回答**: フル機能開発

**信頼性への影響**:
- 詳細な受け入れ基準・ユーザーストーリーを作成する方針に決定 → 🔵

---

### Q2: コードベース詳細分析の要否

**質問日時**: 2026-05-08
**カテゴリ**: 分析範囲
**背景**: 設計文書と実装の一致確認だけで十分か、未読ファイルも分析するか確認。

**回答**: 必要

**信頼性への影響**:
- `left_panel.rs`・`right_panel.rs` の詳細を確認し、追加課題（計算ロジック混在）を発見 → 信頼性 🔴 → 🔵

---

### Q3: 計算ロジックの移動先

**質問日時**: 2026-05-08
**カテゴリ**: アーキテクチャ
**背景**: `left_panel.rs` に `normalize_weights`, `compute_improvement_rate`, `build_best_trial_history` という純粋計算関数が存在し、UI ファイルに置くのは layer-contract に違反する。移動先として state 層・rust_core の2案を提示。

**回答**: rust_core に移動

**信頼性への影響**:
- REQ-002 の配置先が確定し 🔴 → 🔵 に向上
- rust_core への移動により型変換（egui-app TrialRow → rust_core 型）が必要になる可能性を確認

---

### Q4: chart_registry.rs の分割方針

**質問日時**: 2026-05-08
**カテゴリ**: ファイル構成
**背景**: `chart_registry.rs`（~750行）に `render_chart`（描画）と `poll_chart_work`（非同期ディスパッチ）が共存している。既存コードでは private 関数で分離済みだが、ファイル単位で分割するかどうか確認。

**回答**: 分割する

**信頼性への影響**:
- `render_chart.rs` + `poll_chart.rs` + `chart_registry.rs`（薄いラッパー）の3ファイル構成に確定 → 🔴 → 🔵

---

### Q5: HTML レポート構築ロジックの移動

**質問日時**: 2026-05-08
**カテゴリ**: 層分離
**背景**: `app.rs::apply_toolbar_actions` 内の `GenerateHtmlReport` ハンドリングが `HtmlReportSnapshot` 構築ロジックを直書きしており、app.rs が IO 知識を持つ問題がある。

**回答**: io/html_report.rs に移動

**信頼性への影響**:
- REQ-003 の設計が確定し 🟡 → 🔵 に向上

---

### Q6: 左パネル UI 分割

**質問日時**: 2026-05-08
**カテゴリ**: ファイル構成
**背景**: `left_panel.rs` の Trade-off Navigator と Convergence Card は独立した機能だが同一ファイルに存在している。ウィジェット化して分割するか確認。

**回答**: 分割する

**信頼性への影響**:
- REQ-004 の設計が確定し 🟡 → 🔵 に向上

---

## ヒアリング結果サマリー

### 確認できた事項

- Phase 1-3 リファクタリングは完了済み（再実装不要）
- 追加対象は 4 つの独立した改善
- rust_core への計算ロジック移動は型変換コストを伴う
- chart_registry.rs は 3 ファイルに分割（ラッパーを維持して外部 API を保つ）
- 各変更は独立していてフェーズ間依存関係なし

### 追加/変更要件

| ID | 内容 | 信頼性 |
|---|---|---|
| REQ-001 | chart_registry.rs を render_chart.rs + poll_chart.rs に分割 | 🔵 |
| REQ-002 | 計算関数 3 件を rust_core に移動 | 🔵 |
| REQ-003 | HTML レポート構築ロジックを io 層に移動 | 🔵 |
| REQ-004 | Trade-off Navigator / Convergence Card を widgets/ に分割 | 🔵 |

### 残課題

- rust_core に追加するモジュール名（`convergence.rs` vs `analysis.rs` 等）は実装時に決定
- `tradeoff_navigator` の `MessageHandler::trigger_tradeoff_computation` 呼び出しが左パネル内にある点は対象外（I/O 呼び出しではなくメッセージ送信）
- `left_panel.rs` の AHP・MCDM UI は今回対象外

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 3件
- 🟡 黄信号: 3件
- 🔴 赤信号: 2件

**ヒアリング後**:
- 🔵 青信号: 8件 (+5)
- 🟡 黄信号: 0件 (-3)
- 🔴 赤信号: 0件 (-2)

## 関連文書

- **要件定義書**: [requirements.md](requirements.md)
- **ユーザストーリー**: [user-stories.md](user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [note.md](note.md)
