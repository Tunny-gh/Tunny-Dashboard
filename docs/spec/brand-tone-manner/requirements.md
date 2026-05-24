# ブランドトンマナ統一 要件定義書

## 概要

Tunny ブランドのトーン＆マナー（カラーパレット・タイポグラフィ・UIスタイル・言語スタイル）を
Tunny Dashboard の全出力面（egui デスクトップ UI、ヘルプ HTML ブラウザ、HTML エクスポートレポート）に統一する。
`TONMANUAL.md` を唯一の真実の源（Single Source of Truth）とし、ブランドカラーに忠実にマッピングする。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **PRD（ブランドガイド）**: [TONMANUAL.md](../../../TONMANUAL.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: TONMANUAL・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: TONMANUAL・既存実装から妥当な推測による要件
- 🔴 **赤信号**: TONMANUAL・ユーザヒアリングにない推測による要件

---

### [SCOPE-1] egui テーマカラー統一

#### 通常要件（色値の更新）

- REQ-001: システムは `ACCENT_BLUE` を `#3B82F6`（TONMANUAL blue-500）に変更しなければならない 🔵 *TONMANUAL §2 プライマリカラー*
- REQ-002: システムは `ACCENT_BLUE_HOVER` を `#2563EB`（TONMANUAL blue-600）に変更しなければならない 🔵 *TONMANUAL §2 プライマリカラー*
- REQ-003: システムは `PANEL_BG` を `#F3F4F6`（TONMANUAL gray-100）に変更しなければならない 🔵 *TONMANUAL §2 カラー使用方針*
- REQ-004: システムは `CENTRAL_BG` を `#FFFFFF`（white）のまま維持しなければならない 🔵 *TONMANUAL §2 カラー使用方針*
- REQ-005: システムは `TEXT_PRIMARY` を `#111827`（TONMANUAL gray-900）に変更しなければならない 🔵 *TONMANUAL §2 見出しテキスト*
- REQ-006: システムは `TEXT_SECONDARY` を `#4B5563`（TONMANUAL gray-600）に変更しなければならない 🔵 *TONMANUAL §2 本文テキスト*
- REQ-007: システムは `BORDER_COLOR` を `#E5E7EB`（TONMANUAL gray-200）に変更しなければならない 🔵 *TONMANUAL §2 ボーダー・背景*
- REQ-008: システムは `TOOLBAR_BG` を `#BFDBFE`（TONMANUAL blue-200 ナビゲーション背景）に変更しなければならない 🔵 *TONMANUAL §4 ナビゲーションバー*
- REQ-009: システムは `TOOLBAR_TEXT` を `#374151`（TONMANUAL gray-700 相当）に変更しなければならない 🔵 *TONMANUAL §4 ナビゲーションバー*
- REQ-010: システムは `TOOLBAR_INPUT_BG` を `#F3F4F6`（TONMANUAL gray-100）に変更しなければならない 🔵 *TONMANUAL §2*
- REQ-011: システムは `TOOLBAR_INPUT_STROKE` を `#E5E7EB`（TONMANUAL gray-200）に変更しなければならない 🔵 *TONMANUAL §2*
- REQ-012: システムは `TOOLBAR_BTN_ACTIVE` を `#3B82F6`（TONMANUAL blue-500）に変更しなければならない 🔵 *TONMANUAL §4 ボタン プライマリCTA*

#### 新規カラー定数の追加

- REQ-013: システムは `HEADER_BG` 定数（`#93C5FD` = blue-300）を `ui_colors.rs` に追加しなければならない 🔵 *TONMANUAL §2 ヘッダー背景*
- REQ-014: システムは `ANNOUNCE_BG` 定数（`#60A5FA` = blue-400）を `ui_colors.rs` に追加しなければならない 🔵 *TONMANUAL §2 アナウンスバー背景*
- REQ-015: システムは `ACTION_GREEN` 定数（`#22C55E` = green-500）を `ui_colors.rs` に追加しなければならない 🔵 *TONMANUAL §2 アクション（購入など）*
- REQ-016: システムは `ACTION_GREEN_HOVER` 定数（`#16A34A` = green-600）を `ui_colors.rs` に追加しなければならない 🔵 *TONMANUAL §2 ホバー（購入）*
- REQ-017: システムは `TEXT_SUB` 定数（`#374151` = gray-700）を `ui_colors.rs` に追加しなければならない 🔵 *TONMANUAL §3 サブタイトル*
- REQ-018: システムは `ACCENT_BLUE_MUTED` を `#BFDBFE`（TONMANUAL blue-200）に変更しなければならない 🟡 *TONMANUAL §2 から妥当な推測（選択ハイライト用途）*

#### 制約要件

- REQ-019: `ERROR_COLOR`（`#EA4335`）はセマンティックカラーのため TONMANUAL の対象外とし変更しなければならない 🔵 *ユーザヒアリング（スコープ外確認）*

---

### [SCOPE-2] ヘルプ HTML スタイル統一

- REQ-101: システムは `build.rs` の `wrap_as_standalone_html` で生成する CSS の本文テキスト色を `#4B5563`（gray-600）に変更しなければならない 🔵 *TONMANUAL §3 本文*
- REQ-102: システムは見出し（h1/h2/h3）の色を `#111827`（gray-900）、フォントウェイトを `font-weight: 800`（font-extrabold 相当）に変更しなければならない 🔵 *TONMANUAL §3 タイポグラフィ*
- REQ-103: システムはボーダー色を `#E5E7EB`（gray-200）に変更しなければならない 🔵 *TONMANUAL §2*
- REQ-104: システムはコード・プレ背景を `#F3F4F6`（gray-100）に変更しなければならない 🔵 *TONMANUAL §2 ボーダー・背景*
- REQ-105: システムは `th` 要素の背景を `#F3F4F6`（gray-100）に変更しなければならない 🔵 *TONMANUAL §2*
- REQ-106: システムはリンク色を `#2563EB`（blue-600）とし、ホバー時にアンダーラインを表示しなければならない 🔵 *TONMANUAL §3 リンク*
- REQ-107: `body` の `max-width` は現在の `860px` を維持してよい 🟡 *TONMANUAL §5 ドキュメント幅 max-w-screen-lg から妥当な推測*

---

### [SCOPE-3] HTML エクスポートレポート スタイル統一

- REQ-201: システムは `html_report.rs` の CSS 内 `h1, h2` の色を `#111827`（gray-900）に変更しなければならない 🔵 *TONMANUAL §3 H1・H2*
- REQ-202: システムは `body` の基本テキスト色を `#4B5563`（gray-600）に変更しなければならない 🔵 *TONMANUAL §3 本文*
- REQ-203: システムは `th` 背景を `#F3F4F6`（gray-100）、ボーダーを `#E5E7EB`（gray-200）に変更しなければならない 🔵 *TONMANUAL §2*
- REQ-204: システムは `.card` の `border-radius` を `8px`（rounded-lg 相当）に変更しなければならない 🔵 *TONMANUAL §4 ボタン rounded-lg*
- REQ-205: システムは `.card` のボーダー色を `#E5E7EB`（gray-200）に変更しなければならない 🔵 *TONMANUAL §2*
- REQ-206: システムはレポートタイトル（H1）の上部に blue-300（`#93C5FD`）背景のブランドヘッダーバーを追加してよい 🟡 *TONMANUAL §4 ドキュメントヘッダーから妥当な推測*
- REQ-207: システムは散布図 SVG のパレット外 Pareto 点の色（現在 `#3498db`）を `#3B82F6`（blue-500）に変更しなければならない 🔵 *TONMANUAL §2 メインブルー*

---

## 非機能要件

### ビルド互換性

- NFR-001: HTML ヘルプファイルは `cargo build` でコンパイル時に自動再生成されなければならない 🔵 *CLAUDE.md build コマンドより*
- NFR-002: HTML レポートは外部リソースへの参照を持たないスタンドアロン HTML を維持しなければならない 🔵 *html_report.rs の既存仕様より*

### 視認性

- NFR-101: ツールバーが dark (#202124) から light (#BFDBFE) に変わるため、ツールバー内のすべてのテキスト・アイコンが light 背景上で WCAG AA 基準（コントラスト比 4.5:1 以上）を満たさなければならない 🟡 *TONMANUAL §1 対象読者考慮から妥当な推測*

### 言語スタイル

- NFR-201: システム内の英語テキスト（ボタンラベル・メッセージ等）は直接的・簡潔・技術的だが親しみやすい文体でなければならない 🔵 *TONMANUAL §1 ブランドトーン*

## Edge ケース

### ツールバーカラー変更に伴う影響

- EDGE-001: `TOOLBAR_BTN_HOVER`（現在 `#374151` = dark gray）は light 背景では濃すぎるため、light blue 系（`#BFDBFE` の 10% darken 等）に変更が必要な場合がある 🟡 *既存実装から妥当な推測*
- EDGE-002: `TOOLBAR_BTN_FG`（WHITE）は light 背景では視認性が低下するため、アクティブ状態では white を維持しつつ inactive 状態では dark テキストへの切り替えが必要な場合がある 🟡 *既存実装から妥当な推測*
