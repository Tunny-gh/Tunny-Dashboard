//! Scatter plot rendering: precomputed display batches and the egui_plot drawing routine.

use std::collections::{HashMap, HashSet};

use crate::theme::chart_colors::{COLOR_MCDM_NONE, COLOR_UNSELECTED_POINT};
use crate::theme::color_compute::{key_to_color32, point_alpha_in_set, rgba_key};
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{hit_test_nearest, HIT_THRESHOLD};
use egui::Color32;

use super::points::ScatterPoint;

// ──────────────────────────────────────────────────────────────
// Scatter plot rendering
// ──────────────────────────────────────────────────────────────

/// Precomputed display batches (independent of the selection filter · M-17).
///
/// The previous implementation rebuilt a `HashMap` of "color -> points" plus a luminance
/// sort every frame, but this classification doesn't depend on the selection filter, so
/// it is now computed once when the cache is rebuilt and kept around. Only the dimming
/// caused by the selection filter (PCP brush, etc.) is applied lightly at render time via
/// a `HashSet` (M-16).
pub(super) struct DisplayBatches {
    /// Color batches sorted by ascending luminance (ranked feasible points).
    /// Each point is `(trial_id, [x, y])`. `trial_id` is used for selection filter checks.
    color_batches: Vec<(Color32, BatchPoints)>,
    /// Unranked (COLOR_MCDM_NONE) feasible points.
    none_pts: BatchPoints,
}

/// List of points in a display batch. Each point is `(trial_id, [x, y])`.
type BatchPoints = Vec<(u32, [f64; 2])>;

/// Builds the precomputed display batches (`DisplayBatches`).
///
/// The `HashMap` classification by color and luminance sorting are done once here, and
/// are not recomputed except when the cache is rebuilt (M-17). Independent of the
/// selection filter.
pub(super) fn build_display_batches(points: &[ScatterPoint]) -> DisplayBatches {
    let mut none_pts: BatchPoints = Vec::new();
    // color -> coordinate list (also keeps the u32 luminance value for sorting)
    let mut color_groups: HashMap<[u8; 4], (BatchPoints, u32)> = HashMap::new();

    for &(x, y, color, trial_id) in points {
        if color == COLOR_MCDM_NONE() {
            none_pts.push((trial_id, [x, y]));
        } else {
            let key = rgba_key(color);
            let lum = color.r() as u32 + color.g() as u32 + color.b() as u32;
            let entry = color_groups.entry(key).or_insert((Vec::new(), lum));
            entry.0.push((trial_id, [x, y]));
        }
    }

    // Sort by luminance (draw dark-to-light, so lighter points end up on top)
    let mut sorted: Vec<_> = color_groups.into_iter().collect();
    sorted.sort_by_key(|(_, (_, lum))| *lum);
    let color_batches = sorted
        .into_iter()
        .map(|(key, (pts, _))| (key_to_color32(key), pts))
        .collect();

    DisplayBatches {
        color_batches,
        none_pts,
    }
}

/// Renders the scatter plot and returns `(trial_id, row index)` if a point was clicked.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_scatter_plot(
    ui: &mut egui::Ui,
    batches: &DisplayBatches,
    infeasible: &[(f64, f64)],
    hit_candidates: &[(u32, usize, [f64; 2])],
    x_label: &str,
    y_label: &str,
    colormap: &ColorMap,
    top_n: usize,
    selected_indices: &[u32],
) -> Option<(u32, usize)> {
    // When a selection filter (PCP brush, etc.) is active, points outside the selection
    // are dimmed and drawn together in the back. Scores/colors remain based on the full
    // front; branching here only affects visual emphasis.
    // Only dimming via the selection set (HashSet) is applied to the precomputed batches
    // (M-16).
    let selected: HashSet<u32> = selected_indices.iter().copied().collect();
    let mut dim_pts: Vec<[f64; 2]> = Vec::new();
    let mut none_pts: Vec<[f64; 2]> = Vec::new();
    for &(trial_id, pt) in &batches.none_pts {
        if point_alpha_in_set(trial_id, &selected) != 255 {
            dim_pts.push(pt);
        } else {
            none_pts.push(pt);
        }
    }
    // Keep ascending luminance order while routing unselected points into dim_pts.
    let mut color_draw: Vec<(Color32, Vec<[f64; 2]>)> =
        Vec::with_capacity(batches.color_batches.len());
    for (color, pts) in &batches.color_batches {
        let mut drawn: Vec<[f64; 2]> = Vec::with_capacity(pts.len());
        for &(trial_id, pt) in pts {
            if point_alpha_in_set(trial_id, &selected) != 255 {
                dim_pts.push(pt);
            } else {
                drawn.push(pt);
            }
        }
        if !drawn.is_empty() {
            color_draw.push((*color, drawn));
        }
    }

    // Representative colors for the legend
    let best_color = colormap.interpolate(1.0);
    let worst_color = if top_n > 1 {
        colormap.interpolate(0.0)
    } else {
        best_color
    };
    // Always draw since visibility can be toggled from the legend
    let has_infeasible = !infeasible.is_empty();

    let mut clicked_detail: Option<(u32, usize)> = None;
    egui_plot::Plot::new("mcdm_scatter_plot")
        .unified_nav()
        .x_axis_label(x_label)
        .y_axis_label(y_label)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            // Detect the target for opening the detail modal on point click.
            let resp = plot_ui.response();
            if resp.clicked_by(egui::PointerButton::Primary) {
                clicked_detail = resp
                    .interact_pointer_pos()
                    .and_then(|pos| hit_test_nearest(plot_ui, hit_candidates, pos, HIT_THRESHOLD));
            }
            // Draw infeasible solutions in the back
            if has_infeasible {
                let pts: Vec<[f64; 2]> = infeasible.iter().map(|&(x, y)| [x, y]).collect();
                plot_ui.points(
                    egui_plot::Points::new("Infeasible", pts)
                        .color(crate::theme::chart_colors::COLOR_INFEASIBLE())
                        .radius(3.0),
                );
            }
            // Outside the selection filter (gray, drawn in back; grouped under
            // "Others (unselected)" in the legend)
            if !dim_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Others (unselected)", dim_pts)
                        .color(COLOR_UNSELECTED_POINT())
                        .radius(2.5),
                );
            }
            // Unranked (gray)
            if !none_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Others", none_pts)
                        .color(COLOR_MCDM_NONE())
                        .radius(3.0),
                );
            }
            // Ranked: dark (lower rank) to light (higher rank)
            for (color, pts) in color_draw {
                plot_ui.points(egui_plot::Points::new("", pts).color(color).radius(4.0));
            }
            // Legend-only entries (no data, name only)
            plot_ui.points(
                egui_plot::Points::new("Rank 1 (Best)", Vec::<[f64; 2]>::new())
                    .color(best_color)
                    .radius(5.0),
            );
            if top_n > 1 {
                plot_ui.points(
                    egui_plot::Points::new(format!("Rank {top_n}"), Vec::<[f64; 2]>::new())
                        .color(worst_color)
                        .radius(5.0),
                );
            }
        });
    clicked_detail
}
