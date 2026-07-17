use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_OPT_TRIAL;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{hit_test_nearest, HIT_THRESHOLD};

/// EDF series for one comparison Study (value column for the selected objective + color + legend name).
pub struct EdfComparison {
    pub name: String,
    pub color: egui::Color32,
    /// Objective value column for the selected objective (in COMPLETE-trial row order).
    pub values: Vec<f64>,
}

/// EDF (Empirical Distribution Function) chart widget
///
/// Corresponds to Optuna's `plot_edf`, and draws the empirical distribution (fraction of trials
/// with a value <= x) of the selected objective value as a step function. A steep curve means
/// values are concentrated, and a rightward shift (for minimization) indicates more bad results.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EdfPlotChart {
    pub obj_idx: usize,
    /// Toggle for X-axis log scale. When enabled, points with values <= 0 are excluded from the curve.
    pub log_x: bool,
    /// Cached result of `build_edf_points` (sort + two allocations).
    #[serde(skip)]
    cache: Option<EdfCache>,
}

/// Cache to avoid recomputing (O(n log n) sort) the EDF step point sequence.
///
/// The base series can be keyed by the identity of `view.df` (`Arc<DataFrame>`), but the
/// comparison series (`EdfComparison::values`) is built as a fresh `Vec<f64>` every frame by
/// the caller (render_chart.rs), so it has no stable pointer identity. Both are therefore keyed
/// by a content fingerprint of the value column (an FNV-1a-like fold; O(n) but no heap
/// allocation) — sufficiently cheap compared to a sort plus two allocations.
struct EdfCache {
    key: (usize, usize, bool, u64),
    base_points: Vec<[f64; 2]>,
    comparison_points: Vec<(String, egui::Color32, Vec<[f64; 2]>)>,
}

