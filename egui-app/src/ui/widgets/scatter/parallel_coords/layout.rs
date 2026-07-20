//! Pure geometry and axis-layout helpers for the parallel coordinates chart.

use crate::ui::widgets::common::range_math;

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
