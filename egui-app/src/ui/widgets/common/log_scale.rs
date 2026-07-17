//! Common helpers for drawing a log-scale Y axis.
//!
//! On a log scale, values are plotted after a log10 transform, and the Y-axis
//! labels display the original pre-transform value (restored via 10^mark).
//! Factored out here so multiple charts (optimization history, Slice, etc.)
//! can share the same tick placement and label formatting.

/// A grid spacer that places powers of 10 as major ticks on a log10-transformed
/// axis (plot coordinate = log10(value)). Within each decade (10^k to 10^(k+1)),
/// places minor ticks at 2x-9x, distinguishing line thickness by `step_size`
/// (major > minor).
pub fn log10_grid_spacer(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let (min, max) = input.bounds;
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Vec::new();
    }

    // Integer exponents of the decades covering the display range (log10 space).
    // Keep this limited to just covering the visible range, so ticks
    // (especially labels) don't spill outside it.
    let start = min.floor() as i64;
    let end = max.ceil() as i64;

    // If there are too many decades, thin down to major ticks (powers of 10) only.
    let decade_span = end - start;
    let majors_only = decade_span > 12;

    // Only keep ticks within the visible range (allow a small margin past the edges).
    let eps = (max - min) * 1e-9;
    let in_bounds = |v: f64| v >= min - eps && v <= max + eps;

    let mut marks = Vec::new();
    for exp in start..=end {
        let decade = 10f64.powi(exp as i32);
        // Major tick: 10^exp. step_size is the full decade width (1.0 in log10 space).
        if in_bounds(exp as f64) {
            marks.push(egui_plot::GridMark {
                value: exp as f64,
                step_size: 1.0,
            });
        }
        if majors_only {
            continue;
        }
        // Minor ticks: 2x, 3x, ... 9x 10^exp. Position in log10 space is exp + log10(m).
        for m in 2..=9 {
            let value = (decade * m as f64).log10();
            if in_bounds(value) {
                marks.push(egui_plot::GridMark {
                    value,
                    step_size: 0.1,
                });
            }
        }
    }
    marks
}

/// Formats a log-scale Y-axis label for readability, based on the original
/// value (restored via 10^mark). Large/small values use exponential notation;
/// the middle range uses fixed-point notation with a digit count based on magnitude.
pub fn format_log_tick(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    if !(1e-4..1e5).contains(&abs) {
        // Outside the display range, use exponential notation (e.g. 1.2e-5, 3.4e6)
        format!("{value:.1e}")
    } else if abs >= 100.0 {
        format!("{value:.0}")
    } else if abs >= 1.0 {
        format!("{value:.1}")
    } else {
        // Below 1, increase decimal digits to preserve significant figures
        format!("{value:.3}")
    }
}

/// Applies the grid spacer / label formatting for a log-scale Y axis to `plot`.
/// Only major ticks (powers of 10) get labels; minor ticks (2x-9x) are lines only.
pub fn apply_log_y_axis(plot: egui_plot::Plot<'_>) -> egui_plot::Plot<'_> {
    plot.y_grid_spacer(log10_grid_spacer)
        .y_axis_formatter(|mark, _range| {
            // Only major ticks (powers of 10 = integers in log10 space) get
            // labels; minor ticks (2x-9x) are lines only, with no label.
            if (mark.value - mark.value.round()).abs() > 1e-6 {
                return String::new();
            }
            let original = 10f64.powf(mark.value.round());
            format_log_tick(original)
        })
}

/// Applies the grid spacer / label formatting for a log-scale X axis to `plot`.
/// Same logic as `apply_log_y_axis`, only the target axis differs (used for
/// the X-axis log scale in the EDF plot).
pub fn apply_log_x_axis(plot: egui_plot::Plot<'_>) -> egui_plot::Plot<'_> {
    plot.x_grid_spacer(log10_grid_spacer)
        .x_axis_formatter(|mark, _range| {
            if (mark.value - mark.value.round()).abs() > 1e-6 {
                return String::new();
            }
            let original = 10f64.powf(mark.value.round());
            format_log_tick(original)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_log_tick_restores_original_scale() {
        assert_eq!(format_log_tick(0.0), "0");
        assert_eq!(format_log_tick(1.0), "1.0");
        assert_eq!(format_log_tick(550.0), "550");
        assert_eq!(format_log_tick(2.5), "2.5");
        assert_eq!(format_log_tick(0.001), "0.001");
        assert_eq!(format_log_tick(1_000_000.0), "1.0e6");
    }

    #[test]
    fn log10_grid_spacer_places_decades_as_majors() {
        let input = egui_plot::GridInput {
            bounds: (0.0, 3.0),
            base_step_size: 0.01,
        };
        let marks = log10_grid_spacer(input);
        let majors: Vec<f64> = marks
            .iter()
            .filter(|m| m.step_size == 1.0)
            .map(|m| m.value)
            .collect();
        for exp in [0.0, 1.0, 2.0, 3.0] {
            assert!(
                majors.iter().any(|&v| (v - exp).abs() < 1e-9),
                "missing decade major at 10^{exp}"
            );
        }
        assert!(marks.iter().any(|m| m.step_size < 1.0));
    }

    #[test]
    fn log10_grid_spacer_thins_to_majors_for_wide_range() {
        let input = egui_plot::GridInput {
            bounds: (-10.0, 10.0),
            base_step_size: 0.1,
        };
        let marks = log10_grid_spacer(input);
        assert!(marks.iter().all(|m| m.step_size == 1.0));
    }
}
