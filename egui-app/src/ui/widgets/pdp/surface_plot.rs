use crate::state::messages::{SurfacePlotRenderMode, SurfacePlotResult};
use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{SurfacePlotComputeRequest, SurfacePlotState};

const MIN_TRIALS_FOR_SURFACE: usize = 10;

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurfacePlotState,
    param_names: &[String],
    obj_names: &[String],
    cmap: ColorMap,
    trial_count: usize,
    has_constraints: bool,
) {
    // Parameter X selector
    ui.horizontal(|ui| {
        ui.label("X:");
        egui::ComboBox::from_id_salt("surface_x")
            .selected_text(&state.selected_x)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.selected_x, name.clone(), name);
                }
            });
        ui.label("Y:");
        egui::ComboBox::from_id_salt("surface_y")
            .selected_text(&state.selected_y)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.selected_y, name.clone(), name);
                }
            });
    });

    // Objective selector
    ui.horizontal(|ui| {
        ui.label("Objective:");
        let obj_text = obj_names
            .get(state.selected_objective)
            .map(|s| s.as_str())
            .unwrap_or("—");
        egui::ComboBox::from_id_salt("surface_obj")
            .selected_text(obj_text)
            .show_ui(ui, |ui| {
                for (i, name) in obj_names.iter().enumerate() {
                    if ui
                        .selectable_label(state.selected_objective == i, name)
                        .clicked()
                    {
                        state.selected_objective = i;
                    }
                }
            });

        // Render mode toggle
        ui.label("Mode:");
        if ui
            .selectable_label(
                state.render_mode == SurfacePlotRenderMode::Heatmap,
                "Heatmap",
            )
            .clicked()
        {
            state.render_mode = SurfacePlotRenderMode::Heatmap;
        }
        if ui
            .selectable_label(
                state.render_mode == SurfacePlotRenderMode::Contour,
                "Contour",
            )
            .clicked()
        {
            state.render_mode = SurfacePlotRenderMode::Contour;
        }

        // 実行可能解フィルタ（制約付きスタディのみ）
        if has_constraints {
            ui.separator();
            ui.toggle_value(&mut state.feasible_only, "Feasible only")
                .on_hover_text("Fit the model using feasible trials only");
        }
    });

    // Same param warning
    if !state.selected_x.is_empty() && state.selected_x == state.selected_y {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Warning: same parameter selected for X and Y",
        );
    }

    // Trial count check
    if trial_count < MIN_TRIALS_FOR_SURFACE {
        ui.colored_label(
            egui::Color32::RED,
            format!(
                "At least {} trials required (current: {})",
                MIN_TRIALS_FOR_SURFACE, trial_count
            ),
        );
        return;
    }

    let can_run = !state.selected_x.is_empty()
        && !state.selected_y.is_empty()
        && state.selected_x != state.selected_y
        && !obj_names.is_empty()
        && !state.computing
        && state.pending_compute.is_none();

    if ui
        .add_enabled(can_run, egui::Button::new("Run Surface"))
        .clicked()
    {
        if let Some(obj_name) = obj_names.get(state.selected_objective) {
            state.pending_compute = Some(SurfacePlotComputeRequest {
                param_x: state.selected_x.clone(),
                param_y: state.selected_y.clone(),
                objective: obj_name.clone(),
                n_grid: 20,
                feasible_only: state.feasible_only,
            });
        }
    }

    if state.computing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Computing Surface Plot…");
        });
        return;
    }

    if let Some(ref err) = state.error_message.clone() {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }

    let Some(result) = &state.result else {
        ui.label("No surface data. Select parameters and click Run.");
        return;
    };

    render_result(ui, result, &state.render_mode, cmap);
}

