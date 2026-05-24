# TASK-2317 設定作業実行

## 作業概要

- **タスクID**: TASK-2317
- **作業内容**: egui-app/src/io/html_report.rs の CSS とSVG 色定数を TONMANUAL 準拠に更新
- **実行日時**: 2026-05-25
- **実行者**: Claude

## 設計文書参照

- **参照文書**: docs/design/brand-tone-manner/implementation-guide.md §3
- **関連要件**: REQ-201〜REQ-207

## 実行した作業

### CSS 変更内容 (build_html_report 内)

| プロパティ | 旧値 | 新値 | TONMANUAL |
|-----------|------|------|-----------|
| `body color` | (未指定) | `#4B5563` | gray-600 |
| `th,td border` | `#ccc` | `#E5E7EB` | gray-200 |
| `th background` | `#f0f0f0` | `#F3F4F6` | gray-100 |
| `th color` (新規) | — | `#111827` | gray-900 |
| `h1,h2 color` | `#333` | `#111827` | gray-900 |
| `h1,h2 font-weight` (新規) | — | `800` | font-extrabold |
| `h1,h2 letter-spacing` (新規) | — | `-0.025em` | tracking-tight |
| `.card border` | `#ddd` | `#E5E7EB` | gray-200 |
| `.card border-radius` | `4px` | `8px` | rounded-lg |

### SVG 色変更

| 対象 | 旧値 | 新値 | 根拠 |
|------|------|------|------|
| Pareto 外点 (pareto_rank != 0) | `#3498db` | `#3B82F6` | TONMANUAL blue-500 |
| Pareto 前沿点 (pareto_rank == 0) | `#e74c3c` | `#e74c3c` (変更なし) | セマンティックカラー |

## 作業結果

- [x] body color を gray-600 (#4B5563) に変更
- [x] テーブルボーダーを gray-200 (#E5E7EB) に変更
- [x] th 背景を gray-100 (#F3F4F6) に変更
- [x] h1, h2 を gray-900 + extrabold + tracking-tight に変更
- [x] .card の border を gray-200, border-radius を 8px に変更
- [x] SVG Pareto 外点色を #3498db → #3B82F6 (blue-500) に変更
- [x] 外部リソース参照なし（スタンドアロン HTML）を維持
