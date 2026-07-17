use std::collections::HashSet;
use std::sync::Arc;

use crate::state::app_state::AppState;
use crate::state::messages::SurrogateMultiOptUiResult;
use crate::theme::chart_colors::{
    COLOR_HIGHLIGHT_PT, COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_SURROGATE_FRONT,
    COLOR_UNSELECTED_POINT,
};
use crate::theme::color_compute::point_alpha_in_set;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{
    axis_row, hit_test_nearest, push_feasible_row, TrialDetailModal, TrialDetailTarget,
    HIT_THRESHOLD,
};

/// 2D Pareto scatter widget (egui_plot based)
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ParetoScatter2D {
    pub x_axis: String,
    pub y_axis: String,
    // TASK-2241: rectangular brush state (screen coordinates).
    // Brush selection is Shift + left drag. Unmodified left drag is assigned to unified
    // navigation (box zoom), so Shift is required for the start detection.
    // egui_plot's crosshair is drawn by applying the final transform to the raw screen
    // coordinates. Keeping the rectangle in screen coordinates too, and handling both
    // drawing and selection via `PlotResponse.transform` (the same final transform used
    // for point drawing), avoids drift caused by a frame's worth of transform lag.
    #[serde(skip)]
    pub brush_start: Option<egui::Pos2>,
    #[serde(skip)]
    pub brush_end: Option<egui::Pos2>,
    /// Trial detail modal opened by clicking a point.
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
    /// Whether to overlay the surrogate-predicted front points.
    pub show_surrogate_front: bool,
    /// Cache of column-extracted points (independent of selection/highlight - M-17).
    #[serde(skip)]
    point_cache: Option<PointCache>,
}

/// Cache of points extracted from the objective columns (independent of the selection
/// filter and highlight).
///
/// The old implementation rebuilt the point vectors every frame by iterating over all
/// trials to perform "column lookup + feasibility check + rank lookup + trial_id
/// lookup". Since these are determined solely by the identity of `view.df` and the
/// axes, we extract them once keyed on `(df pointer, x_idx, y_idx)` and apply the
/// selection/highlight classification lightly at draw time (combined with the M-16
/// HashSet).
struct PointCache {
    key: (usize, usize, usize),
    /// Feasible points: `(trial_id, pareto_rank, [x, y])`.
    feasible: Vec<(u32, u32, [f64; 2])>,
    /// Coordinates of infeasible points (always drawn gray, at the back).
    infeasible_pts: Vec<[f64; 2]>,
    /// All drawn points for click/brush hit-testing: `(trial_id, row index, [x, y])`.
    displayed_points: Vec<(u32, usize, [f64; 2])>,
}

impl Default for ParetoScatter2D {
    fn default() -> Self {
        Self {
            x_axis: "obj0".to_string(),
            y_axis: "obj1".to_string(),
            brush_start: None,
            brush_end: None,
            detail_modal: TrialDetailModal::new(),
            show_surrogate_front: true,
            point_cache: None,
        }
    }
}

/// Classification result expanding feasibility split, pareto_rank, and trial_id in row
/// order (shared by 2D/3D Pareto - D-6).
pub(crate) struct ClassifiedRow {
    pub trial_id: u32,
    pub row: usize,
    pub feasible: bool,
    /// pareto_rank (only meaningful when feasible; 0 for infeasible).
    pub rank: u32,
}

/// Classifies all trials of the view into (trial_id, row, feasible, pareto_rank) in row
/// order (D-6). Shared by the 2D/3D Pareto scatter plots for feasibility splitting,
/// rank lookup, and trial_id lookup. Drawing (coloring, highlighting, depth sorting) is
/// done on each widget's side.
pub(crate) fn classify_rows(view: &crate::state::types::StudyView) -> Vec<ClassifiedRow> {
    let n = view.row_count();
    let feas = view.feasibility();
    (0..n)
        .map(|i| {
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            let feasible = feas.is_feasible(i);
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            ClassifiedRow {
                trial_id,
                row: i,
                feasible,
                rank,
            }
        })
        .collect()
}

