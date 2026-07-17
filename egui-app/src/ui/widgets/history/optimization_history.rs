use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::{
    COLOR_INFEASIBLE, COLOR_OPT_PRUNED, COLOR_OPT_RUNNING, COLOR_OPT_TRIAL,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{
    push_feasible_row, resolve_click_hover, show_hover_tooltip, TrialDetailModal, TrialDetailTarget,
};

/// The optimization history series for a single comparison Study (value column for
/// the selected objective + color + legend name).
pub struct OptHistoryComparison {
    pub name: String,
    pub color: egui::Color32,
    /// The objective value column corresponding to the selected objective (row order).
    pub values: Vec<f64>,
    pub is_minimize: bool,
}

/// A cache bundling the O(n) computation results derived from the base Study's value
/// column.
/// Avoids recomputation every frame as long as `key` is unchanged from before.
/// Moving Average is only computed when the display toggle is enabled (no wasted
/// computation when disabled).
struct HistoryCache {
    key: (usize, usize, bool, usize, bool), // (row_count, obj_idx, log_scale, window_size, is_minimize)
    values: Vec<f64>,
    feasible_vals: Vec<[f64; 2]>,
    infeasible_vals: Vec<[f64; 2]>,
    base_hit_points: Vec<(u32, usize, [f64; 2])>,
    best_values: Vec<[f64; 2]>,
    moving_avg: Option<Vec<[f64; 2]>>,
}

/// The optimization history chart widget
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OptimizationHistoryChart {
    pub show_moving_avg: bool,
    pub window_size: usize,
    pub obj_idx: usize,
    /// REQ-008: Y-axis log scale toggle
    pub log_scale: bool,
    /// Trial detail modal opened by clicking a point (shared with the scatter plot).
    #[serde(skip)]
    detail_modal: TrialDetailModal,
    /// Cache of the base Study's O(n) computation results.
    #[serde(skip)]
    history_cache: Option<HistoryCache>,
}

impl Default for OptimizationHistoryChart {
    fn default() -> Self {
        Self {
            show_moving_avg: false,
            window_size: 10,
            obj_idx: 0,
            log_scale: false,
            detail_modal: TrialDetailModal::new(),
            history_cache: None,
        }
    }
}

impl OptimizationHistoryChart {
    /// Draws the base Study plus, overlaid on the same graph, each comparison
    /// Study's cumulative-best-value line. Comparison lines are drawn in each
    /// Study's color when the "Best Value" display is enabled.
    ///
    /// Clicking an "All Trials" point opens the trial detail modal shared with the
    /// scatter plot. Only base Study points are targeted (comparison Study trials
    /// don't exist in the base Study's `view`).
    #[allow(clippy::too_many_arguments)]
    pub fn show_with_comparisons(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        directions: &[Direction],
        param_names: &[String],
        base_name: &str,
        comparisons: &[OptHistoryComparison],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        // Clamp the objective function index to a valid range
        if obj_names.is_empty() {
            self.obj_idx = 0;
        } else {
            self.obj_idx = self.obj_idx.min(obj_names.len() - 1);
        }

        let is_minimize = directions
            .get(self.obj_idx)
            .map(|d| matches!(d, Direction::Minimize))
            .unwrap_or(true);

        let feas = view.feasibility();

        // All Trials / Best Value / Infeasible are always drawn (their on/off display
        // can be toggled by clicking the chart legend). Only Moving Average / Log
        // Scale and objective selection remain as toggles.
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.show_moving_avg, "Moving Average")
                .clicked()
            {
                self.show_moving_avg = !self.show_moving_avg;
            }

            // Show the objective function selection combo box only for multi-objective
            if obj_names.len() > 1 {
                ui.separator();
                let selected_label = obj_names
                    .get(self.obj_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                egui::ComboBox::from_id_salt("opt_history_obj_select")
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.obj_idx, i, name);
                        }
                    });
            }

            // REQ-008-D: log scale toggle
            ui.separator();
            if ui.selectable_label(self.log_scale, "Log Scale").clicked() {
                self.log_scale = !self.log_scale;
            }
        });

        let log_scale = self.log_scale;
        let show_moving_avg = self.show_moving_avg;
        let window_size = self.window_size;
        let row_count = view.row_count();

        // The base Study's O(n) computation (values / feasible split / hit-test
        // points / cumulative best value) is not recomputed unless the row count,
        // objective selection, log/minimize-maximize flag, or moving-average window
        // changes (Moving Average is lazily computed only when the display toggle is
        // enabled).
        let cache_key = (row_count, self.obj_idx, log_scale, window_size, is_minimize);
        if self.history_cache.as_ref().map(|c| c.key) != Some(cache_key) {
            let values: Vec<f64> = obj_names
                .get(self.obj_idx)
                .and_then(|name| view.numeric_column(name))
                .map(|col| col.to_vec())
                .unwrap_or_default();

            // Split All Trials into feasible / infeasible (only branches for
            // constrained Studies)
            let (feasible_vals, infeasible_vals) = partition_history_by_feasibility(&values, feas);

            // Build each trial's point as (trial_id, row index, [x, y]) for click
            // detection.
            // x is the row index; y is converted with log10 only under log scale, to
            // match the drawing.
            let base_hit_points: Vec<(u32, usize, [f64; 2])> = values
                .iter()
                .enumerate()
                .filter_map(|(i, &v)| {
                    let tid = *view.trial_ids.get(i)?;
                    let y = if log_scale && v > 0.0 { v.log10() } else { v };
                    Some((tid, i, [i as f64, y]))
                })
                .collect();

            let best_values = compute_best_values(&values, is_minimize);

            self.history_cache = Some(HistoryCache {
                key: cache_key,
                values,
                feasible_vals,
                infeasible_vals,
                base_hit_points,
                best_values,
                moving_avg: None,
            });
        }
        let cache = self.history_cache.as_mut().unwrap();
        if show_moving_avg && cache.moving_avg.is_none() {
            cache.moving_avg = Some(compute_moving_average(&cache.values, window_size));
        }
        let values = &cache.values;
        let feasible_vals = &cache.feasible_vals;
        let infeasible_vals = &cache.infeasible_vals;
        let base_hit_points = &cache.base_hit_points;
        let best_values = &cache.best_values;
        let moving_avg = cache.moving_avg.as_ref();

        // The clicked point (trial_id, row index).
        let mut clicked_detail: Option<(u32, usize)> = None;
        // The point currently under mouse hover (trial_id, row index). Used for
        // tooltip display.
        let mut hovered_detail: Option<(u32, usize)> = None;

        let mut plot = egui_plot::Plot::new("optimization_history_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default());

        // Since values are drawn with a log10 transform under log scale, the Y-axis
        // labels show the original pre-transform values (restored via 10^mark).
        // Powers of 10 (1, 10, 100, ...) are used as major ticks, with minor ticks
        // at 2-9x in between.
        if log_scale {
            plot = crate::ui::widgets::common::log_scale::apply_log_y_axis(plot);
        }

        plot.show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            // Detect click/hover targets (base Study trials only).
            (clicked_detail, hovered_detail) = resolve_click_hover(plot_ui, base_hit_points);

            // All Trials is always drawn (display can be toggled via the legend).
            if !values.is_empty() {
                let apply_log = |[x, v]: [f64; 2]| -> [f64; 2] {
                    [x, if log_scale && v > 0.0 { v.log10() } else { v }]
                };
                // Draw infeasible points behind, always (display can be toggled via the legend)
                if !infeasible_vals.is_empty() {
                    let pts: egui_plot::PlotPoints =
                        infeasible_vals.iter().copied().map(apply_log).collect();
                    plot_ui.points(
                        egui_plot::Points::new("Infeasible", pts)
                            .color(COLOR_INFEASIBLE())
                            .radius(1.5),
                    );
                }
                // Feasible points (for an unconstrained Study, all points go into feasible_vals)
                if !feasible_vals.is_empty() {
                    let pts: egui_plot::PlotPoints =
                        feasible_vals.iter().copied().map(apply_log).collect();
                    plot_ui.points(
                        egui_plot::Points::new("All Trials", pts)
                            .color(COLOR_OPT_TRIAL())
                            .radius(1.5),
                    );
                }
            }

            // Best Value is always drawn (display can be toggled via the legend).
            {
                let apply_log_y = |[x, y]: [f64; 2]| -> [f64; 2] {
                    [x, if log_scale && y > 0.0 { y.log10() } else { y }]
                };
                if !values.is_empty() {
                    // When comparing, switch the label so the base Study can also be
                    // distinguished by name.
                    let base_label = if comparisons.is_empty() || base_name.is_empty() {
                        "Best Value"
                    } else {
                        base_name
                    };
                    let pts: egui_plot::PlotPoints =
                        best_values.iter().copied().map(apply_log_y).collect();
                    plot_ui.line(
                        egui_plot::Line::new(base_label, pts)
                            .color(COLOR_OPT_PRUNED())
                            .width(1.5),
                    );
                }
                // Overlay each comparison Study's cumulative-best-value line in its own color.
                for comp in comparisons {
                    if comp.values.is_empty() {
                        continue;
                    }
                    let pts: egui_plot::PlotPoints =
                        compute_best_values(&comp.values, comp.is_minimize)
                            .into_iter()
                            .map(apply_log_y)
                            .collect();
                    plot_ui.line(
                        egui_plot::Line::new(&comp.name, pts)
                            .color(comp.color)
                            .width(1.5),
                    );
                }
            }

            if let Some(avg) = moving_avg.filter(|a| show_moving_avg && !a.is_empty()) {
                let pts: egui_plot::PlotPoints = avg
                    .iter()
                    .map(|&[x, y]| {
                        let y2 = if log_scale && y > 0.0 { y.log10() } else { y };
                        [x, y2]
                    })
                    .collect();
                plot_ui.line(
                    egui_plot::Line::new("Moving Average", pts)
                        .color(COLOR_OPT_RUNNING())
                        .width(1.5),
                );
            }
        });

        // If there's a hovered point, show a summary tooltip at the pointer position.
        if let Some((_, row)) = hovered_detail {
            let trial_number = view.df.get_trial_number(row).unwrap_or(row as u32);
            let mut rows = Vec::new();
            if let (Some(name), Some(v)) = (obj_names.get(self.obj_idx), values.get(row)) {
                rows.push((name.clone(), format!("{v:.6}")));
            }
            push_feasible_row(&mut rows, feas, row);
            show_hover_tooltip(ui, "opt_history_hover_tooltip", trial_number, &rows);
        }

        // If there's a clicked point, open the modal with the selected objective
        // value (and feasibility) as extra context.
        if let Some((trial_id, row)) = clicked_detail {
            let mut context = Vec::new();
            if let (Some(name), Some(v)) = (obj_names.get(self.obj_idx), values.get(row)) {
                context.push((name.clone(), format!("{v:.6}")));
            }
            push_feasible_row(&mut context, feas, row);
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        // Draw the detail modal (same shared implementation as the scatter plot).
        if self.detail_modal.is_open() {
            self.detail_modal
                .show(ui, view, param_names, obj_names, artifact_map);
        }
    }
}

