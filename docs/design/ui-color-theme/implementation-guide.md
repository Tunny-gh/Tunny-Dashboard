# UIカラー設定一元化 実装ガイド（Rust色定数定義）

**作成日**: 2026-05-07
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/ui-color-theme/requirements.md)

> Rust プロジェクトのため `interfaces.ts` の代わりに本ファイルを使用する。

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存実装・ユーザヒアリングを参考にした確実な定義
- 🟡 **黄信号**: 既存実装から妥当な推測による定義
- 🔴 **赤信号**: ヒアリング・実装にない推測による定義

---

## `egui-app/src/theme/mod.rs` 🔵

**信頼性**: 🔵 *既存 theme.rs の内容そのままを移行 + ERROR_COLOR 追加より*

```rust
use egui::{Color32, Stroke, Visuals};

// ====================================================================
// UI テーマ色
// （既存 theme.rs の内容をそのまま移行）
// ====================================================================

pub const TOOLBAR_BG: Color32 = Color32::from_rgb(26, 35, 50);
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(220, 230, 245);
pub const PANEL_BG: Color32 = Color32::from_rgb(225, 233, 248);
pub const CENTRAL_BG: Color32 = Color32::WHITE;
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(37, 99, 235);
#[allow(dead_code)]
pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(29, 78, 216);
pub const ACCENT_BLUE_MUTED: Color32 = Color32::from_rgb(219, 234, 254);
pub const BORDER_COLOR: Color32 = Color32::from_rgb(203, 213, 225);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 41, 59);
#[allow(dead_code)]
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 116, 139);
pub const CELL_TOOLBAR_BG: Color32 = Color32::from_rgb(232, 239, 251);
pub const WIDGET_BG: Color32 = Color32::from_rgb(235, 241, 252);
pub const WIDGET_BG_HOVER: Color32 = Color32::from_rgb(220, 230, 247);
pub const TOOLBAR_BTN_HOVER: Color32 = Color32::from_rgb(55, 78, 120);
pub const TOOLBAR_BTN_ACTIVE: Color32 = Color32::from_rgb(37, 99, 235);
pub const TOOLBAR_INPUT_BG: Color32 = Color32::from_rgb(45, 62, 90);
pub const TOOLBAR_INPUT_STROKE: Color32 = Color32::from_rgb(100, 130, 180);

// ====================================================================
// セマンティック色（新規追加）
// ====================================================================

/// エラー表示に使用する赤色。Color32::RED より落ち着いた赤。
pub const ERROR_COLOR: Color32 = Color32::from_rgb(220, 50, 50); // 🔵

// ====================================================================
// テーマ関数
// ====================================================================

pub fn tunny_light_visuals() -> Visuals {
    let mut v = Visuals::light();

    v.panel_fill = PANEL_BG;
    v.window_fill = CENTRAL_BG;
    v.window_stroke = Stroke::new(1.0, BORDER_COLOR);
    v.override_text_color = Some(TEXT_PRIMARY);
    v.extreme_bg_color = Color32::WHITE;

    v.widgets.active.bg_fill = ACCENT_BLUE;
    v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_BLUE);

    v.widgets.hovered.bg_fill = WIDGET_BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_BLUE);

    v.widgets.inactive.bg_fill = WIDGET_BG;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_COLOR);

    v.widgets.noninteractive.bg_fill = PANEL_BG;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_COLOR);

    v.selection.bg_fill = ACCENT_BLUE_MUTED;
    v.selection.stroke = Stroke::new(1.0, ACCENT_BLUE);

    v
}

// サブモジュールの公開
pub mod colormap;
pub mod chart_colors;
```

---

## `egui-app/src/theme/colormap.rs` 🔵

**信頼性**: 🔵 *既存 render/colormap.rs の内容をそのまま移行より*

> **移行方法**: `egui-app/src/render/colormap.rs` の内容を
> `egui-app/src/theme/colormap.rs` にコピーする。
> 内容の変更は不要（インポートパスはクレート内相対なので変わらない）。

```rust
// ファイル内容は render/colormap.rs と同一。
// crate::state::app_state への参照もそのまま維持する。

// 変更が必要な箇所はなし。
// ただし #[cfg(test)] 内の以下の行を確認:
//   use crate::render::colormap::ColorMap;
// → render/colormap.rsが削除された後も、このファイルが
//   theme/colormap.rs なので self のシンボルを直接使える。
//   テスト内の use 文は削除するか、
//   use crate::theme::colormap::ColorMap; に変更する。
```

**注意**: テスト内の `use crate::render::colormap::ColorMap;`（`state/types.rs` の `#[cfg(test)]` ブロック内）は `use crate::theme::colormap::ColorMap;` に更新が必要。

