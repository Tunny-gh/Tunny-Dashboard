# chart-csv-export タスク概要

**作成日**: 2026-05-28
**推定工数**: 44時間
**総タスク数**: 9件
**タスク番号**: TASK-2319 〜 TASK-2327

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/chart-csv-export/requirements.md)
- **設計文書**: [📐 architecture.md](../design/chart-csv-export/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/chart-csv-export/dataflow.md)
- **型定義**: [📐 types.rs](../design/chart-csv-export/types.rs)
- **コンテキストノート**: [📝 note.md](../spec/chart-csv-export/note.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 |
|---------|--------|----------|------|
| Phase 1 - 基盤整備 | `io/export.rs` 拡張・`io/csv_export.rs` 骨格 | 2 | 6h |
| Phase 2 - チャート別CSV生成 | 全チャートの CSV 生成関数 | 5 | 28h |
| Phase 3 - UI統合 | ⋯メニュー・ボタングレーアウト | 2 | 10h |

## タスク番号管理

**使用済みタスク番号**: TASK-2319 〜 TASK-2327
**次回開始番号**: TASK-2328

## 全体進捗

- [x] Phase 1: 基盤整備
- [x] Phase 2: チャート別CSV生成
- [x] Phase 3: UI統合

## マイルストーン

- **M1: 基盤完成**: TASK-2320 完了 — `io/csv_export.rs` の dispatch 骨格が動作する
- **M2: CSV生成完成**: TASK-2325 完了 — 全チャートのCSV生成関数が実装される
- **M3: 機能完成**: TASK-2327 完了 — ユーザーがUIからCSVを保存できる

---

## Phase 1: 基盤整備

**目標**: CSV保存・dispatch骨格の基盤を整備する
**成果物**: `io/export.rs` の `save_csv_to_file_named()`・`io/csv_export.rs` の公開インターフェース

### タスク一覧

- [x] [TASK-2319: io/export.rs に save_csv_to_file_named() を追加](TASK-2319.md) - 2h (DIRECT) 🔵
- [x] [TASK-2320: io/csv_export.rs 新規作成 - dispatch骨格・公開インターフェース](TASK-2320.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2319 → TASK-2320
```

---

## Phase 2: チャート別CSV生成

**目標**: 全チャートに対応するCSV生成関数を実装する
**成果物**: `io/csv_export.rs` の全チャード対応の内部関数群

### タスク一覧

- [x] [TASK-2321: OptimizationHistory・HvHistory・SliceChart CSV生成](TASK-2321.md) - 6h (TDD) 🔵
- [x] [TASK-2322: ParallelCoordinates・ScatterMatrix・ClusterScatter・ParetoScatter CSV生成](TASK-2322.md) - 6h (TDD) 🔵
- [x] [TASK-2323: ImportanceChart・SensitivityHeatmap CSV生成](TASK-2323.md) - 6h (TDD) 🔵
- [x] [TASK-2324: MCDM (RankChart/Table/Scatter)・AHP (RankChart/Table) CSV生成](TASK-2324.md) - 6h (TDD) 🔵
- [x] [TASK-2325: PdpChart・PdpChart2D CSV生成](TASK-2325.md) - 4h (TDD) 🟡

### 依存関係

```
TASK-2320 → TASK-2321
TASK-2320 → TASK-2322
TASK-2320 → TASK-2323
TASK-2320 → TASK-2324
TASK-2320 → TASK-2325
```

TASK-2321〜2325 は TASK-2320 完了後に並列実行可能。

---

## Phase 3: UI統合

**目標**: grid_canvas.rs に SaveAsCsv バリアントを追加し、グレーアウト制御を実装する
**成果物**: 動作するCSVエクスポート機能（ボタン・ダイアログ・グレーアウト）

### タスク一覧

- [x] [TASK-2326: CellToolbarAction::SaveAsCsv 追加・handle_toolbar_action() 拡張・UI統合](TASK-2326.md) - 6h (TDD) 🔵
- [x] [TASK-2327: ボタングレーアウト・ツールチップ実装](TASK-2327.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2319 → TASK-2326
TASK-2321 → TASK-2326
TASK-2322 → TASK-2326
TASK-2323 → TASK-2326
TASK-2324 → TASK-2326
TASK-2325 → TASK-2326
TASK-2326 → TASK-2327
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 9件
- 🔵 **青信号**: 8件 (89%)
- 🟡 **黄信号**: 1件 (11%) — TASK-2325 (PDP型要確認)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 4 | 1 | 0 | 5 |
| Phase 3 | 2 | 0 | 0 | 2 |

**品質評価**: 高品質

---

## クリティカルパス

```
TASK-2319 → TASK-2320 → TASK-2325 → TASK-2326 → TASK-2327
```

**クリティカルパス工数**: 20時間
**並行作業可能工数**: 24時間（TASK-2321〜2325 を並列実行）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2319`
