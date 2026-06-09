use egui::Color32;

// ====================================================================
// Pareto 系
// ====================================================================

pub const COLOR_PARETO: Color32 = Color32::from_rgb(234, 67, 53);
pub const COLOR_NON_PARETO: Color32 = Color32::from_rgb(66, 133, 244);
/// premultiplied: r=234*60/255≈55, g=67*60/255≈16, b=53*60/255≈12
pub const COLOR_PARETO_DIM: Color32 = Color32::from_rgba_premultiplied(55, 16, 12, 60);
/// premultiplied: r=66*60/255≈16, g=133*60/255≈31, b=244*60/255≈57
pub const COLOR_NON_PARETO_DIM: Color32 = Color32::from_rgba_premultiplied(16, 31, 57, 60);

// ====================================================================
// 3D 軸色
// ====================================================================

pub const COLOR_AXIS_X: Color32 = Color32::from_rgb(210, 100, 100);
pub const COLOR_AXIS_Y: Color32 = Color32::from_rgb(80, 170, 80);
pub const COLOR_AXIS_Z: Color32 = Color32::from_rgb(100, 100, 200);

// ====================================================================
// MCDM スコア段階色
// ====================================================================

pub const COLOR_MCDM_HIGH: Color32 = Color32::from_rgb(234, 67, 53);
pub const COLOR_MCDM_MID: Color32 = Color32::from_rgb(251, 188, 4);
pub const COLOR_MCDM_LOW: Color32 = Color32::from_rgb(52, 168, 83);
/// 劣解（ランク外）はクラスター図の非パレート色と揃えて淡い水色で表示する
pub const COLOR_MCDM_NONE: Color32 = COLOR_NON_PARETO_DIM;

// ====================================================================
// バー・チャート系
// ====================================================================

pub const COLOR_BAR_PRIMARY: Color32 = Color32::from_rgb(66, 133, 244);
pub const COLOR_BAR_NEGATIVE: Color32 = Color32::from_rgb(234, 67, 53);
pub const COLOR_BAR_ACCENT: Color32 = Color32::from_rgb(251, 188, 4);
/// importance_chart 用ダークネイビー（COLOR_BAR_PRIMARY とは別色）
pub const COLOR_IMPORTANCE_BAR: Color32 = Color32::from_rgb(30, 60, 114);

// ====================================================================
// 最適化履歴系
// ====================================================================

pub const COLOR_OPT_TRIAL: Color32 = Color32::from_rgb(66, 133, 244);
pub const COLOR_OPT_PRUNED: Color32 = Color32::from_rgb(234, 67, 53);
pub const COLOR_OPT_RUNNING: Color32 = Color32::from_rgb(52, 168, 83);
pub const COLOR_OPT_BEST: Color32 = Color32::from_rgb(251, 188, 4);

// ====================================================================
// HV 履歴系
// ====================================================================

pub const COLOR_HV_LINE: Color32 = Color32::from_rgb(52, 168, 83);

// ====================================================================
// フィット品質系
// ====================================================================

pub const COLOR_FIT_LOW: Color32 = Color32::from_rgb(234, 67, 53);
pub const COLOR_FIT_MID: Color32 = Color32::from_rgb(251, 188, 4);
pub const COLOR_FIT_HIGH: Color32 = Color32::from_rgb(52, 168, 83);

// ====================================================================
// PDP 系
// ====================================================================

pub const COLOR_PDP_LINE: Color32 = Color32::from_rgb(66, 133, 244);
/// premultiplied: r=66*50/255≈13, g=133*50/255≈26, b=244*50/255≈48
pub const COLOR_PDP_CI: Color32 = Color32::from_rgba_premultiplied(13, 26, 48, 50);
/// premultiplied: r=150*60/255≈35, g=150*60/255≈35, b=150*60/255≈35
pub const COLOR_ICE_LINE: Color32 = Color32::from_rgba_premultiplied(35, 35, 35, 60);
pub const COLOR_CONTOUR: Color32 = Color32::from_rgb(124, 77, 255);

// ====================================================================
// スキャッタ系
// ====================================================================

pub const COLOR_SCATTER_DOT: Color32 = Color32::from_rgb(66, 133, 244);

// ====================================================================
// 選択ハイライト系
// ====================================================================

/// premultiplied: r=66*40/255≈10, g=133*40/255≈21, b=244*40/255≈38
pub const COLOR_SELECTION_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(10, 21, 38, 40);
/// premultiplied: r=66*80/255≈21, g=133*80/255≈42, b=244*80/255≈77
pub const COLOR_CELL_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(21, 42, 77, 80);

// ====================================================================
// リンク色
// ====================================================================

pub const COLOR_LINK: Color32 = Color32::from_rgb(66, 133, 244);

// ====================================================================
// 3D ビュー系
// ====================================================================

pub const COLOR_3D_BG: Color32 = Color32::from_rgb(240, 242, 245);
/// premultiplied: r=120*70/255≈33, g=120*70/255≈33, b=130*70/255≈36
pub const COLOR_3D_GRID: Color32 = Color32::from_rgba_premultiplied(33, 33, 36, 70);

// ====================================================================
// ハイライト試行点（pareto_2d / pareto_3d 共通）
// ====================================================================

pub const COLOR_HIGHLIGHT_PT: Color32 = Color32::from_rgb(124, 77, 255);

// ====================================================================
// パラレルコーディネート系
// ====================================================================

pub const COLOR_PARALLEL_TICK: Color32 = Color32::from_rgb(95, 99, 104);
pub const COLOR_PARALLEL_LINE_DEFAULT: Color32 = Color32::from_rgb(66, 133, 244);
pub const COLOR_PARALLEL_AXIS: Color32 = Color32::from_rgb(218, 220, 224);
/// ブラシ選択範囲外の線を薄い灰色にグレーアウトする色（premultiplied）
/// premultiplied: rgb(170,170,170) × 14/255 ≈ 9,9,9; alpha = 14
pub const COLOR_PARALLEL_LINE_UNSELECTED: Color32 = Color32::from_rgba_premultiplied(9, 9, 9, 14);

// ====================================================================
// PDP CI バンド凡例マーカー（egui_plot 用、non-premultiplied 相当）
// ====================================================================

/// premultiplied: r=66*120/255≈31, g=133*120/255≈63, b=244*120/255≈115
pub const COLOR_PDP_CI_LEGEND: Color32 = Color32::from_rgba_premultiplied(31, 63, 115, 120);

// ====================================================================
// AHP 一貫性比率
// ====================================================================

pub const COLOR_CR_OK: Color32 = Color32::from_rgb(52, 168, 83);

// ====================================================================
// チャート汎用色
// ====================================================================

/// チャートセル内テキスト（ヒートマップ・相関係数セルなど）
pub const COLOR_CHART_TEXT: Color32 = Color32::from_rgb(32, 33, 36);

/// データなし・空状態の表示色
pub const COLOR_EMPTY_STATE: Color32 = Color32::from_rgb(95, 99, 104);

/// ヒートマップ・マトリクス系のグリッド線色
pub const COLOR_GRID_STROKE: Color32 = Color32::from_rgb(218, 220, 224);

// ====================================================================
// 実行不可能解（制約違反）
// ====================================================================

/// 実行不可能解のグレーアウト色（premultiplied）
/// premultiplied: rgb(180,180,180) × 80/255 ≈ 56,56,56; alpha = 80
pub const COLOR_INFEASIBLE: Color32 = Color32::from_rgba_premultiplied(56, 56, 56, 80);