---

## `egui-app/src/theme/chart_colors.rs` 🔵

**信頼性**: 🔵 *既存コードの色値をすべて抽出・命名より*

```rust
use egui::Color32;

// ====================================================================
// Pareto 系色
// ====================================================================

/// Pareto 最適解の色（赤）
/// 旧: pareto_2d.rs, slice_chart.rs の COLOR_PARETO
pub const COLOR_PARETO: Color32 = Color32::from_rgb(220, 50, 50); // 🔵

/// 非 Pareto 点の色（青）
/// 旧: pareto_2d.rs, slice_chart.rs の COLOR_NON_PARETO
pub const COLOR_NON_PARETO: Color32 = Color32::from_rgb(50, 150, 250); // 🔵

/// 非選択時の Pareto 点（薄い赤、プリマルチプライド）
/// 旧: pareto_2d.rs の COLOR_PARETO_DIM (from_rgba_premultiplied(220,50,50,60))
pub const COLOR_PARETO_DIM: Color32 = Color32::from_rgba_premultiplied(220, 50, 50, 60); // 🔵

/// 非選択時の非 Pareto 点（薄い青、プリマルチプライド）
/// 旧: pareto_2d.rs の COLOR_NON_PARETO_DIM (from_rgba_premultiplied(50,150,250,60))
pub const COLOR_NON_PARETO_DIM: Color32 = Color32::from_rgba_premultiplied(50, 150, 250, 60); // 🔵

// ====================================================================
// 3D軸色
// ====================================================================

/// 3D チャート X 軸色（赤）
/// 旧: pareto_3d.rs のインライン from_rgb(220, 80, 80)
pub const COLOR_AXIS_X: Color32 = Color32::from_rgb(220, 80, 80); // 🔵

/// 3D チャート Y 軸色（緑）
/// 旧: pareto_3d.rs のインライン from_rgb(80, 220, 80)
pub const COLOR_AXIS_Y: Color32 = Color32::from_rgb(80, 220, 80); // 🔵

/// 3D チャート Z 軸色（青）
/// 旧: pareto_3d.rs のインライン from_rgb(80, 80, 220)
pub const COLOR_AXIS_Z: Color32 = Color32::from_rgb(80, 80, 220); // 🔵

// ====================================================================
// MCDM スコア段階色
// ====================================================================

/// MCDM スコア高（赤）— mcdm_scatter_chart の上位スコア
/// 旧: mcdm_scatter_chart.rs の COLOR_RED
pub const COLOR_MCDM_HIGH: Color32 = Color32::from_rgb(255, 0, 0); // 🔵

/// MCDM スコア中（橙）— mcdm_scatter_chart の中位スコア
/// 旧: mcdm_scatter_chart.rs の COLOR_ORANGE
pub const COLOR_MCDM_MID: Color32 = Color32::from_rgb(255, 165, 0); // 🔵

/// MCDM スコア低（黄）— mcdm_scatter_chart の低位スコア
/// 旧: mcdm_scatter_chart.rs の COLOR_YELLOW
pub const COLOR_MCDM_LOW: Color32 = Color32::from_rgb(255, 255, 0); // 🔵

/// MCDM スコアなし（灰）
/// 旧: mcdm_scatter_chart.rs の COLOR_GRAY
pub const COLOR_MCDM_NONE: Color32 = Color32::from_rgb(200, 200, 200); // 🔵

// ====================================================================
// バー・チャート系色
// ====================================================================

/// バーチャートの主色（青）— mcdm_chart, importance_chart のメインバー
/// 旧: mcdm_chart.rs の from_rgb(0x0c, 0x6a, 0xc0)
pub const COLOR_BAR_PRIMARY: Color32 = Color32::from_rgb(12, 106, 192); // 🔵

/// バーチャートの負値色（赤）— mcdm_chart の負値バー
/// 旧: mcdm_chart.rs の from_rgb(0xc0, 0x20, 0x20)
pub const COLOR_BAR_NEGATIVE: Color32 = Color32::from_rgb(192, 32, 32); // 🔵

/// バーチャートのアクセント色（橙）— mcdm_chart の代替指標バー
/// 旧: mcdm_chart.rs の from_rgb(0xe0, 0x70, 0x00)
pub const COLOR_BAR_ACCENT: Color32 = Color32::from_rgb(224, 112, 0); // 🔵

/// importance_chart のバー暗青色（mcdm_chart のバー青とは別色）
/// 旧: importance_chart.rs の from_rgb(0x0c, 0x0c, 0x6a)
pub const COLOR_IMPORTANCE_BAR: Color32 = Color32::from_rgb(12, 12, 106); // 🟡

// ====================================================================
// 最適化履歴系色
// ====================================================================

/// 通常試行ライン色（青）— optimization_history
/// 旧: from_rgb(50, 150, 250)
pub const COLOR_OPT_TRIAL: Color32 = Color32::from_rgb(50, 150, 250); // 🔵

/// プルーニング済み試行ライン色（赤）— optimization_history
/// 旧: from_rgb(220, 50, 50)
pub const COLOR_OPT_PRUNED: Color32 = Color32::from_rgb(220, 50, 50); // 🔵

/// 実行中試行ライン色（緑）— optimization_history
/// 旧: from_rgb(50, 200, 120)
pub const COLOR_OPT_RUNNING: Color32 = Color32::from_rgb(50, 200, 120); // 🔵

/// 最良試行ハイライト色（金）— optimization_history
/// 旧: Color32::GOLD
pub const COLOR_OPT_BEST: Color32 = Color32::GOLD; // 🔵

// ====================================================================
// HV 履歴系色
// ====================================================================

/// HV 履歴ラインの色（緑）
/// 旧: hv_history.rs の from_rgb(50, 200, 100)
pub const COLOR_HV_LINE: Color32 = Color32::from_rgb(50, 200, 100); // 🔵

// ====================================================================
// フィット品質色
// ====================================================================

/// フィット品質低（赤）— importance_chart
/// 旧: from_rgb(220, 80, 80)
pub const COLOR_FIT_LOW: Color32 = Color32::from_rgb(220, 80, 80); // 🔵

/// フィット品質中（黄）— importance_chart
/// 旧: from_rgb(200, 160, 0)
pub const COLOR_FIT_MID: Color32 = Color32::from_rgb(200, 160, 0); // 🔵

/// フィット品質高（緑）— importance_chart
/// 旧: from_rgb(60, 180, 60)
pub const COLOR_FIT_HIGH: Color32 = Color32::from_rgb(60, 180, 60); // 🔵

// ====================================================================
// PDP / ICE チャート色
// ====================================================================

/// PDP 主ライン色（青）
/// 旧: pdp_chart.rs の from_rgb(50, 100, 255)
pub const COLOR_PDP_LINE: Color32 = Color32::from_rgb(50, 100, 255); // 🔵

/// PDP 信頼区間フィル色（薄い青、プリマルチプライド）
/// 旧: pdp_chart.rs の from_rgba_unmultiplied(50, 100, 255, 50)
/// 注: unmultiplied→premultiplied 変換: r=50*50/255≈10, g=100*50/255≈20, b=255*50/255≈50
pub const COLOR_PDP_CI: Color32 = Color32::from_rgba_premultiplied(10, 20, 50, 50); // 🟡

/// ICE ライン色（薄い灰、プリマルチプライド）
/// 旧: pdp_chart.rs の from_rgba_unmultiplied(150, 150, 150, 60)
/// 注: unmultiplied→premultiplied 変換: r=g=b=150*60/255≈35
pub const COLOR_ICE_LINE: Color32 = Color32::from_rgba_premultiplied(35, 35, 35, 60); // 🟡

/// 等高線色（黄）— pdp_2d
/// 旧: Color32::YELLOW
pub const COLOR_CONTOUR: Color32 = Color32::YELLOW; // 🔵

// ====================================================================
// スキャッタ系色
// ====================================================================

/// スキャッターマトリクスのドット色（青）
/// 旧: scatter_matrix.rs の from_rgb(70, 130, 220)
pub const COLOR_SCATTER_DOT: Color32 = Color32::from_rgb(70, 130, 220); // 🔵

// ====================================================================
// 選択・ハイライト系色
// ====================================================================

/// グリッドセルの選択ハイライト色（薄い青、プリマルチプライド）
/// 旧: grid_canvas.rs の from_rgba_unmultiplied(37, 99, 235, 40)
/// 注: unmultiplied→premultiplied 変換: r=37*40/255≈6, g=99*40/255≈16, b=235*40/255≈37
pub const COLOR_SELECTION_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(6, 16, 37, 40); // 🟡

/// グリッドセルのドラッグハイライト色（中程度の青、プリマルチプライド）
/// 旧: grid_canvas.rs の from_rgba_unmultiplied(37, 99, 235, 80)
/// 注: unmultiplied→premultiplied 変換: r=37*80/255≈12, g=99*80/255≈31, b=235*80/255≈74
pub const COLOR_CELL_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(12, 31, 74, 80); // 🟡

// ====================================================================
// リンク・テーブル系色
// ====================================================================

/// リンク・テーブルセルの青系色
/// 旧: trial_table.rs, bottom_panel.rs の from_rgb(80, 120, 180)
pub const COLOR_LINK: Color32 = Color32::from_rgb(80, 120, 180); // 🔵

// ====================================================================
// 信頼性レベルサマリー
// ====================================================================
// - 🔵 青信号: 23件 (79%)
// - 🟡 黄信号: 6件 (21%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
```