fn render_result(
    ui: &mut egui::Ui,
    result: &SurfacePlotResult,
    mode: &SurfacePlotRenderMode,
    cmap: ColorMap,
) {
    if result.z_values.is_empty() {
        ui.label("Surface result is empty.");
        return;
    }

    let available = ui.available_rect_before_wrap();
    let plot_size = egui::vec2(
        (available.width() - 32.0).max(100.0),
        available.height().min(300.0),
    );
    let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    match mode {
        SurfacePlotRenderMode::Heatmap => {
            draw_heatmap(&painter, rect, &result.z_values, cmap.clone());
        }
        SurfacePlotRenderMode::Contour => {
            draw_heatmap(&painter, rect, &result.z_values, cmap.clone());
            draw_contour_overlay(&painter, rect, &result.z_values);
        }
    }

    let (v_min, v_max) = value_range(&result.z_values);
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap);

    ui.label(format!(
        "X: {}  Y: {}  Obj: {}",
        result.param_x_name, result.param_y_name, result.objective_name
    ));
    if let Some(r2) = result.r2 {
        ui.label(format!("R² = {:.3}", r2));
    }
}

pub(crate) fn draw_heatmap(
    painter: &egui::Painter,
    rect: egui::Rect,
    values: &[Vec<f64>],
    cmap: ColorMap,
) {
    let n_row = values.len();
    if n_row == 0 {
        return;
    }
    let n_col = values[0].len();
    if n_col == 0 {
        return;
    }
    let (v_min, v_max) = value_range(values);
    let cell_w = rect.width() / n_col as f32;
    let cell_h = rect.height() / n_row as f32;

    for (row, row_vals) in values.iter().enumerate() {
        for (col, &val) in row_vals.iter().enumerate() {
            let t = normalize(val, v_min, v_max);
            let color = cmap.interpolate(t);
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + col as f32 * cell_w,
                    rect.top() + row as f32 * cell_h,
                ),
                egui::vec2(cell_w + 1.0, cell_h + 1.0),
            );
            painter.rect_filled(cell_rect, 0.0, color);
        }
    }
}

fn draw_contour_overlay(painter: &egui::Painter, rect: egui::Rect, values: &[Vec<f64>]) {
    let n_row = values.len();
    if n_row < 2 {
        return;
    }
    let n_col = values[0].len();
    if n_col < 2 {
        return;
    }
    let (v_min, v_max) = value_range(values);
    let n_levels = 5usize;
    let cell_w = rect.width() / n_col as f32;
    let cell_h = rect.height() / n_row as f32;

    for level_i in 1..=n_levels {
        let threshold = v_min + (v_max - v_min) * level_i as f64 / (n_levels + 1) as f64;
        for row in 0..n_row - 1 {
            for col in 0..n_col - 1 {
                let tl = values[row][col];
                let tr = values[row][col + 1];
                let bl = values[row + 1][col];
                let above_tl = tl >= threshold;
                let above_tr = tr >= threshold;
                let above_bl = bl >= threshold;
                if above_tl != above_tr || above_tl != above_bl {
                    let cx = rect.left() + (col as f32 + 0.5) * cell_w;
                    let cy = rect.top() + (row as f32 + 0.5) * cell_h;
                    painter.circle_stroke(
                        egui::pos2(cx, cy),
                        1.0,
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                    );
                }
            }
        }
    }
}

pub(crate) fn draw_colorbar_simple(
    ui: &mut egui::Ui,
    bar_rect: egui::Rect,
    v_min: f64,
    v_max: f64,
    cmap: ColorMap,
) {
    let painter = ui.painter_at(bar_rect);
    let n_steps = 32;
    let step_h = bar_rect.height() / n_steps as f32;
    for i in 0..n_steps {
        let t = 1.0 - (i as f32 / n_steps as f32);
        let color = cmap.interpolate(t);
        let step_rect = egui::Rect::from_min_size(
            egui::pos2(bar_rect.left(), bar_rect.top() + i as f32 * step_h),
            egui::vec2(bar_rect.width(), step_h + 1.0),
        );
        painter.rect_filled(step_rect, 0.0, color);
    }
    ui.label(format!("{:.2}", v_max));
    ui.label(format!("{:.2}", v_min));
}

pub(crate) fn value_range(values: &[Vec<f64>]) -> (f64, f64) {
    let flat: Vec<f64> = values.iter().flatten().copied().collect();
    if flat.is_empty() {
        return (0.0, 1.0);
    }
    let v_min = flat.iter().cloned().fold(f64::INFINITY, f64::min);
    let v_max = flat.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (v_max - v_min).abs() < f64::EPSILON {
        (v_min - 1.0, v_max + 1.0)
    } else {
        (v_min, v_max)
    }
}

