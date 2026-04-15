use crate::render::colormap::ColorMap;
use crate::state::messages::PdpResult2d;
use crate::ui::widgets::pdp_chart::ModelType;

/// Pending 2D PDP computation request, placed by show() and consumed by grid_canvas.
pub struct Pdp2dComputeRequest {
    pub param1: String,
    pub param2: String,
    pub objective: String,
    pub n_grid: usize,
    pub model_type: String,
}

/// PDP 2D ウィジェット状態
pub struct PdpChart2DState {
    pub selected_param1: String,
    pub selected_param2: String,
    pub selected_objective: usize,
    pub selected_model: ModelType,
    pub result: Option<PdpResult2d>,
    pub computing: bool,
    pub pending_compute: Option<Pdp2dComputeRequest>,
}

impl Default for PdpChart2DState {
    fn default() -> Self {
        Self {
            selected_param1: String::new(),
            selected_param2: String::new(),
            selected_objective: 0,
            selected_model: ModelType::Ridge,
            result: None,
            computing: false,
            pending_compute: None,
        }
    }
}

impl PdpChart2DState {
    pub fn show(&mut self, ui: &mut egui::Ui, param_names: &[String], obj_names: &[String]) {
        // Row 1: Parameter 1 + Parameter 2
        ui.horizontal(|ui| {
            ui.label("Parameter 1:");
            egui::ComboBox::from_id_salt("pdp2d_p1")
                .selected_text(&self.selected_param1)
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.selected_param1, name.clone(), name);
                    }
                });
            ui.label("Parameter 2:");
            egui::ComboBox::from_id_salt("pdp2d_p2")
                .selected_text(&self.selected_param2)
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.selected_param2, name.clone(), name);
                    }
                });
        });

        // Row 2: Objective + Model selector
        ui.horizontal(|ui| {
            ui.label("Objective:");
            let obj_text = obj_names
                .get(self.selected_objective)
                .map(|s| s.as_str())
                .unwrap_or("—");
            egui::ComboBox::from_id_salt("pdp2d_obj")
                .selected_text(obj_text)
                .show_ui(ui, |ui| {
                    for (i, name) in obj_names.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_objective == i, name)
                            .clicked()
                        {
                            self.selected_objective = i;
                        }
                    }
                });

            ui.label("Model:");
            egui::ComboBox::from_id_salt("pdp2d_model")
                .selected_text(self.selected_model.label())
                .show_ui(ui, |ui| {
                    for model in [
                        ModelType::Ridge,
                        ModelType::Kriging,
                        ModelType::SparseKriging,
                    ] {
                        let selected = self.selected_model == model;
                        if ui.selectable_label(selected, model.label()).clicked() {
                            self.selected_model = model;
                        }
                    }
                });
        });

        // 同一パラメータ警告
        if !self.selected_param1.is_empty() && self.selected_param1 == self.selected_param2 {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Warning: the same parameter is selected",
            );
        }

        // Run button — only enabled when params are different and objectives exist
        let can_run = check_params_different(&self.selected_param1, &self.selected_param2)
            && !obj_names.is_empty()
            && !self.computing;
        if ui
            .add_enabled(can_run, egui::Button::new("Run 2D PDP"))
            .clicked()
        {
            if let Some(obj_name) = obj_names.get(self.selected_objective) {
                self.pending_compute = Some(Pdp2dComputeRequest {
                    param1: self.selected_param1.clone(),
                    param2: self.selected_param2.clone(),
                    objective: obj_name.clone(),
                    n_grid: 20,
                    model_type: self.selected_model.to_str().to_string(),
                });
            }
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing 2D PDP...");
            });
            return;
        }

        let Some(result) = &self.result else {
            ui.label("No 2D PDP data");
            return;
        };

        let available = ui.available_rect_before_wrap();
        let plot_h = available.height().min(300.0);

        if let Some(uncertainties) = &result.uncertainties {
            // Dual heatmap: left = mean (viridis), right = σ (plasma)
            let half_w = (available.width() - 48.0) / 2.0;

            // Left pane: mean
            let left_size = egui::vec2(half_w, plot_h);
            let (left_rect, _) = ui.allocate_exact_size(left_size, egui::Sense::hover());
            let painter = ui.painter_at(left_rect);
            draw_heatmap_values(&painter, left_rect, &result.z_values, ColorMap::viridis());
            let bar_left = egui::Rect::from_min_size(
                egui::pos2(left_rect.right() + 4.0, left_rect.top()),
                egui::vec2(16.0, left_rect.height()),
            );
            let (v_min, v_max) = value_range_of(&result.z_values);
            draw_colorbar(ui, bar_left, v_min, v_max, ColorMap::viridis());

            // Right pane: σ (sqrt of variance)
            let sigma: Vec<Vec<f64>> = uncertainties
                .iter()
                .map(|row| row.iter().map(|&v| v.sqrt()).collect())
                .collect();
            let right_size = egui::vec2(half_w, plot_h);
            let (right_rect, _) = ui.allocate_exact_size(right_size, egui::Sense::hover());
            let painter = ui.painter_at(right_rect);
            draw_heatmap_values(&painter, right_rect, &sigma, ColorMap::plasma());
            let bar_right = egui::Rect::from_min_size(
                egui::pos2(right_rect.right() + 4.0, right_rect.top()),
                egui::vec2(16.0, right_rect.height()),
            );
            let (s_min, s_max) = value_range_of(&sigma);
            draw_colorbar(ui, bar_right, s_min, s_max, ColorMap::plasma());

            ui.horizontal(|ui| {
                ui.label(format!(
                    "Mean  —  X: {} / Y: {} / Z: {}",
                    result.param1_name, result.param2_name, result.objective_name
                ));
                ui.separator();
                ui.label("σ (Std. Dev.)");
            });
        } else {
            // Single heatmap: mean only (Ridge)
            let plot_size = egui::vec2(available.width() - 32.0, plot_h);
            let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
            let painter = ui.painter_at(rect);
            draw_heatmap_values(&painter, rect, &result.z_values, ColorMap::viridis());

            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(rect.right() + 4.0, rect.top()),
                egui::vec2(16.0, rect.height()),
            );
            let (v_min, v_max) = value_range_of(&result.z_values);
            draw_colorbar(ui, bar_rect, v_min, v_max, ColorMap::viridis());

            ui.label(format!(
                "X: {} / Y: {} / Z: {}",
                result.param1_name, result.param2_name, result.objective_name
            ));
        }
    }
}

