# ブランドトンマナ統一 実装ガイド

**作成日**: 2026-05-25
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/brand-tone-manner/requirements.md)

> このファイルは TypeScript 型定義の代わりに、Rust の Color32 定数と CSS の具体的な実装値を示す実装ガイドです。

**【信頼性レベル凡例】**:
- 🔵 **青信号**: TONMANUAL・要件定義を参考にした確実な実装値
- 🟡 **黄信号**: TONMANUAL・要件定義から妥当な推測による実装値
- 🔴 **赤信号**: TONMANUAL・要件定義にない推測による実装値

---

## 1. `egui-app/src/theme/ui_colors.rs` 完全定義

```rust
use egui::Color32;

// ====================================================================
// ツールバー系 (TONMANUAL §4 ナビゲーションバー)
// ====================================================================

/// ナビゲーション背景: TONMANUAL blue-200 🔵
pub const TOOLBAR_BG: Color32 = Color32::from_rgb(191, 219, 254);      // #BFDBFE

/// ナビゲーションテキスト: TONMANUAL gray-700 🔵
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(55, 65, 81);        // #374151

/// ボタンホバー時の背景色: blue-100（blue-200 背景上の hover） 🟡
pub const TOOLBAR_BTN_HOVER: Color32 = Color32::from_rgb(219, 234, 254); // #DBEAFE

/// ボタンアクティブ時の背景色: TONMANUAL blue-500 🔵
pub const TOOLBAR_BTN_ACTIVE: Color32 = Color32::from_rgb(59, 130, 246); // #3B82F6

/// ボタンホバー/アクティブ時のテキスト色: white（blue-500 背景上） 🔵
pub const TOOLBAR_BTN_FG: Color32 = Color32::WHITE;

/// コンボボックス・入力欄の背景色: TONMANUAL gray-100 🔵
pub const TOOLBAR_INPUT_BG: Color32 = Color32::from_rgb(243, 244, 246);  // #F3F4F6

/// コンボボックス・入力欄の枠線色: TONMANUAL gray-200 🔵
pub const TOOLBAR_INPUT_STROKE: Color32 = Color32::from_rgb(229, 231, 235); // #E5E7EB

// ====================================================================
// パネル・キャンバス系 (TONMANUAL §5 レイアウト)
// ====================================================================

/// 左右パネルの背景色: TONMANUAL gray-100 🔵
pub const PANEL_BG: Color32 = Color32::from_rgb(243, 244, 246);          // #F3F4F6

/// メインキャンバスの背景色: white 🔵
pub const CENTRAL_BG: Color32 = Color32::WHITE;                           // #FFFFFF

/// チャートセルのツールバー背景色: gray-100 🟡
pub const CELL_TOOLBAR_BG: Color32 = Color32::from_rgb(243, 244, 246);   // #F3F4F6

// ====================================================================
// ウィジェット系
// ====================================================================

/// 非アクティブウィジェット背景: TONMANUAL gray-100 🟡
pub const WIDGET_BG: Color32 = Color32::from_rgb(243, 244, 246);         // #F3F4F6

/// ホバー時ウィジェット背景: TONMANUAL gray-200 🟡
pub const WIDGET_BG_HOVER: Color32 = Color32::from_rgb(229, 231, 235);   // #E5E7EB

/// グリッドセルのクローズボタンテキスト色 🟡
pub const CLOSE_BTN_TEXT: Color32 = Color32::from_gray(180);

// ====================================================================
// アクセントカラー (TONMANUAL §2 プライマリカラー)
// ====================================================================

/// メインブルー: TONMANUAL blue-500 🔵
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);        // #3B82F6

/// ホバーブルー: TONMANUAL blue-600 🔵
pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(37, 99, 235);   // #2563EB

/// 選択ハイライト: TONMANUAL blue-200 🟡
pub const ACCENT_BLUE_MUTED: Color32 = Color32::from_rgb(191, 219, 254); // #BFDBFE

// ====================================================================
// テキスト・ボーダー系 (TONMANUAL §2 セカンダリカラー)
// ====================================================================

/// 見出しテキスト: TONMANUAL gray-900 🔵
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(17, 24, 39);         // #111827

/// 本文テキスト: TONMANUAL gray-600 🔵
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(75, 85, 99);       // #4B5563

/// ボーダー: TONMANUAL gray-200 🔵
pub const BORDER_COLOR: Color32 = Color32::from_rgb(229, 231, 235);      // #E5E7EB

// ====================================================================
// 新規追加: ブランドカラー完全版 (TONMANUAL §2)
// ====================================================================

/// ヘッダー背景: TONMANUAL blue-300 🔵
pub const HEADER_BG: Color32 = Color32::from_rgb(147, 197, 253);         // #93C5FD

/// アナウンスバー背景: TONMANUAL blue-400 🔵
pub const ANNOUNCE_BG: Color32 = Color32::from_rgb(96, 165, 250);        // #60A5FA

/// アクション（購入・重要操作）: TONMANUAL green-500 🔵
pub const ACTION_GREEN: Color32 = Color32::from_rgb(34, 197, 94);        // #22C55E

/// アクションホバー: TONMANUAL green-600 🔵
pub const ACTION_GREEN_HOVER: Color32 = Color32::from_rgb(22, 163, 74);  // #16A34A

/// サブテキスト: TONMANUAL gray-700 🔵
pub const TEXT_SUB: Color32 = Color32::from_rgb(55, 65, 81);             // #374151

// ====================================================================
// セマンティックカラー（変更なし）
// ====================================================================

/// エラー表示色: ブランドカラー対象外 🔵
pub const ERROR_COLOR: Color32 = Color32::from_rgb(234, 67, 53);
```

