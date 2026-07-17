//! Theme-following definitions for chart drawing colors.
//!
//! Data series colors (blue/red/green/gold) are shared across both themes;
//! only background, text, grid, and semi-transparent grayout colors have
//! separate dark values. Semi-transparent colors use premultiplied alpha
//! (`from_rgba_premultiplied`), tuned for visibility on a white background in
//! light mode and a gray-900 background in dark mode.
//! All are functions in the `NAME()` form, following [`crate::theme::is_dark_mode`].

use egui::Color32;

use super::themed_color;

// ====================================================================
// Pareto colors
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
    /// Dimmed color for non-Pareto points.
    /// light premultiplied: r=66*60/255≈16, g=133*60/255≈31, b=244*60/255≈57
    /// dark uses a slightly higher alpha (80) to keep visibility on a dark background.
    COLOR_NON_PARETO_DIM,
    Color32::from_rgba_premultiplied(16, 31, 57, 60),
    Color32::from_rgba_premultiplied(21, 42, 77, 80)
);

// ====================================================================
// 3D axis colors
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
// MCDM score tier colors
// ====================================================================

/// Dominated solutions (outside the ranked range) are shown in a pale light
/// blue, matching the non-Pareto color used in the cluster plot.
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_MCDM_NONE() -> Color32 {
    COLOR_NON_PARETO_DIM()
}

// ====================================================================
// Bar chart colors
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
    /// Bar color for importance_chart. light uses dark navy; dark switches to
    /// a bright steel blue since dark navy would sink into a dark background.
    COLOR_IMPORTANCE_BAR,
    Color32::from_rgb(30, 60, 114),
    Color32::from_rgb(91, 141, 239)
);

// ====================================================================
// Optimization history colors
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
// Convergence metric chart colors
// ====================================================================

themed_color!(
    COLOR_CONVERGENCE_LINE,
    Color32::from_rgb(52, 168, 83),
    Color32::from_rgb(52, 168, 83)
);

// ====================================================================
// Fit quality colors
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
// PDP colors
// ====================================================================

