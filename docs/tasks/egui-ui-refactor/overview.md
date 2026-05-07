# egui UI リファクタリング タスク概要

**作成日**: 2026-05-08
**プロジェクト期間**: 約 4 日間（25時間）
**総タスク数**: 9件
**推定工数**: 25時間

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/egui-ui-refactor/requirements.md)
- **設計文書**: [📐 architecture.md](../design/egui-ui-refactor/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/egui-ui-refactor/dataflow.md)
- **関数シグネチャ**: [📝 interfaces.rs](../design/egui-ui-refactor/interfaces.rs)
- **ヒアリング記録**: [💬 design-interview.md](../design/egui-ui-refactor/design-interview.md)
- **コンテキストノート**: [📝 note.md](../spec/egui-ui-refactor/note.md)

## フェーズ構成

| フェーズ | 対象要件 | 成果物 | タスク数 | 工数 |
|---------|---------|--------|---------|------|
| Phase A | REQ-002 | convergence.rs + weights.rs 新規作成・公開・左パネル切り替え | 3 | 8h |
| Phase B | REQ-004 | widgets/ 分割（convergence_card + tradeoff_navigator） | 2 | 8h |
| Phase C | REQ-003 | HTML レポート io 層移動 | 2 | 3h |
| Phase D | REQ-001 | chart_registry.rs 3 分割 | 2 | 6h |

## タスク番号管理

**使用済みタスク番号**: TASK-2198 〜 TASK-2206
**次回開始番号**: TASK-2207

## 全体進捗

- [x] Phase A: REQ-002 rust_core 計算ロジック移動
- [x] Phase B: REQ-004 左パネル UI 分割
- [x] Phase C: REQ-003 HTML レポート io 層移動
- [x] Phase D: REQ-001 chart_registry 分割

## マイルストーン

- **M1: rust_core 公開完了** (Phase A 完了): `tunny_core::convergence` / `tunny_core::multi_objective::weights` が egui-app から呼び出し可能
- **M2: UI ウィジェット分割完了** (Phase B 完了): `left_panel.rs` が widgets/ 経由でのみ呼び出す状態に
- **M3: io 層整理完了** (Phase C 完了): `app.rs` から HtmlReportSnapshot の直接操作が消える
- **M4: chart_registry 分割完了** (Phase D 完了): 全受け入れ基準がパス

---

## Phase A: REQ-002 rust_core 計算ロジック移動

**目標**: `left_panel.rs` の純粋計算関数を `rust_core` に移植し、egui-app から参照可能にする
**推奨順序**: TASK-2198 と TASK-2199 は並行実行可能。その後 TASK-2200 を実行

### タスク一覧

- [x] [TASK-2198: rust_core/src/convergence.rs の新規作成](TASK-2198.md) - 4h (TDD) 🔵
- [x] [TASK-2199: rust_core/src/multi_objective/weights.rs の新規作成](TASK-2199.md) - 2h (TDD) 🔵
- [x] [TASK-2200: lib.rs / mod.rs への pub mod 追加 + left_panel.rs 呼び出し切り替え](TASK-2200.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-2198 ─┐
            ├─► TASK-2200
TASK-2199 ─┘
```

---

## Phase B: REQ-004 左パネル UI 分割

**目標**: `left_panel.rs` から `show_tradeoff_navigator` / `show_convergence_card` を独立ウィジェットに移動する
**推奨順序**: TASK-2201 → TASK-2202（順次実行）

### タスク一覧

- [x] [TASK-2201: ui/widgets/convergence_card.rs 新規作成 + widgets/mod.rs 作成](TASK-2201.md) - 4h (TDD) 🔵
- [x] [TASK-2202: ui/widgets/tradeoff_navigator.rs 新規作成 + left_panel.rs 定義削除・切り替え](TASK-2202.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2200 → TASK-2201 → TASK-2202
```

---

## Phase C: REQ-003 HTML レポート io 層移動

**目標**: `app.rs` から HtmlReportSnapshot の構築責務を削除し、`io/html_report.rs` に集約する
**推奨順序**: TASK-2203 → TASK-2204（順次実行）

### タスク一覧

- [x] [TASK-2203: io/html_report.rs に build_and_send_report 追加](TASK-2203.md) - 2h (TDD) 🔵
- [x] [TASK-2204: app.rs の GenerateHtmlReport ハンドリング簡素化](TASK-2204.md) - 1h (DIRECT) 🔵

### 依存関係

```
TASK-2202 → TASK-2203 → TASK-2204
```

---

## Phase D: REQ-001 chart_registry 分割

**目標**: `chart_registry.rs`（~750行）を描画専用・dispatch 専用・薄いラッパーの 3 ファイルに分割する
**推奨順序**: TASK-2205 → TASK-2206（順次実行）

### タスク一覧

- [x] [TASK-2205: ui/render_chart.rs + ui/poll_chart.rs 新規作成（コード抽出）](TASK-2205.md) - 4h (DIRECT) 🔵
- [x] [TASK-2206: chart_registry.rs の薄いラッパー化 + cargo test 最終確認](TASK-2206.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-2204 → TASK-2205 → TASK-2206
```

---

## 全体クリティカルパス

```
TASK-2198 ─┐
            ├─► TASK-2200 → TASK-2201 → TASK-2202 → TASK-2203 → TASK-2204 → TASK-2205 → TASK-2206
TASK-2199 ─┘
```

**クリティカルパス工数**: 25時間（TASK-2199 の 2h は TASK-2198 と並行実行可能なため実質 23h）

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 9件
- 🔵 **青信号**: 9件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase A | 3 | 0 | 0 | 3 |
| Phase B | 2 | 0 | 0 | 2 |
| Phase C | 2 | 0 | 0 | 2 |
| Phase D | 2 | 0 | 0 | 2 |

**品質評価**: ✅ 高品質

---

## 実装時の注意事項

- 各フェーズ完了時に `cargo test --workspace` がグリーンであることを確認する（NFR-001）
- `chart_registry::show_chart` / `show_cell_chart` のシグネチャを変更しない（NFR-002）
- `show_tradeoff_navigator` / `show_convergence_card` のシグネチャを変更しない（NFR-002）
- `rust_core` の新規モジュールに `egui::*` を使用しない（NFR-003）
- WASM 対応は不要（ネイティブ API 自由使用可）

---

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2198`