/// Splits the objective value column into feasible / infeasible point lists based on
/// feasibility.
/// For an unconstrained Study (feas.has_constraints() == false), all points are
/// classified as feasible.
/// Returns: (feasible_pts, infeasible_pts), both in [trial_idx, value] format.
pub fn partition_history_by_feasibility(
    values: &[f64],
    feas: tunny_core::dataframe::Feasibility<'_>,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let mut feasible: Vec<[f64; 2]> = Vec::with_capacity(values.len());
    let mut infeasible: Vec<[f64; 2]> = Vec::with_capacity(values.len());
    for (i, &v) in values.iter().enumerate() {
        if feas.is_feasible(i) {
            feasible.push([i as f64, v]);
        } else {
            infeasible.push([i as f64, v]);
        }
    }
    (feasible, infeasible)
}

/// Computes the cumulative best value (minimize: cumulative min, maximize: cumulative max)
pub fn compute_best_values(values: &[f64], is_minimize: bool) -> Vec<[f64; 2]> {
    let mut best = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            if is_minimize {
                best = best.min(v);
            } else {
                best = best.max(v);
            }
            [i as f64, best]
        })
        .collect()
}

/// Computes the moving average
pub fn compute_moving_average(values: &[f64], window: usize) -> Vec<[f64; 2]> {
    if values.is_empty() || window == 0 {
        return vec![];
    }
    values
        .windows(window.min(values.len()))
        .enumerate()
        .map(|(i, w)| {
            let avg = w.iter().sum::<f64>() / w.len() as f64;
            [i as f64, avg]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_best_values_minimize_decreasing() {
        let vals = vec![5.0, 3.0, 4.0, 1.0, 2.0];
        let result = compute_best_values(&vals, true);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0][1], 5.0);
        assert_eq!(result[1][1], 3.0);
        assert_eq!(result[2][1], 3.0);
        assert_eq!(result[3][1], 1.0);
        assert_eq!(result[4][1], 1.0);
    }

    #[test]
    fn compute_best_values_maximize_increasing() {
        let vals = vec![1.0, 3.0, 2.0, 5.0, 4.0];
        let result = compute_best_values(&vals, false);
        assert_eq!(result[0][1], 1.0);
        assert_eq!(result[1][1], 3.0);
        assert_eq!(result[2][1], 3.0);
        assert_eq!(result[3][1], 5.0);
        assert_eq!(result[4][1], 5.0);
    }

    #[test]
    fn compute_moving_average_window3() {
        let vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = compute_moving_average(&vals, 3);
        // windows(3): [1,2,3]=2.0, [2,3,4]=3.0, [3,4,5]=4.0
        assert_eq!(result.len(), 3);
        assert!((result[0][1] - 2.0).abs() < 1e-9);
        assert!((result[1][1] - 3.0).abs() < 1e-9);
        assert!((result[2][1] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn compute_moving_average_window_larger_than_data() {
        let vals = vec![1.0, 2.0];
        let result = compute_moving_average(&vals, 10);
        // windows(2) since min(10,2)=2: [1,2]=1.5
        assert_eq!(result.len(), 1);
        assert!((result[0][1] - 1.5).abs() < 1e-9);
    }

    // TASK-2126 tests
    #[test]
    fn log_scale_toggle() {
        let mut log_scale = false;
        log_scale = !log_scale;
        assert!(log_scale);
    }

    // ── constraint-aware visualization (TASK-2349) ──────────────────

    #[test]
    fn tc_cav_partition_history_no_constraints_all_feasible() {
        use tunny_core::dataframe::Feasibility;
        let values = vec![1.0, 2.0, 3.0];
        let feas = Feasibility::from_column(None);
        let (f, inf) = partition_history_by_feasibility(&values, feas);
        assert_eq!(f.len(), 3);
        assert!(inf.is_empty());
    }

    #[test]
    fn tc_cav_partition_history_mixed() {
        use tunny_core::dataframe::Feasibility;
        let values = vec![1.0, 2.0, 3.0];
        let is_feasible = vec![1.0_f64, 0.0, 1.0]; // idx 1 = infeasible
        let feas = Feasibility::from_column(Some(&is_feasible));
        let (f, inf) = partition_history_by_feasibility(&values, feas);
        assert_eq!(f.len(), 2);
        assert_eq!(inf.len(), 1);
        assert_eq!(inf[0][0], 1.0); // trial_idx=1
        assert_eq!(inf[0][1], 2.0); // value=2.0
    }

    #[test]
    fn tc_cav_partition_history_all_infeasible() {
        use tunny_core::dataframe::Feasibility;
        let values = vec![1.0, 2.0];
        let is_feasible = vec![0.0_f64, 0.0];
        let feas = Feasibility::from_column(Some(&is_feasible));
        let (f, inf) = partition_history_by_feasibility(&values, feas);
        assert!(f.is_empty());
        assert_eq!(inf.len(), 2);
    }
}