/// ヒートマップを描画する
fn draw_heatmap_values(
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

    let (v_min, v_max) = value_range_of(values);
    let cell_w = rect.width() / n_col as f32;
    let cell_h = rect.height() / n_row as f32;

    for (row, row_vals) in values.iter().enumerate() {
        for (col, &val) in row_vals.iter().enumerate() {
            let t = normalize_value(val, v_min, v_max);
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

/// カラーバーを描画する
fn draw_colorbar(ui: &mut egui::Ui, bar_rect: egui::Rect, v_min: f64, v_max: f64, cmap: ColorMap) {
    let painter = ui.painter_at(bar_rect);
    let n_steps = 64;
    let step_h = bar_rect.height() / n_steps as f32;

    for i in 0..n_steps {
        let t = 1.0 - (i as f32 / n_steps as f32); // top = max
        let color = cmap.interpolate(t);
        let step_rect = egui::Rect::from_min_size(
            egui::pos2(bar_rect.left(), bar_rect.top() + i as f32 * step_h),
            egui::vec2(bar_rect.width(), step_h + 1.0),
        );
        painter.rect_filled(step_rect, 0.0, color);
    }

    // 値テキスト
    ui.label(format!("{:.2}", v_max));
    ui.label(format!("{:.2}", (v_min + v_max) / 2.0));
    ui.label(format!("{:.2}", v_min));
}

/// 値を [0.0, 1.0] に正規化する
pub fn normalize_value(v: f64, v_min: f64, v_max: f64) -> f32 {
    if (v_max - v_min).abs() < f64::EPSILON {
        return 0.5;
    }
    ((v - v_min) / (v_max - v_min)).clamp(0.0, 1.0) as f32
}

/// 値グリッドの値域 [min, max] を返す
pub fn value_range_of(values: &[Vec<f64>]) -> (f64, f64) {
    let flat: Vec<f64> = values.iter().flatten().copied().collect();
    if flat.is_empty() {
        return (0.0, 1.0);
    }
    let v_min = flat.iter().cloned().fold(f64::INFINITY, f64::min);
    let v_max = flat.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (v_min, v_max)
}

/// param1 と param2 が異なることを確認する（同一の場合 false）
pub fn check_params_different(p1: &str, p2: &str) -> bool {
    !p1.is_empty() && !p2.is_empty() && p1 != p2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_value_midpoint() {
        let t = normalize_value(0.5, 0.0, 1.0);
        assert!((t - 0.5).abs() < 1e-5);
    }

    #[test]
    fn normalize_value_clamps_below_zero() {
        let t = normalize_value(-1.0, 0.0, 1.0);
        assert_eq!(t, 0.0);
    }

    #[test]
    fn normalize_value_clamps_above_one() {
        let t = normalize_value(2.0, 0.0, 1.0);
        assert_eq!(t, 1.0);
    }

    #[test]
    fn normalize_value_equal_range_returns_half() {
        let t = normalize_value(5.0, 5.0, 5.0);
        assert_eq!(t, 0.5);
    }

    #[test]
    fn value_range_of_correct() {
        let grid = vec![vec![1.0, 3.0], vec![2.0, 0.5]];
        let (v_min, v_max) = value_range_of(&grid);
        assert!((v_min - 0.5).abs() < 1e-9);
        assert!((v_max - 3.0).abs() < 1e-9);
    }

    #[test]
    fn value_range_of_empty_returns_default() {
        let grid: Vec<Vec<f64>> = vec![];
        let (v_min, v_max) = value_range_of(&grid);
        assert_eq!(v_min, 0.0);
        assert_eq!(v_max, 1.0);
    }

    #[test]
    fn check_params_different_true_for_different() {
        assert!(check_params_different("x", "y"));
    }

    #[test]
    fn check_params_different_false_for_same() {
        assert!(!check_params_different("x", "x"));
    }

    #[test]
    fn check_params_different_false_for_empty() {
        assert!(!check_params_different("", "y"));
        assert!(!check_params_different("x", ""));
    }
}
