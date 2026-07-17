//! Radar comparison widget that overlays pinned trials.
//!
//! Axes are the objective functions (always shown) + numeric parameters
//! (added via a toggle). Each axis is min-max normalized over the whole
//! Study's numeric column, and if "Outward = better" is enabled, only
//! minimize-objective axes are flipped (so "outward = better" is
//! consistent across all axes). The drawing itself is delegated to
//! [`crate::ui::widgets::common::radar_chart::draw_radar`], shared with the
//! trial detail modal's radar chart.
//! See `theory/{en,ja}/widgets/radar-comparison.md` for details.

use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::COLOR_EMPTY_STATE;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::radar_chart::{draw_radar, swatch, RadarSeries};
use crate::ui::widgets::common::range_math::finite_value_range;

/// UI state for the radar comparison widget. Holds no computation cache
/// (recomputing a handful of polygons every frame is cheap enough).
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RadarComparisonChart {
    /// Whether to also include numeric parameters as axes (default is
    /// objectives only).
    pub include_params: bool,
    /// Whether to flip minimize-objective axes so "outward = better" is consistent.
    pub outward_better: bool,
}

impl Default for RadarComparisonChart {
    fn default() -> Self {
        Self {
            include_params: false,
            outward_better: true,
        }
    }
}

/// Information for a single radar axis (including a borrow of the column).
/// Shared between `show` and CSV export.
pub struct AxisInfo<'a> {
    pub name: &'a str,
    pub col: &'a [f64],
    pub is_objective: bool,
    /// For an objective-function axis, the index within `directions` (used
    /// to decide whether to flip).
    pub obj_idx: Option<usize>,
}

/// Builds the axis list (objectives, then numeric parameters). Objectives
/// / parameters without a numeric column are skipped. If `include_params`
/// is false, no parameter axes are added.
pub fn build_axes<'a>(
    view: &'a StudyView,
    param_names: &'a [String],
    obj_names: &'a [String],
    include_params: bool,
) -> Vec<AxisInfo<'a>> {
    let mut axes = Vec::with_capacity(obj_names.len() + param_names.len());
    for (i, name) in obj_names.iter().enumerate() {
        if let Some(col) = view.numeric_column(name) {
            axes.push(AxisInfo {
                name,
                col,
                is_objective: true,
                obj_idx: Some(i),
            });
        }
    }
    if include_params {
        for name in param_names {
            if let Some(col) = view.numeric_column(name) {
                axes.push(AxisInfo {
                    name,
                    col,
                    is_objective: false,
                    obj_idx: None,
                });
            }
        }
    }
    axes
}

/// The axis's value range (min/max excluding non-finite values). If the
/// range is empty, returns `(0.0, 0.0)` (treated as degenerate).
fn axis_range(col: &[f64]) -> (f64, f64) {
    finite_value_range(col.iter().copied()).unwrap_or((0.0, 0.0))
}

/// Normalizes a value to a radial fraction [0,1] on the axis. Returns 0.5
/// when `min == max` (degenerate). If `flip` is true, applies `u -> 1 - u`
/// (for "outward = better").
pub fn normalize(v: f64, min: f64, max: f64, flip: bool) -> f64 {
    let span = max - min;
    let u = if span.abs() <= f64::EPSILON {
        0.5
    } else {
        ((v - min) / span).clamp(0.0, 1.0)
    };
    if flip {
        1.0 - u
    } else {
        u
    }
}

impl RadarComparisonChart {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        pinned_trials: &[u32],
    ) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.include_params, "Include parameters");
            ui.checkbox(&mut self.outward_better, "Outward = better")
                .on_hover_text("Flip minimized-objective axes so larger polygons are better");
        });

        let axes = build_axes(view, param_names, obj_names, self.include_params);
        if axes.len() < 3 {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "Radar needs at least 3 axes — enable parameters.",
                );
            });
            return;
        }

        if pinned_trials.is_empty() {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "Pin trials (📌) in the Trial Table to compare them here.",
                );
            });
            return;
        }

        let ranges: Vec<(f64, f64)> = axes.iter().map(|a| axis_range(a.col)).collect();
        let axis_labels: Vec<(String, bool)> = axes
            .iter()
            .map(|a| (a.name.to_string(), a.is_objective))
            .collect();
        let cmap = ColorMap::turbo();
        let n_pins = pinned_trials.len();

        let mut series: Vec<RadarSeries> = Vec::with_capacity(n_pins);
        let mut legend_entries: Vec<(egui::Color32, String)> = Vec::with_capacity(n_pins);
        for (pin_idx, &trial_id) in pinned_trials.iter().enumerate() {
            let Some(row) = view.trial_ids.iter().position(|&t| t == trial_id) else {
                continue;
            };
            let fractions: Vec<Option<f32>> = axes
                .iter()
                .enumerate()
                .map(|(k, axis)| {
                    let raw = axis.col.get(row).copied().unwrap_or(f64::NAN);
                    let (lo, hi) = ranges[k];
                    let flip = self.outward_better
                        && axis.is_objective
                        && axis
                            .obj_idx
                            .and_then(|oi| directions.get(oi))
                            .map(|d| matches!(d, Direction::Minimize))
                            .unwrap_or(false);
                    let u = if raw.is_finite() {
                        normalize(raw, lo, hi, flip)
                    } else {
                        0.5
                    };
                    Some(u as f32)
                })
                .collect();

            let number = view.df.get_trial_number(row).unwrap_or(trial_id);
            let label = format!("Trial #{number}");
            let color = cmap.sample_categorical(pin_idx, n_pins);
            legend_entries.push((color, label.clone()));
            series.push(RadarSeries {
                color,
                fractions,
                // No emphasis (fan-mesh fill + dots); use only a thicker
                // outline so multiple overlaid trials stay distinguishable.
                width: 2.0,
                emphasized: false,
            });
        }

        draw_radar(ui, &axis_labels, &series);

        // ── Legend (color swatch + trial number for each pinned trial) ──────
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (color, label) in &legend_entries {
                swatch(ui, *color);
                ui.label(egui::RichText::new(label).small());
                ui.add_space(10.0);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_objectives_only_and_outward_better() {
        let chart = RadarComparisonChart::default();
        assert!(!chart.include_params);
        assert!(chart.outward_better);
    }

    #[test]
    fn normalize_normal_case() {
        assert!((normalize(5.0, 0.0, 10.0, false) - 0.5).abs() < 1e-9);
        assert!((normalize(2.0, 0.0, 10.0, false) - 0.2).abs() < 1e-9);
    }

    #[test]
    fn normalize_flip_case() {
        assert!((normalize(5.0, 0.0, 10.0, true) - 0.5).abs() < 1e-9);
        assert!((normalize(2.0, 0.0, 10.0, true) - 0.8).abs() < 1e-9);
    }

    #[test]
    fn normalize_degenerate_range_is_mid_radius() {
        assert!((normalize(5.0, 3.0, 3.0, false) - 0.5).abs() < 1e-9);
        assert!((normalize(5.0, 3.0, 3.0, true) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_clamps_out_of_range_values() {
        assert!((normalize(-5.0, 0.0, 10.0, false) - 0.0).abs() < 1e-9);
        assert!((normalize(15.0, 0.0, 10.0, false) - 1.0).abs() < 1e-9);
    }
}
