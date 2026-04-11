use crate::state::app_state::HvHistory;

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

        let points: Vec<[f64; 2]> = history
            .trial_ids
            .iter()
            .zip(history.hv_values.iter())
            .map(|(&id, &hv)| [id as f64, hv])
            .collect();

        egui_plot::Plot::new("hv_history_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                if !points.is_empty() {
                    let plot_points: egui_plot::PlotPoints = points.into_iter().collect();
                    plot_ui.line(
                        egui_plot::Line::new(plot_points)
                            .name("Hypervolume")
                            .color(egui::Color32::from_rgb(50, 200, 100)),
                    );
                }
            });
    }
}

/// HvHistoryResult を HvHistory ポイント列に変換する
pub fn hv_history_to_plot_points(history: &HvHistory) -> Vec<[f64; 2]> {
    history
        .trial_ids
        .iter()
        .zip(history.hv_values.iter())
        .map(|(&id, &hv)| [id as f64, hv])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::HvHistory;

    #[test]
    fn hv_history_to_plot_points_correct_mapping() {
        let history = HvHistory {
            trial_ids: vec![0, 1, 2],
            hv_values: vec![0.1, 0.5, 0.8],
        };
        let pts = hv_history_to_plot_points(&history);
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], [0.0, 0.1]);
        assert_eq!(pts[1], [1.0, 0.5]);
        assert_eq!(pts[2], [2.0, 0.8]);
    }

    #[test]
    fn hv_history_to_plot_points_empty() {
        let history = HvHistory {
            trial_ids: vec![],
            hv_values: vec![],
        };
        let pts = hv_history_to_plot_points(&history);
        assert_eq!(pts.len(), 0);
    }

    #[test]
    fn hv_history_chart_default() {
        let chart = HvHistoryChart::default();
        assert!(chart.hv_history.is_none());
        assert!(!chart.computing);
    }
}