---

## `egui-app/src/render/mod.rs` 変更後 🔵

**信頼性**: 🔵 *ユーザヒアリング（全呼び出し元を直接更新）より*

`render/colormap.rs` 削除後は `render/mod.rs` から `colormap` モジュール宣言を削除する：

```rust
// colormap モジュール行を削除
// pub mod colormap;  ← この行を削除

pub mod gpu_buffer;
```

---

## 呼び出し元のインポートパス更新例 🔵

**信頼性**: 🔵 *コードベース分析・ユーザヒアリングより*

### `state/types.rs`

```rust
// 変更前
pub fn to_colormap(&self) -> crate::render::colormap::ColorMap {
    match self {
        Self::Viridis => crate::render::colormap::ColorMap::viridis(),
        // ...
    }
}

// 変更後
pub fn to_colormap(&self) -> crate::theme::colormap::ColorMap {
    match self {
        Self::Viridis => crate::theme::colormap::ColorMap::viridis(),
        // ...
    }
}
```

### `state/app_state.rs`

```rust
// 変更前
self.chart_colors = crate::render::colormap::compute_chart_colors(...);

// 変更後
self.chart_colors = crate::theme::colormap::compute_chart_colors(...);
```

### ウィジェットファイル（例: `ui/widgets/pareto_2d.rs`）

