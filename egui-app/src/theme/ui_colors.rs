//! Theme-following colors for UI chrome (toolbar, panels, text, etc.).
//!
//! Light values follow the TONMANUAL palette (existing constant values are
//! preserved). Dark values are the dark-side scale of the same palette
//! (gray-800/900 base + blue accent retained). All are `NAME()`-style
//! functions that follow [`crate::theme::is_dark_mode`].

use egui::Color32;

use super::themed_color;

// ====================================================================
// Toolbar (TONMANUAL §4 Navigation Bar)
// ====================================================================

themed_color!(
    /// Navigation background: light = blue-200 / dark = blue-950
    TOOLBAR_BG,
    Color32::from_rgb(191, 219, 254), // #BFDBFE
    Color32::from_rgb(23, 37, 84)     // #172554
);

themed_color!(
    /// Navigation text: light = gray-700 / dark = gray-300
    TOOLBAR_TEXT,
    Color32::from_rgb(55, 65, 81),    // #374151
    Color32::from_rgb(209, 213, 219)  // #D1D5DB
);

themed_color!(
    /// Button hover background: light = blue-300 / dark = blue-800
    TOOLBAR_BTN_HOVER,
    Color32::from_rgb(147, 197, 253), // #93C5FD
    Color32::from_rgb(30, 64, 175)    // #1E40AF
);

themed_color!(
    /// Button active background: blue-500 (same in both themes)
    TOOLBAR_BTN_ACTIVE,
    Color32::from_rgb(59, 130, 246), // #3B82F6
    Color32::from_rgb(59, 130, 246)
);

themed_color!(
    /// Button hover/active text color: white (on blue-500 background, same in both themes)
    TOOLBAR_BTN_FG,
    Color32::WHITE,
    Color32::WHITE
);

