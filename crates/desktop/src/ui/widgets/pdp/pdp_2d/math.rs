//! Pure numeric helpers for the 2D PDP widget: axis/value range extraction, value
//! normalization, parameter-selection validation, and the 95% CI band computation.
//! None of these functions touch `egui` or drawing state.

use crate::ui::widgets::common::range_math;

use super::BandGrids;

/// Returns the value range [min, max] from axis grid values (an ascending linspace)
pub(crate) fn axis_range_of(values: &[f64]) -> (f64, f64) {
    match range_math::value_range(values.iter().copied()) {
        Some((mn, mx)) if mn.is_finite() && mx.is_finite() => (mn, mx),
        _ => (-1.0, 1.0),
    }
}

/// Normalizes a value to [0.0, 1.0]
pub(crate) fn normalize_value(v: f64, v_min: f64, v_max: f64) -> f32 {
    range_math::normalize01(v, v_min, v_max)
}

/// Returns the value range [min, max] of a value grid.
/// `value_range_of` differs from the heatmap side's `value_range` in that it does not
/// expand a degenerate range (min==max), so it does not use the shared helper's
/// degenerate-range expansion.
pub(crate) fn value_range_of(values: &[Vec<f64>]) -> (f64, f64) {
    range_math::value_range(values.iter().flatten().copied()).unwrap_or((0.0, 1.0))
}

/// Checks that param1 and param2 are different (returns false if identical)
pub(crate) fn check_params_different(p1: &str, p2: &str) -> bool {
    !p1.is_empty() && !p2.is_empty() && p1 != p2
}

/// Builds the lower/upper grids for the 95% CI from the Mean grid and variance grid.
/// A variance that is negative due to numerical error is treated as 0 (avoids producing NaN).
/// Ragged rows are truncated to the shorter length.
pub(crate) fn band_grids(z_values: &[Vec<f64>], variances: &[Vec<f64>]) -> BandGrids {
    let mut lower = Vec::with_capacity(z_values.len());
    let mut upper = Vec::with_capacity(z_values.len());
    for (z_row, var_row) in z_values.iter().zip(variances.iter()) {
        let mut l_row = Vec::with_capacity(z_row.len());
        let mut u_row = Vec::with_capacity(z_row.len());
        for (&z, &var) in z_row.iter().zip(var_row.iter()) {
            let sigma = var.max(0.0).sqrt();
            l_row.push(z - 1.96 * sigma);
            u_row.push(z + 1.96 * sigma);
        }
        lower.push(l_row);
        upper.push(u_row);
    }
    (lower, upper)
}