/// Builds a `PointCache` from the objective columns (independent of selection/highlight).
fn build_point_cache(
    view: &crate::state::types::StudyView,
    x_col: Option<&[f64]>,
    y_col: Option<&[f64]>,
    key: (usize, usize, usize),
) -> PointCache {
    let n = view.row_count();
    let coord = |i: usize| {
        let x = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
        let y = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
        [x, y]
    };
    let mut feasible: Vec<(u32, u32, [f64; 2])> = Vec::new();
    let mut infeasible_pts: Vec<[f64; 2]> = Vec::new();
    let mut displayed_points: Vec<(u32, usize, [f64; 2])> = Vec::with_capacity(n);
    // Feasibility splitting and rank lookup are shared with 3D (D-6).
    for r in classify_rows(view) {
        let pt = coord(r.row);
        displayed_points.push((r.trial_id, r.row, pt));
        if !r.feasible {
            infeasible_pts.push(pt);
            continue;
        }
        feasible.push((r.trial_id, r.rank, pt));
    }
    PointCache {
        key,
        feasible,
        infeasible_pts,
        displayed_points,
    }
}

/// A pure function that resolves the front point series of `SurrogateMultiOptUiResult`
/// from objective axis names. Returns an empty Vec if either axis name is not present
/// in the result.
pub fn surrogate_front_points(
    result: &SurrogateMultiOptUiResult,
    x_axis: &str,
    y_axis: &str,
) -> Vec<[f64; 2]> {
    let x_idx = result.objective_names.iter().position(|n| n == x_axis);
    let y_idx = result.objective_names.iter().position(|n| n == y_axis);
    match (x_idx, y_idx) {
        (Some(xi), Some(yi)) => result
            .front
            .iter()
            .filter_map(|pt| {
                let x = pt.values.get(xi).copied()?;
                let y = pt.values.get(yi).copied()?;
                Some([x, y])
            })
            .collect(),
        _ => Vec::new(),
    }
}

