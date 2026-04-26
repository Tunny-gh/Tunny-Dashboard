# chart-implementation タスク概要

**作成日**: 2026-04-12
**プロジェクト期間**: 2026-04-12 - 2026-04-15（4日）
**推定工数**: 22時間
**総タスク数**: 7件

## 関連文書

- **設計文書**: [📐 architecture.md](../design/chart-implementation/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/chart-implementation/dataflow.md)
- **ヒアリング記録**: [📋 design-interview.md](../design/chart-implementation/design-interview.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | Day 1 | 基盤整備（Cargo.toml + WidgetStates 拡張） | 2 | 6h | TASK-2046~2047 |
| Phase 2 | Day 2-3 | 4チャートの show() 実装 | 4 | 16h | TASK-2048~2051 |
| Phase 3 | Day 4 | 統合確認・動作テスト | 1 | 4h | TASK-2052 |

## タスク番号管理

**使用済みタスク番号**: TASK-2046 ~ TASK-2052
**次回開始番号**: TASK-2053

## 全体進捗

- [x] Phase 1: 基盤整備
- [x] Phase 2: チャート実装
- [x] Phase 3: 統合確認

## マイルストーン

- **M1: 基盤完成** (Day 1): Cargo.toml 更新・WidgetStates 拡張・grid_canvas.rs 接続完了
- **M2: チャート実装完成** (Day 3): 4チャートすべての show() 実装完了
- **M3: リリース準備完了** (Day 4): 統合テスト・目視確認完了

---

## Phase 1: 基盤整備

**期間**: Day 1
**目標**: 外部クレートの追加と WidgetStates 拡張による接続準備
**成果物**: Cargo.toml 更新、WidgetStates 4フィールド追加、grid_canvas.rs 修正

### タスク一覧

- [x] [TASK-2046: Cargo.toml に linfa 系クレート追加・ビルド確認](TASK-2046.md) - 2h (DIRECT) 🔵
- [x] [TASK-2047: WidgetStates 拡張 + grid_canvas.rs の show_chart() 修正](TASK-2047.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2046 → TASK-2047
TASK-2046 → TASK-2051（並行）
```

---

## Phase 2: チャート実装

**期間**: Day 2-3
**目標**: 4つのチャートウィジェットに show() メソッドを実装
**成果物**: SensitivityHeatmap, ScatterMatrix, ParallelCoordinates, ClusterScatter の show() 実装

### タスク一覧

- [x] [TASK-2048: SensitivityHeatmap::show() 実装](TASK-2048.md) - 4h (TDD) 🔵
- [x] [TASK-2049: ScatterMatrix::show() 実装](TASK-2049.md) - 4h (TDD) 🔵
- [x] [TASK-2050: ParallelCoordinates::show() 実装](TASK-2050.md) - 4h (TDD) 🔵
- [x] [TASK-2051: ClusterScatter::show() + PCA キャッシュ実装](TASK-2051.md) - 4h (TDD) 🟡

### 依存関係

```
TASK-2047 → TASK-2048
TASK-2047 → TASK-2049
TASK-2047 → TASK-2050
TASK-2046 + TASK-2047 → TASK-2051
TASK-2048, TASK-2049, TASK-2050, TASK-2051 → TASK-2052（Phase 3）
```

TASK-2048, 2049, 2050, 2051 は並行実行可能。

---

## Phase 3: 統合確認

**期間**: Day 4
**目標**: 全チャートの統合動作確認
**成果物**: 統合済みアプリ、動作確認済みの4チャート

### タスク一覧

- [x] [TASK-2052: 統合確認・動作テスト](TASK-2052.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2048 + TASK-2049 + TASK-2050 + TASK-2051 → TASK-2052
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 7件
- 🔵 **青信号**: 6件 (86%)
- 🟡 **黄信号**: 1件 (14%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 3 | 1 | 0 | 4 |
| Phase 3 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2046 → TASK-2047 → TASK-2051 → TASK-2052
```

**クリティカルパス工数**: 14時間
**並行作業可能工数**: 12時間（TASK-2048, 2049, 2050）

## スコープ外

- **ParetoScatter3D**: wgpu GPU レンダリングが必要なため今回スコープ外

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2046`
