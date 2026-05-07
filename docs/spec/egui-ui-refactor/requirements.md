# egui UI リファクタリング 要件定義書

**作成日**: 2026-05-08

## 概要

Phase 1-3 リファクタリング（state/ 分割・chart_registry 抽出・MessageHandler 抽出）完了後に残存する責務混在を解消し、egui アプリとして適切な層境界を確立する追加リファクタリング。

対象クレート: `egui-app`（一部 `rust_core` への移動を含む）

**前提**: `docs/design/responsibility-separation-refactoring/layer-contract.md` で定義された層境界契約に従う。

---

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **層境界契約**: [docs/design/responsibility-separation-refactoring/layer-contract.md](../../design/responsibility-separation-refactoring/layer-contract.md)
- **既存アーキテクチャ**: [docs/design/responsibility-separation-refactoring/architecture.md](../../design/responsibility-separation-refactoring/architecture.md)

---

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存コード分析・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: 既存コード分析・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: 既存コード・ユーザヒアリングにない推測による要件

---

### REQ-001: chart_registry.rs の描画・ディスパッチ分割 🔵

**信頼性**: 🔵 *ユーザヒアリング Q4・既存コード chart_registry.rs 分析より*

システムは `ui/chart_registry.rs`（~750行）を以下の 3 ファイルに分割しなければならない：

| ファイル | 責務 | 関数 |
|---|---|---|
| `ui/render_chart.rs` | 描画専用。`AppState` 読み取り・`egui::Ui` 操作のみ | `pub(crate) fn render_chart(...)` |
| `ui/poll_chart.rs` | 非同期ディスパッチ専用。`spawn_task` 呼び出しのみ | `pub(crate) fn poll_chart_work(...)` |
| `ui/chart_registry.rs` | 薄いラッパー。外部公開 API を維持 | `pub fn show_chart(...)`, `pub fn show_cell_chart(...)` |

**制約**:
- `chart_registry::show_chart` / `show_cell_chart` のシグネチャは変更しない
- `render_chart` 関数は `tx: &mpsc::SyncSender<AppMessage>` 引数を持たない
- `poll_chart_work` 関数は `ui: &mut egui::Ui` 引数を持たない

---

### REQ-002: 計算ロジックを rust_core に移動 🔵

**信頼性**: 🔵 *ユーザヒアリング Q3・既存コード left_panel.rs 分析より*

システムは `egui-app/src/ui/left_panel.rs` に存在する以下の純粋計算関数を `rust_core` クレートに移動しなければならない：

| 関数 | 現在の場所 | 移動先 |
|---|---|---|
| `normalize_weights(weights: &mut [f64])` | `left_panel.rs` | `rust_core/src/` の新規または既存モジュール |
| `compute_improvement_rate(history: &[(u32, f64)], last_n: usize) -> f64` | `left_panel.rs` | `rust_core/src/convergence.rs`（新規） |
| `build_best_trial_history(...)` | `left_panel.rs` | `rust_core/src/convergence.rs`（新規） |

**制約**:
- `build_best_trial_history` は `rust_core` の型を使うよう引数を変換する
- `left_panel.rs` からは `tunny_core::xxx` として呼び出す
- rust_core の公開 API に追加する（`lib.rs` の `pub mod` / `pub use`）

---

### REQ-003: HTML レポート構築ロジックを io 層に移動 🔵

**信頼性**: 🔵 *ユーザヒアリング Q5・既存コード app.rs:77-108 分析より*

システムは `app.rs::apply_toolbar_actions` 内の `ToolbarAction::GenerateHtmlReport` ハンドリングにある `HtmlReportSnapshot` 構築ロジックを `io/html_report.rs` に移動しなければならない。

**変更前** (`app.rs`):
```
ToolbarAction::GenerateHtmlReport => {
    // HtmlReportSnapshot の構築が inline で 30 行存在
    let snap = HtmlReportSnapshot { ... };
    generate_html_report_async(snap, self.sender());
}
```

