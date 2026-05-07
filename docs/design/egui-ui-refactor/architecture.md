# egui UI リファクタリング アーキテクチャ設計

**作成日**: 2026-05-08
**関連要件定義**: [requirements.md](../../spec/egui-ui-refactor/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・既存 architecture.md (Phase 1-3) より*

Phase 1-3 リファクタリング完了後の egui-app クレートに残存する責務混在 4 件を解消する。
既存の 3 層境界契約（Pure Logic / App State / UI View）を維持し、追加の 4 要件（REQ-001〜004）を段階的に適用する。

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *layer-contract.md・ユーザヒアリングより*

- **パターン**: egui MVU + Registry + 3-Layer Contract
- **選択理由**: Phase 1-3 で採用済みのパターンを継続し、段階的移行コストを最小化する

### 3 層境界契約（変更なし） 🔵

**信頼性**: 🔵 *layer-contract.md より*

| 層 | クレート/モジュール | 禁止依存 |
|---|---|---|
| Pure Logic/Data | `rust_core` (`tunny_core`) | egui・theme |
| App State | `egui-app/src/state/*` | egui・theme |
| UI View | `egui-app/src/ui/*`, `app.rs` | 直接I/O・計算起動 |

---

## 変更対象コンポーネント

### REQ-001: chart_registry.rs の 3 分割 🔵

**信頼性**: 🔵 *ユーザヒアリング Q4・既存コード分析より*

**変更前**:
```
ui/chart_registry.rs  (~750行)
├── show_chart()         # 公開 API
├── show_cell_chart()    # 公開 API
├── render_chart()       # private: 描画専用
└── poll_chart_work()    # private: ディスパッチ専用
```

**変更後**:
```
ui/chart_registry.rs  (~20行)  # 薄いラッパー
├── show_chart()         # 公開 API（シグネチャ不変）
└── show_cell_chart()    # 公開 API（シグネチャ不変）

ui/render_chart.rs    (~350行)  # 描画専用
└── pub(crate) render_chart()
    ├── 引数: (&mut Ui, &mut AppState, &mut WidgetStates, &ChartId)
    └── 禁止: spawn_task・tx 呼び出し

ui/poll_chart.rs      (~380行)  # ディスパッチ専用
└── pub(crate) poll_chart_work()
    ├── 引数: (&mut AppState, &mut WidgetStates, &ChartId, &SyncSender<AppMessage>)
    └── 禁止: egui::Ui 引数・描画操作
```

---

### REQ-002: 計算ロジックを rust_core へ移動 🔵

**信頼性**: 🔵 *ユーザヒアリング Q3・既存コード left_panel.rs 分析より*

**新規モジュール 1**: `rust_core/src/convergence.rs`

```
tunny_core::convergence
├── pub fn compute_improvement_rate(history: &[(u32, f64)], last_n: usize) -> f64
└── pub fn build_best_trial_history(
        trial_ids: &[u32],
        objective_values: &[f64],
        is_minimize: bool,
    ) -> Vec<(u32, f64)>
```

**注意**: `build_best_trial_history` は egui-app 側の `TrialRow` を受け取らず、
ID と値の slice を受け取ることで rust_core と egui-app 間の型依存を排除する。

**新規サブモジュール 2**: `rust_core/src/multi_objective/weights.rs`

```
tunny_core::multi_objective::weights
└── pub fn normalize_weights(weights: &mut [f64])
```

**rust_core/src/lib.rs への追加**:
```rust
pub mod convergence;
// multi_objective は既存。weights はサブモジュールとして追加。
```

---

### REQ-003: HTML レポート構築を io 層へ移動 🔵

**信頼性**: 🔵 *ユーザヒアリング Q5・app.rs:77-108 分析より*

**変更後の責務**:

```
app.rs::apply_toolbar_actions
  └── GenerateHtmlReport 処理 (~3行)
        └── 呼び出す → io::html_report::build_and_send_report(ctx, indices, tx)

io/html_report.rs
  └── pub fn build_and_send_report(
          ctx: &StudyContext,
          selected_indices: &[u32],
          tx: mpsc::SyncSender<AppMessage>,
      )
      ├── HtmlReportSnapshot を構築
      └── generate_html_report_async(snap, tx) を呼び出す
```

`app.rs` は `HtmlReportSnapshot` / `HtmlTrialRow` / `TrialStatistics` を一切 import しなくなる。

---

### REQ-004: 左パネル UI のウィジェット分割 🔵

**信頼性**: 🔵 *ユーザヒアリング Q6・left_panel.rs:221-344 分析より*

**新規ウィジェット**:

```
ui/widgets/tradeoff_navigator.rs
  └── pub fn show_tradeoff_navigator(
          ui: &mut Ui,
          app_state: &mut AppState,
          objective_names: &[String],
          is_minimize: &[bool],
          tx: &SyncSender<AppMessage>,
      )

ui/widgets/convergence_card.rs
  └── pub fn show_convergence_card(ui: &mut Ui, app_state: &AppState)
```

左パネル側の変更:
- `left_panel.rs` から `show_tradeoff_navigator` / `show_convergence_card` の定義を削除
- `use crate::ui::widgets::{tradeoff_navigator, convergence_card};` に切り替え
- `normalize_weights` の定義を削除し `tunny_core::multi_objective::weights::normalize_weights` を使用

---

## 変更後のモジュール構成 🔵

**信頼性**: 🔵 *ユーザヒアリング全設問・既存コード分析より*

```
egui-app/src/
├── app.rs                         # 変更: HTML レポート構築ロジック除去（~30行削減）
├── io/
│   └── html_report.rs             # 変更: build_and_send_report 追加
├── ui/
│   ├── chart_registry.rs          # 変更: 薄いラッパーのみ（~730行削減）
│   ├── render_chart.rs            # 新規: 描画専用 (~350行)
│   ├── poll_chart.rs              # 新規: 非同期ディスパッチ専用 (~380行)
│   ├── left_panel.rs              # 変更: 計算ロジック除去、widget 呼び出しに変換
│   ├── mod.rs                     # 変更: render_chart, poll_chart を pub mod 追加
│   └── widgets/
│       ├── tradeoff_navigator.rs  # 新規: Trade-off Navigator UI
│       ├── convergence_card.rs    # 新規: Convergence Card UI
│       └── mod.rs                 # 変更: 2モジュールの pub mod 追加
└── state/                         # 変更なし

rust_core/src/
├── convergence.rs                 # 新規: 収束診断計算関数
├── lib.rs                         # 変更: pub mod convergence 追加
└── multi_objective/
    ├── mod.rs                     # 変更: pub mod weights 追加
    └── weights.rs                 # 新規: normalize_weights
```

---

## 実装フェーズ 🔵

**信頼性**: 🔵 *NFR-001（テストグリーン維持）・段階的リファクタリング方針より*

各フェーズ完了時に `cargo test` がグリーンであることを確認する。

| フェーズ | 対象要件 | 変更ファイル | 難易度 |
|---|---|---|---|
| Phase A | REQ-002 | rust_core 追加 (3ファイル) | 低 |
| Phase B | REQ-004 | widgets 2 ファイル新規, left_panel 変更 | 低 |
| Phase C | REQ-003 | html_report.rs 変更, app.rs 変更 | 低 |
| Phase D | REQ-001 | chart_registry 分割 | 中 |

**推奨順序**: Phase A → B → C → D（依存関係なし、ただし A を先にすることで B のウィジェット内で tunny_core を使用可能）

---

## 技術的制約 🔵

**信頼性**: 🔵 *layer-contract.md・既存コード分析より*

- `poll_chart.rs` は `ui: &mut egui::Ui` 引数を持たない
- `render_chart.rs` は `tx: &SyncSender<AppMessage>` 引数を持たない
- `rust_core/src/convergence.rs` は `egui::*` / `crate::ui::*` に依存しない
- `rust_core/src/multi_objective/weights.rs` は UI 型を使用しない
- `chart_registry::show_chart` / `show_cell_chart` のシグネチャは変更しない
- `show_tradeoff_navigator` / `show_convergence_card` のシグネチャは変更しない

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/egui-ui-refactor/requirements.md)
- **既存 Phase 1-3 設計**: [../responsibility-separation-refactoring/architecture.md](../responsibility-separation-refactoring/architecture.md)
- **層境界契約**: [../responsibility-separation-refactoring/layer-contract.md](../responsibility-separation-refactoring/layer-contract.md)

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
