# egui UI リファクタリング ユーザストーリー

**作成日**: 2026-05-08
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存コード分析・ユーザヒアリングを参考にした確実なストーリー
- 🟡 **黄信号**: 既存コード分析・ユーザヒアリングから妥当な推測によるストーリー
- 🔴 **赤信号**: 既存コード・ユーザヒアリングにない推測によるストーリー

---

## エピック 1: chart_registry.rs の責務分割

### ストーリー 1.1: 描画ロジックを独立ファイルで管理する 🔵

**信頼性**: 🔵 *ユーザヒアリング Q4・既存コード chart_registry.rs 分析より*

**私は** 開発者 **として**
**`render_chart.rs` という描画専用ファイルを参照してチャートの描画ロジックを修正したい**
**そうすることで** 非同期処理コードに触れることなく、描画の問題を素早く特定・修正できる

**関連要件**: REQ-001

**詳細シナリオ**:
1. `render_chart.rs` を開くと、すべての `ChartId` に対する `egui::Ui` 操作のみが含まれている
2. `spawn_task` や `tx.clone()` の呼び出しは一切存在しない
3. `AppState` へのアクセスはすべて読み取り専用（または `pareto_2d` / `pareto_3d` の `&mut AppState` のみ）

**前提条件**: chart_registry.rs の分割が完了している

**優先度**: Must Have

---

### ストーリー 1.2: ディスパッチロジックを独立ファイルで管理する 🔵

**信頼性**: 🔵 *ユーザヒアリング Q4・既存コード chart_registry.rs poll_chart_work 分析より*

**私は** 開発者 **として**
**`poll_chart.rs` という非同期ディスパッチ専用ファイルを参照してバックグラウンドタスクの起動ロジックを修正したい**
**そうすることで** UI 描画コードに触れることなく、計算処理の起動フローを理解・修正できる

**関連要件**: REQ-001

**詳細シナリオ**:
1. `poll_chart.rs` を開くと、すべての `ChartId` に対する `spawn_task` 呼び出しのみが含まれている
2. `egui::Ui` 引数を一切持たない
3. `AppMessage` を送信するロジックが集中している

**前提条件**: chart_registry.rs の分割が完了している

**優先度**: Must Have

---

## エピック 2: 計算ロジックの層分離

### ストーリー 2.1: 重み正規化を tunny_core で利用する 🔵

**信頼性**: 🔵 *ユーザヒアリング Q3・layer-contract.md より*

**私は** 開発者 **として**
**`tunny_core::xxx::normalize_weights` を呼び出してトレードオフウェイトを正規化したい**
**そうすることで** UI コードに計算ロジックが混在せず、正規化ロジックを独立してテストできる

**関連要件**: REQ-002

**詳細シナリオ**:
1. `rust_core` の適切なモジュールに `normalize_weights(weights: &mut [f64])` が公開されている
2. `left_panel.rs` から `normalize_weights` の定義が消え、代わりに `tunny_core::xxx::normalize_weights(...)` の呼び出しになっている
3. rust_core のユニットテストで正規化ロジックを単独でテストできる

**前提条件**: rust_core の公開 API に追加済み

**優先度**: Must Have

---

### ストーリー 2.2: 収束診断計算を tunny_core で利用する 🔵

**信頼性**: 🔵 *ユーザヒアリング Q3・既存コード left_panel.rs:278-320 より*

**私は** 開発者 **として**
**`tunny_core::convergence::compute_improvement_rate` と `build_best_trial_history` を呼び出して収束診断データを取得したい**
**そうすることで** 収束診断の計算ロジックを UI コードから分離し、再利用・テストが容易になる

**関連要件**: REQ-002

**詳細シナリオ**:
1. `rust_core/src/convergence.rs` が新規作成され、`compute_improvement_rate` と `build_best_trial_history` が公開されている
2. `left_panel.rs` から両関数の定義が消え、`tunny_core::convergence::xxx` 経由で呼び出している
3. rust_core のテストで両関数を独立してテストできる

