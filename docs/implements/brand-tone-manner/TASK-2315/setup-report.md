# TASK-2315 設定作業実行

## 作業概要

- **タスクID**: TASK-2315
- **作業内容**: egui-app/src/theme/ui_colors.rs を TONMANUAL §2 カラーパレット準拠に全面更新
- **実行日時**: 2026-05-25
- **実行者**: Claude

## 設計文書参照

- **参照文書**: docs/design/brand-tone-manner/implementation-guide.md §1
- **関連要件**: REQ-001〜REQ-019

## 実行した作業

### 1. 変更した定数（13件）

| 定数名 | 旧値 | 新値 | TONMANUAL |
|--------|------|------|-----------|
| `TOOLBAR_BG` | `from_rgb(32,33,36)` | `from_rgb(191,219,254)` | blue-200 |
| `TOOLBAR_TEXT` | `from_rgb(232,234,237)` | `from_rgb(55,65,81)` | gray-700 |
| `TOOLBAR_BTN_HOVER` | `from_rgb(55,65,81)` | `from_rgb(219,234,254)` | blue-100 |
| `TOOLBAR_BTN_ACTIVE` | `from_rgb(66,133,244)` | `from_rgb(59,130,246)` | blue-500 |
| `TOOLBAR_INPUT_BG` | `from_rgb(48,49,52)` | `from_rgb(243,244,246)` | gray-100 |
| `TOOLBAR_INPUT_STROKE` | `from_rgb(95,99,104)` | `from_rgb(229,231,235)` | gray-200 |
| `PANEL_BG` | `from_rgb(240,242,245)` | `from_rgb(243,244,246)` | gray-100 |
| `CELL_TOOLBAR_BG` | `from_rgb(245,247,250)` | `from_rgb(243,244,246)` | gray-100 |
| `WIDGET_BG` | `from_rgb(240,244,248)` | `from_rgb(243,244,246)` | gray-100 |
| `WIDGET_BG_HOVER` | `from_rgb(232,236,242)` | `from_rgb(229,231,235)` | gray-200 |
| `ACCENT_BLUE` | `from_rgb(66,133,244)` | `from_rgb(59,130,246)` | blue-500 |
| `ACCENT_BLUE_HOVER` | `from_rgb(51,103,214)` | `from_rgb(37,99,235)` | blue-600 |
| `ACCENT_BLUE_MUTED` | `from_rgb(232,240,254)` | `from_rgb(191,219,254)` | blue-200 |
| `TEXT_PRIMARY` | `from_rgb(32,33,36)` | `from_rgb(17,24,39)` | gray-900 |
| `TEXT_SECONDARY` | `from_rgb(95,99,104)` | `from_rgb(75,85,99)` | gray-600 |
| `BORDER_COLOR` | `from_rgb(218,220,224)` | `from_rgb(229,231,235)` | gray-200 |

### 2. 追加した定数（5件）

- `HEADER_BG`: `from_rgb(147,197,253)` — blue-300 (#93C5FD)
- `ANNOUNCE_BG`: `from_rgb(96,165,250)` — blue-400 (#60A5FA)
- `ACTION_GREEN`: `from_rgb(34,197,94)` — green-500 (#22C55E)
- `ACTION_GREEN_HOVER`: `from_rgb(22,163,74)` — green-600 (#16A34A)
- `TEXT_SUB`: `from_rgb(55,65,81)` — gray-700 (#374151)

### 3. 変更しなかった定数

- `TOOLBAR_BTN_FG`: `Color32::WHITE` (変更なし)
- `CENTRAL_BG`: `Color32::WHITE` (変更なし)
- `CLOSE_BTN_TEXT`: `Color32::from_gray(180)` (変更なし)
- `ERROR_COLOR`: `from_rgb(234,67,53)` (セマンティックカラー、変更なし)

## 作業結果

- [x] 既存定数の HEX 値変更完了（13件）
- [x] 新規定数の追加完了（5件）
- [x] `#[allow(dead_code)]` を未使用定数に付与
- [x] ファイル全体を TONMANUAL §2 準拠に書き換え完了

## 次のステップ

- `direct-verify` を実行して `cargo build --workspace` でエラーがないことを確認