/// Folds a value column's contents into a cheap u64 fingerprint (FNV-1a-like; no heap allocation).
fn fingerprint_values(values: &[f64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ (values.len() as u64);
    for &v in values {
        h = (h ^ v.to_bits()).wrapping_mul(0x100000001b3);
    }
    h
}

/// Folds the content fingerprint of the comparison-Study list into the base fingerprint.
/// The series name is also included, so if the values are the same but the Study was
/// swapped, the key still differs.
fn fold_comparisons_fingerprint(mut h: u64, comparisons: &[EdfComparison]) -> u64 {
    for c in comparisons {
        h = h.wrapping_mul(0x100000001b3) ^ fingerprint_values(&c.values);
        for &b in c.name.as_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
    }
    h
}

impl EdfPlotChart {
    /// Draws the EDF chart. Overlays comparison Studies' EDF curves on the same graph.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        base_name: &str,
        comparisons: &[EdfComparison],
    ) {
        if obj_names.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No objectives.").weak());
            });
            return;
        }

        self.obj_idx = self.obj_idx.min(obj_names.len() - 1);

        ui.horizontal(|ui| {
            if obj_names.len() > 1 {
                ui.label("Objective:");
                egui::ComboBox::from_id_salt("edf_plot_obj_combo")
                    .selected_text(&obj_names[self.obj_idx])
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.obj_idx, i, name);
                        }
                    });
                ui.separator();
            }
            if ui.selectable_label(self.log_x, "Log Scale").clicked() {
                self.log_x = !self.log_x;
            }
        });

        let obj_name = &obj_names[self.obj_idx];
        let log_x = self.log_x;
        // numeric_column only borrows the DataFrame column with zero copies, so neither the
        // fingerprint computation nor the sort needs it converted to a Vec.
        let base_values: &[f64] = view.numeric_column(obj_name).unwrap_or(&[]);

        // Cache key: identity of view (DataFrame) + selected objective + log scale +
        // content fingerprint of the base/comparison value columns.
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;
        let fp = fold_comparisons_fingerprint(fingerprint_values(base_values), comparisons);
        let key = (df_ptr, self.obj_idx, log_x, fp);

        let cache_valid = self.cache.as_ref().is_some_and(|c| c.key == key);
        if !cache_valid {
            let base_points = build_edf_points(base_values, log_x);
            let comparison_points: Vec<(String, egui::Color32, Vec<[f64; 2]>)> = comparisons
                .iter()
                .map(|c| (c.name.clone(), c.color, build_edf_points(&c.values, log_x)))
                .collect();
            self.cache = Some(EdfCache {
                key,
                base_points,
                comparison_points,
            });
        }
        let cache = self.cache.as_ref().expect("cache just populated above");
        let base_points = &cache.base_points;
        let comparison_points = &cache.comparison_points;

        if base_points.is_empty() && comparison_points.iter().all(|(_, _, pts)| pts.is_empty()) {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No finite objective values to plot.").weak());
            });
            return;
        }

        // When the X axis uses a log scale, draw with values converted via log10
        // (build_edf_points has already excluded non-positive values, so log10 is always finite).
        let apply_log = |[x, y]: [f64; 2]| -> [f64; 2] { [if log_x { x.log10() } else { x }, y] };

        let base_label = if comparisons.is_empty() || base_name.is_empty() {
            "EDF"
        } else {
            base_name
        };

        let mut plot = egui_plot::Plot::new("edf_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .x_axis_label(obj_name)
            .y_axis_label("Cumulative Probability")
            .include_y(0.0)
            .include_y(1.0);
        if log_x {
            plot = crate::ui::widgets::common::log_scale::apply_log_x_axis(plot);
        }

        // Nearest hovered point (legend name, original value, cumulative fraction).
        let mut hovered: Option<(String, f64, f64)> = None;

        plot.show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);

            if let Some(pos) = plot_ui.response().hover_pos() {
                // Reuses the same shared hit test as other history widgets.
                // Candidates are laid out in drawing coordinates (after apply_log), and the
                // lookup index for the tooltip data (legend name, original value, cumulative
                // fraction) is embedded in the `usize` field.
                let mut hit_points: Vec<(u32, usize, [f64; 2])> = Vec::new();
                let mut lookup: Vec<(String, f64, f64)> = Vec::new();
                let mut push = |name: &str, pts: &[[f64; 2]]| {
                    for &p in pts {
                        let plot_pt = apply_log(p);
                        hit_points.push((0, lookup.len(), plot_pt));
                        lookup.push((name.to_string(), p[0], p[1]));
                    }
                };
                push(base_label, base_points);
                for (name, _, pts) in comparison_points {
                    push(name, pts);
                }
                hovered = hit_test_nearest(plot_ui, &hit_points, pos, HIT_THRESHOLD)
                    .and_then(|(_, i)| lookup.get(i).cloned());
            }

            if !base_points.is_empty() {
                let pts: egui_plot::PlotPoints =
                    base_points.iter().copied().map(apply_log).collect();
                plot_ui.line(
                    egui_plot::Line::new(base_label, pts)
                        .color(COLOR_OPT_TRIAL())
                        .width(1.5),
                );
            }
            for (name, color, pts) in comparison_points {
                if pts.is_empty() {
                    continue;
                }
                let plot_pts: egui_plot::PlotPoints = pts.iter().copied().map(apply_log).collect();
                plot_ui.line(
                    egui_plot::Line::new(name.as_str(), plot_pts)
                        .color(*color)
                        .width(1.5),
                );
            }
        });

        // EDF points are values on the curve rather than individual trials, so instead of the
        // shared `show_hover_tooltip` (whose heading is fixed to "Trial N"), draw a dedicated
        // tooltip with the series name as the heading.
        if let Some((name, value, frac)) = hovered {
            egui::Tooltip::always_open(
                ui.ctx().clone(),
                ui.layer_id(),
                egui::Id::new("edf_plot_hover_tooltip"),
                egui::PopupAnchor::Pointer,
            )
            .show(|ui| {
                ui.strong(name);
                egui::Grid::new("edf_plot_hover_grid")
                    .num_columns(2)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(obj_name.as_str())
                                .color(crate::theme::TEXT_SECONDARY()),
                        );
                        ui.label(format!("{value:.6}"));
                        ui.end_row();
                        ui.label(
                            egui::RichText::new("Cumulative Fraction")
                                .color(crate::theme::TEXT_SECONDARY()),
                        );
                        ui.label(format!("{frac:.4}"));
                        ui.end_row();
                    });
            });
        }
    }
}

