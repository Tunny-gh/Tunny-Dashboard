use crate::theme::chart_colors::{
    COLOR_CHART_TEXT, COLOR_INFEASIBLE, COLOR_PARALLEL_AXIS, COLOR_PARALLEL_LINE_DEFAULT,
    COLOR_PARALLEL_LINE_UNSELECTED, COLOR_PARALLEL_TICK,
};
use crate::theme::color_compute::{rgba_key, rgba_to_color32};
use crate::theme::CENTRAL_BG;
use crate::ui::widgets::common::range_math;
use crate::ui::widgets::scatter_matrix::downsample_indices_to_cap;

mod brush;
mod layout;
#[cfg(test)]
mod tests;

use brush::BrushDrag;
pub use brush::{
    filter_trials_by_brushes, ordered_brush_range, shifted_brush_range, trial_passes_brushes,
};
pub use layout::{
    build_axis_order, feasible_color_range, fmt_tick_value, normalize_value,
    normalized_to_screen_y, visible_axis_indices,
};

/// Max number of PCP polylines to draw. Each trial draws (n_visible-1) line
/// segments, which is more expensive than a scatter point, so we adopt the
/// same cap as `MAX_SCATTER_POINTS` in scatter_matrix (trials currently
/// selected by a brush are exempt from downsampling and are always drawn).
const MAX_PCP_POLYLINES: usize = 1500;

/// Invalidation key for draw_targets_cache: (df_ptr, trial_count, brush range snapshot).
type DrawTargetsKey = (
    usize,
    usize,
    std::collections::HashMap<String, Option<(f32, f32)>>,
);

/// Parallel coordinates plot widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ParallelCoordsChart {
    pub axis_order: Vec<String>,
    pub show_params: bool,
    pub show_objectives: bool,
    pub brush_ranges: std::collections::HashMap<String, Option<(f32, f32)>>,
    #[serde(skip)]
    pub drag_start: Option<(String, f32)>,
    /// Kind of drag in progress (creating a new range or moving an existing
    /// one). Used together with drag_start.
    #[serde(skip)]
    brush_drag: Option<BrushDrag>,
    /// REQ-004: per-axis show/hide flag (true = visible).
    pub axis_visibility: std::collections::HashMap<String, bool>,
    #[serde(skip)]
    col_ranges_cache: Option<Vec<(f64, f64)>>,
    #[serde(skip)]
    cache_key: (usize, usize, usize, usize), // (df_ptr, trial_count, n_params, n_objs)
    /// Cache of the draw targets (t_idx, in_selection) list while a brush
    /// selection is active (M-14). Skips recomputing the per-trial ×
    /// per-axis brush check unless the brush ranges or data change.
    #[serde(skip)]
    draw_targets_cache: Option<Vec<(usize, bool)>>,
    /// Invalidation key for draw_targets_cache: (df_ptr, trial_count, brush range snapshot).
    #[serde(skip)]
    draw_targets_key: Option<DrawTargetsKey>,
    /// Downsampling index cache for polyline drawing (recomputed only when trial_count changes).
    #[serde(skip)]
    polyline_indices_cache: Option<Vec<u32>>,
    #[serde(skip)]
    polyline_indices_cache_key: Option<usize>, // trial_count
    /// Pre-laid-out Galley cache for axis labels (recomputed only when the axis name list changes).
    #[serde(skip)]
    label_galleys_cache: Option<Vec<std::sync::Arc<egui::Galley>>>,
    #[serde(skip)]
    label_galleys_cache_key: Option<Vec<String>>,
    // TASK-2242: pending selection from completed brush drag
    #[serde(skip)]
    pub pending_selection: Option<Vec<u32>>,
    /// Whether to show infeasible solutions (only meaningful for a Study with constraints).
    pub show_infeasible: bool,
    /// Axis name used to color the lines (falls back to the last axis = last
    /// objective when None).
    pub color_axis: Option<String>,
}

impl Default for ParallelCoordsChart {
    fn default() -> Self {
        Self {
            axis_order: Vec::new(),
            show_params: true,
            show_objectives: true,
            brush_ranges: std::collections::HashMap::new(),
            drag_start: None,
            brush_drag: None,
            axis_visibility: std::collections::HashMap::new(),
            col_ranges_cache: None,
            cache_key: (0, 0, 0, 0),
            draw_targets_cache: None,
            draw_targets_key: None,
            polyline_indices_cache: None,
            polyline_indices_cache_key: None,
            label_galleys_cache: None,
            label_galleys_cache_key: None,
            pending_selection: None,
            show_infeasible: true,
            color_axis: None,
        }
    }
}