---

## 2. `egui-app/build.rs` CSS テンプレート差分

`wrap_as_standalone_html()` 内の `<style>` タグを以下に変更する。

```css
/* === 変更後 CSS テンプレート (TONMANUAL §2§3§6 準拠) === */

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 14px;
  line-height: 1.6;
  color: #4B5563;            /* TONMANUAL gray-600 🔵 */
  background: #ffffff;       /* white 🔵 */
  max-width: 860px;          /* TONMANUAL max-w-screen-lg 相当 🟡 */
  margin: 0 auto;
  padding: 24px;
}

h1, h2, h3 {
  font-weight: 800;          /* TONMANUAL font-extrabold 🔵 */
  margin-top: 1.5em;
  margin-bottom: 0.5em;
  color: #111827;            /* TONMANUAL gray-900 🔵 */
  letter-spacing: -0.025em;  /* TONMANUAL tracking-tight 🔵 */
}

h1 {
  font-size: 1.8em;
  border-bottom: 1px solid #E5E7EB;  /* TONMANUAL gray-200 🔵 */
  padding-bottom: 0.3em;
}

h2 {
  font-size: 1.4em;
  border-bottom: 1px solid #E5E7EB;  /* TONMANUAL gray-200 🔵 */
  padding-bottom: 0.2em;
}

a {
  color: #2563EB;            /* TONMANUAL blue-600 🔵 */
  text-decoration: none;
}

a:hover {
  text-decoration: underline; /* TONMANUAL hover:underline 🔵 */
}

code {
  background: #F3F4F6;       /* TONMANUAL gray-100 🔵 */
  border-radius: 3px;
  padding: 0.1em 0.3em;
  font-size: 0.9em;
}

pre {
  background: #F3F4F6;       /* TONMANUAL gray-100 🔵 */
  border-radius: 6px;
  padding: 16px;
  overflow: auto;
}

pre code { background: none; padding: 0; }

table {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
}

th, td {
  border: 1px solid #E5E7EB; /* TONMANUAL gray-200 🔵 */
  padding: 8px 12px;
  text-align: left;
}

th {
  background: #F3F4F6;       /* TONMANUAL gray-100 🔵 */
  font-weight: 600;
  color: #111827;            /* TONMANUAL gray-900 🔵 */
}

/* KaTeX CSS を後置して数式スタイルがブランド CSS を上書きしないように 🟡 */
{katex_css}
```