/// Builds the EDF (Empirical Distribution Function) step point sequence.
///
/// - Values that are NaN / ±Inf are excluded.
/// - When `log_x` is true, values <= 0 are also excluded (they can't be shown on a log axis).
/// - The remaining values are sorted ascending, and equal values are grouped into a single step
///   (right-continuous: the cumulative fraction jumps up exactly at the point matching value `v`).
/// - y is the cumulative fraction in 0..1 (normalized by the post-filter count).
/// - If no values remain after filtering, returns empty.
pub fn build_edf_points(values: &[f64], log_x: bool) -> Vec<[f64; 2]> {
    let mut filtered: Vec<f64> = values
        .iter()
        .copied()
        .filter(|v| v.is_finite() && (!log_x || *v > 0.0))
        .collect();
    if filtered.is_empty() {
        return Vec::new();
    }
    filtered.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = filtered.len();

    let mut points = Vec::with_capacity(n * 2);
    let mut prev_frac = 0.0;
    let mut cum = 0usize;
    let mut i = 0;
    while i < filtered.len() {
        let v = filtered[i];
        let mut j = i;
        while j < filtered.len() && filtered[j] == v {
            j += 1;
        }
        cum += j - i;
        let frac = cum as f64 / n as f64;
        points.push([v, prev_frac]);
        points.push([v, frac]);
        prev_frac = frac;
        i = j;
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_edf_points_basic_staircase() {
        let pts = build_edf_points(&[1.0, 2.0, 3.0], false);
        assert_eq!(
            pts,
            vec![
                [1.0, 0.0],
                [1.0, 1.0 / 3.0],
                [2.0, 1.0 / 3.0],
                [2.0, 2.0 / 3.0],
                [3.0, 2.0 / 3.0],
                [3.0, 1.0],
            ]
        );
    }

    #[test]
    fn build_edf_points_handles_ties() {
        let pts = build_edf_points(&[1.0, 1.0, 2.0], false);
        assert_eq!(
            pts,
            vec![[1.0, 0.0], [1.0, 2.0 / 3.0], [2.0, 2.0 / 3.0], [2.0, 1.0],]
        );
    }

    #[test]
    fn build_edf_points_skips_nan_and_inf() {
        let pts = build_edf_points(
            &[1.0, f64::NAN, 2.0, f64::INFINITY, f64::NEG_INFINITY],
            false,
        );
        assert_eq!(pts, vec![[1.0, 0.0], [1.0, 0.5], [2.0, 0.5], [2.0, 1.0],]);
    }

    #[test]
    fn build_edf_points_log_scale_drops_non_positive() {
        let pts = build_edf_points(&[-1.0, 0.0, 1.0, 2.0], true);
        assert_eq!(pts, vec![[1.0, 0.0], [1.0, 0.5], [2.0, 0.5], [2.0, 1.0],]);
    }

    #[test]
    fn build_edf_points_empty_input_returns_empty() {
        assert!(build_edf_points(&[], false).is_empty());
    }

    #[test]
    fn build_edf_points_all_non_finite_returns_empty() {
        assert!(build_edf_points(&[f64::NAN, f64::INFINITY], false).is_empty());
    }

    #[test]
    fn build_edf_points_all_dropped_by_log_filter_returns_empty() {
        assert!(build_edf_points(&[-1.0, 0.0], true).is_empty());
    }

    #[test]
    fn edf_plot_chart_default() {
        let chart = EdfPlotChart::default();
        assert_eq!(chart.obj_idx, 0);
        assert!(!chart.log_x);
    }
}