**変更後** (`app.rs`):
```
ToolbarAction::GenerateHtmlReport => {
    if let Some(ctx) = &self.app_state.current_study {
        crate::io::html_report::build_and_send_report(
            ctx, &self.app_state.selected_indices, self.sender()
        );
    }
}
```

**新規関数** (`io/html_report.rs`):
```rust
pub fn build_and_send_report(
    ctx: &StudyContext,
    selected_indices: &[u32],
    tx: mpsc::SyncSender<AppMessage>,
)
```

---

### REQ-004: Trade-off Navigator と Convergence Card を widgets/ に分割 🔵

**信頼性**: 🔵 *ユーザヒアリング Q6・既存コード left_panel.rs:222-344 分析より*

システムは `left_panel.rs` 内の Trade-off Navigator UI と Convergence Card UI を独立したウィジェットファイルに分割しなければならない：

| 新ファイル | 移動する UI 関数 |
|---|---|
| `ui/widgets/tradeoff_navigator.rs` | `show_tradeoff_navigator(...)` |
| `ui/widgets/convergence_card.rs` | `show_convergence_card(...)` |

**制約**:
- `left_panel.rs` からは `use crate::ui::widgets::xxx` で呼び出す
- 各ウィジェットファイルは `ui/widgets/mod.rs` で `pub mod` 宣言する
- REQ-002 で rust_core に移動した計算関数はウィジェット内から `tunny_core::xxx` 経由で呼び出す

---

## 非機能要件

### NFR-001: テストのグリーン維持 🔵

**信頼性**: 🔵 *既存アーキテクチャ設計方針（段階的リファクタリング）より*

各要件の変更完了時に `cargo test` がグリーンでなければならない。

### NFR-002: 外部 API の後方互換性維持 🔵

**信頼性**: 🔵 *layer-contract.md・既存コード use 宣言分析より*

`chart_registry::show_chart` / `show_cell_chart` のシグネチャは変更してはならない。`left_panel::show_tradeoff_navigator` / `show_convergence_card` の公開シグネチャは変更してはならない（`pub` を維持）。

### NFR-003: 層境界の遵守 🔵

**信頼性**: 🔵 *layer-contract.md より*

- `state/*` モジュールは `egui` / `theme` に依存してはならない
- `render_chart.rs` は `spawn_task` を呼び出してはならない
- `poll_chart.rs` は `egui::Ui` を引数に取ってはならない
- rust_core の新規モジュールは UI 型（`egui::Color32` 等）を使用してはならない

---

## スコープ外

- `widget_states.rs` の構造変更
- 各チャートウィジェット（scatter_matrix.rs 等）の内部リファクタリング
- `layout_state.rs` の分割
- AHP / MCDM UI の left_panel 分離（今回対象外）
- rust_core/lib.rs の WASM バインディング分離

---

## 変更後のモジュール構成 🔵

**信頼性**: 🔵 *ユーザヒアリング全設問より*

```
egui-app/src/
├── app.rs                         # 変更: HTML レポート構築ロジック除去
├── io/
│   └── html_report.rs             # 変更: build_and_send_report 追加
├── ui/
│   ├── chart_registry.rs          # 変更: 薄いラッパーのみに
│   ├── render_chart.rs            # 新規: 描画専用
│   ├── poll_chart.rs              # 新規: 非同期ディスパッチ専用
│   ├── left_panel.rs              # 変更: 計算ロジック除去、widget 呼び出しに
│   ├── mod.rs                     # 変更: render_chart, poll_chart 追加
│   └── widgets/
│       ├── tradeoff_navigator.rs  # 新規: Trade-off Navigator UI
│       ├── convergence_card.rs    # 新規: Convergence Card UI
│       └── mod.rs                 # 変更: 2 ウィジェット追加
└── （その他変更なし）

rust_core/src/
├── convergence.rs                 # 新規: 収束診断計算関数
└── （normalize_weights は適切な既存または新規モジュールへ）
```