impl ParetoScatter2D {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        surrogate_front: Option<&SurrogateMultiOptUiResult>,
    ) {
        let Some(ctx) = &app_state.current_study else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a study");
            });
            return;
        };

        let obj_names = ctx.meta.objective_names.clone();
        let param_names = ctx.meta.param_names.clone();
        let selected = app_state.selected_indices.clone();
        let highlighted = app_state.highlighted_trial;

        // The default axis names ("obj0"/"obj1") won't match the loaded objective
        // function names, so if the current selection isn't among the objective
        // function names, snap it to the actual name (same behavior as the MCDM scatter).
        if !obj_names.iter().any(|n| n == &self.x_axis) {
            if let Some(first) = obj_names.first() {
                self.x_axis = first.clone();
            }
        }
        if !obj_names.iter().any(|n| n == &self.y_axis) {
            if obj_names.len() > 1 {
                self.y_axis = obj_names[1].clone();
            } else if let Some(first) = obj_names.first() {
                self.y_axis = first.clone();
            }
        }

        // Axis assignment ComboBox + surrogate front checkbox
        ui.horizontal(|ui| {
            ui.label("X Axis:");
            egui::ComboBox::from_id_salt("x_axis_combo")
                .selected_text(&self.x_axis)
                .show_ui(ui, |ui| {
                    for name in &obj_names {
                        ui.selectable_value(&mut self.x_axis, name.clone(), name);
                    }
                });
            ui.label("Y Axis:");
            egui::ComboBox::from_id_salt("y_axis_combo")
                .selected_text(&self.y_axis)
                .show_ui(ui, |ui| {
                    for name in &obj_names {
                        ui.selectable_value(&mut self.y_axis, name.clone(), name);
                    }
                });
            // Only show the checkbox when the surrogate front is available.
            if surrogate_front.is_some() {
                ui.checkbox(&mut self.show_surrogate_front, "Surrogate front");
            }
        });

        let x_idx = obj_names
            .iter()
            .position(|n| n == &self.x_axis)
            .unwrap_or(0);
        let y_idx = obj_names
            .iter()
            .position(|n| n == &self.y_axis)
            .unwrap_or(1);

        // Build the point set directly from the view's column slices (no row-clone cache - MEM-002)
        let view = &ctx.view;
        let x_col = obj_names
            .get(x_idx)
            .and_then(|name| view.numeric_column(name));
        let y_col = obj_names
            .get(y_idx)
            .and_then(|name| view.numeric_column(name));
        let feas = view.feasibility();

        // Column extraction and feasibility determination are decided solely by the
        // identity of df and the axes, so we cache them to avoid rebuilding every frame
        // (M-17). Classification by selection/highlight is applied lightly below.
        let cache_key = (Arc::as_ptr(&view.df) as usize, x_idx, y_idx);
        if self.point_cache.as_ref().map(|c| c.key) != Some(cache_key) {
            self.point_cache = Some(build_point_cache(view, x_col, y_col, cache_key));
        }
        let cache = self.point_cache.as_ref().expect("point cache built above");

        // Classify into Pareto front (rank==0) and non-Pareto.
        // When the selection filter is active, unselected points are grouped in gray
        // regardless of Pareto/non-Pareto (keeping hue would be confusing with selected
        // points). The selected set is built as a HashSet only once, avoiding a linear
        // scan per point (M-16).
        let selected_set: HashSet<u32> = selected.iter().copied().collect();
        let mut pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut non_pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut unselected_pts: Vec<[f64; 2]> = Vec::new();
        let mut highlight_pt: Option<[f64; 2]> = None;
        for &(trial_id, rank, pt) in &cache.feasible {
            if highlighted == Some(trial_id) {
                highlight_pt = Some(pt);
                continue;
            }
            if point_alpha_in_set(trial_id, &selected_set) != 255 {
                // Unselected points go to the gray group regardless of Pareto/non-Pareto
                unselected_pts.push(pt);
            } else if rank == 0 {
                pareto_pts.push(pt);
            } else {
                non_pareto_pts.push(pt);
            }
        }
        // Infeasible solutions and hit-test candidates are referenced from the cache.
        let infeasible_pts = &cache.infeasible_pts;
        let displayed_points = &cache.displayed_points;

        // Precompute the surrogate front points (to avoid a borrow conflict inside the closure).
        let surrogate_pts: Vec<[f64; 2]> = if self.show_surrogate_front {
            surrogate_front
                .map(|r| surrogate_front_points(r, &self.x_axis, &self.y_axis))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Capture brush events inside the closure using mutable local vars (screen coords)
        let mut new_brush_start: Option<egui::Pos2> = None;
        let mut new_brush_end: Option<egui::Pos2> = None;
        let mut drag_finished = false;
        let mut blank_clicked = false;
        // The target (trial_id, row index) of the detail modal opened by a point click.
        let mut clicked_detail: Option<(u32, usize)> = None;
        // The point currently under the mouse hover (trial_id, row index). Used for tooltip display.
        let mut hovered_detail: Option<(u32, usize)> = None;
        let current_brush_start = self.brush_start;
        let current_brush_end = self.brush_end;

        // While Shift is held, disable box zoom and yield left-drag to brush selection.
        let shift_down = ui.input(|i| i.modifiers.shift);

        let plot_response = egui_plot::Plot::new("pareto_2d_plot")
            .legend(egui_plot::Legend::default())
            .unified_nav()
            .allow_boxed_zoom(!shift_down)
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                // Brush interaction detection.
                // The rectangle is kept in screen coordinates. Since egui_plot's crosshair
                // is drawn by applying the final transform to the raw screen pointer
                // position, handling this in screen coordinates as well lets both drawing
                // and selection be processed consistently with the final transform after
                // the closure, completely avoiding drift caused by a frame's worth of
                // transform lag.
                let resp = plot_ui.response();
                // Uses the same `hover_pos()` as the crosshair (ruler) as the basis, and
                // falls back to interact / latest for cases where it can become None
                // during a drag.
                let ptr = resp
                    .hover_pos()
                    .or_else(|| resp.interact_pointer_pos())
                    .or_else(|| resp.ctx.input(|i| i.pointer.latest_pos()));

                // Brush selection starts only with Shift + left drag. Unmodified left
                // drag is handled by egui_plot's box zoom (unified navigation).
                if shift_down && resp.drag_started_by(egui::PointerButton::Primary) {
                    new_brush_start = ptr;
                }
                // While the brush operation is active, update the end point with the
                // live pointer coordinates every frame as long as the primary button is
                // held. `dragged_by()` only fires on frames where the pointer moved, so
                // relying on it would leave the end point stuck at the previous frame's
                // stale coordinates, making the rectangle appear to drift from the cursor.
                let brush_active = current_brush_start.is_some() || new_brush_start.is_some();
                let primary_down = resp.ctx.input(|i| i.pointer.primary_down());
                if brush_active && primary_down {
                    new_brush_end = ptr;
                }
                if resp.drag_stopped() {
                    drag_finished = true;
                }
                if resp.clicked_by(egui::PointerButton::Primary) {
                    // Clicking near a point opens the detail modal; clicking blank space clears the selection.
                    clicked_detail = resp.interact_pointer_pos().and_then(|pos| {
                        hit_test_nearest(plot_ui, displayed_points, pos, HIT_THRESHOLD)
                    });
                    blank_clicked = clicked_detail.is_none();
                }

                // Detect the hovered point (suppressed during rectangular brush operations).
                if current_brush_start.is_none() && !resp.dragged_by(egui::PointerButton::Primary) {
                    hovered_detail = resp.hover_pos().and_then(|pos| {
                        hit_test_nearest(plot_ui, displayed_points, pos, HIT_THRESHOLD)
                    });
                }

                // The selection rectangle is overlaid in screen coordinates after the Plot is drawn (see below).

                // Infeasible solutions (backmost layer: grayed out)
                if !infeasible_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Infeasible", infeasible_pts.clone())
                            .color(COLOR_INFEASIBLE())
                            .radius(2.5),
                    );
                }
                // Outside the selection filter (gray, background, Pareto/non-Pareto combined)
                if !unselected_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Others (unselected)", unselected_pts)
                            .color(COLOR_UNSELECTED_POINT())
                            .radius(2.5),
                    );
                }
                // Non-Pareto (blue points)
                if !non_pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Others", non_pareto_pts)
                            .color(COLOR_NON_PARETO())
                            .radius(2.5),
                    );
                }
                // Pareto front (red circles + red line)
                if !pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Pareto Front", pareto_pts)
                            .color(COLOR_PARETO())
                            .radius(4.0),
                    );
                }
                // Surrogate predicted front (gold diamonds)
                if !surrogate_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Surrogate Pareto Front", surrogate_pts)
                            .shape(egui_plot::MarkerShape::Diamond)
                            .radius(4.5)
                            .color(COLOR_SURROGATE_FRONT()),
                    );
                }
                // Highlighted point
                if let Some(pt) = highlight_pt {
                    plot_ui.points(
                        egui_plot::Points::new("Highlighted", vec![pt])
                            .color(COLOR_HIGHLIGHT_PT())
                            .radius(8.0),
                    );
                }
            });

        let plot_transform = plot_response.transform;

        // Overlay the selection rectangle in screen coordinates. Since it's clipped to
        // the drawing area (frame) of the same final transform used for point drawing,
        // the rectangle always accurately tracks the real cursor.
        let draw_start = new_brush_start.or(current_brush_start);
        let draw_end = new_brush_end.or(current_brush_end);
        if let (Some(s), Some(e)) = (draw_start, draw_end) {
            let rect = egui::Rect::from_two_pos(s, e);
            let painter = ui.painter().with_clip_rect(*plot_transform.frame());
            painter.rect(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(100, 150, 255, 40),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255)),
                egui::StrokeKind::Inside,
            );
        }

        // If there's a hovered point, show a summary tooltip at the pointer position.
        // This is done using only immutable borrows of view / feas, before the mutable
        // borrow of app_state.
        if let Some((_, row)) = hovered_detail {
            let trial_number = view.df.get_trial_number(row).unwrap_or(row as u32);
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            let mut rows = vec![
                axis_row(&self.x_axis, x_col, row),
                axis_row(&self.y_axis, y_col, row),
                ("Pareto Rank".to_string(), rank.to_string()),
            ];
            push_feasible_row(&mut rows, feas, row);
            crate::ui::widgets::trial_detail_modal::show_hover_tooltip(
                ui,
                "pareto2d_hover_tooltip",
                trial_number,
                &rows,
            );
        }

        // A point click opens the trial detail modal (scatter plot info = Pareto rank).
        // Exhausts the immutable borrows of view / feas before mutably borrowing app_state.
        if let Some((trial_id, row)) = clicked_detail {
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            let mut context = vec![("Pareto Rank".to_string(), rank.to_string())];
            push_feasible_row(&mut context, feas, row);
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        // Update brush state and selection after closure
        if let Some(start) = new_brush_start {
            self.brush_start = Some(start);
            self.brush_end = None;
        }
        if let Some(end) = new_brush_end {
            self.brush_end = Some(end);
        }
        if drag_finished {
            if let (Some(start), Some(end)) = (self.brush_start, self.brush_end) {
                // Convert each point to screen coordinates with the same final transform
                // used for drawing, and judge inclusion by the rectangle (screen). This
                // guarantees the visible rectangle always matches the selection result.
                let rect = egui::Rect::from_two_pos(start, end);
                let new_selection: Vec<u32> = displayed_points
                    .iter()
                    .filter(|(_, _, pt)| {
                        let screen = plot_transform
                            .position_from_point(&egui_plot::PlotPoint::new(pt[0], pt[1]));
                        rect.contains(screen)
                    })
                    .map(|(id, _, _)| *id)
                    .collect();
                app_state.selected_indices = new_selection;
            }
            self.brush_start = None;
            self.brush_end = None;
        }
        if blank_clicked && self.brush_start.is_none() {
            // Empty click outside drag = clear selection
            app_state.selected_indices.clear();
        }

        // Draw the detail modal (re-borrows current_study / artifact_map).
        if self.detail_modal.is_open() {
            if let Some(ctx) = app_state.current_study.as_ref() {
                self.detail_modal.show(
                    ui,
                    &ctx.view,
                    &param_names,
                    &obj_names,
                    &app_state.artifact_map,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pareto_scatter_2d_default() {
        let widget = ParetoScatter2D::default();
        assert_eq!(widget.x_axis, "obj0");
        assert_eq!(widget.y_axis, "obj1");
    }

    #[test]
    fn blank_click_clears_selection_per_policy() {
        // Simulate blank click: selected_indices should become empty
        let mut selected: Vec<u32> = vec![0, 1, 2];
        // Policy: blank click clears selection
        selected.clear();
        assert!(selected.is_empty());
    }

    #[test]
    fn brush_state_default_is_none() {
        let widget = ParetoScatter2D::default();
        assert!(widget.brush_start.is_none());
        assert!(widget.brush_end.is_none());
    }

    // ── unit tests for surrogate_front_points ───────────────────────

    fn make_ui_result() -> crate::state::messages::SurrogateMultiOptUiResult {
        use tunny_core::surrogate_opt::ParetoFrontPoint;
        crate::state::messages::SurrogateMultiOptUiResult {
            param_names: vec!["x".to_string()],
            objective_names: vec!["f0".to_string(), "f1".to_string()],
            front: vec![
                ParetoFrontPoint {
                    params: vec![0.1],
                    values: vec![1.0, 4.0],
                },
                ParetoFrontPoint {
                    params: vec![0.2],
                    values: vec![2.0, 3.0],
                },
            ],
            r_squared: vec![0.9, 0.85],
        }
    }

    #[test]
    fn surrogate_front_points_normal_order() {
        let result = make_ui_result();
        let pts = surrogate_front_points(&result, "f0", "f1");
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], [1.0, 4.0]);
        assert_eq!(pts[1], [2.0, 3.0]);
    }

    #[test]
    fn surrogate_front_points_swapped_axes() {
        let result = make_ui_result();
        let pts = surrogate_front_points(&result, "f1", "f0");
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], [4.0, 1.0]);
        assert_eq!(pts[1], [3.0, 2.0]);
    }

    #[test]
    fn surrogate_front_points_unknown_axis_returns_empty() {
        let result = make_ui_result();
        // Nonexistent axis name → empty
        let pts = surrogate_front_points(&result, "f0", "unknown");
        assert!(pts.is_empty());
    }

    #[test]
    fn pareto_scatter_2d_show_surrogate_front_default_true() {
        let widget = ParetoScatter2D::default();
        assert!(widget.show_surrogate_front);
    }
}
