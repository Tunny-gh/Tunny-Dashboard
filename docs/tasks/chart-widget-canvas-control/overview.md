# chart-widget-canvas-control タスク概要

**作成日**: 2026-04-17
**推定工数**: 20時間
**総タスク数**: 7件

## 関連文書

- **設計文書**: [📐 architecture.md](../../design/chart-widget-canvas-control/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/chart-widget-canvas-control/dataflow.md)
- **型定義**: [📝 interfaces.rs](../../design/chart-widget-canvas-control/interfaces.rs)
- **ヒアリング記録**: [📝 design-interview.md](../../design/chart-widget-canvas-control/design-interview.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | 型定義・データモデル | 2 | 4h | TASK-2077~2078 |
| Phase 2 | UI機能実装 | 4 | 14h | TASK-2079~2082 |
| Phase 3 | 統合テスト・動作確認 | 1 | 2h | TASK-2083 |

## タスク番号管理

**使用済みタスク番号**: TASK-2077 ~ TASK-2083
**次回開始番号**: TASK-2084

## 全体進捗

- [x] Phase 1: 型定義とデータモデル
- [x] Phase 2: UI機能実装
- [x] Phase 3: 統合と動作確認

## マイルストーン

- **M1: データモデル完成**: DragPayload 型と safe_expand メソッドが利用可能
- **M2: UI機能完成**: ✕ボタン・D&D移動・ハンドルリサイズが動作
- **M3: リリース準備完了**: 全テスト・動作確認完了

---

## Phase 1: 型定義とデータモデル

**目標**: D&D移動と安全なリサイズに必要な型とメソッドを定義する
**成果物**: DragPayload 型、safe_expand_* メソッド

### タスク一覧

- [x] [TASK-2077: DragPayload 型の定義と単体テスト](TASK-2077.md) - 2h (TDD) 🔵
- [x] [TASK-2078: safe_expand_* メソッドの実装と単体テスト](TASK-2078.md) - 2h (TDD) 🔵

### 依存関係

```
TASK-2077 ← TASK-2080 ← TASK-2081
TASK-2078 ← TASK-2082
```

### 並行開発

TASK-2077 と TASK-2078 は互いに依存しないため並行実装可能。

---

## Phase 2: UI機能実装

**目標**: ✕ボタン、セル間D&D移動、ハンドルリサイズの3機能を実装する
**成果物**: 3つのUI操作機能

### タスク一覧

- [x] [TASK-2079: ✕ボタンによる削除UI実装とテスト](TASK-2079.md) - 3h (TDD) 🔵
- [x] [TASK-2080: DragPayload 対応: 右パネル変更とドロップ処理変更](TASK-2080.md) - 3h (TDD) 🔵
- [x] [TASK-2081: セル間D&D移動: セル内ドラッグソース実装](TASK-2081.md) - 4h (TDD) 🔵
- [x] [TASK-2082: ドラッグハンドルによるサイズ変更の実装とテスト](TASK-2082.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2079: 独立（Phase 1 に依存しない）
TASK-2077 → TASK-2080 → TASK-2081
TASK-2078 → TASK-2082
```

### 並行開発

以下のグループを並行して実装可能:
- **グループA**: TASK-2079（✕ボタン）— 完全独立
- **グループB**: TASK-2080 → TASK-2081（D&D移動チェーン）
- **グループC**: TASK-2082（ハンドルリサイズ）

---

## Phase 3: 統合と動作確認

**目標**: 全機能が統合して正しく動作することを確認する
**成果物**: テスト完了・リリース可能状態

### タスク一覧

- [x] [TASK-2083: 全機能統合テスト・ビルド確認](TASK-2083.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-2079 + TASK-2081 + TASK-2082 → TASK-2083
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 7件
- 🔵 **青信号**: 7件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 4 | 0 | 0 | 4 |
| Phase 3 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2077 → TASK-2080 → TASK-2081 → TASK-2083
```

**クリティカルパス工数**: 11時間
**並行作業可能工数**: 9時間（TASK-2078: 2h, TASK-2079: 3h, TASK-2082: 4h）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2077`
