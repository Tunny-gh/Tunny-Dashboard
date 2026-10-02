//! Pure geometry and axis-layout helpers for the parallel coordinates chart.

use crate::ui::widgets::common::range_math;

pub(super) struct AxisLabelLayout {
    pub axis_x: Vec<f32>,
    pub axis_top: f32,
    pub axis_bottom: f32,
    pub label_pos: Vec<egui::Pos2>,
    pub angle: f32,
}

/// Reserve the full label bounds, including glyph overhang, at both outer
/// axes and above the plot. In cramped charts the clip may cut off text, but
/// the axis geometry must remain ordered with a positive normalization span.
pub(super) fn axis_label_layout(
    available: egui::Rect,
    galleys: &[std::sync::Arc<egui::Galley>],
) -> AxisLabelLayout {
    let max_width = galleys.iter().map(|g| g.size().x).fold(0.0_f32, f32::max);
    let horizontal_margin = 40.0_f32.max(max_width * 0.5 + 2.0);
    let spacing =
        (available.width() - 2.0 * horizontal_margin).max(0.0) / (galleys.len() - 1) as f32;
    let angle = if max_width > spacing - 4.0 {
        -std::f32::consts::FRAC_PI_4
    } else {
        0.0
    };
    let mut offsets = Vec::with_capacity(galleys.len());
    let (mut left, mut right, mut height) = (40.0_f32, 40.0_f32, 0.0_f32);
    for galley in galleys {
        let offset = if angle == 0.0 {
            egui::vec2(-galley.size().x * 0.5, -galley.size().y)
        } else {
            let lowest = crate::ui::widgets::common::axis_labels::rotated_label_corners(
                galley.size(),
                angle,
            )
            .lowest;
            -egui::vec2(lowest.0, lowest.1)
        };
        let bounds = galley
            .rect
            .union(galley.mesh_bounds)
            .rotate_bb(egui::emath::Rot2::from_angle(angle))
            .translate(offset);
        left = left.max(-bounds.left() + 2.0);
        right = right.max(bounds.right() + 2.0);
        height = height.max(-bounds.top() + bounds.bottom().max(0.0));
        offsets.push(offset);
    }
    let margin_scale = ((available.width() - 1.0) / (left + right)).clamp(0.0, 1.0);
    left *= margin_scale;
    right *= margin_scale;
    let axis_x: Vec<_> = (0..galleys.len())
        .map(|i| {
            available.left()
                + left
                + (available.width() - left - right).max(0.0) * i as f32
                    / (galleys.len() - 1) as f32
        })
        .collect();
    let axis_bottom = available.bottom() - 10.0;
    let axis_top = available.top() + (height + 8.0).min(available.height() - 11.0);
    let label_pos = axis_x
        .iter()
        .zip(offsets)
        .map(|(&x, offset)| egui::pos2(x, axis_top - 4.0) + offset)
        .collect();
    AxisLabelLayout {
        axis_x,
        axis_top,
        axis_bottom,
        label_pos,
        angle,
    }
}

/// Formats an axis tick value with precision scaled to the value range.
pub fn fmt_tick_value(v: f64, mn: f64, mx: f64) -> String {
    let range = (mx - mn).abs();
    if range < 1e-9 {
        format!("{:.3}", v)
    } else if v.abs() >= 10_000.0 || (v.abs() < 0.001 && v.abs() > 0.0) {
        format!("{:.2e}", v)
    } else if range < 0.01 {
        format!("{:.4}", v)
    } else if range < 1.0 {
        format!("{:.3}", v)
    } else {
        format!("{:.2}", v)
    }
}

/// Normalizes a value to [0, 1] (returns 0.5 when min == max).
pub fn normalize_value(v: f64, v_min: f64, v_max: f64) -> f32 {
    range_math::normalize01(v, v_min, v_max)
}

/// Converts a normalized value [0,1] to a screen Y coordinate (0 = bottom, 1 = top).
pub fn normalized_to_screen_y(normalized: f32, plot_top: f32, plot_bottom: f32) -> f32 {
    plot_bottom - normalized * (plot_bottom - plot_top)
}

/// Builds the list of axis display names from parameter names and objective names.
pub fn build_axis_order(param_names: &[String], objective_names: &[String]) -> Vec<String> {
    param_names
        .iter()
        .chain(objective_names.iter())
        .cloned()
        .collect()
}

/// Returns the original indices of the axes to draw (visible ones), based on
/// `axis_visibility`. Unregistered axes default to visible (`unwrap_or(true)`),
/// so all axes are visible by default.
pub fn visible_axis_indices(
    all_names: &[String],
    axis_visibility: &std::collections::HashMap<String, bool>,
) -> Vec<usize> {
    (0..all_names.len())
        .filter(|&i| axis_visibility.get(&all_names[i]).copied().unwrap_or(true))
        .collect()
}

/// Computes the normalization range used for coloring from feasible solutions only.
/// Kept separate from the axis coordinate range so infeasible outliers don't
/// compress the colormap. Uses all values when there are no constraints
/// (feas.has_constraints() == false); returns `fallback` if no valid value exists.
pub fn feasible_color_range(
    col: &[f64],
    feas: tunny_core::dataframe::Feasibility<'_>,
    fallback: (f64, f64),
) -> (f64, f64) {
    let (mn, mx) = col
        .iter()
        .enumerate()
        .filter(|(idx, v)| v.is_finite() && feas.is_feasible(*idx))
        .map(|(_, &v)| v)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), v| {
            (mn.min(v), mx.max(v))
        });
    if mn <= mx {
        (mn, mx)
    } else {
        fallback
    }
}
