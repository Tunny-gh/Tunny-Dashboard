# brand-tone-manner タスク概要

**作成日**: 2026-05-25
**プロジェクト期間**: 2026-05-25 - 2026-05-27（3日）
**推定工数**: 18時間
**総タスク数**: 4件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/brand-tone-manner/requirements.md)
- **設計文書**: [📐 architecture.md](../design/brand-tone-manner/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/brand-tone-manner/dataflow.md)
- **実装ガイド**: [📐 implementation-guide.md](../design/brand-tone-manner/implementation-guide.md)
- **コンテキストノート**: [📝 note.md](../spec/brand-tone-manner/note.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | 1日 | ui_colors.rs TONMANUAL 準拠更新 | 1 | 6h | [TASK-2315](#phase-1-egui-テーマカラー更新) |
| Phase 2 | 1日 | ヘルプ HTML・エクスポートレポート CSS 更新 | 2 | 8h | [TASK-2316, TASK-2317](#phase-2-html-スタイル更新) |
| Phase 3 | 1日 | 統合確認・視認性検証 | 1 | 4h | [TASK-2318](#phase-3-統合確認) |

## タスク番号管理

**使用済みタスク番号**: TASK-2315 ~ TASK-2318
**次回開始番号**: TASK-2319

## 全体進捗

- [x] Phase 1: egui テーマカラー更新
- [x] Phase 2: HTML スタイル更新
- [x] Phase 3: 統合確認

## マイルストーン

- **M1: egui テーマ完成** (2026-05-25): ui_colors.rs の全定数を TONMANUAL 準拠に更新
- **M2: HTML スタイル完成** (2026-05-26): ヘルプ HTML・エクスポートレポートの CSS 更新
- **M3: 統合確認完了** (2026-05-27): 全出力面の視認性・コントラスト確認

---

## Phase 1: egui テーマカラー更新

**期間**: 1日（6時間）
**目標**: TONMANUAL §2 カラーパレットに準拠した egui テーマカラーへの更新
**成果物**: `egui-app/src/theme/ui_colors.rs` 完全更新版

### タスク一覧

- [x] [TASK-2315: egui-app/src/theme/ui_colors.rs を TONMANUAL 準拠に全面更新](TASK-2315.md) - 6h (DIRECT) 🔵 ✅ 完了 (2026-05-25)

### 変更定数サマリー

| 定数名 | 旧 HEX | 新 HEX | TONMANUAL |
|--------|--------|--------|-----------|
| `TOOLBAR_BG` | `#202124` | `#BFDBFE` | blue-200 |
| `TOOLBAR_TEXT` | `#E8EAED` | `#374151` | gray-700 |
| `TOOLBAR_BTN_HOVER` | `#374151` | `#DBEAFE` | blue-100 |
| `TOOLBAR_BTN_ACTIVE` | `#4285F4` | `#3B82F6` | blue-500 |
| `TOOLBAR_INPUT_BG` | `#303134` | `#F3F4F6` | gray-100 |
| `TOOLBAR_INPUT_STROKE` | `#5F6368` | `#E5E7EB` | gray-200 |
| `PANEL_BG` | `#F0F2F5` | `#F3F4F6` | gray-100 |
| `ACCENT_BLUE` | `#4285F4` | `#3B82F6` | blue-500 |
| `ACCENT_BLUE_HOVER` | `#3367D6` | `#2563EB` | blue-600 |
| `ACCENT_BLUE_MUTED` | `#E8F0FE` | `#BFDBFE` | blue-200 |
| `TEXT_PRIMARY` | `#202124` | `#111827` | gray-900 |
| `TEXT_SECONDARY` | `#5F6368` | `#4B5563` | gray-600 |
| `BORDER_COLOR` | `#DADCE0` | `#E5E7EB` | gray-200 |

**新規追加**: `HEADER_BG`, `ANNOUNCE_BG`, `ACTION_GREEN`, `ACTION_GREEN_HOVER`, `TEXT_SUB`

### 依存関係

```
（前提なし）TASK-2315
```

---

## Phase 2: HTML スタイル更新

**期間**: 1日（8時間）
**目標**: ヘルプ HTML・HTML エクスポートレポートの CSS を TONMANUAL 準拠に更新
**成果物**: `build.rs`・`html_report.rs` の CSS 更新版

### タスク一覧

- [x] [TASK-2316: egui-app/build.rs ヘルプ HTML CSS を TONMANUAL 準拠に更新](TASK-2316.md) - 4h (DIRECT) 🔵 ✅ 完了 (2026-05-25)
- [x] [TASK-2317: egui-app/src/io/html_report.rs CSS・SVG 色を TONMANUAL 準拠に更新](TASK-2317.md) - 4h (DIRECT) 🔵 ✅ 完了 (2026-05-25)

### 依存関係

```
（並行実施可能）
TASK-2316 ─┐
            ├→ TASK-2318
TASK-2317 ─┘
```

TASK-2316 と TASK-2317 は互いに独立しており並行実施可能。

---

## Phase 3: 統合確認

**期間**: 1日（4時間）
**目標**: 全出力面（egui UI・ヘルプ HTML・HTML レポート）の統合確認と WCAG AA 視認性検証
**成果物**: 品質確認済みブランドカラー統一

### タスク一覧

- [x] [TASK-2318: ブランドカラー統一の統合確認と視認性検証](TASK-2318.md) - 4h (DIRECT) 🔵 ✅ 完了 (2026-05-25)

### 依存関係

```
TASK-2315 ─┐
TASK-2316 ─┼→ TASK-2318
TASK-2317 ─┘
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 4件
- 🔵 **青信号**: 4件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 1 | 0 | 0 | 1 |
| Phase 2 | 2 | 0 | 0 | 2 |
| Phase 3 | 1 | 0 | 0 | 1 |

**品質評価**: 高品質

## クリティカルパス

```
TASK-2315 → TASK-2318
TASK-2316 → TASK-2318
TASK-2317 → TASK-2318
```

**クリティカルパス工数**: 10時間（Phase 1 + Phase 3）
**並行作業可能工数**: 8時間（Phase 2 の TASK-2316 と TASK-2317）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2315`
