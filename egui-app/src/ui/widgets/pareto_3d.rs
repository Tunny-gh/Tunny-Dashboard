use crate::state::app_state::AppState;
use crate::theme::chart_colors::{
    COLOR_HIGHLIGHT_PT, COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO,
};
use crate::theme::color_compute::compute_point_alpha;
use crate::theme::TOOLBAR_BTN_FG;
use crate::ui::widgets::scatter_3d::{
    compute_range_from_col, draw_3d_axes, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    show_objective_combo, ArcballCamera,
};

/// Pareto 3D チャートウィジェット
pub struct Pareto3dChart {
    pub x_objective: usize,
    pub y_objective: usize,
    pub z_objective: usize,
    pub camera: ArcballCamera,
    range_cache: [(f64, f64); 3],
    range_cache_key: (usize, usize, usize, usize),
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
}

impl Default for Pareto3dChart {
    fn default() -> Self {
        // Y軸45° + X軸-30° のアイソメトリック初期視点
        let camera = ArcballCamera {
            rotation: [-0.2391, 0.3696, 0.0990, 0.8924],
            ..Default::default()
        };
        Self {
            x_objective: 0,
            y_objective: 1,
            z_objective: 2,
            camera,
            range_cache: [(-1.0, 1.0); 3],
            range_cache_key: (usize::MAX, usize::MAX, usize::MAX, 0),
            show_infeasible: true,
        }
    }
}

/// Pareto ランク・選択状態から 3D 点（feasible のみ）の描画色とサイズを返す。
/// infeasible 点の除外と COLOR_INFEASIBLE 適用は呼び出し元で行う。
pub fn determine_point_color_3d(rank: u32, alpha: u8) -> (egui::Color32, f32) {
    let (base_color, radius) = if rank == 0 {
        (COLOR_PARETO, 5.0_f32)
    } else {
        (COLOR_NON_PARETO, 3.0_f32)
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

        let downsample_indices = app_state.downsample_cache.scatter.clone();
        let ctx = app_state.current_study.as_ref().unwrap();
        let view = &ctx.view;
        let trial_count = view.row_count();

        let range_cache_key = (
            self.x_objective,
            self.y_objective,
            self.z_objective,
            trial_count,
        );
        if self.range_cache_key != range_cache_key {
            let col = |idx: usize| obj_names.get(idx).and_then(|n| view.numeric_column(n));
            self.range_cache = [
                compute_range_from_col(col(self.x_objective)),
                compute_range_from_col(col(self.y_objective)),
                compute_range_from_col(col(self.z_objective)),
            ];
            self.range_cache_key = range_cache_key;
        }
        let [(x_min, x_max), (y_min, y_max), (z_min, z_max)] = self.range_cache;

        let has_constraints = ctx.meta.has_constraints;

        ui.horizontal(|ui| {
            show_objective_combo(ui, "X:", "pareto3d_x", &mut self.x_objective, obj_names);
            show_objective_combo(ui, "Y:", "pareto3d_y", &mut self.y_objective, obj_names);
            show_objective_combo(ui, "Z:", "pareto3d_z", &mut self.z_objective, obj_names);
            if has_constraints {
                ui.separator();
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
        });

        let (painter, _rect, project) = setup_3d_canvas(ui, &mut self.camera);

        draw_3d_grid(&painter, &project);

        let x_name = obj_names.get(self.x_objective).cloned().unwrap_or_default();
        let y_name = obj_names.get(self.y_objective).cloned().unwrap_or_default();
        let z_name = obj_names.get(self.z_objective).cloned().unwrap_or_default();
        draw_3d_axes(
            &painter, &project,
            &x_name, &y_name, &z_name,
            x_min, x_max, y_min, y_max, z_min, z_max,
        );

        let selected = &app_state.selected_indices;
        let highlighted = app_state.highlighted_trial;
        let x_col = obj_names.get(self.x_objective).and_then(|n| view.numeric_column(n));
        let y_col = obj_names.get(self.y_objective).and_then(|n| view.numeric_column(n));
        let z_col = obj_names.get(self.z_objective).and_then(|n| view.numeric_column(n));
        let is_feasible_col = view.numeric_column("is_feasible");

        let displayed: Vec<usize> = match downsample_indices.as_deref() {
            Some(idx) => idx
                .iter()
                .map(|&i| i as usize)
                .filter(|&i| i < trial_count)
                .collect(),
            None => (0..trial_count).collect(),
        };

        let mut draw_calls: Vec<(egui::Pos2, f32, egui::Color32, f32)> =
            Vec::with_capacity(displayed.len());
        let mut infeasible_draw_calls: Vec<(egui::Pos2, f32, egui::Color32, f32)> =
            Vec::with_capacity(32);
        let mut highlight_call: Option<egui::Pos2> = None;
        let show_infeasible = self.show_infeasible;

        for i in displayed {
            let xv = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let yv = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let zv = z_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let clip = [
                normalize_to_clip(xv, x_min, x_max),
                normalize_to_clip(yv, y_min, y_max),
                normalize_to_clip(zv, z_min, z_max),
            ];
            let (screen_pos, depth) = project(clip);
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);

            let feasible = is_feasible_col
                .and_then(|c| c.get(i))
                .map(|&v| v > 0.5)
                .unwrap_or(true);

            if !feasible {
                if show_infeasible {
                    infeasible_draw_calls.push((screen_pos, depth, COLOR_INFEASIBLE, 3.0));
                }
                continue;
            }

            if highlighted == Some(trial_id) {
                highlight_call = Some(screen_pos);
                continue;
            }

            let alpha = compute_point_alpha(trial_id, selected);
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            let (color, radius) = determine_point_color_3d(rank, alpha);
            draw_calls.push((screen_pos, depth, color, radius));
        }

        infeasible_draw_calls
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _, color, radius) in &infeasible_draw_calls {
            painter.circle_filled(*pos, *radius, *color);
        }

        draw_calls.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _, color, radius) in &draw_calls {
            painter.circle_filled(*pos, *radius, *color);
        }

        if let Some(pos) = highlight_call {
            painter.circle_filled(pos, 8.0, COLOR_HIGHLIGHT_PT);
            painter.circle_stroke(pos, 9.5, egui::Stroke::new(1.5, TOOLBAR_BTN_FG));
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
        assert!(!chart.camera.is_identity_rotation());
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
        assert_eq!(color, COLOR_PARETO);
        assert!((radius - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn tc_cav_non_pareto_returns_non_pareto_color() {
        let (color, _radius) = determine_point_color_3d(1, 255);
        assert_eq!(color, COLOR_NON_PARETO);
    }

    #[test]
    fn tc_cav_dimmed_point_uses_alpha_60() {
        let (color, _) = determine_point_color_3d(0, 60);
        assert_eq!(color.a(), 60);
    }
}
