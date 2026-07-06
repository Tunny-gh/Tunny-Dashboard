//! チャート描画色のテーマ追従定義。
//!
//! データ系列色（青・赤・緑・金）は両テーマ共通で、背景・テキスト・
//! グリッド・半透明グレーアウト系のみダーク値を持つ。半透明色は
//! premultiplied alpha（`from_rgba_premultiplied`）で、ライトは
//! 白背景・ダークは gray-900 背景での視認性に合わせて調整している。
//! すべて `NAME()` 形式の関数で、[`crate::theme::is_dark_mode`] に追従する。

use egui::Color32;

use super::themed_color;

// ====================================================================
// Pareto 系
// ====================================================================

themed_color!(
    COLOR_PARETO,
    Color32::from_rgb(234, 67, 53),
    Color32::from_rgb(234, 67, 53)
);

themed_color!(
    COLOR_NON_PARETO,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

themed_color!(
    /// 非パレート点の減光色。
    /// light premultiplied: r=66*60/255≈16, g=133*60/255≈31, b=244*60/255≈57
    /// dark はやや高い alpha（80）で暗背景上の視認性を確保する。
    COLOR_NON_PARETO_DIM,
    Color32::from_rgba_premultiplied(16, 31, 57, 60),
    Color32::from_rgba_premultiplied(21, 42, 77, 80)
);

// ====================================================================
// 3D 軸色
// ====================================================================

themed_color!(
    COLOR_AXIS_X,
    Color32::from_rgb(210, 100, 100),
    Color32::from_rgb(230, 120, 120)
);

themed_color!(
    COLOR_AXIS_Y,
    Color32::from_rgb(80, 170, 80),
    Color32::from_rgb(110, 200, 110)
);

themed_color!(
    COLOR_AXIS_Z,
    Color32::from_rgb(100, 100, 200),
    Color32::from_rgb(130, 130, 230)
);

// ====================================================================
// MCDM スコア段階色
// ====================================================================

/// 劣解（ランク外）はクラスター図の非パレート色と揃えて淡い水色で表示する
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_MCDM_NONE() -> Color32 {
    COLOR_NON_PARETO_DIM()
}

// ====================================================================
// バー・チャート系
// ====================================================================

themed_color!(
    COLOR_BAR_PRIMARY,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

themed_color!(
    COLOR_BAR_NEGATIVE,
    Color32::from_rgb(234, 67, 53),
    Color32::from_rgb(234, 67, 53)
);

themed_color!(
    COLOR_BAR_ACCENT,
    Color32::from_rgb(251, 188, 4),
    Color32::from_rgb(251, 188, 4)
);

themed_color!(
    /// importance_chart 用バー色。light はダークネイビー、dark では
    /// 暗背景に沈むため明るいスチールブルーに切り替える。
    COLOR_IMPORTANCE_BAR,
    Color32::from_rgb(30, 60, 114),
    Color32::from_rgb(91, 141, 239)
);

// ====================================================================
// 最適化履歴系
// ====================================================================

themed_color!(
    COLOR_OPT_TRIAL,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

themed_color!(
    COLOR_OPT_PRUNED,
    Color32::from_rgb(234, 67, 53),
    Color32::from_rgb(234, 67, 53)
);

themed_color!(
    COLOR_OPT_RUNNING,
    Color32::from_rgb(52, 168, 83),
    Color32::from_rgb(52, 168, 83)
);

// ====================================================================
// 収束指標チャート系
// ====================================================================

themed_color!(
    COLOR_CONVERGENCE_LINE,
    Color32::from_rgb(52, 168, 83),
    Color32::from_rgb(52, 168, 83)
);

// ====================================================================
// フィット品質系
// ====================================================================

themed_color!(
    COLOR_FIT_LOW,
    Color32::from_rgb(234, 67, 53),
    Color32::from_rgb(234, 67, 53)
);

themed_color!(
    COLOR_FIT_MID,
    Color32::from_rgb(251, 188, 4),
    Color32::from_rgb(251, 188, 4)
);

themed_color!(
    COLOR_FIT_HIGH,
    Color32::from_rgb(52, 168, 83),
    Color32::from_rgb(52, 168, 83)
);

// ====================================================================
// PDP 系
// ====================================================================

themed_color!(
    COLOR_PDP_LINE,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

themed_color!(
    /// PDP 信頼区間バンド。
    /// light premultiplied: r=66*50/255≈13, g=133*50/255≈26, b=244*50/255≈48
    COLOR_PDP_CI,
    Color32::from_rgba_premultiplied(13, 26, 48, 50),
    Color32::from_rgba_premultiplied(18, 37, 67, 70)
);

themed_color!(
    /// ICE 線。light は濃灰の半透明、dark は明灰の半透明。
    /// light premultiplied: r=150*60/255≈35（×3成分）
    /// dark premultiplied: r=200*60/255≈47（×3成分）
    COLOR_ICE_LINE,
    Color32::from_rgba_premultiplied(35, 35, 35, 60),
    Color32::from_rgba_premultiplied(47, 47, 47, 60)
);

themed_color!(
    COLOR_CONTOUR,
    Color32::from_rgb(124, 77, 255),
    Color32::from_rgb(149, 117, 255)
);

// ====================================================================
// スキャッタ系
// ====================================================================

themed_color!(
    COLOR_SCATTER_DOT,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

// ====================================================================
// 選択ハイライト系
// ====================================================================

themed_color!(
    /// 選択ハイライト面。
    /// light premultiplied: r=66*40/255≈10, g=133*40/255≈21, b=244*40/255≈38
    COLOR_SELECTION_HIGHLIGHT,
    Color32::from_rgba_premultiplied(10, 21, 38, 40),
    Color32::from_rgba_premultiplied(16, 31, 57, 60)
);

themed_color!(
    /// 選択フィルタ外の散布図点を表す中間灰色（元の色相を残さず、選択点と
    /// 明確に区別する）。半透明にして選択点を引き立てつつ、灰色であることが
    /// 分かる程度の不透明度を保つ。
    /// premultiplied: rgb(150,150,150) × 90/255 ≈ 53（×3成分）
    COLOR_UNSELECTED_POINT,
    Color32::from_rgba_premultiplied(53, 53, 53, 90),
    Color32::from_rgba_premultiplied(53, 53, 53, 90)
);

// ====================================================================
// リンク色
// ====================================================================

themed_color!(
    COLOR_LINK,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(102, 157, 246)
);

// ====================================================================
// 3D ビュー系
// ====================================================================

themed_color!(
    /// 3D ビューの背景。light = 淡灰 / dark = gray-800。
    COLOR_3D_BG,
    Color32::from_rgb(240, 242, 245),
    Color32::from_rgb(31, 41, 55) // #1F2937
);

themed_color!(
    /// 3D グリッド線。
    /// light premultiplied: r=120*70/255≈33, g=33, b=130*70/255≈36
    /// dark premultiplied: rgb(170,170,180) × 70/255 ≈ 47,47,49
    COLOR_3D_GRID,
    Color32::from_rgba_premultiplied(33, 33, 36, 70),
    Color32::from_rgba_premultiplied(47, 47, 49, 70)
);

// ====================================================================
// ハイライト試行点（pareto_2d / pareto_3d 共通）
// ====================================================================

themed_color!(
    COLOR_HIGHLIGHT_PT,
    Color32::from_rgb(124, 77, 255),
    Color32::from_rgb(149, 117, 255)
);

// ====================================================================
// パラレルコーディネート系
// ====================================================================

themed_color!(
    /// 軸目盛テキスト: light = gray-600 相当 / dark = gray-400
    COLOR_PARALLEL_TICK,
    Color32::from_rgb(95, 99, 104),
    Color32::from_rgb(156, 163, 175) // #9CA3AF
);

themed_color!(
    COLOR_PARALLEL_LINE_DEFAULT,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

themed_color!(
    /// 軸線: light = 淡灰 / dark = gray-600
    COLOR_PARALLEL_AXIS,
    Color32::from_rgb(218, 220, 224),
    Color32::from_rgb(75, 85, 99) // #4B5563
);

themed_color!(
    /// ブラシ選択範囲外の線をグレーアウトする色（premultiplied）。
    /// light premultiplied: rgb(170,170,170) × 14/255 ≈ 9（×3成分）
    /// dark premultiplied: rgb(200,200,200) × 14/255 ≈ 11（×3成分）
    COLOR_PARALLEL_LINE_UNSELECTED,
    Color32::from_rgba_premultiplied(9, 9, 9, 14),
    Color32::from_rgba_premultiplied(11, 11, 11, 14)
);

// ====================================================================
// PDP CI バンド凡例マーカー（egui_plot 用、non-premultiplied 相当）
// ====================================================================

themed_color!(
    /// premultiplied: r=66*120/255≈31, g=133*120/255≈63, b=244*120/255≈115
    COLOR_PDP_CI_LEGEND,
    Color32::from_rgba_premultiplied(31, 63, 115, 120),
    Color32::from_rgba_premultiplied(31, 63, 115, 120)
);

// ====================================================================
// チャート汎用色
// ====================================================================

themed_color!(
    /// チャートセル内テキスト（ヒートマップ・相関係数セルなど）
    COLOR_CHART_TEXT,
    Color32::from_rgb(32, 33, 36),
    Color32::from_rgb(229, 231, 235) // #E5E7EB
);

themed_color!(
    /// データなし・空状態の表示色: light = gray-600 相当 / dark = gray-400
    COLOR_EMPTY_STATE,
    Color32::from_rgb(95, 99, 104),
    Color32::from_rgb(156, 163, 175) // #9CA3AF
);

// ====================================================================
// サロゲート多目的フロント（pareto_2d オーバーレイ）
// ====================================================================

themed_color!(
    /// サロゲート予測パレートフロント点の色（金色系。既存の赤・青と被らない）。
    COLOR_SURROGATE_FRONT,
    Color32::from_rgb(255, 193, 7),
    Color32::from_rgb(255, 193, 7)
);

themed_color!(
    /// ヒートマップ・マトリクス系のグリッド線色: light = gray-500 相当 / dark = gray-600
    COLOR_GRID_STROKE,
    Color32::from_rgb(154, 160, 166),
    Color32::from_rgb(75, 85, 99) // #4B5563
);

// ====================================================================
// 実行不可能解（制約違反）
// ====================================================================

themed_color!(
    /// 実行不可能解のグレーアウト色（premultiplied）
    /// premultiplied: rgb(180,180,180) × 80/255 ≈ 56（×3成分）
    COLOR_INFEASIBLE,
    Color32::from_rgba_premultiplied(56, 56, 56, 80),
    Color32::from_rgba_premultiplied(56, 56, 56, 80)
);

// ====================================================================
// trial state 系（Intermediate Values / Timeline 共通）
// ====================================================================

/// COMPLETE trial（正常終了）。最適化履歴の "All Trials" と同じ青系を再利用する。
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_COMPLETE() -> Color32 {
    COLOR_OPT_TRIAL()
}

/// PRUNED trial（枝刈り）。バーチャートのアクセント色（金色系）を再利用しオレンジとして扱う。
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_PRUNED() -> Color32 {
    COLOR_BAR_ACCENT()
}

/// RUNNING trial（実行中）。ニュートラルな灰色（空状態表示と同色）を再利用する。
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_RUNNING() -> Color32 {
    COLOR_EMPTY_STATE()
}

/// FAIL trial（失敗）。バーチャートの警告色（赤）を再利用する。
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_FAIL() -> Color32 {
    COLOR_BAR_NEGATIVE()
}

/// WAITING trial（開始待ち）。グリッド線と同じ薄い灰色を再利用する。
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_WAITING() -> Color32 {
    COLOR_GRID_STROKE()
}
