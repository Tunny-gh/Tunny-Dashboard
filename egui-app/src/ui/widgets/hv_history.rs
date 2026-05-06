use crate::state::app_state::HvHistory;
use crate::theme::chart_colors::COLOR_HV_LINE;

/// Hypervolume 推移チャートウィジェット
#[derive(Default)]
pub struct HvHistoryChart {
    pub hv_history: Option<HvHistory>,
    pub computing: bool,
}

impl HvHistoryChart {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing hypervolume...");
            });
            return;
        }

        let Some(history) = &self.hv_history else {
            ui.label("No hypervolume data");
            return;
        };

        let step = history.sample_step;
        // X 軸はサンプリング順の連番 × ステップ (0, step, 2*step, …)。
        // trial_id は途中試行から始まる場合があり 0 スタートにならないため使わない。
        let points: Vec<[f64; 2]> = history
            .hv_values
            .iter()
            .enumerate()
            .map(|(i, &hv)| [(i * step) as f64, hv])
            .collect();

        let sampling_label = if step <= 1 {
            "Sampling: Every trial".to_string()
        } else {
            format!("Sampling: Every {} trials", step)
        };
        ui.label(
            egui::RichText::new(sampling_label)
                .small()
                .color(crate::theme::TEXT_SECONDARY),
        );

        egui_plot::Plot::new("hv_history_plot")
            .legend(egui_plot::Legend::default())
            .x_axis_label("Trial")
            .y_axis_label("Hypervolume")
            .include_x(0.0)
            .show(ui, |plot_ui| {
                if !points.is_empty() {
                    let color = COLOR_HV_LINE;
                    let plot_points: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.line(
                        egui_plot::Line::new(plot_points)
                            .name("Hypervolume")
                            .color(color),
                    );
                    let scatter: egui_plot::PlotPoints = points.into_iter().collect();
                    plot_ui.points(
                        egui_plot::Points::new(scatter)
                            .name("Sampled points")
                            .color(color)
                            .radius(4.0),
                    );
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::HvHistory;

    #[test]
    fn hv_history_chart_default() {
        let chart = HvHistoryChart::default();
        assert!(chart.hv_history.is_none());
        assert!(!chart.computing);
    }

    #[test]
    fn hv_history_show_uses_index_times_step() {
        let history = HvHistory {
            trial_ids: vec![10000, 10050, 10100],
            hv_values: vec![0.1, 0.5, 0.8],
            sample_step: 50,
        };
        // x values should be 0, 50, 100 — not 10000, 10050, 10100
        let step = history.sample_step;
        let points: Vec<[f64; 2]> = history
            .hv_values
            .iter()
            .enumerate()
            .map(|(i, &hv)| [(i * step) as f64, hv])
            .collect();
        assert_eq!(points[0][0], 0.0);
        assert_eq!(points[1][0], 50.0);
        assert_eq!(points[2][0], 100.0);
    }
}
