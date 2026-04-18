# ui-color-theming タスク概要

**作成日**: 2026-04-18
**プロジェクト期間**: 2026-04-18（1日）
**推定工数**: 7時間
**総タスク数**: 4件

## 関連文書

- **設計文書**: [📐 architecture.md](../design/ui-color-theming/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/ui-color-theming/dataflow.md)
- **型定義**: [📝 interfaces.rs](../design/ui-color-theming/interfaces.rs)
- **ヒアリング記録**: [📝 design-interview.md](../design/ui-color-theming/design-interview.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | 1日 | ライトテーマ適用済みeguiアプリ | 4 | 7h | TASK-0001〜0004 |

## タスク番号管理

**使用済みタスク番号**: TASK-0001 〜 TASK-0004
**次回開始番号**: TASK-0005

## 全体進捗

- [ ] Phase 1: UIカラーテーマ実装

## マイルストーン

- **M1: テーマ実装完了** (2026-04-18): theme.rs 作成・全ファイル適用完了・目視確認完了

---

## Phase 1: UIカラーテーマ実装

**期間**: 1日
**目標**: egui-appのUIにライトテーマを適用し、添付スクリーンショットの雰囲気を再現する
**成果物**: 
- `egui-app/src/theme.rs`（新規）
- `egui-app/src/app.rs`（変更）
- `egui-app/src/ui/layout.rs`（変更）
- `egui-app/src/ui/grid_canvas.rs`（変更）

### タスク一覧

- [ ] [TASK-0001: theme.rs 新規作成（カラーパレット定数・Visuals構築）](TASK-0001.md) - 2h (DIRECT) 🔵
- [ ] [TASK-0002: グローバルテーマ適用（app.rs + layout.rs）](TASK-0002.md) - 2h (TDD) 🔵
- [ ] [TASK-0003: grid_canvas.rs カラー定数置き換え](TASK-0003.md) - 2h (TDD) 🔵
- [ ] [TASK-0004: ビジュアル動作確認・最終検証](TASK-0004.md) - 1h (DIRECT) 🟡

### 依存関係

```
TASK-0001 → TASK-0002
TASK-0001 → TASK-0003
TASK-0002 → TASK-0004
TASK-0003 → TASK-0004
```

### 並行実行可能なタスク

TASK-0002 と TASK-0003 は TASK-0001 完了後に並行実行可能。

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 4件
- 🔵 **青信号**: 3件 (75%)
- 🟡 **黄信号**: 1件 (25%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 1 | 0 | 4 |

**品質評価**: ✅ 高品質（TASK-0004のビジュアル確認のみ実行後判断）

## クリティカルパス

```
TASK-0001 → TASK-0002 → TASK-0004
            TASK-0003 ↗
```

**クリティカルパス工数**: 5時間（TASK-0001 → TASK-0002 → TASK-0004）
**並行作業削減**: TASK-0003 を TASK-0002 と並行実行することで最大2時間短縮可能

## カラーパレット早見表

| 用途 | 定数名 | 色 | Hex |
|---|---|---|---|
| ツールバー背景 | `TOOLBAR_BG` | ダークネイビー | #1a2332 |
| ツールバーテキスト | `TOOLBAR_TEXT` | 明るいグレー | #dce6f5 |
| パネル背景 | `PANEL_BG` | ライトグレー | #f5f7fa |
| キャンバス背景 | `CENTRAL_BG` | 白 | #ffffff |
| アクセント | `ACCENT_BLUE` | ブルー | #2563eb |
| 境界線 | `BORDER_COLOR` | ライトグレー | #cbd5e1 |
| メインテキスト | `TEXT_PRIMARY` | ダーク | #1e293b |

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-0001`