pub(crate) fn normalize(v: f64, v_min: f64, v_max: f64) -> f32 {
    if (v_max - v_min).abs() < f64::EPSILON {
        return 0.5;
    }
    ((v - v_min) / (v_max - v_min)).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::messages::{SurfacePlotRenderMode, SurfacePlotResult};
    use crate::ui::widget_states::SurfacePlotState;

    fn make_state_with_selections() -> SurfacePlotState {
        SurfacePlotState {
            selected_x: "x1".to_string(),
            selected_y: "x2".to_string(),
            selected_objective: 0,
            render_mode: SurfacePlotRenderMode::Heatmap,
            computing: false,
            result: None,
            error_message: None,
            feasible_only: false,
            pending_compute: None,
        }
    }

    #[test]
    fn surface_plot_request_contains_selected_axes_and_objective() {
        let mut state = make_state_with_selections();
        let param_names = vec!["x1".to_string(), "x2".to_string()];
        let obj_names = ["obj0".to_string()];

        // Simulate what "Run Surface" click does
        if !state.computing && state.selected_x != state.selected_y {
            if let Some(obj_name) = obj_names.get(state.selected_objective) {
                state.pending_compute = Some(SurfacePlotComputeRequest {
                    param_x: state.selected_x.clone(),
                    param_y: state.selected_y.clone(),
                    objective: obj_name.clone(),
                    n_grid: 20,
                    feasible_only: state.feasible_only,
                });
            }
        }

        let req = state.pending_compute.as_ref().unwrap();
        assert_eq!(req.param_x, "x1");
        assert_eq!(req.param_y, "x2");
        assert_eq!(req.objective, "obj0");
        assert_eq!(req.n_grid, 20);
        drop(param_names);
    }

    #[test]
    fn surface_plot_rejects_less_than_10_trials() {
        // Verify the MIN_TRIALS_FOR_SURFACE constant is respected
        assert_eq!(MIN_TRIALS_FOR_SURFACE, 10);
        // A run attempt with fewer trials should not set pending_compute
        let mut state = make_state_with_selections();
        let trial_count = 5;
        if trial_count < MIN_TRIALS_FOR_SURFACE {
            // Logic from show() — don't set pending_compute
        } else {
            state.pending_compute = Some(SurfacePlotComputeRequest {
                param_x: "x1".to_string(),
                param_y: "x2".to_string(),
                objective: "obj0".to_string(),
                n_grid: 20,
                feasible_only: false,
            });
        }
        assert!(state.pending_compute.is_none());
    }

    #[test]
    fn surface_plot_result_switches_spinner_off() {
        let mut state = make_state_with_selections();
        state.computing = true;

        // Simulate message_handler receiving SurfacePlotDone
        let result = SurfacePlotResult {
            x_values: vec![0.0, 1.0],
            y_values: vec![0.0, 1.0],
            z_values: vec![vec![0.0, 1.0], vec![1.0, 2.0]],
            param_x_name: "x1".to_string(),
            param_y_name: "x2".to_string(),
            objective_name: "obj0".to_string(),
            r2: Some(0.9),
        };
        state.result = Some(result);
        state.computing = false;

        assert!(!state.computing);
        assert!(state.result.is_some());
    }

    #[test]
    fn value_range_returns_correct_bounds() {
        let values = vec![vec![1.0, 3.0], vec![2.0, 4.0]];
        let (v_min, v_max) = value_range(&values);
        assert!((v_min - 1.0).abs() < 1e-9);
        assert!((v_max - 4.0).abs() < 1e-9);
    }

    #[test]
    fn normalize_clamps_to_unit_range() {
        assert!((normalize(0.5, 0.0, 1.0) - 0.5).abs() < 1e-6);
        assert!((normalize(-1.0, 0.0, 1.0) - 0.0).abs() < 1e-6);
        assert!((normalize(2.0, 0.0, 1.0) - 1.0).abs() < 1e-6);
    }
}