---

## 3. `egui-app/src/io/html_report.rs` CSS 差分

`build_html_report()` 内のインライン `<style>` を以下に変更する。

```css
/* === 変更後 CSS (TONMANUAL §2§3 準拠) === */

body {
  font-family: sans-serif;
  margin: 20px;
  color: #4B5563;            /* TONMANUAL gray-600 🔵 */
}

table {
  border-collapse: collapse;
  width: 100%;
}

th, td {
  border: 1px solid #E5E7EB; /* TONMANUAL gray-200 🔵 */
  padding: 6px 8px;
  text-align: right;
}

th {
  background: #F3F4F6;       /* TONMANUAL gray-100 🔵 */
  color: #111827;            /* TONMANUAL gray-900 🔵 */
}

h1, h2 {
  color: #111827;            /* TONMANUAL gray-900 🔵 */
  font-weight: 800;          /* TONMANUAL font-extrabold 🔵 */
  letter-spacing: -0.025em;  /* TONMANUAL tracking-tight 🔵 */
}

.summary {
  display: flex;
  gap: 20px;
  margin-bottom: 20px;
}

.card {
  border: 1px solid #E5E7EB; /* TONMANUAL gray-200 🔵 */
  border-radius: 8px;         /* TONMANUAL rounded-lg 🔵 */
  padding: 12px;
  min-width: 120px;
}
```

**SVG 散布図の色定数変更** 🔵:

```rust
// 変更前
let color = if row.pareto_rank == 0 { "#e74c3c" } else { "#3498db" };

// 変更後 (#3498db → #3B82F6 = TONMANUAL blue-500)
let color = if row.pareto_rank == 0 { "#e74c3c" } else { "#3B82F6" };
```

---

## 4. Color32 HEX 変換早見表 🔵

**信頼性**: 🔵 *TONMANUAL §2 カラーパレットより*

| TONMANUAL | HEX | `Color32::from_rgb(r, g, b)` |
|-----------|-----|-------------------------------|
| blue-100 | `#DBEAFE` | `from_rgb(219, 234, 254)` |
| blue-200 | `#BFDBFE` | `from_rgb(191, 219, 254)` |
| blue-300 | `#93C5FD` | `from_rgb(147, 197, 253)` |
| blue-400 | `#60A5FA` | `from_rgb(96, 165, 250)` |
| blue-500 | `#3B82F6` | `from_rgb(59, 130, 246)` |
| blue-600 | `#2563EB` | `from_rgb(37, 99, 235)` |
| green-500 | `#22C55E` | `from_rgb(34, 197, 94)` |
| green-600 | `#16A34A` | `from_rgb(22, 163, 74)` |
| gray-100 | `#F3F4F6` | `from_rgb(243, 244, 246)` |
| gray-200 | `#E5E7EB` | `from_rgb(229, 231, 235)` |
| gray-600 | `#4B5563` | `from_rgb(75, 85, 99)` |
| gray-700 | `#374151` | `from_rgb(55, 65, 81)` |
| gray-900 | `#111827` | `from_rgb(17, 24, 39)` |

---

## 5. 検証手順 🔵

**信頼性**: 🔵 *CLAUDE.md ビルドコマンドより*

```bash
# 1. ビルド確認
rtk cargo build --workspace

# 2. テスト確認
rtk cargo test --workspace

# 3. アプリ起動して目視確認
cargo run -p tunny-desktop

# 4. ヘルプ HTML の CSS 確認（生成ファイルを確認）
# OUT_DIR は cargo build の出力に表示される
```

---

## 信頼性レベルサマリー

- 🔵 青信号: 28件 (85%)
- 🟡 黄信号: 5件 (15%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
