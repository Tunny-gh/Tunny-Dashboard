use crate::state::app_state::AppState;
use crate::theme::chart_colors::{COLOR_INFEASIBLE, COLOR_NON_PARETO_DIM};
use crate::theme::colormap_name::colormap_from_name;
use crate::ui::widgets::scatter_3d::{
    compute_range_from_col, draw_3d_axes, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    show_objective_combo, ArcballCamera,
};

/// クラスタ 3D 散布図ウィジェット
pub struct ClusterScatter3D {
    pub x_objective: usize,
    pub y_objective: usize,
    pub z_objective: usize,
    pub camera: ArcballCamera,
    pub show_infeasible: bool,
    range_cache: [(f64, f64); 3],
    range_cache_key: (usize, usize, usize, usize),
}

impl Default for ClusterScatter3D {
    fn default() -> Self {
        Self {
            x_objective: 0,
            y_objective: 1,
            z_objective: 2,
            camera: ArcballCamera {
                rotation: [-0.2391, 0.3696, 0.0990, 0.8924],
                ..Default::default()
            },
            show_infeasible: true,
            range_cache: [(-1.0, 1.0); 3],
            range_cache_key: (usize::MAX, usize::MAX, usize::MAX, 0),
        }
    }
}

impl ClusterScatter3D {
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
                ui.label("Need at least 3 objectives for 3D cluster view");
            });
            return;
        }

        let ctx = app_state.current_study.as_ref().unwrap();
        let obj_names = &ctx.meta.objective_names;
        let view = &ctx.view;
        let trial_count = view.row_count();
        let has_constraints = ctx.meta.has_constraints;

        // Range cache
        let cache_key = (
            self.x_objective,
            self.y_objective,
            self.z_objective,
            trial_count,
        );
        if self.range_cache_key != cache_key {
            let col = |idx: usize| obj_names.get(idx).and_then(|n| view.numeric_column(n));
            self.range_cache = [
                compute_range_from_col(col(self.x_objective)),
                compute_range_from_col(col(self.y_objective)),
                compute_range_from_col(col(self.z_objective)),
            ];
            self.range_cache_key = cache_key;
        }
        let [(x_min, x_max), (y_min, y_max), (z_min, z_max)] = self.range_cache;

        let x_name = obj_names.get(self.x_objective).cloned().unwrap_or_default();
        let y_name = obj_names.get(self.y_objective).cloned().unwrap_or_default();
        let z_name = obj_names.get(self.z_objective).cloned().unwrap_or_default();

        // Column data
        let x_col = obj_names
            .get(self.x_objective)
            .and_then(|n| view.numeric_column(n));
        let y_col = obj_names
            .get(self.y_objective)
            .and_then(|n| view.numeric_column(n));
        let z_col = obj_names
            .get(self.z_objective)
            .and_then(|n| view.numeric_column(n));
        let is_feasible_col = view.numeric_column("is_feasible");

        // Axis selectors
        ui.horizontal(|ui| {
            show_objective_combo(ui, "X:", "clu3d_x", &mut self.x_objective, obj_names);
            show_objective_combo(ui, "Y:", "clu3d_y", &mut self.y_objective, obj_names);
            show_objective_combo(ui, "Z:", "clu3d_z", &mut self.z_objective, obj_names);
            if has_constraints {
                ui.separator();
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
        });

        // Cluster coloring
        let n_clusters = app_state
            .cluster_result
            .as_ref()
            .map(|r| r.n_clusters)
            .unwrap_or(1)
            .max(1);
        let has_cluster = app_state.cluster_result.is_some();
        let colormap = colormap_from_name(&app_state.selected_colormap);
        let cluster_color = |label: i32| -> egui::Color32 {
            if label < 0 {
                return egui::Color32::GRAY;
            }
            let t = if n_clusters == 1 {
                0.5_f32
            } else {
                (label as f32 / (n_clusters - 1) as f32).clamp(0.0, 1.0)
            };
            colormap.interpolate(t)
        };

        let (painter, rect, project) = setup_3d_canvas(ui, &mut self.camera);

        draw_3d_grid(&painter, &project);
        draw_3d_axes(
            &painter,
            &project,
            [&x_name, &y_name, &z_name],
            [(x_min, x_max), (y_min, y_max), (z_min, z_max)],
        );

        // Collect points
        let show_infeasible = self.show_infeasible;
        let mut feasible_pts: Vec<(egui::Pos2, f32, egui::Color32)> =
            Vec::with_capacity(trial_count);
        let mut infeasible_pts: Vec<(egui::Pos2, f32)> = Vec::new();
        // クラスタリング対象外（非パレートフロント）の実行可能解 → 半透明で背面描画
        let mut other_pts: Vec<(egui::Pos2, f32)> = Vec::new();

        for i in 0..trial_count {
            let xv = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let yv = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let zv = z_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let clip = [
                normalize_to_clip(xv, x_min, x_max),
                normalize_to_clip(yv, y_min, y_max),
                normalize_to_clip(zv, z_min, z_max),
            ];
            let (pos, depth) = project(clip);

            let feasible = is_feasible_col
                .and_then(|c| c.get(i))
                .map(|&v| v > 0.5)
                .unwrap_or(true);

            if !feasible {
                if show_infeasible {
                    infeasible_pts.push((pos, depth));
                }
                continue;
            }

            let label = app_state
                .cluster_result
                .as_ref()
                .and_then(|r| r.labels.get(i).copied())
                .unwrap_or(0);

            if has_cluster && label < 0 {
                // クラスタリング済みだが非パレートフロント → 半透明で描画
                other_pts.push((pos, depth));
            } else {
                feasible_pts.push((pos, depth, cluster_color(label)));
            }
        }

        infeasible_pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _) in &infeasible_pts {
            painter.circle_filled(*pos, 3.0, COLOR_INFEASIBLE);
        }
        other_pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _) in &other_pts {
            painter.circle_filled(*pos, 2.5, COLOR_NON_PARETO_DIM);
        }
        feasible_pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _, color) in &feasible_pts {
            painter.circle_filled(*pos, 3.5, *color);
        }

        if !has_cluster {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Run clustering first (Cluster tab → Run button)",
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(180, 180, 180),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_scatter_3d_default_axes() {
        let w = ClusterScatter3D::default();
        assert_eq!(w.x_objective, 0);
        assert_eq!(w.y_objective, 1);
        assert_eq!(w.z_objective, 2);
        assert!(w.show_infeasible);
        assert!(!w.camera.is_identity_rotation());
    }
}
