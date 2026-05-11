use egui::Color32;

// ====================================================================
// ツールバー系
// ====================================================================

pub const TOOLBAR_BG: Color32 = Color32::from_rgb(32, 33, 36);
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(232, 234, 237);
/// ボタンホバー時の背景色
pub const TOOLBAR_BTN_HOVER: Color32 = Color32::from_rgb(55, 65, 81);
/// ボタンアクティブ時の背景色
pub const TOOLBAR_BTN_ACTIVE: Color32 = Color32::from_rgb(66, 133, 244);
/// ボタンホバー/アクティブ時のテキスト色
pub const TOOLBAR_BTN_FG: Color32 = Color32::WHITE;
/// コンボボックス・入力欄の背景色
pub const TOOLBAR_INPUT_BG: Color32 = Color32::from_rgb(48, 49, 52);
/// コンボボックス・入力欄の枠線色
pub const TOOLBAR_INPUT_STROKE: Color32 = Color32::from_rgb(95, 99, 104);

// ====================================================================
// パネル・キャンバス系
// ====================================================================

/// 左右パネルの背景色
pub const PANEL_BG: Color32 = Color32::from_rgb(240, 242, 245);
/// メインキャンバス（グリッドセル）の背景色
pub const CENTRAL_BG: Color32 = Color32::WHITE;
/// チャートセルのツールバー背景色
pub const CELL_TOOLBAR_BG: Color32 = Color32::from_rgb(245, 247, 250);

// ====================================================================
// ウィジェット系
// ====================================================================

pub const WIDGET_BG: Color32 = Color32::from_rgb(240, 244, 248);
pub const WIDGET_BG_HOVER: Color32 = Color32::from_rgb(232, 236, 242);
/// グリッドセルのクローズボタンテキスト色
pub const CLOSE_BTN_TEXT: Color32 = Color32::from_gray(180);

// ====================================================================
// アクセントカラー
// ====================================================================

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(66, 133, 244);
#[allow(dead_code)]
pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(51, 103, 214);
pub const ACCENT_BLUE_MUTED: Color32 = Color32::from_rgb(232, 240, 254);

// ====================================================================
// テキスト・ボーダー系
// ====================================================================

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(32, 33, 36);
#[allow(dead_code)]
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(95, 99, 104);
pub const BORDER_COLOR: Color32 = Color32::from_rgb(218, 220, 224);

// ====================================================================
// セマンティックカラー
// ====================================================================

/// エラー表示色。Color32::RED より落ち着いた赤。
pub const ERROR_COLOR: Color32 = Color32::from_rgb(234, 67, 53);
