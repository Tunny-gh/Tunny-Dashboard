# colormap-fix タスク概要

**作成日**: 2026-04-16
**推定工数**: 20時間
**総タスク数**: 7件

## 関連文書

- **設計文書**: [📐 architecture.md](../../design/colormap-fix/architecture.md)
- **データフロー**: [🔄 dataflow.md](../../design/colormap-fix/dataflow.md)
- **型定義**: [📝 interfaces.rs](../../design/colormap-fix/interfaces.rs)
- **ヒアリング記録**: [📋 design-interview.md](../../design/colormap-fix/design-interview.md)
- **コンテキストノート**: [📝 note.md](../../spec/colormap-fix/note.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 - カラーマップ基盤 | ColormapName, ColorMap拡張, 色計算関数 | 2 | 7h | TASK-2070~2071 |
| Phase 2 - 状態管理 | AppState拡張, セレクタUI | 2 | 5h | TASK-2072~2073 |
| Phase 3 - ウィジェット対応 | 全4ウィジェット色対応 | 3 | 8h | TASK-2074~2076 |

## タスク番号管理

**使用済みタスク番号**: TASK-2070 ~ TASK-2076
**次回開始番号**: TASK-2077

## 全体進捗

- [x] Phase 1: カラーマップ基盤
- [x] Phase 2: 状態管理
- [x] Phase 3: ウィジェット対応

## マイルストーン

- **M1: 基盤完成**: ColormapName + ColorMap + 色計算関数が完成
- **M2: 状態管理完成**: AppState + UIセレクタが動作
- **M3: 全機能完成**: 全ウィジェットがカラーマップに対応

---

## Phase 1: カラーマップ基盤

**目標**: ColormapName列挙体、ColorMap新規定義、離散パレット、色計算関数を実装
**成果物**: types.rs に ColormapName、colormap.rs に新規カラーマップと色計算関数

### タスク一覧

- [x] [TASK-2070: ColormapName列挙体とColorMap新規定義](TASK-2070.md) - 3h (TDD) 🔵
- [x] [TASK-2071: 離散パレット・正規化関数・色計算関数](TASK-2071.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2070 → TASK-2071
```

---

## Phase 2: 状態管理

**目標**: AppStateにカラーマップ状態を追加し、Left Panelにセレクタを配置
**成果物**: app_state.rs 拡張、left_panel.rs 拡張

### タスク一覧

- [x] [TASK-2072: AppState拡張と初期色計算フロー](TASK-2072.md) - 3h (TDD) 🔵
- [x] [TASK-2073: Left Panel ColormapNameセレクタUI追加](TASK-2073.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-2071 → TASK-2072 → TASK-2073
```

---

## Phase 3: ウィジェット対応

**目標**: 全4ウィジェットがchart_colorsから色を取得するように変更
**成果物**: pareto_2d, scatter_matrix, cluster_scatter, pdp_2d の色対応

### タスク一覧

- [x] [TASK-2074: pareto_2dウィジェットのカラーマップ対応](TASK-2074.md) - 3h (TDD) 🔵
- [x] [TASK-2075: scatter_matrix・cluster_scatterウィジェット対応](TASK-2075.md) - 3h (TDD) 🔵
- [x] [TASK-2076: pdp_2dウィジェットのColormapName連動](TASK-2076.md) - 2h (TDD) 🔵

### 依存関係

```
TASK-2073 → TASK-2074
TASK-2073 → TASK-2075
TASK-2073 → TASK-2076
```

※ TASK-2074, 2075, 2076 は並行実行可能

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
| Phase 2 | 2 | 0 | 0 | 2 |
| Phase 3 | 3 | 0 | 0 | 3 |

**品質評価**: ✅ 高品質（全タスクが設計文書とユーザヒアリングに基づく）

## クリティカルパス

```
TASK-2070 → TASK-2071 → TASK-2072 → TASK-2073 → TASK-2074 (or 2075, 2076)
```

**クリティカルパス工数**: 15時間
**並行作業可能工数**: 5時間（TASK-2074, 2075, 2076 は並行可能）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement colormap-fix TASK-2070 TASK-2076`
- 特定タスクを実装: `/tsumiki:kairo-implement colormap-fix TASK-2070`
