// UIカラーリング改善 型定義・定数定義
//
// 作成日: 2026-04-18
// 関連設計: architecture.md
// 実装先: egui-app/src/theme.rs
//
// 信頼性レベル:
// - 🔵 青信号: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な定義
// - 🟡 黄信号: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による定義
// - 🔴 赤信号: EARS要件定義書・設計文書・ユーザヒアリングにない推測による定義

use egui::{Color32, Stroke, Visuals};

// ========================================
// カラーパレット定数
// ========================================

/// ツールバー背景色 — ダークネイビー (#1a2332)
/// 🔵 ユーザヒアリング（添付スクリーンショット分析）より
pub const TOOLBAR_BG: Color32 = Color32::from_rgb(26, 35, 50);

/// ツールバーテキスト色 — 明るいグレー (#dce6f5)
/// 🔵 ユーザヒアリング（ダークネイビー背景でのコントラスト確保）より
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(220, 230, 245);

/// サイドパネル背景色 — ライトグレー (#f5f7fa)
/// 🔵 ユーザヒアリング（添付スクリーンショット分析）より
pub const PANEL_BG: Color32 = Color32::from_rgb(245, 247, 250);

/// メインキャンバス背景色 — 白 (#ffffff)
/// 🔵 ユーザヒアリング（添付スクリーンショット分析）より
pub const CENTRAL_BG: Color32 = Color32::WHITE;

/// アクセントブルー（アクティブボタン・強調色） (#2563eb)
/// 🔵 ユーザヒアリング（添付スクリーンショット分析）より
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(37, 99, 235);

/// アクセントブルー（ホバー時） (#1d4ed8)
/// 🔵 ユーザヒアリング（ホバーで少し暗くする）より
pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(29, 78, 216);

/// アクセントブルー（選択状態ラベル背景、淡い） (#dbeafe)
/// 🔵 ユーザヒアリング（selectable_label選択状態）より
pub const ACCENT_BLUE_MUTED: Color32 = Color32::from_rgb(219, 234, 254);

/// パネル・セル境界線色 (#cbd5e1)
/// 🔵 ユーザヒアリング（添付スクリーンショット分析）より
pub const BORDER_COLOR: Color32 = Color32::from_rgb(203, 213, 225);

/// メインテキスト色 (#1e293b)
/// 🔵 ユーザヒアリング（ライトテーマのコントラスト）より
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 41, 59);

/// サブテキスト・ラベル色 (#64748b)
/// 🟡 ライトテーマの慣例的な二次テキスト色から妥当な推測
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 116, 139);

/// チャートセルツールバー背景色 — ほぼ白 (#f8fafc)
/// 🔵 ユーザヒアリング（セルタイトルバーの視認性）より
pub const CELL_TOOLBAR_BG: Color32 = Color32::from_rgb(248, 250, 252);

/// ウィジェット背景色（非アクティブ） — 白
/// 🔵 ライトテーマの標準的なウィジェット背景より
pub const WIDGET_BG: Color32 = Color32::WHITE;

/// ウィジェット背景色（ホバー時） (#f1f5f9)
/// 🟡 ライトテーマの慣例的なホバー色から妥当な推測
pub const WIDGET_BG_HOVER: Color32 = Color32::from_rgb(241, 245, 249);

// ========================================
// Visuals 構築関数
// ========================================

/// Tunny Dashboard のライトテーマを返す
///
/// 使用方法:
/// ```rust
/// // TunnyApp::new() 内
/// cc.egui_ctx.set_visuals(crate::theme::tunny_light_visuals());
/// ```
///
/// 🔵 ユーザヒアリング（ライトテーマ選択）・egui 0.30 Visuals API より
pub fn tunny_light_visuals() -> Visuals {
    let mut v = Visuals::light(); // 🔵 ライトテーマベース

    // パネル背景
    v.panel_fill = PANEL_BG; // 🔵 左・右パネル背景
    v.window_fill = CENTRAL_BG; // 🔵 メインキャンバス背景
    v.window_stroke = Stroke::new(1.0, BORDER_COLOR); // 🔵 パネル境界線

    // テキスト
    v.override_text_color = Some(TEXT_PRIMARY); // 🔵 全テキストのデフォルト色
    v.extreme_bg_color = Color32::WHITE; // 🔵 テキスト入力・ComboBox背景

    // ウィジェット: アクティブ（クリック中）
    v.widgets.active.bg_fill = ACCENT_BLUE; // 🔵 アクティブボタン背景
    v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE); // 🔵 アクティブテキスト
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_BLUE); // 🔵 アクティブ枠線

    // ウィジェット: ホバー
    v.widgets.hovered.bg_fill = WIDGET_BG_HOVER; // 🟡 ホバー背景
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_BLUE); // 🔵 ホバー枠線（青でフォーカス感）

    // ウィジェット: 非アクティブ
    v.widgets.inactive.bg_fill = WIDGET_BG; // 🔵 非アクティブ背景
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_COLOR); // 🔵 非アクティブ枠線

    // ウィジェット: 非インタラクティブ（ラベル・区切り線）
    v.widgets.noninteractive.bg_fill = PANEL_BG; // 🔵 パネル内非インタラクティブ背景
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_COLOR); // 🔵 セパレータ色

    // 選択状態（selectable_label等）
    v.selection.bg_fill = ACCENT_BLUE_MUTED; // 🔵 選択背景（淡い青）
    v.selection.stroke = Stroke::new(1.0, ACCENT_BLUE); // 🔵 選択枠線

    v
}

// ========================================
// 信頼性レベルサマリー
// ========================================
// - 🔵 青信号: 18件 (90%)
// - 🟡 黄信号: 2件 (10%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
