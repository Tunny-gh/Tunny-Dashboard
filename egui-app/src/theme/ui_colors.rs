use egui::Color32;

// ====================================================================
// ツールバー系 (TONMANUAL §4 ナビゲーションバー)
// ====================================================================

/// ナビゲーション背景: TONMANUAL blue-200
pub const TOOLBAR_BG: Color32 = Color32::from_rgb(191, 219, 254); // #BFDBFE

/// ナビゲーションテキスト: TONMANUAL gray-700
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(55, 65, 81); // #374151

/// ボタンホバー時の背景色: blue-300（blue-200 背景より暗くして視認性を確保）
pub const TOOLBAR_BTN_HOVER: Color32 = Color32::from_rgb(147, 197, 253); // #93C5FD

/// ボタンアクティブ時の背景色: TONMANUAL blue-500
pub const TOOLBAR_BTN_ACTIVE: Color32 = Color32::from_rgb(59, 130, 246); // #3B82F6

/// ボタンホバー/アクティブ時のテキスト色: white（blue-500 背景上）
pub const TOOLBAR_BTN_FG: Color32 = Color32::WHITE;

/// コンボボックス・入力欄の背景色: TONMANUAL gray-100
pub const TOOLBAR_INPUT_BG: Color32 = Color32::from_rgb(243, 244, 246); // #F3F4F6

/// コンボボックス・入力欄の枠線色: TONMANUAL gray-200
pub const TOOLBAR_INPUT_STROKE: Color32 = Color32::from_rgb(229, 231, 235); // #E5E7EB

// ====================================================================
// パネル・キャンバス系 (TONMANUAL §5 レイアウト)
// ====================================================================

/// 左右パネルの背景色: TONMANUAL gray-100
pub const PANEL_BG: Color32 = Color32::from_rgb(243, 244, 246); // #F3F4F6

/// メインキャンバス（グリッドセル）の背景色: white
pub const CENTRAL_BG: Color32 = Color32::WHITE; // #FFFFFF

/// チャートセルのツールバー背景色: gray-100
pub const CELL_TOOLBAR_BG: Color32 = Color32::from_rgb(243, 244, 246); // #F3F4F6

// ====================================================================
// ウィジェット系
// ====================================================================

/// 非アクティブウィジェット背景: TONMANUAL gray-100
pub const WIDGET_BG: Color32 = Color32::from_rgb(243, 244, 246); // #F3F4F6

/// ホバー時ウィジェット背景: TONMANUAL gray-200
pub const WIDGET_BG_HOVER: Color32 = Color32::from_rgb(229, 231, 235); // #E5E7EB

/// グリッドセルのクローズボタンテキスト色
pub const CLOSE_BTN_TEXT: Color32 = Color32::from_gray(180);

// ====================================================================
// アクセントカラー (TONMANUAL §2 プライマリカラー)
// ====================================================================

/// メインブルー: TONMANUAL blue-500
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246); // #3B82F6

/// ホバーブルー: TONMANUAL blue-600
#[allow(dead_code)]
pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(37, 99, 235); // #2563EB

/// 選択ハイライト: blue-300（TOOLBAR_BG の blue-200 より暗くして選択状態を視認可能に）
pub const ACCENT_BLUE_MUTED: Color32 = Color32::from_rgb(147, 197, 253); // #93C5FD

// ====================================================================
// テキスト・ボーダー系 (TONMANUAL §2 セカンダリカラー)
// ====================================================================

/// 見出しテキスト: TONMANUAL gray-900
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(17, 24, 39); // #111827

/// 本文テキスト: TONMANUAL gray-600
#[allow(dead_code)]
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(75, 85, 99); // #4B5563

/// ボーダー: TONMANUAL gray-200
pub const BORDER_COLOR: Color32 = Color32::from_rgb(229, 231, 235); // #E5E7EB

// ====================================================================
// 新規追加: ブランドカラー完全版 (TONMANUAL §2)
// ====================================================================

/// ヘッダー背景: TONMANUAL blue-300
#[allow(dead_code)]
pub const HEADER_BG: Color32 = Color32::from_rgb(147, 197, 253); // #93C5FD

/// アナウンスバー背景: TONMANUAL blue-400
#[allow(dead_code)]
pub const ANNOUNCE_BG: Color32 = Color32::from_rgb(96, 165, 250); // #60A5FA

/// アクション（購入・重要操作）: TONMANUAL green-500
#[allow(dead_code)]
pub const ACTION_GREEN: Color32 = Color32::from_rgb(34, 197, 94); // #22C55E

/// アクションホバー: TONMANUAL green-600
#[allow(dead_code)]
pub const ACTION_GREEN_HOVER: Color32 = Color32::from_rgb(22, 163, 74); // #16A34A

/// サブテキスト: TONMANUAL gray-700
#[allow(dead_code)]
pub const TEXT_SUB: Color32 = Color32::from_rgb(55, 65, 81); // #374151

// ====================================================================
// セマンティックカラー（変更なし）
// ====================================================================

/// エラー表示色: ブランドカラー対象外
pub const ERROR_COLOR: Color32 = Color32::from_rgb(234, 67, 53);
