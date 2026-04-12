# free-layout-dashboard タスク概要

**作成日**: 2026-04-12
**プロジェクト期間**: 2026-04-12 - 2026-04-18（5日）
**推定工数**: 54時間
**総タスク数**: 13件

## 関連文書

- **設計文書**: [📐 architecture.md](../design/free-layout-dashboard/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/free-layout-dashboard/dataflow.md)
- **型定義**: [📝 interfaces.rs](../design/free-layout-dashboard/interfaces.rs)
- **ヒアリング記録**: [📋 design-interview.md](../design/free-layout-dashboard/design-interview.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | Day 1-2 | データモデル + 状態管理 | 5 | 18h | TASK-2033~2037 |
| Phase 2 | Day 3-4 | UI コンポーネント | 5 | 24h | TASK-2038~2042 |
| Phase 3 | Day 5 | 統合 + テスト | 3 | 12h | TASK-2043~2045 |

## タスク番号管理

**使用済みタスク番号**: TASK-2001 ~ TASK-2045
**次回開始番号**: TASK-2046

## 全体進捗

- [ ] Phase 1: データモデル + 状態管理
- [ ] Phase 2: UI コンポーネント
- [x] Phase 3: 統合 + テスト

## マイルストーン

- **M1: データモデル完成** (Day 2): GridLayout・PanelItem・LayoutState 実装完了
- **M2: UI完成** (Day 4): RightPanel・GridCanvas・D&D・コンテキストメニュー完了
- **M3: リリース準備完了** (Day 5): 統合テスト・検証完了

---

## Phase 1: データモデル + 状態管理

**期間**: Day 1-2
**目標**: 新しいデータモデルと状態管理の実装
**成果物**: PanelItem, GridCell, GridLayout, RightPanelState, TrialTable widget

### タスク一覧

- [x] [TASK-2033: PanelItem + GridCell 型定義](TASK-2033.md) - 4h (TDD) 🔵
- [x] [TASK-2034: GridLayout + メソッド実装](TASK-2034.md) - 4h (TDD) 🔵
- [x] [TASK-2035: RightPanelState + LayoutState 改修](TASK-2035.md) - 4h (TDD) 🔵
- [x] [TASK-2036: TrialTable → widgets/trial_table.rs 移植](TASK-2036.md) - 4h (TDD) 🔵
- [x] [TASK-2037: BottomPanel 廃止 + left_panel クリーンアップ](TASK-2037.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-2033 → TASK-2034
TASK-2034 → TASK-2035
TASK-2036 ← TASK-2037（2036完了後に2037実施）
```

---

## Phase 2: UI コンポーネント

**期間**: Day 3-4
**目標**: 右パネル・グリッドキャンバス・D&D・コンテキストメニュー実装
**成果物**: right_panel.rs, grid_canvas.rs, D&D統合, セル結合UI

### タスク一覧

- [x] [TASK-2038: right_panel.rs 新規作成](TASK-2038.md) - 4h (TDD) 🔵
- [x] [TASK-2039: layout.rs に SidePanel::right 追加](TASK-2039.md) - 4h (TDD) 🔵
- [x] [TASK-2040: grid_canvas.rs 基本レンダリング](TASK-2040.md) - 4h (TDD) 🔵
- [x] [TASK-2041: D&D 実装（egui DnD API）](TASK-2041.md) - 4h (TDD) 🔵
- [x] [TASK-2042: 右クリックコンテキストメニュー](TASK-2042.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2035 → TASK-2038
TASK-2035 → TASK-2040
TASK-2038 → TASK-2039
TASK-2040 → TASK-2039
TASK-2040 → TASK-2041
TASK-2040 → TASK-2042
```

---

## Phase 3: 統合 + テスト

**期間**: Day 5
**目標**: 全コンポーネントの統合と動作確認
**成果物**: 統合済みアプリ、動作確認済みのフリーレイアウト機能

### タスク一覧

- [x] [TASK-2043: 行・列追加/削除ボタン](TASK-2043.md) - 4h (TDD) 🔵
- [x] [TASK-2044: main_canvas.rs → grid_canvas.rs 委譲](TASK-2044.md) - 4h (TDD) 🔵
- [x] [TASK-2045: 統合テストと動作確認](TASK-2045.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2041 → TASK-2043
TASK-2042 → TASK-2043
TASK-2039 → TASK-2044
TASK-2043 → TASK-2044
TASK-2044 → TASK-2045
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 13件
- 🔵 **青信号**: 11件 (85%)
- 🟡 **黄信号**: 2件 (15%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 5 | 0 | 0 | 5 |
| Phase 2 | 4 | 1 | 0 | 5 |
| Phase 3 | 2 | 1 | 0 | 3 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2033 → TASK-2034 → TASK-2035 → TASK-2038 → TASK-2039 → TASK-2044 → TASK-2045
```

**クリティカルパス工数**: 26時間
**並行作業可能工数**: 28時間

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2033`
