# TASK-2316 設定作業実行

## 作業概要

- **タスクID**: TASK-2316
- **作業内容**: egui-app/build.rs の wrap_as_standalone_html() 内 CSS を TONMANUAL 準拠に更新
- **実行日時**: 2026-05-25
- **実行者**: Claude

## 設計文書参照

- **参照文書**: docs/design/brand-tone-manner/implementation-guide.md §2
- **関連要件**: REQ-101〜REQ-107

## 実行した作業

### CSS 変更内容

| プロパティ | 旧値 | 新値 | TONMANUAL |
|-----------|------|------|-----------|
| `body color` | `#24292f` | `#4B5563` | gray-600 |
| `h1,h2,h3 font-weight` | `600` | `800` | font-extrabold |
| `h1,h2,h3 color` | (未指定) | `#111827` | gray-900 |
| `h1,h2,h3 letter-spacing` | (未指定) | `-0.025em` | tracking-tight |
| `h1,h2 border-bottom color` | `#d0d7de` | `#E5E7EB` | gray-200 |
| `a` (新規追加) | — | `color: #2563EB` | blue-600 |
| `a:hover` (新規追加) | — | `text-decoration: underline` | — |
| `code/pre background` | `#f6f8fa` | `#F3F4F6` | gray-100 |
| `th,td border` | `#d0d7de` | `#E5E7EB` | gray-200 |
| `th background` | `#f6f8fa` | `#F3F4F6` | gray-100 |
| `th color` (新規追加) | — | `#111827` | gray-900 |

### KaTeX CSS 配置

`{katex_css}` プレースホルダーは `<style>` 末尾に維持（ブランド CSS を上書きしない）。

## 作業結果

- [x] body color を gray-600 (#4B5563) に変更
- [x] 見出しを gray-900 + extrabold + tracking-tight に変更
- [x] ボーダー・背景を gray-200/gray-100 に統一
- [x] リンク色を blue-600 (#2563EB) に追加
- [x] {katex_css} を <style> 末尾に維持
