use std::collections::HashSet;

use crate::state::app_state::AppState;
use crate::theme::chart_colors::{
    COLOR_HIGHLIGHT_PT, COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO,
};
use crate::theme::color_compute::point_alpha_in_set;
use crate::theme::TOOLBAR_BTN_FG;
use crate::ui::widgets::pareto_2d::classify_rows;
use crate::ui::widgets::scatter_3d::{
    compute_range_from_col, draw_3d_axes, draw_3d_grid, draw_depth_sorted_points, project_value_3d,
    setup_3d_canvas, show_hover_and_click_detail, show_objective_combo, ArcballCamera, DepthPoint,
    Range3DCache,
};
use crate::ui::widgets::trial_detail_modal::{axis_row, push_feasible_row, TrialDetailModal};

/// The Pareto 3D chart widget
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pareto3dChart {
    pub x_objective: usize,
    pub y_objective: usize,
    pub z_objective: usize,
    pub camera: ArcballCamera,
    #[serde(skip)]
    range_cache: Range3DCache<(usize, usize, usize, usize)>,
    /// Whether to show infeasible solutions (effective only for constrained Studies)
    pub show_infeasible: bool,
    /// Trial detail modal opened by clicking a point
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
}

impl Default for Pareto3dChart {
    fn default() -> Self {
        Self {
            x_objective: 0,
            y_objective: 1,
            z_objective: 2,
            camera: ArcballCamera::isometric_default(),
            range_cache: Range3DCache::default(),
            show_infeasible: true,
            detail_modal: TrialDetailModal::new(),
        }
    }
}

/// Returns the drawing color and size of a 3D point (feasible only) from its Pareto rank
/// and selection state. Excluding infeasible points and applying COLOR_INFEASIBLE is
/// done by the caller.
pub fn determine_point_color_3d(rank: u32, alpha: u8) -> (egui::Color32, f32) {
    let (base_color, radius) = if rank == 0 {
        (COLOR_PARETO(), 5.0_f32)
    } else {
        (COLOR_NON_PARETO(), 3.0_f32)
    };
    let color = if alpha == 255 {
        base_color
    } else {
        egui::Color32::from_rgba_unmultiplied(base_color.r(), base_color.g(), base_color.b(), 60)
    };
    (color, radius)
}

