# 責務分離リファクタリング タスク概要

**作成日**: 2026-04-15
**プロジェクト期間**: 1日
**推定工数**: 10時間
**総タスク数**: 3件

## 関連文書

- **設計文書**: [architecture.md](../../design/responsibility-separation-refactoring/architecture.md)
- **データフロー図**: [dataflow.md](../../design/responsibility-separation-refactoring/dataflow.md)
- **型定義**: [interfaces.rs](../../design/responsibility-separation-refactoring/interfaces.rs)
- **ヒアリング記録**: [design-interview.md](../../design/responsibility-separation-refactoring/design-interview.md)
- **egui移行設計**: [egui-migration architecture.md](../../design/egui-migration/architecture.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | app_state.rs 型・フィルター・結果の3ファイル分割 | 1 | 4h | [TASK-2028](#phase-1-型フィルター結果の分離) |
| Phase 2 | grid_canvas.rs から chart_registry.rs 抽出 | 1 | 3h | [TASK-2029](#phase-2-チャートディスパッチの分離) |
| Phase 3 | app.rs から message_handler.rs 抽出 | 1 | 3h | [TASK-2030](#phase-3-メッセージ処理の分離) |

## タスク番号管理

**使用済みタスク番号**: TASK-2028 ~ TASK-2030
**次回開始番号**: TASK-2031

## 全体進捗

- [x] Phase 1: 型・フィルター・結果の分離
- [x] Phase 2: チャートディスパッチの分離
- [x] Phase 3: メッセージ処理の分離

## マイルストーン

- **M1: Phase 1 完了**: app_state.rs の3ファイル分割完了 + cargo test グリーン
- **M2: Phase 2 完了**: chart_registry.rs 抽出完了 + cargo test グリーン
- **M3: リファクタリング完了**: 全フェーズ完了 + 最終 cargo test グリーン

---

## Phase 1: 型・フィルター・結果の分離

**目標**: app_state.rs (648行) を types.rs, filter.rs, results.rs に分割
**成果物**: 3つの新規ファイル + 再エクスポート対応の app_state.rs

### タスク一覧

- [x] [TASK-2028: app_state.rs 型・フィルター・結果の分離](TASK-2028.md) - 4h (TDD) 🔵

### 依存関係

```
(なし) → TASK-2028
```

---

## Phase 2: チャートディスパッチの分離

**目標**: grid_canvas.rs から show_chart() を chart_registry.rs に抽出
**成果物**: chart_registry.rs + グリッド描画専任の grid_canvas.rs

### タスク一覧

- [x] [TASK-2029: grid_canvas.rs チャートディスパッチの分離](TASK-2029.md) - 3h (TDD) 🔵

### 依存関係

```
TASK-2028 → TASK-2029
```

---

## Phase 3: メッセージ処理の分離

**目標**: app.rs の poll_messages() を MessageHandler に抽出
**成果物**: message_handler.rs + 薄いラッパーの app.rs

### タスク一覧

- [x] [TASK-2030: app.rs メッセージ処理の分離](TASK-2030.md) - 3h (TDD) 🔵

### 依存関係

```
TASK-2029 → TASK-2030
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 3件
- 🔵 **青信号**: 3件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 1 | 0 | 0 | 1 |
| Phase 2 | 1 | 0 | 0 | 1 |
| Phase 3 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2028 → TASK-2029 → TASK-2030
```

**クリティカルパス工数**: 10時間
**並行作業可能工数**: 0時間（全タスク直列）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2028`
