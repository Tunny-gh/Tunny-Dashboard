//! UI クローム（ツールバー・パネル・テキスト等）のテーマ追従色。
//!
//! ライト値は TONMANUAL パレット準拠（従来の定数値を維持）。ダーク値は
//! 同パレットのダーク側スケール（gray-800/900 基調 + blue アクセント維持）。
//! すべて `NAME()` 形式の関数で、[`crate::theme::is_dark_mode`] に追従する。

use egui::Color32;

use super::themed_color;

// ====================================================================
// ツールバー系 (TONMANUAL §4 ナビゲーションバー)
// ====================================================================

themed_color!(
    /// ナビゲーション背景: light = blue-200 / dark = blue-950
    TOOLBAR_BG,
    Color32::from_rgb(191, 219, 254), // #BFDBFE
    Color32::from_rgb(23, 37, 84)     // #172554
);

themed_color!(
    /// ナビゲーションテキスト: light = gray-700 / dark = gray-300
    TOOLBAR_TEXT,
    Color32::from_rgb(55, 65, 81),    // #374151
    Color32::from_rgb(209, 213, 219)  // #D1D5DB
);

themed_color!(
    /// ボタンホバー時の背景色: light = blue-300 / dark = blue-800
    TOOLBAR_BTN_HOVER,
    Color32::from_rgb(147, 197, 253), // #93C5FD
    Color32::from_rgb(30, 64, 175)    // #1E40AF
);

themed_color!(
    /// ボタンアクティブ時の背景色: blue-500（両テーマ共通）
    TOOLBAR_BTN_ACTIVE,
    Color32::from_rgb(59, 130, 246), // #3B82F6
    Color32::from_rgb(59, 130, 246)
);

themed_color!(
    /// ボタンホバー/アクティブ時のテキスト色: white（blue-500 背景上、両テーマ共通）
    TOOLBAR_BTN_FG,
    Color32::WHITE,
    Color32::WHITE
);

themed_color!(
    /// コンボボックス・入力欄の背景色: light = gray-100 / dark = gray-800
    TOOLBAR_INPUT_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// コンボボックス・入力欄の枠線色: light = gray-200 / dark = gray-700
    TOOLBAR_INPUT_STROKE,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

// ====================================================================
// パネル・キャンバス系 (TONMANUAL §5 レイアウト)
// ====================================================================

themed_color!(
    /// 左右パネルの背景色: light = gray-100 / dark = gray-800
    PANEL_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// メインキャンバス（グリッドセル）の背景色: light = white / dark = gray-900
    CENTRAL_BG,
    Color32::WHITE,
    Color32::from_rgb(17, 24, 39) // #111827
);

themed_color!(
    /// チャートセルのツールバー背景色: light = gray-100 / dark = gray-800
    CELL_TOOLBAR_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// テーブルのストライプ（奇数行）背景色: light = gray-200 / dark = gray-700
    ///
    /// egui のデフォルト faint_bg_color は背景上でほぼ見えないため、
    /// 視認性を上げる目的でテーブル描画前にこの色へ上書きする。
    TABLE_STRIPE_BG,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

themed_color!(
    /// 自由配置キャンバスの背景色: light = white / dark = gray-900
    CANVAS_BG,
    Color32::WHITE,
    Color32::from_rgb(17, 24, 39) // #111827
);

themed_color!(
    /// 自由配置キャンバスのドットグリッド色: light = gray-300 / dark = gray-700
    CANVAS_DOT,
    Color32::from_rgb(209, 213, 219), // #D1D5DB
    Color32::from_rgb(55, 65, 81)     // #374151
);

// ====================================================================
// ウィジェット系
// ====================================================================

themed_color!(
    /// 非アクティブウィジェット背景: light = gray-100 / dark = gray-800
    WIDGET_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// ホバー時ウィジェット背景: light = gray-200 / dark = gray-700
    WIDGET_BG_HOVER,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

themed_color!(
    /// グリッドセルのクローズボタンテキスト色
    CLOSE_BTN_TEXT,
    Color32::from_gray(180),
    Color32::from_gray(120)
);

// ====================================================================
// アクセントカラー (TONMANUAL §2 プライマリカラー)
// ====================================================================

themed_color!(
    /// メインブルー: blue-500（両テーマ共通）
    ACCENT_BLUE,
    Color32::from_rgb(59, 130, 246), // #3B82F6
    Color32::from_rgb(59, 130, 246)
);

themed_color!(
    /// 選択ハイライト: light = blue-300 / dark = blue-800
    ACCENT_BLUE_MUTED,
    Color32::from_rgb(147, 197, 253), // #93C5FD
    Color32::from_rgb(30, 64, 175)    // #1E40AF
);

// ====================================================================
// テキスト・ボーダー系 (TONMANUAL §2 セカンダリカラー)
// ====================================================================

themed_color!(
    /// 見出しテキスト: light = gray-900 / dark = gray-100
    TEXT_PRIMARY,
    Color32::from_rgb(17, 24, 39),    // #111827
    Color32::from_rgb(243, 244, 246)  // #F3F4F6
);

themed_color!(
    /// 本文テキスト: light = gray-600 / dark = gray-400
    TEXT_SECONDARY,
    Color32::from_rgb(75, 85, 99),    // #4B5563
    Color32::from_rgb(156, 163, 175)  // #9CA3AF
);

themed_color!(
    /// ボーダー: light = gray-200 / dark = gray-700
    BORDER_COLOR,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

// ====================================================================
// セマンティックカラー
// ====================================================================

themed_color!(
    /// エラー表示色: ブランドカラー対象外（両テーマ共通）
    ERROR_COLOR,
    Color32::from_rgb(234, 67, 53),
    Color32::from_rgb(234, 67, 53)
);
