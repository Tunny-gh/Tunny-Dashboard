# mcdm-topsis タスク概要

**作成日**: 2026-04-23
**プロジェクト期間**: 9日（1.8週間）
**推定工数**: 72時間
**総タスク数**: 9件

## 関連文書

- **要件定義書**: [tunny-dashboard-requirements.md](../../spec/tunny-dashboard-requirements.md)
- **理論文書**: [theory/topsis.md](../../../theory/topsis.md)
- **設計文書**: [architecture.md](../../design/mcdm-topsis/architecture.md)
- **データフロー**: [dataflow.md](../../design/mcdm-topsis/dataflow.md)
- **型定義**: [interfaces.rs](../../design/mcdm-topsis/interfaces.rs)
- **ヒアリング記録**: [design-interview.md](../../design/mcdm-topsis/design-interview.md)
- **egui移行設計**: [egui-migration/architecture.md](../../design/egui-migration/architecture.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | 3日 | 型・状態基盤 | 3件 | 24h | TASK-2101~2103 |
| Phase 2 | 4日 | McdmChartウィジェット | 4件 | 32h | TASK-2104~2107 |
| Phase 3 | 2日 | カラーモード・統合 | 2件 | 16h | TASK-2108~2109 |

## タスク番号管理

**使用済みタスク番号**: TASK-2101 ~ TASK-2109
**次回開始番号**: TASK-2110

## 全体進捗

- [ ] Phase 1: 型・状態基盤
- [ ] Phase 2: McdmChartウィジェット実装
- [ ] Phase 3: カラーモード・統合テスト

## マイルストーン

- **M1: 基盤完成** (Day 3): 型定義・AppState・ChartId骨組み完了
- **M2: ウィジェット完成** (Day 7): McdmChart全機能実装完了
- **M3: リリース準備完了** (Day 9): E2Eテスト・バグ修正完了

---

## Phase 1: 型・状態基盤

**期間**: Day 1-3
**目標**: McdmMethod/McdmResult統一型の定義、AppState・AppMessageの更新、UI骨組みの追加
**成果物**: コンパイル可能な状態基盤

### タスク一覧

- [ ] [TASK-2101: McdmMethod/McdmResult/TopsisResult型定義更新](TASK-2101.md) - 8h (DIRECT) 🔵
- [ ] [TASK-2102: AppState + AppMessage + MessageHandler更新](TASK-2102.md) - 8h (DIRECT) 🔵
- [ ] [TASK-2103: ChartId + WidgetStates + 右パネル + chart_registry骨組み](TASK-2103.md) - 8h (DIRECT) 🔵

### 依存関係

```
TASK-2101 → TASK-2102 → TASK-2103
```

---

## Phase 2: McdmChartウィジェット実装

**期間**: Day 4-7
**目標**: McdmChartウィジェットの全UI要素実装、非同期計算フロー統合
**成果物**: 動作するMCDM分析ウィジェット

### タスク一覧

- [ ] [TASK-2104: McdmChart基本UI (手法セレクタ + 重みスライダー + Runボタン)](TASK-2104.md) - 8h (TDD) 🔵
- [ ] [TASK-2105: ランキングバーチャート (egui_plot BarChart + ハイライト連携)](TASK-2105.md) - 8h (TDD) 🔵
- [ ] [TASK-2106: ランキングテーブル (egui TableBuilder + クリック連携)](TASK-2106.md) - 8h (TDD) 🟡
- [ ] [TASK-2107: chart_registryディスパッチ + 非同期計算フロー統合](TASK-2107.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2103 → TASK-2104 → TASK-2105 → TASK-2107
                    ↘ TASK-2106 ↗
```

TASK-2105 と TASK-2106 は並行開発可能。

---

## Phase 3: カラーモード・統合テスト

**期間**: Day 8-9
**目標**: 散布図カラーモード追加、E2Eテスト・バグ修正
**成果物**: リリース可能なMCDM機能

### タスク一覧

- [ ] [TASK-2108: ColorMode::McdmScore + 散布図色付け](TASK-2108.md) - 8h (TDD) 🔵
- [ ] [TASK-2109: E2Eテスト・バグ修正](TASK-2109.md) - 8h (TDD) 🟡

### 依存関係

```
TASK-2107 → TASK-2108 → TASK-2109
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 9件
- 🔵 **青信号**: 6件 (67%)
- 🟡 **黄信号**: 3件 (33%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 0 | 0 | 3 |
| Phase 2 | 3 | 1 | 0 | 4 |
| Phase 3 | 1 | 1 | 0 | 2 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2101 → TASK-2102 → TASK-2103 → TASK-2104 → TASK-2105 → TASK-2107 → TASK-2108 → TASK-2109
```

**クリティカルパス工数**: 64時間（8日）
**並行作業可能工数**: 8時間（TASK-2106 が TASK-2105 と並行可能）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2101`
