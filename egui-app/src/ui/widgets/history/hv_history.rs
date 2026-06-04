use crate::state::app_state::HvHistory;
use crate::theme::chart_colors::COLOR_HV_LINE;

/// 1 本の HV 推移系列（凡例名 + 色 + データ）。
pub struct HvSeries {
    pub name: String,
    pub color: egui::Color32,
    pub history: HvHistory,
}

/// Hypervolume 推移チャートウィジェット
#[derive(Default)]
pub struct HvHistoryChart {
    pub hv_history: Option<HvHistory>,
    pub computing: bool,
    /// 基準 Study の凡例名（比較系列と区別するために表示する）。
    pub base_name: String,
    /// 同一グラフに重ね描きする比較 Study の系列。
    pub comparisons: Vec<HvSeries>,
}

impl HvHistoryChart {
    /// `history` のサンプリングステップを使って (x=連番×step, y=hv) の点列を作る。
    /// X 軸はサンプリング順の連番 × ステップ (0, step, 2*step, …)。
    /// trial_id は途中試行から始まる場合があり 0 スタートにならないため使わない。
    fn to_points(history: &HvHistory) -> Vec<[f64; 2]> {
        let step = history.sample_step.max(1);
        history
            .hv_values
            .iter()
            .enumerate()
            .map(|(i, &hv)| [(i * step) as f64, hv])
            .collect()
    }

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
        let base_points = Self::to_points(history);
        let base_label = if self.base_name.is_empty() {
            "Hypervolume".to_string()
        } else {
            self.base_name.clone()
        };

        // 比較系列の点列を事前計算（空履歴はスキップ）。
        let comparison_series: Vec<(&str, egui::Color32, Vec<[f64; 2]>)> = self
            .comparisons
            .iter()
            .filter(|s| !s.history.hv_values.is_empty())
            .map(|s| (s.name.as_str(), s.color, Self::to_points(&s.history)))
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
                // 基準 Study
                if !base_points.is_empty() {
                    let color = COLOR_HV_LINE;
                    let line_pts: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.line(
                        egui_plot::Line::new(line_pts)
                            .name(&base_label)
                            .color(color),
                    );
                    let scatter: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(scatter)
                            .name(&base_label)
                            .color(color)
                            .radius(3.0),
                    );
                }

                // 比較 Study を色分けして重ね描きする
                for (name, color, points) in &comparison_series {
                    let line_pts: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.line(egui_plot::Line::new(line_pts).name(*name).color(*color));
                    let scatter: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(scatter)
                            .name(*name)
                            .color(*color)
                            .radius(3.0),
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