impl Pareto3dChart {
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        let Some(ctx) = &app_state.current_study else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a study");
            });
            return;
        };

        let obj_names = &ctx.meta.objective_names;
        if obj_names.len() < 3 {
            ui.centered_and_justified(|ui| {
                ui.label("Need at least 3 objectives for 3D view");
            });
            return;
        }

        let ctx = app_state.current_study.as_ref().unwrap();
        let view = &ctx.view;
        let trial_count = view.row_count();

        let range_cache_key = (
            self.x_objective,
            self.y_objective,
            self.z_objective,
            trial_count,
        );
        let col = |idx: usize| obj_names.get(idx).and_then(|n| view.numeric_column(n));
        let ranges = self.range_cache.get_or_compute(range_cache_key, || {
            [
                compute_range_from_col(col(self.x_objective)),
                compute_range_from_col(col(self.y_objective)),
                compute_range_from_col(col(self.z_objective)),
            ]
        });
        let [(x_min, x_max), (y_min, y_max), (z_min, z_max)] = ranges;

        let has_constraints = view.feasibility().has_constraints();

        ui.horizontal(|ui| {
            show_objective_combo(ui, "X:", "pareto3d_x", &mut self.x_objective, obj_names);
            show_objective_combo(ui, "Y:", "pareto3d_y", &mut self.y_objective, obj_names);
            show_objective_combo(ui, "Z:", "pareto3d_z", &mut self.z_objective, obj_names);
            if has_constraints {
                ui.separator();
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
        });

        let (painter, _rect, project, click_pos, hover_pos) = setup_3d_canvas(ui, &mut self.camera);

        draw_3d_grid(&painter, &project);

        let x_name = obj_names.get(self.x_objective).cloned().unwrap_or_default();
        let y_name = obj_names.get(self.y_objective).cloned().unwrap_or_default();
        let z_name = obj_names.get(self.z_objective).cloned().unwrap_or_default();
        draw_3d_axes(
            &painter,
            &project,
            [&x_name, &y_name, &z_name],
            [(x_min, x_max), (y_min, y_max), (z_min, z_max)],
        );

        // The selected set is built as a HashSet only once, avoiding a linear scan per point (M-16).
        let selected_set: HashSet<u32> = app_state.selected_indices.iter().copied().collect();
        let highlighted = app_state.highlighted_trial;
        let x_col = obj_names
            .get(self.x_objective)
            .and_then(|n| view.numeric_column(n));
        let y_col = obj_names
            .get(self.y_objective)
            .and_then(|n| view.numeric_column(n));
        let z_col = obj_names
            .get(self.z_objective)
            .and_then(|n| view.numeric_column(n));
        let feas = view.feasibility();

        let mut draw_calls: Vec<DepthPoint> = Vec::with_capacity(trial_count);
        let mut infeasible_draw_calls: Vec<DepthPoint> = Vec::with_capacity(32);
        let mut highlight_call: Option<egui::Pos2> = None;
        let show_infeasible = self.show_infeasible;
        // For left-click point hit testing (trial_id, row, and screen coordinates of drawn points)
        let mut candidates: Vec<(u32, usize, egui::Pos2)> = Vec::with_capacity(trial_count);

        // Feasibility splitting and rank lookup are shared with 2D (D-6). normalize→project is a shared helper (D-1).
        for r in classify_rows(view) {
            let i = r.row;
            let xv = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let yv = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let zv = z_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let (screen_pos, depth) = project_value_3d(&project, [xv, yv, zv], ranges);
            let trial_id = r.trial_id;

            if !r.feasible {
                if show_infeasible {
                    infeasible_draw_calls.push(DepthPoint {
                        pos: screen_pos,
                        depth,
                        color: COLOR_INFEASIBLE(),
                        radius: 3.0,
                    });
                    candidates.push((trial_id, i, screen_pos));
                }
                continue;
            }

            candidates.push((trial_id, i, screen_pos));

            if highlighted == Some(trial_id) {
                highlight_call = Some(screen_pos);
                continue;
            }

            let alpha = point_alpha_in_set(trial_id, &selected_set);
            let (color, radius) = determine_point_color_3d(r.rank, alpha);
            draw_calls.push(DepthPoint {
                pos: screen_pos,
                depth,
                color,
                radius,
            });
        }

        draw_depth_sorted_points(&painter, &mut infeasible_draw_calls, None);
        draw_depth_sorted_points(&painter, &mut draw_calls, None);

        if let Some(pos) = highlight_call {
            painter.circle_filled(pos, 8.0, COLOR_HIGHLIGHT_PT());
            painter.circle_stroke(pos, 9.5, egui::Stroke::new(1.5, TOOLBAR_BTN_FG()));
        }

        show_hover_and_click_detail(
            ui,
            view,
            &candidates,
            hover_pos,
            click_pos,
            "pareto3d_hover_tooltip",
            &mut self.detail_modal,
            |row| {
                let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
                let mut rows = vec![
                    axis_row(&x_name, x_col, row),
                    axis_row(&y_name, y_col, row),
                    axis_row(&z_name, z_col, row),
                    ("Pareto Rank".to_string(), rank.to_string()),
                ];
                push_feasible_row(&mut rows, feas, row);
                rows
            },
            |row| {
                let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
                let mut context = vec![("Pareto Rank".to_string(), rank.to_string())];
                push_feasible_row(&mut context, feas, row);
                context
            },
        );

        // Draws the detail modal.
        if self.detail_modal.is_open() {
            self.detail_modal.show(
                ui,
                view,
                &ctx.meta.param_names,
                obj_names,
                &app_state.artifact_map,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::chart_colors::{COLOR_NON_PARETO, COLOR_PARETO};

    #[test]
    fn pareto_3d_chart_default_objectives() {
        let chart = Pareto3dChart::default();
        assert_eq!(chart.x_objective, 0);
        assert_eq!(chart.y_objective, 1);
        assert_eq!(chart.z_objective, 2);
        assert_ne!(chart.camera.rotation, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn tc_cav_pareto3d_show_infeasible_default_true() {
        let chart = Pareto3dChart::default();
        assert!(
            chart.show_infeasible,
            "show_infeasible must default to true"
        );
    }

    #[test]
    fn tc_cav_pareto_front_returns_pareto_color() {
        let (color, radius) = determine_point_color_3d(0, 255);
        assert_eq!(color, COLOR_PARETO());
        assert!((radius - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tc_cav_non_pareto_returns_non_pareto_color() {
        let (color, _radius) = determine_point_color_3d(1, 255);
        assert_eq!(color, COLOR_NON_PARETO());
    }

    #[test]
    fn tc_cav_dimmed_point_uses_alpha_60() {
        let (color, _) = determine_point_color_3d(0, 60);
        assert_eq!(color.a(), 60);
    }
}
