use egui::Color32;

// ====================================================================
// Pareto 系
// ====================================================================

pub const COLOR_PARETO: Color32 = Color32::from_rgb(234, 67, 53);
pub const COLOR_NON_PARETO: Color32 = Color32::from_rgb(66, 133, 244);
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

// ====================================================================
// 収束指標チャート系
// ====================================================================

pub const COLOR_CONVERGENCE_LINE: Color32 = Color32::from_rgb(52, 168, 83);

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
/// 選択フィルタ外の散布図点を表す中間灰色（元の色相を残さず、選択点と明確に区別する）。
/// 半透明にして選択点を引き立てつつ、灰色であることが分かる程度の不透明度を保つ。
/// premultiplied: rgb(150,150,150) × 90/255 ≈ 53,53,53; alpha = 90
pub const COLOR_UNSELECTED_POINT: Color32 = Color32::from_rgba_premultiplied(53, 53, 53, 90);

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
// チャート汎用色
// ====================================================================

/// チャートセル内テキスト（ヒートマップ・相関係数セルなど）
pub const COLOR_CHART_TEXT: Color32 = Color32::from_rgb(32, 33, 36);

/// データなし・空状態の表示色
pub const COLOR_EMPTY_STATE: Color32 = Color32::from_rgb(95, 99, 104);

// ====================================================================
// サロゲート多目的フロント（pareto_2d オーバーレイ）
// ====================================================================

/// サロゲート予測パレートフロント点の色（金色系。既存の赤・青と被らない）。
pub const COLOR_SURROGATE_FRONT: Color32 = Color32::from_rgb(255, 193, 7);

/// ヒートマップ・マトリクス系のグリッド線色
pub const COLOR_GRID_STROKE: Color32 = Color32::from_rgb(154, 160, 166);

// ====================================================================
// 実行不可能解（制約違反）
// ====================================================================

/// 実行不可能解のグレーアウト色（premultiplied）
/// premultiplied: rgb(180,180,180) × 80/255 ≈ 56,56,56; alpha = 80
pub const COLOR_INFEASIBLE: Color32 = Color32::from_rgba_premultiplied(56, 56, 56, 80);

// ====================================================================
// trial state 系（Intermediate Values / Timeline 共通）
// ====================================================================

/// COMPLETE trial（正常終了）。最適化履歴の "All Trials" と同じ青系を再利用する。
pub const COLOR_STATE_COMPLETE: Color32 = COLOR_OPT_TRIAL;
/// PRUNED trial（枝刈り）。バーチャートのアクセント色（金色系）を再利用しオレンジとして扱う。
pub const COLOR_STATE_PRUNED: Color32 = COLOR_BAR_ACCENT;
/// RUNNING trial（実行中）。ニュートラルな灰色（空状態表示と同色）を再利用する。
pub const COLOR_STATE_RUNNING: Color32 = COLOR_EMPTY_STATE;
/// FAIL trial（失敗）。バーチャートの警告色（赤）を再利用する。
pub const COLOR_STATE_FAIL: Color32 = COLOR_BAR_NEGATIVE;
/// WAITING trial（開始待ち）。グリッド線と同じ薄い灰色を再利用する。
pub const COLOR_STATE_WAITING: Color32 = COLOR_GRID_STROKE;
