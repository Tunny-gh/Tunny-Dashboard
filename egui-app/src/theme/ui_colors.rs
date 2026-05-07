use egui::Color32;

// ====================================================================
// ツールバー系
// ====================================================================

pub const TOOLBAR_BG: Color32 = Color32::from_rgb(26, 35, 50);
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(220, 230, 245);
/// ボタンホバー時の背景色
pub const TOOLBAR_BTN_HOVER: Color32 = Color32::from_rgb(55, 78, 120);
/// ボタンアクティブ時の背景色
pub const TOOLBAR_BTN_ACTIVE: Color32 = Color32::from_rgb(37, 99, 235);
/// ボタンホバー/アクティブ時のテキスト色
pub const TOOLBAR_BTN_FG: Color32 = Color32::WHITE;
/// コンボボックス・入力欄の背景色
pub const TOOLBAR_INPUT_BG: Color32 = Color32::from_rgb(45, 62, 90);
/// コンボボックス・入力欄の枠線色
pub const TOOLBAR_INPUT_STROKE: Color32 = Color32::from_rgb(100, 130, 180);

// ====================================================================
// パネル・キャンバス系
// ====================================================================

/// 左右パネルの背景色
pub const PANEL_BG: Color32 = Color32::from_rgb(225, 233, 248);
/// メインキャンバス（グリッドセル）の背景色
pub const CENTRAL_BG: Color32 = Color32::WHITE;
/// チャートセルのツールバー背景色
pub const CELL_TOOLBAR_BG: Color32 = Color32::from_rgb(232, 239, 251);

// ====================================================================
// ウィジェット系
// ====================================================================

pub const WIDGET_BG: Color32 = Color32::from_rgb(235, 241, 252);
pub const WIDGET_BG_HOVER: Color32 = Color32::from_rgb(220, 230, 247);
/// グリッドセルのクローズボタンテキスト色
pub const CLOSE_BTN_TEXT: Color32 = Color32::from_gray(180);

// ====================================================================
// アクセントカラー
// ====================================================================

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(37, 99, 235);
#[allow(dead_code)]
pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(29, 78, 216);
pub const ACCENT_BLUE_MUTED: Color32 = Color32::from_rgb(219, 234, 254);

// ====================================================================
// テキスト・ボーダー系
// ====================================================================

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 41, 59);
#[allow(dead_code)]
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 116, 139);
pub const BORDER_COLOR: Color32 = Color32::from_rgb(203, 213, 225);

// ====================================================================
// セマンティックカラー
// ====================================================================

/// エラー表示色。Color32::RED より落ち着いた赤。
pub const ERROR_COLOR: Color32 = Color32::from_rgb(220, 50, 50);