impl ParallelCoordsChart {
    /// Draws the parallel coordinates plot.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &crate::state::app_state::StudyView,
        param_names: &[String],
        obj_names: &[String],
        cmap: &crate::theme::colormap::ColorMap,
    ) {
        let trial_count = view.row_count();
        if trial_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No trial data.").weak());
            });
            return;
        }

        let all_names = build_axis_order(param_names, obj_names);
        let n_axes = all_names.len();
        if n_axes < 2 {
            return;
        }

        let n_params = param_names.len();
        // Borrow each axis's column slice from view (no copy, MEM-003).
        let cols = view.numeric_columns(&all_names);

        // Include the DataFrame's Arc identity in the key to prevent stale
        // rendering when switching to a different Study with the same
        // dimensions (M-5).
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;
        let cache_key = (df_ptr, trial_count, n_params, obj_names.len());
        if self.col_ranges_cache.is_none() || self.cache_key != cache_key {
            let col_ranges: Vec<(f64, f64)> = cols
                .iter()
                .map(|data| match data {
                    Some(c) => range_math::value_range(c.iter().cloned())
                        .unwrap_or((f64::INFINITY, f64::NEG_INFINITY)),
                    None => (0.0, 1.0),
                })
                .collect();
            self.col_ranges_cache = Some(col_ranges);
            self.cache_key = cache_key;
        }
        let col_ranges = self.col_ranges_cache.as_ref().unwrap();

        let feas = view.feasibility();
        let has_constraints = feas.has_constraints();

        // Control row: choose axes to draw + coloring axis + "Show Infeasible".
        ui.horizontal(|ui| {
            // Dropdown with checkboxes to choose which axes to draw (all shown by default).
            let visible_count = all_names
                .iter()
                .filter(|n| self.axis_visibility.get(*n).copied().unwrap_or(true))
                .count();
            ui.label("Axes:");
            egui::ComboBox::from_id_salt("pcp_visible_axes")
                .selected_text(format!("{visible_count}/{n_axes}"))
                .show_ui(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("All").clicked() {
                            for name in &all_names {
                                self.axis_visibility.insert(name.clone(), true);
                            }
                        }
                        if ui.button("None").clicked() {
                            for name in &all_names {
                                self.axis_visibility.insert(name.clone(), false);
                            }
                        }
                    });
                    ui.separator();
                    for name in &all_names {
                        let mut vis = self.axis_visibility.get(name).copied().unwrap_or(true);
                        if ui.checkbox(&mut vis, name.as_str()).changed() {
                            self.axis_visibility.insert(name.clone(), vis);
                        }
                    }
                });

            // Resolve the axis used to color the lines (falls back to the
            // last axis = last objective if unset).
            let current_axis = self
                .color_axis
                .clone()
                .filter(|name| all_names.iter().any(|n| n == name))
                .unwrap_or_else(|| all_names[n_axes - 1].clone());
            ui.label("Color by:");
            egui::ComboBox::from_id_salt("pcp_color_axis")
                .selected_text(current_axis.clone())
                .show_ui(ui, |ui| {
                    for name in &all_names {
                        if ui
                            .selectable_label(*name == current_axis, name.as_str())
                            .clicked()
                        {
                            self.color_axis = Some(name.clone());
                        }
                    }
                });
            if has_constraints {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
        });

        // Original indices of the axes to draw (visible ones); unregistered = visible.
        let visible = visible_axis_indices(&all_names, &self.axis_visibility);
        let n_visible = visible.len();
        if n_visible < 2 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Select at least 2 axes to display.").weak());
            });
            return;
        }

        // Index of the coloring axis used for drawing (resolved after the dropdown update).
        let color_axis_idx = self
            .color_axis
            .as_ref()
            .and_then(|name| all_names.iter().position(|n| n == name))
            .unwrap_or(n_axes - 1);

        let available = ui.available_rect_before_wrap();
        let axis_margin = 40.0_f32;
        let axis_x: Vec<f32> = (0..n_visible)
            .map(|i| {
                available.min.x
                    + axis_margin
                    + (available.width() - 2.0 * axis_margin) * i as f32 / (n_visible - 1) as f32
            })
            .collect();

        let painter = ui.painter().clone();
        let text_color = COLOR_CHART_TEXT();
        let label_font = egui::FontId::proportional(10.0);

        // Pre-lay-out the axis labels and rotate them diagonally when wider
        // than the adjacent axis spacing to avoid overlap. Layout
        // (layout_no_wrap) has text-shaping cost, so we don't recompute it
        // every frame unless the axis name list changes.
        if self.label_galleys_cache.is_none()
            || self.label_galleys_cache_key.as_deref() != Some(&all_names[..])
        {
            let galleys: Vec<std::sync::Arc<egui::Galley>> = all_names
                .iter()
                .map(|name| painter.layout_no_wrap(name.clone(), label_font.clone(), text_color))
                .collect();
            self.label_galleys_cache = Some(galleys);
            self.label_galleys_cache_key = Some(all_names.clone());
        }
        let label_galleys = self.label_galleys_cache.as_ref().unwrap();
        let max_label_w = visible
            .iter()
            .map(|&i| label_galleys[i].size().x)
            .fold(0.0_f32, f32::max);
        let label_h = label_galleys.first().map(|g| g.size().y).unwrap_or(12.0);
        let axis_spacing = (available.width() - 2.0 * axis_margin) / (n_visible - 1) as f32;
        let rotate_labels = max_label_w > axis_spacing - 4.0;
        let label_angle = if rotate_labels {
            std::f32::consts::FRAC_PI_4 // 45° rotation (rising to the right)
        } else {
            0.0
        };
        // Height occupied by labels at the top (diagonal height when rotated).
        let label_area = if rotate_labels {
            (max_label_w * label_angle.sin() + label_h * label_angle.cos()).min(110.0) + 8.0
        } else {
            label_h + 8.0
        };
        let axis_top = available.min.y + label_area;
        let axis_bottom = available.max.y - 10.0;

        painter.rect_filled(available, 0.0, CENTRAL_BG());

        const N_TICKS: usize = 5;
        let tick_len = 4.0_f32;
        let tick_color = COLOR_PARALLEL_TICK();
        let tick_font = egui::FontId::proportional(9.0);

        let show_infeasible = self.show_infeasible;

        // The normalization range for coloring is computed from feasible
        // solutions only (kept separate from the axis coordinate range).
        let color_range: (f64, f64) = match cols.get(color_axis_idx).and_then(|c| c.as_ref()) {
            Some(col) => feasible_color_range(col, feas, col_ranges[color_axis_idx]),
            None => col_ranges[color_axis_idx],
        };

        // Gray out lines outside the selection if any brush is active.
        // Updated in real time since `brush_ranges` changes while dragging.
        let has_active_brush = self.brush_ranges.values().any(|range| range.is_some());

        // Downsample the trials to draw: drawing everything is expensive, so
        // cap it at MAX_PCP_POLYLINES. Recomputed only when trial_count changes.
        if self.polyline_indices_cache_key != Some(trial_count) {
            let all: Vec<u32> = (0..trial_count as u32).collect();
            self.polyline_indices_cache = Some(downsample_indices_to_cap(&all, MAX_PCP_POLYLINES));
            self.polyline_indices_cache_key = Some(trial_count);
        }
        let downsampled = self.polyline_indices_cache.as_ref().unwrap();

        // List of draw targets (t_idx, in_selection). Trials currently
        // selected by a brush are exempt from downsampling and always drawn
        // (union of the downsampled set and the brush-passing trials).
        // Checking brushes for every trial × axis plus building a HashSet is
        // expensive, so we reuse the cache unless the df identity,
        // trial_count, or brush ranges change (M-14).
        let draw_targets_key_matches = self.draw_targets_cache.is_some()
            && self.draw_targets_key.as_ref().is_some_and(|(p, tc, br)| {
                *p == df_ptr && *tc == trial_count && br == &self.brush_ranges
            });
        if !draw_targets_key_matches {
            let targets: Vec<(usize, bool)> = if has_active_brush {
                let downsampled_set: std::collections::HashSet<usize> =
                    downsampled.iter().map(|&i| i as usize).collect();
                (0..trial_count)
                    .filter_map(|t_idx| {
                        let passes = trial_passes_brushes(
                            t_idx,
                            &self.brush_ranges,
                            &cols,
                            col_ranges,
                            &all_names,
                        );
                        (downsampled_set.contains(&t_idx) || passes).then_some((t_idx, passes))
                    })
                    .collect()
            } else {
                downsampled.iter().map(|&i| (i as usize, true)).collect()
            };
            self.draw_targets_cache = Some(targets);
            self.draw_targets_key = Some((df_ptr, trial_count, self.brush_ranges.clone()));
        }
        let draw_targets = self.draw_targets_cache.as_ref().unwrap();

        // Draw each trial as a (semi-transparent) polyline.
        // Draw the out-of-selection (grayed-out) lines first, then overlay
        // the in-selection lines on top. The scratch buffer is reused for
        // lines drawn immediately (not selected) to avoid a per-trial
        // allocation; only the in-selection lines drawn on top are cloned
        // individually so they can be drawn together afterward.
        let mut selected_polylines: Vec<(Vec<egui::Pos2>, egui::Color32)> = Vec::new();
        let mut point_scratch: Vec<egui::Pos2> = Vec::with_capacity(n_visible);
        for &(t_idx, in_selection) in draw_targets {
            let feasible = feas.is_feasible(t_idx);

            if !feasible && !show_infeasible {
                continue;
            }

            let color = if !in_selection {
                COLOR_PARALLEL_LINE_UNSELECTED()
            } else if feasible {
                // Normalize the selected axis's value to [0,1] and look up the color via the colormap.
                let base_color = cols
                    .get(color_axis_idx)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.get(t_idx))
                    .copied()
                    .map(|v| {
                        let (mn, mx) = color_range;
                        cmap.interpolate(normalize_value(v, mn, mx))
                    })
                    .unwrap_or(COLOR_PARALLEL_LINE_DEFAULT());
                let [r, g, b, _] = rgba_key(base_color);
                rgba_to_color32([r, g, b, 120])
            } else {
                COLOR_INFEASIBLE()
            };

            point_scratch.clear();
            let mut valid = true;
            for (disp, &orig) in visible.iter().enumerate() {
                let val_opt = cols
                    .get(orig)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.get(t_idx))
                    .copied();
                let Some(val) = val_opt else {
                    valid = false;
                    break;
                };
                let (mn, mx) = col_ranges[orig];
                let norm = normalize_value(val, mn, mx);
                let y = normalized_to_screen_y(norm, axis_top, axis_bottom);
                point_scratch.push(egui::pos2(axis_x[disp], y));
            }
            if valid && point_scratch.len() >= 2 {
                if in_selection && has_active_brush {
                    // Draw in-selection lines together on top afterward.
                    selected_polylines.push((point_scratch.clone(), color));
                } else {
                    for pair in point_scratch.windows(2) {
                        painter.line_segment([pair[0], pair[1]], egui::Stroke::new(0.8, color));
                    }
                }
            }
        }
        // Overlay the in-selection lines on top.
        for (points, color) in &selected_polylines {
            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], egui::Stroke::new(0.8, *color));
            }
        }

        // Draw the vertical axes, labels, and ticks on top.
        for (disp, &orig) in visible.iter().enumerate() {
            let x = axis_x[disp];
            painter.line_segment(
                [egui::pos2(x, axis_top), egui::pos2(x, axis_bottom)],
                egui::Stroke::new(1.5, COLOR_PARALLEL_AXIS()),
            );
            let galley = label_galleys[orig].clone();
            if rotate_labels {
                // Align the lowest corner (start of the string, bottom-left)
                // of the "/"-shaped label rotated by -label_angle
                // (counter-clockwise) to each axis's top point (x, axis_top)
                // (shared helper, D-12).
                let applied = -label_angle;
                let lowest = super::rotated_label_corners(galley.size(), applied).lowest;
                // Choose pos so the lowest corner sits just above the axis top.
                let gap = 2.0_f32;
                let anchor = egui::pos2(x, axis_top - gap);
                let pos = anchor - egui::vec2(lowest.0, lowest.1);
                painter
                    .add(egui::epaint::TextShape::new(pos, galley, text_color).with_angle(applied));
            } else {
                let size = galley.size();
                painter.galley(
                    egui::pos2(x - size.x * 0.5, available.min.y + 4.0),
                    galley,
                    text_color,
                );
            }

            let (mn, mx) = col_ranges[orig];
            for t in 0..N_TICKS {
                let frac = t as f32 / (N_TICKS - 1) as f32;
                let y = normalized_to_screen_y(frac, axis_top, axis_bottom);
                painter.line_segment(
                    [egui::pos2(x - tick_len, y), egui::pos2(x + tick_len, y)],
                    egui::Stroke::new(1.0, tick_color),
                );
                let val = mn + frac as f64 * (mx - mn);
                painter.text(
                    egui::pos2(x - tick_len - 2.0, y),
                    egui::Align2::RIGHT_CENTER,
                    fmt_tick_value(val, mn, mx),
                    tick_font.clone(),
                    tick_color,
                );
            }
        }

        // Draw brush range overlays (visible axes only).
        for (disp, &orig) in visible.iter().enumerate() {
            let name = &all_names[orig];
            if let Some(Some((y_lo, y_hi))) = self.brush_ranges.get(name.as_str()) {
                let x = axis_x[disp];
                let screen_hi = normalized_to_screen_y(*y_hi, axis_top, axis_bottom);
                let screen_lo = normalized_to_screen_y(*y_lo, axis_top, axis_bottom);
                let brush_rect = egui::Rect::from_min_max(
                    egui::pos2(x - 6.0, screen_hi),
                    egui::pos2(x + 6.0, screen_lo),
                );
                painter.rect_filled(
                    brush_rect,
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(100, 150, 255, 80),
                );
                painter.rect_stroke(
                    brush_rect,
                    2.0,
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 150, 255)),
                    egui::StrokeKind::Inside,
                );
            }
        }

        let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

        // Brush drag interaction
        if let Some(ptr) = response.interact_pointer_pos() {
            // Find closest axis
            let closest_axis_idx = axis_x
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (ptr.x - **a)
                        .abs()
                        .partial_cmp(&(ptr.x - **b).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i);

            if let Some(disp_idx) = closest_axis_idx {
                let axis_name = all_names[visible[disp_idx]].clone();
                // Normalize pointer Y to [0, 1]
                let norm_y = ((axis_bottom - ptr.y) / (axis_bottom - axis_top)).clamp(0.0, 1.0);

                if response.drag_started() {
                    // Move mode if grabbing inside an existing brush, otherwise create mode.
                    let existing = self
                        .brush_ranges
                        .get(axis_name.as_str())
                        .and_then(|r| *r)
                        .filter(|(lo, hi)| norm_y >= *lo && norm_y <= *hi);
                    self.brush_drag = Some(match existing {
                        Some(orig_range) => BrushDrag::Move {
                            grab_norm_y: norm_y,
                            orig_range,
                        },
                        None => BrushDrag::Create,
                    });
                    self.drag_start = Some((axis_name, norm_y));
                } else if response.dragged() {
                    if let Some((ref start_name, start_y)) = self.drag_start.clone() {
                        if *start_name == axis_name {
                            let new_range = match self.brush_drag {
                                Some(BrushDrag::Move {
                                    grab_norm_y,
                                    orig_range,
                                }) => shifted_brush_range(orig_range, norm_y - grab_norm_y),
                                _ => ordered_brush_range(start_y, norm_y),
                            };
                            self.brush_ranges.insert(axis_name, Some(new_range));
                        }
                    }
                } else if response.drag_stopped() {
                    self.drag_start = None;
                    self.brush_drag = None;
                    // Compute selection from all active brush ranges
                    let new_sel = filter_trials_by_brushes(
                        &view.trial_ids,
                        &self.brush_ranges,
                        &cols,
                        col_ranges,
                        &all_names,
                    );
                    self.pending_selection = Some(new_sel);
                }
            }
        }

        // Show a grab cursor while hovering inside an existing brush rect to
        // indicate it can be grabbed and moved.
        if let Some(ptr) = response.hover_pos() {
            let hovering_brush = visible.iter().enumerate().any(|(disp, &orig)| {
                // The brush rect spans ±6px around the axis (same as when drawing).
                if (ptr.x - axis_x[disp]).abs() > 6.0 {
                    return false;
                }
                let name = &all_names[orig];
                self.brush_ranges
                    .get(name.as_str())
                    .and_then(|r| *r)
                    .map(|(lo, hi)| {
                        let norm_y =
                            ((axis_bottom - ptr.y) / (axis_bottom - axis_top)).clamp(0.0, 1.0);
                        norm_y >= lo && norm_y <= hi
                    })
                    .unwrap_or(false)
            });
            if hovering_brush {
                ui.ctx().set_cursor_icon(if response.dragged() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
            }
        }

        // Clear brushes on right-click or double-click
        if response.secondary_clicked() || response.double_clicked() {
            self.brush_ranges.clear();
            self.brush_drag = None;
            self.pending_selection = Some(vec![]); // empty = no selection filter
        }
    }
}