```rust
// 変更前
use crate::render::colormap::compute_point_alpha;
const COLOR_PARETO: egui::Color32 = egui::Color32::from_rgb(220, 50, 50);
const COLOR_NON_PARETO: egui::Color32 = egui::Color32::from_rgb(50, 150, 250);

// 変更後
use crate::theme::colormap::compute_point_alpha;
use crate::theme::chart_colors::{COLOR_PARETO, COLOR_NON_PARETO, COLOR_PARETO_DIM, COLOR_NON_PARETO_DIM};
// （ローカルの const 定義は削除）
```

### `ui/widgets/mcdm_scatter_chart.rs`

```rust
// 変更前
pub(crate) const COLOR_RED: Color32 = Color32::from_rgb(255, 0, 0);
pub(crate) const COLOR_ORANGE: Color32 = Color32::from_rgb(255, 165, 0);
pub(crate) const COLOR_YELLOW: Color32 = Color32::from_rgb(255, 255, 0);
pub(crate) const COLOR_GRAY: Color32 = Color32::from_rgb(200, 200, 200);

// 変更後（定数定義を削除し、theme からインポート）
use crate::theme::chart_colors::{COLOR_MCDM_HIGH, COLOR_MCDM_MID, COLOR_MCDM_LOW, COLOR_MCDM_NONE};
// 使用箇所: COLOR_RED → COLOR_MCDM_HIGH, COLOR_ORANGE → COLOR_MCDM_MID 等に置換
```

---

## 注意事項

### `from_rgba_unmultiplied` → `const` 変換について 🟡

**信頼性**: 🟡 *Rust const fn 仕様から妥当な推測*

`Color32::from_rgba_unmultiplied` は `const fn` 非対応のため、以下の2つの選択肢がある：

**選択肢A（推奨）**: `from_rgba_premultiplied` に変換して `const` 化
- 計算式: `premultiplied_channel = unmultiplied_channel * alpha / 255`
- 例: `from_rgba_unmultiplied(37, 99, 235, 40)` → `from_rgba_premultiplied(6, 16, 37, 40)`
- ただし整数丸めにより微妙に色が変わる可能性がある

**選択肢B**: `pub fn` として定義
```rust
pub fn color_selection_highlight() -> Color32 {
    Color32::from_rgba_unmultiplied(37, 99, 235, 40)
}
```

- `const` ではないため呼び出し側が関数呼び出しになる（パフォーマンスへの影響はほぼゼロ）

実装者の判断で選択してよい。

### `COLOR_IMPORTANCE_BAR` の色値確認 🟡

**信頼性**: 🟡 *コードベース分析から妥当な推測*

`importance_chart.rs` の `from_rgb(0x0c, 0x0c, 0x6a)`（= R:12, G:12, B:106 ダークネイビー）は
`mcdm_chart.rs` の `from_rgb(0x0c, 0x6a, 0xc0)`（= R:12, G:106, B:192 ミディアムブルー）と
異なる色であることを実装前に目視確認すること。
