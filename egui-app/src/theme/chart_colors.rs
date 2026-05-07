use egui::Color32;

// ====================================================================
// Pareto 系
// ====================================================================

pub const COLOR_PARETO: Color32 = Color32::from_rgb(220, 50, 50);
pub const COLOR_NON_PARETO: Color32 = Color32::from_rgb(50, 150, 250);
/// premultiplied: r=220*60/255≈52, g=50*60/255≈12, b=50*60/255≈12
pub const COLOR_PARETO_DIM: Color32 = Color32::from_rgba_premultiplied(52, 12, 12, 60);
/// premultiplied: r=50*60/255≈12, g=150*60/255≈35, b=250*60/255≈59
pub const COLOR_NON_PARETO_DIM: Color32 = Color32::from_rgba_premultiplied(12, 35, 59, 60);

// ====================================================================
// 3D 軸色
// ====================================================================

pub const COLOR_AXIS_X: Color32 = Color32::from_rgb(220, 80, 80);
pub const COLOR_AXIS_Y: Color32 = Color32::from_rgb(80, 220, 80);
pub const COLOR_AXIS_Z: Color32 = Color32::from_rgb(80, 80, 220);

// ====================================================================
// MCDM スコア段階色
// ====================================================================

pub const COLOR_MCDM_HIGH: Color32 = Color32::from_rgb(255, 0, 0);
pub const COLOR_MCDM_MID: Color32 = Color32::from_rgb(255, 165, 0);
pub const COLOR_MCDM_LOW: Color32 = Color32::from_rgb(255, 255, 0);
pub const COLOR_MCDM_NONE: Color32 = Color32::from_rgb(200, 200, 200);

// ====================================================================
// バー・チャート系
// ====================================================================

pub const COLOR_BAR_PRIMARY: Color32 = Color32::from_rgb(12, 106, 192);
pub const COLOR_BAR_NEGATIVE: Color32 = Color32::from_rgb(192, 32, 32);
pub const COLOR_BAR_ACCENT: Color32 = Color32::from_rgb(224, 112, 0);
/// importance_chart 用ダークネイビー（COLOR_BAR_PRIMARY とは別色）
pub const COLOR_IMPORTANCE_BAR: Color32 = Color32::from_rgb(12, 12, 106);

// ====================================================================
// 最適化履歴系
// ====================================================================

pub const COLOR_OPT_TRIAL: Color32 = Color32::from_rgb(50, 150, 250);
pub const COLOR_OPT_PRUNED: Color32 = Color32::from_rgb(220, 50, 50);
pub const COLOR_OPT_RUNNING: Color32 = Color32::from_rgb(50, 200, 120);
pub const COLOR_OPT_BEST: Color32 = Color32::GOLD;

// ====================================================================
// HV 履歴系
// ====================================================================

pub const COLOR_HV_LINE: Color32 = Color32::from_rgb(50, 200, 100);

// ====================================================================
// フィット品質系
// ====================================================================

pub const COLOR_FIT_LOW: Color32 = Color32::from_rgb(220, 80, 80);
pub const COLOR_FIT_MID: Color32 = Color32::from_rgb(200, 160, 0);
pub const COLOR_FIT_HIGH: Color32 = Color32::from_rgb(60, 180, 60);

// ====================================================================
// PDP 系
// ====================================================================

pub const COLOR_PDP_LINE: Color32 = Color32::from_rgb(50, 100, 255);
/// premultiplied: r=50*50/255≈10, g=100*50/255≈20, b=255*50/255≈50
pub const COLOR_PDP_CI: Color32 = Color32::from_rgba_premultiplied(10, 20, 50, 50);
/// premultiplied: r=150*60/255≈35, g=150*60/255≈35, b=150*60/255≈35
pub const COLOR_ICE_LINE: Color32 = Color32::from_rgba_premultiplied(35, 35, 35, 60);
pub const COLOR_CONTOUR: Color32 = Color32::YELLOW;

// ====================================================================
// スキャッタ系
// ====================================================================

pub const COLOR_SCATTER_DOT: Color32 = Color32::from_rgb(70, 130, 220);

// ====================================================================
// 選択ハイライト系
// ====================================================================

/// premultiplied: r=37*40/255≈6, g=99*40/255≈16, b=235*40/255≈37
pub const COLOR_SELECTION_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(6, 16, 37, 40);
/// premultiplied: r=37*80/255≈12, g=99*80/255≈31, b=235*80/255≈74
pub const COLOR_CELL_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(12, 31, 74, 80);

// ====================================================================
// リンク色
// ====================================================================

pub const COLOR_LINK: Color32 = Color32::from_rgb(80, 120, 180);

// ====================================================================
// 3D ビュー系
// ====================================================================

pub const COLOR_3D_BG: Color32 = Color32::from_rgb(20, 20, 30);
/// premultiplied: r=120*70/255≈33, g=120*70/255≈33, b=140*70/255≈38
pub const COLOR_3D_GRID: Color32 = Color32::from_rgba_premultiplied(33, 33, 38, 70);

// ====================================================================
// ハイライト試行点（pareto_2d / pareto_3d 共通）
// ====================================================================

pub const COLOR_HIGHLIGHT_PT: Color32 = Color32::YELLOW;

// ====================================================================
// パラレルコーディネート系
// ====================================================================

pub const COLOR_PARALLEL_TICK: Color32 = Color32::from_gray(60);
pub const COLOR_PARALLEL_LINE_DEFAULT: Color32 = Color32::from_rgb(100, 150, 220);
pub const COLOR_PARALLEL_AXIS: Color32 = Color32::from_gray(80);

// ====================================================================
// PDP CI バンド凡例マーカー（egui_plot 用、non-premultiplied 相当）
// ====================================================================

/// premultiplied: r=50*120/255≈24, g=100*120/255≈47, b=255*120/255=120
pub const COLOR_PDP_CI_LEGEND: Color32 = Color32::from_rgba_premultiplied(24, 47, 120, 120);

// ====================================================================
// AHP 一貫性比率
// ====================================================================

pub const COLOR_CR_OK: Color32 = Color32::GREEN;

// ====================================================================
// チャート汎用色
// ====================================================================

/// チャートセル内テキスト（ヒートマップ・相関係数セルなど）
pub const COLOR_CHART_TEXT: Color32 = Color32::BLACK;

/// データなし・空状態の表示色
pub const COLOR_EMPTY_STATE: Color32 = Color32::GRAY;

/// ヒートマップ・マトリクス系のグリッド線色
pub const COLOR_GRID_STROKE: Color32 = Color32::GRAY;