themed_color!(
    /// Combo box / input field background: light = gray-100 / dark = gray-800
    TOOLBAR_INPUT_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// Combo box / input field border color: light = gray-200 / dark = gray-700
    TOOLBAR_INPUT_STROKE,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

// ====================================================================
// Panels / Canvas (TONMANUAL §5 Layout)
// ====================================================================

themed_color!(
    /// Left/right panel background: light = gray-100 / dark = gray-800
    PANEL_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// Main canvas (grid cell) background: light = white / dark = gray-900
    CENTRAL_BG,
    Color32::WHITE,
    Color32::from_rgb(17, 24, 39) // #111827
);

themed_color!(
    /// Chart cell toolbar background: light = gray-100 / dark = gray-800
    CELL_TOOLBAR_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// Table stripe (odd row) background: light = gray-200 / dark = gray-700
    ///
    /// egui's default faint_bg_color is nearly invisible against the
    /// background, so this color overrides it before table drawing to
    /// improve visibility.
    TABLE_STRIPE_BG,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

themed_color!(
    /// Free-layout canvas background: light = white / dark = gray-900
    CANVAS_BG,
    Color32::WHITE,
    Color32::from_rgb(17, 24, 39) // #111827
);

themed_color!(
    /// Free-layout canvas dot grid color: light = gray-300 / dark = gray-700
    CANVAS_DOT,
    Color32::from_rgb(209, 213, 219), // #D1D5DB
    Color32::from_rgb(55, 65, 81)     // #374151
);

// ====================================================================
// Widgets
// ====================================================================

themed_color!(
    /// Inactive widget background: light = gray-100 / dark = gray-800
    WIDGET_BG,
    Color32::from_rgb(243, 244, 246), // #F3F4F6
    Color32::from_rgb(31, 41, 55)     // #1F2937
);

themed_color!(
    /// Widget background on hover: light = gray-200 / dark = gray-700
    WIDGET_BG_HOVER,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

themed_color!(
    /// Grid cell close button text color
    CLOSE_BTN_TEXT,
    Color32::from_gray(180),
    Color32::from_gray(120)
);

// ====================================================================
// Widget group icon colors (pastel tone)
//
// In the right panel's widget list, icons are color-coded per group. Since
// these are used as multiply tints on solid-white SVGs, they're unified as
// high-brightness pastel colors. Light uses a slightly darker tone that
// keeps visibility on the panel background (gray-100); dark uses a 300-level
// tone that pops against gray-800.
// ====================================================================

themed_color!(
    /// Convergence group: light = sky-500 / dark = sky-300
    GROUP_CONVERGENCE,
    Color32::from_rgb(14, 165, 233),  // #0EA5E9
    Color32::from_rgb(125, 211, 252)  // #7DD3FC
);

themed_color!(
    /// Pareto / Multi-Objective group: light = orange-400 / dark = orange-300
    GROUP_PARETO,
    Color32::from_rgb(251, 146, 60),  // #FB923C
    Color32::from_rgb(253, 186, 116)  // #FDBA74
);

themed_color!(
    /// Variable Analysis group: light = emerald-500 / dark = emerald-300
    GROUP_VARIABLE_ANALYSIS,
    Color32::from_rgb(16, 185, 129),  // #10B981
    Color32::from_rgb(110, 231, 183)  // #6EE7B7
);

themed_color!(
    /// Statistics group: light = violet-400 / dark = violet-300
    GROUP_STATISTICS,
    Color32::from_rgb(167, 139, 250), // #A78BFA
    Color32::from_rgb(196, 181, 253)  // #C4B5FD
);

themed_color!(
    /// Response Surface (model-based) group: light = pink-400 / dark = pink-300
    GROUP_RESPONSE_SURFACE,
    Color32::from_rgb(244, 114, 182), // #F472B6
    Color32::from_rgb(249, 168, 212)  // #F9A8D4
);

themed_color!(
    /// Optimization group: light = amber-500 / dark = amber-300
    GROUP_OPTIMIZATION,
    Color32::from_rgb(245, 158, 11),  // #F59E0B
    Color32::from_rgb(252, 211, 77)   // #FCD34D
);

themed_color!(
    /// Clustering group: light = teal-500 / dark = teal-300
    GROUP_CLUSTERING,
    Color32::from_rgb(20, 184, 166),  // #14B8A6
    Color32::from_rgb(94, 234, 212)   // #5EEAD4
);

themed_color!(
    /// MCDM group: light = red-400 / dark = red-300
    GROUP_MCDM,
    Color32::from_rgb(248, 113, 113), // #F87171
    Color32::from_rgb(252, 165, 165)  // #FCA5A5
);

themed_color!(
    /// Artifacts / Data group: light = indigo-400 / dark = indigo-300
    GROUP_ARTIFACTS,
    Color32::from_rgb(129, 140, 248), // #818CF8
    Color32::from_rgb(165, 180, 252)  // #A5B4FC
);

// ====================================================================
// Accent colors (TONMANUAL §2 Primary Color)
// ====================================================================

themed_color!(
    /// Main blue: blue-500 (same in both themes)
    ACCENT_BLUE,
    Color32::from_rgb(59, 130, 246), // #3B82F6
    Color32::from_rgb(59, 130, 246)
);

themed_color!(
    /// Selection highlight: light = blue-300 / dark = blue-800
    ACCENT_BLUE_MUTED,
    Color32::from_rgb(147, 197, 253), // #93C5FD
    Color32::from_rgb(30, 64, 175)    // #1E40AF
);

// ====================================================================
// Text / Border (TONMANUAL §2 Secondary Color)
// ====================================================================

themed_color!(
    /// Heading text: light = gray-900 / dark = gray-100
    TEXT_PRIMARY,
    Color32::from_rgb(17, 24, 39),    // #111827
    Color32::from_rgb(243, 244, 246)  // #F3F4F6
);

themed_color!(
    /// Body text: light = gray-600 / dark = gray-400
    TEXT_SECONDARY,
    Color32::from_rgb(75, 85, 99),    // #4B5563
    Color32::from_rgb(156, 163, 175)  // #9CA3AF
);

themed_color!(
    /// Border: light = gray-200 / dark = gray-700
    BORDER_COLOR,
    Color32::from_rgb(229, 231, 235), // #E5E7EB
    Color32::from_rgb(55, 65, 81)     // #374151
);

// ====================================================================
// Semantic colors
// ====================================================================

themed_color!(
    /// Error display color: not a brand color target (same in both themes)
    ERROR_COLOR,
    Color32::from_rgb(234, 67, 53),
    Color32::from_rgb(234, 67, 53)
);

themed_color!(
    /// Warning display color (non-blocking notice): light = amber-600 / dark = amber-400
    WARNING_COLOR,
    Color32::from_rgb(217, 119, 6),   // #D97706
    Color32::from_rgb(251, 191, 36)   // #FBBF24
);
