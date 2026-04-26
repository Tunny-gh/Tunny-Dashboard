use crate::state::app_state::SensitivityResult;
/// 発散型カラーマップ: -1.0 → 青, 0.0 → 白, +1.0 → 赤
pub fn diverging_colormap(score: f64) -> egui::Color32 {
    let t = ((score + 1.0) / 2.0).clamp(0.0, 1.0) as f32;
    if t < 0.5 {
        // -1 → blue, 0 → white
        let f = t * 2.0;
        egui::Color32::from_rgb((255.0 * f) as u8, (255.0 * f) as u8, 255)
    } else {
        // 0 → white, +1 → red
        let f = (t - 0.5) * 2.0;
        egui::Color32::from_rgb(255, (255.0 * (1.0 - f)) as u8, (255.0 * (1.0 - f)) as u8)
    }
}

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
                painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(0.5, egui::Color32::GRAY));
                painter.text(
                    cell_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.2}", val),
                    egui::FontId::proportional(9.0),
                    egui::Color32::BLACK,
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
    fn diverging_colormap_negative_one_is_blue() {
        let color = diverging_colormap(-1.0);
        assert!(color.b() > color.r(), "score=-1 should be blue-dominant");
    }

    #[test]
    fn diverging_colormap_zero_is_white() {
        let color = diverging_colormap(0.0);
        assert_eq!(color.r(), 255);
        assert_eq!(color.g(), 255);
        assert_eq!(color.b(), 255);
    }

    #[test]
    fn diverging_colormap_positive_one_is_red() {
        let color = diverging_colormap(1.0);
        assert!(color.r() > color.b(), "score=+1 should be red-dominant");
    }

    #[test]
    fn diverging_colormap_intermediate_values_bounded() {
        for i in -10..=10 {
            let score = i as f64 / 10.0;
            let _ = diverging_colormap(score); // must not panic
        }
    }

    #[test]
    fn sensitivity_heatmap_default() {
        let hm = SensitivityHeatmap::default();
        assert!(!hm.computing);
        assert!(hm.result.is_none());
    }
}
