# ブランドトンマナ統一 コンテキストノート

## 技術スタック

| レイヤー | 技術 |
|---------|------|
| UI フレームワーク | Rust + egui / eframe |
| テーマ管理 | `egui-app/src/theme/ui_colors.rs`（Color32 定数） |
| ヘルプ HTML 生成 | `egui-app/build.rs`（pulldown-cmark + KaTeX） |
| HTML エクスポート | `egui-app/src/io/html_report.rs` |
| ブランドガイド | `TONMANUAL.md`（Twind CSS ベース） |

## 変更対象ファイル

| ファイル | 役割 | 変更内容 |
|---------|------|---------|
| `egui-app/src/theme/ui_colors.rs` | egui カラー定数 | HEX 値を TONMANUAL に揃える |
| `egui-app/src/theme/mod.rs` | visuals 適用 | カラー定数変更に追従 |
| `egui-app/build.rs` | ヘルプ HTML 生成 | `wrap_as_standalone_html` の CSS を TONMANUAL に揃える |
| `egui-app/src/io/html_report.rs` | HTML レポート生成 | CSS / レイアウトを TONMANUAL に揃える |

## TONMANUAL → egui カラーマッピング表

| TONMANUAL 役割 | Twind クラス | HEX | egui 定数（現在値） | 現在の HEX |
|--------------|------------|-----|-----------------|-----------|
| メインブルー（CTA） | blue-500 | `#3B82F6` | `ACCENT_BLUE` | `#4285F4` |
| ホバーブルー | blue-600 | `#2563EB` | `ACCENT_BLUE_HOVER` | `#3367D6` |
| ヘッダー背景 | blue-300 | `#93C5FD` | （未定義） | — |
| ナビゲーション背景 | blue-200 | `#BFDBFE` | `TOOLBAR_BG` | `#202124` |
| アナウンスバー | blue-400 | `#60A5FA` | （未定義） | — |
| アクション（購入等） | green-500 | `#22C55E` | （未定義） | — |
| ホバー（購入） | green-600 | `#16A34A` | （未定義） | — |
| 本文テキスト | gray-600 | `#4B5563` | `TEXT_SECONDARY` | `#5F6368` |
| 見出しテキスト | gray-900 | `#111827` | `TEXT_PRIMARY` | `#202124` |
| サブテキスト | gray-700 | `#374151` | （未定義） | — |
| ボーダー | gray-200 | `#E5E7EB` | `BORDER_COLOR` | `#DADCE0` |
| パネル背景 | gray-100 | `#F3F4F6` | `PANEL_BG` | `#F0F2F5` |
| ページ背景 | white | `#FFFFFF` | `CENTRAL_BG` | `#FFFFFF` |

## 注意事項

- TOOLBAR_BG の変更（#202124 → #BFDBFE）は dark→light への大きなデザイン変更
  - TOOLBAR_TEXT も対応する必要あり（現在 #E8EAED → gray-900 #111827 相当）
  - TOOLBAR_INPUT_BG / TOOLBAR_INPUT_STROKE も light 系に変更必要
- egui の Color32 は HEX 値を `from_rgb(r, g, b)` で指定
- ヘルプ HTML は build.rs でコンパイル時に生成（変更後は `cargo build` が必要）
- HTML レポートは外部リソース参照なし（スタンドアロン）制約あり
