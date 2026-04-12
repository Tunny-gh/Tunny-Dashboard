use crate::state::app_state::TrialRow;

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryMode {
    BestValue,
    AllTrials,
    MovingAverage,
}

impl HistoryMode {
    pub fn label(&self) -> &'static str {
        match self {
            HistoryMode::BestValue => "Best Value",
            HistoryMode::AllTrials => "All Trials",
            HistoryMode::MovingAverage => "Moving Average",
        }
    }
}

/// 最適化履歴チャートウィジェット
pub struct OptimizationHistoryChart {
    pub mode: HistoryMode,
    pub window_size: usize,
    pub obj_idx: usize,
}

impl Default for OptimizationHistoryChart {
    fn default() -> Self {
        Self {
            mode: HistoryMode::BestValue,
            window_size: 10,
            obj_idx: 0,
        }
    }
}

impl OptimizationHistoryChart {
    pub fn show(&mut self, ui: &mut egui::Ui, trial_rows: &[TrialRow], is_minimize: bool) {
        // モード選択
        ui.horizontal(|ui| {
            for mode in [
                HistoryMode::BestValue,
                HistoryMode::AllTrials,
                HistoryMode::MovingAverage,
            ] {
                let selected = self.mode == mode;
                if ui.selectable_label(selected, mode.label()).clicked() {
                    self.mode = mode;
                }
            }
        });

        let points = compute_history_points(
            trial_rows,
            self.obj_idx,
            &self.mode,
            self.window_size,
            is_minimize,
        );

        egui_plot::Plot::new("optimization_history_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                if !points.is_empty() {
                    let plot_points: egui_plot::PlotPoints = points.into_iter().collect();
                    plot_ui.line(
                        egui_plot::Line::new(plot_points)
                            .name(self.mode.label())
                            .color(egui::Color32::from_rgb(50, 150, 250)),
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