themed_color!(
    COLOR_PDP_LINE,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

themed_color!(
    /// PDP confidence interval band.
    /// light premultiplied: r=66*50/255≈13, g=133*50/255≈26, b=244*50/255≈48
    COLOR_PDP_CI,
    Color32::from_rgba_premultiplied(13, 26, 48, 50),
    Color32::from_rgba_premultiplied(18, 37, 67, 70)
);

themed_color!(
    /// ICE lines. light is semi-transparent dark gray, dark is semi-transparent light gray.
    /// light premultiplied: r=150*60/255≈35 (all 3 components)
    /// dark premultiplied: r=200*60/255≈47 (all 3 components)
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
// Scatter plot colors
// ====================================================================

themed_color!(
    COLOR_SCATTER_DOT,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(66, 133, 244)
);

// ====================================================================
// Selection highlight colors
// ====================================================================

themed_color!(
    /// Selection highlight surface.
    /// light premultiplied: r=66*40/255≈10, g=133*40/255≈21, b=244*40/255≈38
    COLOR_SELECTION_HIGHLIGHT,
    Color32::from_rgba_premultiplied(10, 21, 38, 40),
    Color32::from_rgba_premultiplied(16, 31, 57, 60)
);

themed_color!(
    /// Mid-gray used for scatter points outside the selection filter (drops
    /// the original hue to clearly distinguish them from selected points).
    /// Semi-transparent to keep selected points prominent, while keeping
    /// enough opacity to be recognizable as gray.
    /// premultiplied: rgb(150,150,150) x 90/255 ~= 53 (all 3 components)
    COLOR_UNSELECTED_POINT,
    Color32::from_rgba_premultiplied(53, 53, 53, 90),
    Color32::from_rgba_premultiplied(53, 53, 53, 90)
);

// ====================================================================
// Link color
// ====================================================================

themed_color!(
    COLOR_LINK,
    Color32::from_rgb(66, 133, 244),
    Color32::from_rgb(102, 157, 246)
);

// ====================================================================
// 3D view colors
// ====================================================================

themed_color!(
    /// 3D view background. light = pale gray / dark = gray-800.
    COLOR_3D_BG,
    Color32::from_rgb(240, 242, 245),
    Color32::from_rgb(31, 41, 55) // #1F2937
);

themed_color!(
    /// 3D grid lines.
    /// light premultiplied: r=120*70/255≈33, g=33, b=130*70/255≈36
    /// dark premultiplied: rgb(170,170,180) x 70/255 ~= 47,47,49
    COLOR_3D_GRID,
    Color32::from_rgba_premultiplied(33, 33, 36, 70),
    Color32::from_rgba_premultiplied(47, 47, 49, 70)
);

// ====================================================================
// Highlighted trial point (shared by pareto_2d / pareto_3d)
// ====================================================================

themed_color!(
    COLOR_HIGHLIGHT_PT,
    Color32::from_rgb(124, 77, 255),
    Color32::from_rgb(149, 117, 255)
);

// ====================================================================
// Parallel coordinates colors
// ====================================================================

themed_color!(
    /// Axis tick text: light = gray-600 equivalent / dark = gray-400
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
    /// Axis line: light = pale gray / dark = gray-600
    COLOR_PARALLEL_AXIS,
    Color32::from_rgb(218, 220, 224),
    Color32::from_rgb(75, 85, 99) // #4B5563
);

themed_color!(
    /// Color for graying out lines outside the brush selection range (premultiplied).
    /// light premultiplied: rgb(170,170,170) x 14/255 ~= 9 (all 3 components)
    /// dark premultiplied: rgb(200,200,200) x 14/255 ~= 11 (all 3 components)
    COLOR_PARALLEL_LINE_UNSELECTED,
    Color32::from_rgba_premultiplied(9, 9, 9, 14),
    Color32::from_rgba_premultiplied(11, 11, 11, 14)
);

// ====================================================================
// PDP CI band legend marker (for egui_plot, non-premultiplied equivalent)
// ====================================================================

themed_color!(
    /// premultiplied: r=66*120/255≈31, g=133*120/255≈63, b=244*120/255≈115
    COLOR_PDP_CI_LEGEND,
    Color32::from_rgba_premultiplied(31, 63, 115, 120),
    Color32::from_rgba_premultiplied(31, 63, 115, 120)
);

// ====================================================================
// General-purpose chart colors
// ====================================================================

themed_color!(
    /// Text inside chart cells (heatmap, correlation coefficient cells, etc.)
    COLOR_CHART_TEXT,
    Color32::from_rgb(32, 33, 36),
    Color32::from_rgb(229, 231, 235) // #E5E7EB
);

themed_color!(
    /// Display color for no-data/empty state: light = gray-600 equivalent / dark = gray-400
    COLOR_EMPTY_STATE,
    Color32::from_rgb(95, 99, 104),
    Color32::from_rgb(156, 163, 175) // #9CA3AF
);

// ====================================================================
// Surrogate multi-objective front (pareto_2d overlay)
// ====================================================================

themed_color!(
    /// Color for surrogate-predicted Pareto front points (gold-ish, to avoid
    /// clashing with the existing red/blue).
    COLOR_SURROGATE_FRONT,
    Color32::from_rgb(255, 193, 7),
    Color32::from_rgb(255, 193, 7)
);

themed_color!(
    /// Grid line color for heatmap/matrix-style charts: light = gray-500 equivalent / dark = gray-600
    COLOR_GRID_STROKE,
    Color32::from_rgb(154, 160, 166),
    Color32::from_rgb(75, 85, 99) // #4B5563
);

// ====================================================================
// Infeasible solutions (constraint violation)
// ====================================================================

themed_color!(
    /// Grayout color for infeasible solutions (premultiplied)
    /// premultiplied: rgb(180,180,180) x 80/255 ~= 56 (all 3 components)
    COLOR_INFEASIBLE,
    Color32::from_rgba_premultiplied(56, 56, 56, 80),
    Color32::from_rgba_premultiplied(56, 56, 56, 80)
);

// ====================================================================
// Trial state colors (shared by Intermediate Values / Timeline)
// ====================================================================

/// COMPLETE trial (finished normally). Reuses the same blue used for "All Trials" in the optimization history.
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_COMPLETE() -> Color32 {
    COLOR_OPT_TRIAL()
}

/// PRUNED trial. Reuses the bar chart's accent color (gold-ish), treated as orange.
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_PRUNED() -> Color32 {
    COLOR_BAR_ACCENT()
}

/// RUNNING trial. Reuses the neutral gray (same color as the empty state display).
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_RUNNING() -> Color32 {
    COLOR_EMPTY_STATE()
}

/// FAIL trial. Reuses the bar chart's warning color (red).
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_FAIL() -> Color32 {
    COLOR_BAR_NEGATIVE()
}

/// WAITING trial (not yet started). Reuses the same light gray as the grid lines.
#[allow(non_snake_case)]
#[inline]
pub fn COLOR_STATE_WAITING() -> Color32 {
    COLOR_GRID_STROKE()
}
