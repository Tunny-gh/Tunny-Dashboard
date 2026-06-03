use crate::state::app_state::SensitivityResult;
use crate::theme::chart_colors::{COLOR_CHART_TEXT, COLOR_GRID_STROKE};
use crate::theme::color_compute::diverging_colormap;

/// 感度ヒートマップウィジェット
#[derive(Default)]
pub struct SensitivityHeatmap {
    pub computing: bool,
    pub result: Option<SensitivityResult>,
}

impl SensitivityHeatmap {
    pub fn new() -> Self {
        Self::default()
    }

    /// 感度ヒートマップを描画する
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let Some(sens) = self.result.as_ref() else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No sensitivity data.").weak());
            });
            return;
        };

        let n_params = sens.param_names.len();
        let n_objs = sens.objective_names.len();
        if n_params == 0 || n_objs == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        }

        let header_w = 80.0_f32;
        let header_h = 20.0_f32;
        let available = ui.available_rect_before_wrap();
        let cell_w = (available.width() - header_w) / n_objs as f32;
        let cell_h = (available.height() - header_h) / n_params as f32;

        let painter = ui.painter();
        let text_color = ui.visuals().text_color();

        // 列ヘッダ（目的関数名）
        for (j, obj_name) in sens.objective_names.iter().enumerate() {
            let x = available.min.x + header_w + j as f32 * cell_w;
            let rect = egui::Rect::from_min_size(
                egui::pos2(x, available.min.y),
                egui::vec2(cell_w, header_h),
            );
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                obj_name,
                egui::FontId::proportional(10.0),
                text_color,
            );
        }

        // 行ヘッダ（パラメータ名）+ セルグリッド
        for (i, param_name) in sens.param_names.iter().enumerate() {
            let y = available.min.y + header_h + i as f32 * cell_h;

            let row_header_rect = egui::Rect::from_min_size(
                egui::pos2(available.min.x, y),
                egui::vec2(header_w, cell_h),
            );
            painter.text(
                row_header_rect.center(),
                egui::Align2::CENTER_CENTER,
                param_name,
                egui::FontId::proportional(10.0),
                text_color,
            );

            for (j, &val) in sens.spearman[i].iter().enumerate() {
                let x = available.min.x + header_w + j as f32 * cell_w;
                let cell_rect =
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_w, cell_h));
                let color = diverging_colormap(val);
                painter.rect_filled(cell_rect, 0.0, color);
                painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(0.5, COLOR_GRID_STROKE));
                painter.text(
                    cell_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.2}", val),
                    egui::FontId::proportional(9.0),
                    COLOR_CHART_TEXT,
                );
            }
        }

        ui.allocate_rect(available, egui::Sense::hover());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitivity_heatmap_default() {
        let hm = SensitivityHeatmap::default();
        assert!(!hm.computing);
        assert!(hm.result.is_none());
    }
}
