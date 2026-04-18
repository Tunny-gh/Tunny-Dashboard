use crate::state::app_state::TrialRow;
use crate::state::types::Direction;

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryMode {
    BestValue,
    AllTrials,
    MovingAverage,
}

/// 最適化履歴チャートウィジェット
pub struct OptimizationHistoryChart {
    pub show_all: bool,
    pub show_best: bool,
    pub show_moving_avg: bool,
    pub window_size: usize,
    pub obj_idx: usize,
}

impl Default for OptimizationHistoryChart {
    fn default() -> Self {
        Self {
            show_all: false,
            show_best: true,
            show_moving_avg: false,
            window_size: 10,
            obj_idx: 0,
        }
    }
}

impl OptimizationHistoryChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        trial_rows: &[TrialRow],
        obj_names: &[String],
        directions: &[Direction],
    ) {
        // 目的関数インデックスを有効範囲に収める
        if obj_names.is_empty() {
            self.obj_idx = 0;
        } else {
            self.obj_idx = self.obj_idx.min(obj_names.len() - 1);
        }

        let is_minimize = directions
            .get(self.obj_idx)
            .map(|d| matches!(d, Direction::Minimize))
            .unwrap_or(true);

        ui.horizontal(|ui| {
            if ui.selectable_label(self.show_all, "All Trials").clicked() {
                self.show_all = !self.show_all;
            }
            if ui.selectable_label(self.show_best, "Best Value").clicked() {
                self.show_best = !self.show_best;
            }
            if ui
                .selectable_label(self.show_moving_avg, "Moving Average")
                .clicked()
            {
                self.show_moving_avg = !self.show_moving_avg;
            }

            // 多目的の場合のみ目的関数選択コンボボックスを表示
            if obj_names.len() > 1 {
                ui.separator();
                let selected_label = obj_names
                    .get(self.obj_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                egui::ComboBox::from_id_salt("opt_history_obj_select")
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.obj_idx, i, name);
                        }
                    });
            }
        });

        let values: Vec<f64> = trial_rows
            .iter()
            .map(|r| r.objectives.get(self.obj_idx).copied().unwrap_or(0.0))
            .collect();

        egui_plot::Plot::new("optimization_history_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                if self.show_all && !values.is_empty() {
                    let pts: egui_plot::PlotPoints = values
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [i as f64, v])
                        .collect();
                    plot_ui.points(
                        egui_plot::Points::new(pts)
                            .name("All Trials")
                            .color(egui::Color32::from_rgb(50, 150, 250))
                            .radius(1.5),
                    );
                }

                if self.show_best && !values.is_empty() {
                    let pts: egui_plot::PlotPoints =
                        compute_best_values(&values, is_minimize).into_iter().collect();
                    plot_ui.line(
                        egui_plot::Line::new(pts)
                            .name("Best Value")
                            .color(egui::Color32::from_rgb(220, 50, 50))
                            .width(1.5),
                    );
                }

                if self.show_moving_avg && !values.is_empty() {
                    let pts: egui_plot::PlotPoints =
                        compute_moving_average(&values, self.window_size)
                            .into_iter()
                            .collect();
                    plot_ui.line(
                        egui_plot::Line::new(pts)
                            .name("Moving Average")
                            .color(egui::Color32::from_rgb(50, 200, 120))
                            .width(1.5),
                    );
                }
            });
    }
}

/// trial_rows から [trial_idx, value] の点列を計算する
pub fn compute_history_points(
    trial_rows: &[TrialRow],
    obj_idx: usize,
    mode: &HistoryMode,
    window_size: usize,
    is_minimize: bool,
) -> Vec<[f64; 2]> {
    let values: Vec<f64> = trial_rows
        .iter()
        .map(|r| r.objectives.get(obj_idx).copied().unwrap_or(0.0))
        .collect();

    match mode {
        HistoryMode::AllTrials => values
            .iter()
            .enumerate()
            .map(|(i, &v)| [i as f64, v])
            .collect(),
        HistoryMode::BestValue => compute_best_values(&values, is_minimize),
        HistoryMode::MovingAverage => compute_moving_average(&values, window_size),
    }
}

/// 累積ベスト値（最小化: 累積最小, 最大化: 累積最大）を計算する
pub fn compute_best_values(values: &[f64], is_minimize: bool) -> Vec<[f64; 2]> {
    let mut best = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            if is_minimize {
                best = best.min(v);
            } else {
                best = best.max(v);
            }
            [i as f64, best]
        })
        .collect()
}

/// 移動平均を計算する
pub fn compute_moving_average(values: &[f64], window: usize) -> Vec<[f64; 2]> {
    if values.is_empty() || window == 0 {
        return vec![];
    }
    values
        .windows(window.min(values.len()))
        .enumerate()
        .map(|(i, w)| {
            let avg = w.iter().sum::<f64>() / w.len() as f64;
            [i as f64, avg]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_best_values_minimize_decreasing() {
        let vals = vec![5.0, 3.0, 4.0, 1.0, 2.0];
        let result = compute_best_values(&vals, true);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0][1], 5.0);
        assert_eq!(result[1][1], 3.0);
        assert_eq!(result[2][1], 3.0);
        assert_eq!(result[3][1], 1.0);
        assert_eq!(result[4][1], 1.0);
    }

    #[test]
    fn compute_best_values_maximize_increasing() {
        let vals = vec![1.0, 3.0, 2.0, 5.0, 4.0];
        let result = compute_best_values(&vals, false);
        assert_eq!(result[0][1], 1.0);
        assert_eq!(result[1][1], 3.0);
        assert_eq!(result[2][1], 3.0);
        assert_eq!(result[3][1], 5.0);
        assert_eq!(result[4][1], 5.0);
    }

    #[test]
    fn compute_moving_average_window3() {
        let vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = compute_moving_average(&vals, 3);
        // windows(3): [1,2,3]=2.0, [2,3,4]=3.0, [3,4,5]=4.0
        assert_eq!(result.len(), 3);
        assert!((result[0][1] - 2.0).abs() < 1e-9);
        assert!((result[1][1] - 3.0).abs() < 1e-9);
        assert!((result[2][1] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn compute_moving_average_window_larger_than_data() {
        let vals = vec![1.0, 2.0];
        let result = compute_moving_average(&vals, 10);
        // windows(2) since min(10,2)=2: [1,2]=1.5
        assert_eq!(result.len(), 1);
        assert!((result[0][1] - 1.5).abs() < 1e-9);
    }

    #[test]
    fn history_mode_labels_not_empty() {
        assert!(!HistoryMode::BestValue.label().is_empty());
        assert!(!HistoryMode::AllTrials.label().is_empty());
        assert!(!HistoryMode::MovingAverage.label().is_empty());
    }
}