**前提条件**: rust_core の convergence.rs が作成済み

**優先度**: Must Have

---

## エピック 3: HTML レポート構築の io 層集約

### ストーリー 3.1: app.rs を HTML レポートの構築知識から解放する 🔵

**信頼性**: 🔵 *ユーザヒアリング Q5・既存コード app.rs:77-108 分析より*

**私は** 開発者 **として**
**`app.rs` の `apply_toolbar_actions` を修正するときに HTML レポートのデータ構造を知らなくてよい状態にしたい**
**そうすることで** app.rs が薄いコーディネーターとして機能し、修正範囲が明確になる

**関連要件**: REQ-003

**詳細シナリオ**:
1. `app.rs::apply_toolbar_actions` の `GenerateHtmlReport` ハンドリングが 3 行程度に縮小している
2. `HtmlReportSnapshot` の構築ロジックが `io/html_report.rs` に `build_and_send_report` として存在する
3. `app.rs` は `HtmlReportSnapshot` 型や `HtmlTrialRow` 型をインポートしない

**前提条件**: `io/html_report.rs` に `build_and_send_report` が追加済み

**優先度**: Must Have

---

## エピック 4: 左パネルのウィジェット分割

### ストーリー 4.1: Trade-off Navigator を独立ウィジェットとして管理する 🔵

**信頼性**: 🔵 *ユーザヒアリング Q6・既存コード left_panel.rs:221-270 分析より*

**私は** 開発者 **として**
**`ui/widgets/tradeoff_navigator.rs` を参照して Trade-off Navigator の UI を修正したい**
**そうすることで** 他の左パネルコードに触れることなく、トレードオフ機能を独立して開発できる

**関連要件**: REQ-004

**詳細シナリオ**:
1. `ui/widgets/tradeoff_navigator.rs` が作成され、`show_tradeoff_navigator` が含まれている
2. `left_panel.rs` からは `use crate::ui::widgets::tradeoff_navigator::show_tradeoff_navigator;` で呼び出している
3. 計算ロジックは REQ-002 で移動した `tunny_core` 関数を呼び出している

**前提条件**: REQ-002 完了（`normalize_weights` が tunny_core に存在する）

**優先度**: Must Have

---

### ストーリー 4.2: Convergence Card を独立ウィジェットとして管理する 🔵

**信頼性**: 🔵 *ユーザヒアリング Q6・既存コード left_panel.rs:322-344 分析より*

**私は** 開発者 **として**
**`ui/widgets/convergence_card.rs` を参照して収束診断 UI を修正したい**
**そうすることで** 収束カードの表示ロジックを集中管理できる

**関連要件**: REQ-004

**詳細シナリオ**:
1. `ui/widgets/convergence_card.rs` が作成され、`show_convergence_card` が含まれている
2. `left_panel.rs` からは `use crate::ui::widgets::convergence_card::show_convergence_card;` で呼び出している
3. 計算ロジックは REQ-002 で移動した `tunny_core` 関数を呼び出している

**前提条件**: REQ-002 完了（`compute_improvement_rate` が tunny_core に存在する）

**優先度**: Must Have

---

## ストーリーマップ

```
エピック 1: chart_registry.rs 分割
├── ストーリー 1.1 描画ロジック独立 (🔵 Must Have)
└── ストーリー 1.2 ディスパッチロジック独立 (🔵 Must Have)

エピック 2: 計算ロジック層分離
├── ストーリー 2.1 normalize_weights → tunny_core (🔵 Must Have)
└── ストーリー 2.2 convergence 計算 → tunny_core (🔵 Must Have)

エピック 3: HTML レポート io 層集約
└── ストーリー 3.1 app.rs から構築ロジック除去 (🔵 Must Have)

エピック 4: 左パネルウィジェット分割
├── ストーリー 4.1 tradeoff_navigator.rs 独立 (🔵 Must Have)
└── ストーリー 4.2 convergence_card.rs 独立 (🔵 Must Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 7件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
