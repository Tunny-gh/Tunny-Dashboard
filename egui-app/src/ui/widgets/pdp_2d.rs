use crate::render::colormap::ColorMap;
use crate::state::messages::PdpResult2d;

/// PDP 2D ウィジェット状態
#[derive(Default)]
pub struct PdpChart2DState {
    pub selected_param1: String,
    pub selected_param2: String,
    pub selected_objective: usize,
    pub result: Option<PdpResult2d>,
    pub computing: bool,
}

impl PdpChart2DState {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        param_names: &[String],
        obj_names: &[String],
    ) {
        // 2変数選択
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

        // 同一パラメータ警告
        if !self.selected_param1.is_empty()
            && self.selected_param1 == self.selected_param2
        {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Warning: the same parameter is selected",
            );
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

        // ヒートマップ描画
        let available = ui.available_rect_before_wrap();
        let plot_size = egui::vec2(available.width() - 32.0, available.height().min(300.0));
        let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        draw_heatmap(&painter, rect, result);

        // カラーバー
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() + 4.0, rect.top()),
            egui::vec2(16.0, rect.height()),
        );
        let (v_min, v_max) = compute_value_range(result);
        draw_colorbar(ui, bar_rect, v_min, v_max);

        // 軸ラベル
        ui.label(format!(
            "X: {} / Y: {}",
            result.param1_name, result.param2_name
        ));
        let _ = obj_names; // 将来用
    }
}

/// ヒートマップを描画する
fn draw_heatmap(painter: &egui::Painter, rect: egui::Rect, result: &PdpResult2d) {
    let n_row = result.z_values.len();
    if n_row == 0 {
        return;
    }
    let n_col = result.z_values[0].len();
    if n_col == 0 {
        return;
    }

    let (v_min, v_max) = compute_value_range(result);
    let cmap = ColorMap::viridis();
    let cell_w = rect.width() / n_col as f32;
    let cell_h = rect.height() / n_row as f32;

    for (row, row_vals) in result.z_values.iter().enumerate() {
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
fn draw_colorbar(
    ui: &mut egui::Ui,
    bar_rect: egui::Rect,
    v_min: f64,
    v_max: f64,
) {
    let painter = ui.painter_at(bar_rect);
    let cmap = ColorMap::viridis();
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

/// PdpResult2d の値域 [min, max] を返す
pub fn compute_value_range(result: &PdpResult2d) -> (f64, f64) {
    let flat: Vec<f64> = result.z_values.iter().flatten().copied().collect();
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
    use crate::state::messages::PdpResult2d;

    fn make_result_2d(grid: Vec<Vec<f64>>) -> PdpResult2d {
        let n = grid.len();
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        PdpResult2d {
            x_values: x.clone(),
            y_values: x,
            z_values: grid,
            param1_name: "x".to_string(),
            param2_name: "y".to_string(),
            objective_name: "obj0".to_string(),
        }
    }

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
    fn compute_value_range_correct() {
        let result = make_result_2d(vec![vec![1.0, 3.0], vec![2.0, 0.5]]);
        let (v_min, v_max) = compute_value_range(&result);
        assert!((v_min - 0.5).abs() < 1e-9);
        assert!((v_max - 3.0).abs() < 1e-9);
    }

    #[test]
    fn compute_value_range_empty_returns_default() {
        let result = make_result_2d(vec![]);
        let (v_min, v_max) = compute_value_range(&result);
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
