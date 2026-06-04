use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::{
    COLOR_INFEASIBLE, COLOR_OPT_BEST, COLOR_OPT_PRUNED, COLOR_OPT_RUNNING, COLOR_OPT_TRIAL,
};

/// 比較 Study 1 件分の最適化履歴系列（選択中の目的に対する値列 + 色 + 凡例名）。
pub struct OptHistoryComparison {
    pub name: String,
    pub color: egui::Color32,
    /// 選択中の目的に対応する目的値列（行順）。
    pub values: Vec<f64>,
    pub is_minimize: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryMode {
    BestValue,
    AllTrials,
    MovingAverage,
}

impl HistoryMode {
    pub fn label(&self) -> &str {
        match self {
            HistoryMode::BestValue => "Best Value",
            HistoryMode::AllTrials => "All Trials",
            HistoryMode::MovingAverage => "Moving Average",
        }
    }
}

/// 最適化履歴チャートウィジェット
pub struct OptimizationHistoryChart {
    pub show_all: bool,
    pub show_best: bool,
    pub show_moving_avg: bool,
    pub window_size: usize,
    pub obj_idx: usize,
    /// REQ-008: 累積ベスト値ラインの表示切替
    pub show_best_line: bool,
    /// REQ-008: Y 軸対数スケール切替
    pub log_scale: bool,
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
}

impl Default for OptimizationHistoryChart {
    fn default() -> Self {
        Self {
            show_all: false,
            show_best: true,
            show_moving_avg: false,
            window_size: 10,
            obj_idx: 0,
            show_best_line: true,
            log_scale: false,
            show_infeasible: true,
        }
    }
}

impl OptimizationHistoryChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        directions: &[Direction],
    ) {
        self.show_with_history(ui, view, obj_names, directions, None);
    }

    pub fn show_with_history(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        directions: &[Direction],
        best_history: Option<&[(u32, f64)]>,
    ) {
        self.show_with_comparisons(ui, view, obj_names, directions, best_history, "", &[]);
    }

    /// 基準 Study に加えて、比較 Study の累積ベスト値ラインを同一グラフに重ねて描画する。
    /// 比較ラインは「Best Value」表示が有効なときに各 Study の色で描かれる。
    #[allow(clippy::too_many_arguments)]
    pub fn show_with_comparisons(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        directions: &[Direction],
        best_history: Option<&[(u32, f64)]>,
        base_name: &str,
        comparisons: &[OptHistoryComparison],
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

        let is_feasible_col = view.numeric_column("is_feasible");
        let has_constraints = is_feasible_col.is_some();

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

            // REQ-008-C: Best ライン表示トグル
            ui.separator();
            if ui
                .selectable_label(self.show_best_line, "[*] Best Line")
                .clicked()
            {
                self.show_best_line = !self.show_best_line;
            }

            // REQ-008-D: 対数スケールトグル
            if ui.selectable_label(self.log_scale, "Log Scale").clicked() {
                self.log_scale = !self.log_scale;
            }

            // 制約あり Study のみ "Show Infeasible" トグルを表示
            if has_constraints {
                ui.separator();
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
        });

        let show_infeasible = self.show_infeasible;

        let values: Vec<f64> = obj_names
            .get(self.obj_idx)
            .and_then(|name| view.numeric_column(name))
            .map(|col| col.to_vec())
            .unwrap_or_default();

        let show_best_line = self.show_best_line;
        let log_scale = self.log_scale;
        let show_all = self.show_all;
        let show_best = self.show_best;
        let show_moving_avg = self.show_moving_avg;
        let window_size = self.window_size;

        // All Trials の feasible / infeasible 分割（制約あり Study のみ分岐）
        let (feasible_vals, infeasible_vals) =
            partition_history_by_feasibility(&values, is_feasible_col);

        egui_plot::Plot::new("optimization_history_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                if show_all && !values.is_empty() {
                    let apply_log = |[x, v]: [f64; 2]| -> [f64; 2] {
                        [x, if log_scale && v > 0.0 { v.ln() } else { v }]
                    };
                    // infeasible を背面に描画
                    if show_infeasible && !infeasible_vals.is_empty() {
                        let pts: egui_plot::PlotPoints =
                            infeasible_vals.iter().copied().map(apply_log).collect();
                        plot_ui.points(
                            egui_plot::Points::new(pts)
                                .name("Infeasible")
                                .color(COLOR_INFEASIBLE)
                                .radius(1.5),
                        );
                    }
                    // feasible 点（制約なし Study は全点 feasible_vals に入る）
                    if !feasible_vals.is_empty() {
                        let pts: egui_plot::PlotPoints =
                            feasible_vals.iter().copied().map(apply_log).collect();
                        plot_ui.points(
                            egui_plot::Points::new(pts)
                                .name("All Trials")
                                .color(COLOR_OPT_TRIAL)
                                .radius(1.5),
                        );
                    }
                }

                if show_best {
                    let apply_log_y = |[x, y]: [f64; 2]| -> [f64; 2] {
                        [x, if log_scale && y > 0.0 { y.ln() } else { y }]
                    };
                    if !values.is_empty() {
                        // 比較時は基準 Study も名前で区別できるようラベルを切り替える。
                        let base_label = if comparisons.is_empty() || base_name.is_empty() {
                            "Best Value"
                        } else {
                            base_name
                        };
                        let pts: egui_plot::PlotPoints = compute_best_values(&values, is_minimize)
                            .into_iter()
                            .map(apply_log_y)
                            .collect();
                        plot_ui.line(
                            egui_plot::Line::new(pts)
                                .name(base_label)
                                .color(COLOR_OPT_PRUNED)
                                .width(1.5),
                        );
                    }
                    // 比較 Study の累積ベスト値ラインを各色で重ね描きする。
                    for comp in comparisons {
                        if comp.values.is_empty() {
                            continue;
                        }
                        let pts: egui_plot::PlotPoints =
                            compute_best_values(&comp.values, comp.is_minimize)
                                .into_iter()
                                .map(apply_log_y)
                                .collect();
                        plot_ui.line(
                            egui_plot::Line::new(pts)
                                .name(&comp.name)
                                .color(comp.color)
                                .width(1.5),
                        );
                    }
                }

                if show_moving_avg && !values.is_empty() {
                    let pts: egui_plot::PlotPoints = compute_moving_average(&values, window_size)
                        .into_iter()
                        .map(|[x, y]| {
                            let y2 = if log_scale && y > 0.0 { y.ln() } else { y };
                            [x, y2]
                        })
                        .collect();
                    plot_ui.line(
                        egui_plot::Line::new(pts)
                            .name("Moving Average")
                            .color(COLOR_OPT_RUNNING)
                            .width(1.5),
                    );
                }

                // REQ-008-C: Best ライン（best_history から）
                if show_best_line {
                    if let Some(history) = best_history {
                        let pts: egui_plot::PlotPoints = history
                            .iter()
                            .enumerate()
                            .map(|(i, &(_id, v))| {
                                let y = if log_scale && v > 0.0 { v.ln() } else { v };
                                [i as f64, y]
                            })
                            .collect();
                        plot_ui.line(
                            egui_plot::Line::new(pts)
                                .color(COLOR_OPT_BEST)
                                .width(2.0)
                                .name("[*] Best"),
                        );
                    }
                }
            });
    }
}

/// feasibility に基づいて目的値列を feasible / infeasible 点列に分割する。
/// `is_feasible_col` が None（制約なし Study）の場合は全点を feasible に分類する。
/// 戻り値: (feasible_pts, infeasible_pts) いずれも [trial_idx, value] 形式。
pub fn partition_history_by_feasibility(
    values: &[f64],
    is_feasible_col: Option<&[f64]>,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let mut feasible: Vec<[f64; 2]> = Vec::with_capacity(values.len());
    let mut infeasible: Vec<[f64; 2]> = Vec::with_capacity(values.len());
    for (i, &v) in values.iter().enumerate() {
        let feas = is_feasible_col
            .and_then(|c| c.get(i))
            .map(|&f| f > 0.5)
            .unwrap_or(true);
        if feas {
            feasible.push([i as f64, v]);
        } else {
            infeasible.push([i as f64, v]);
        }
    }
    (feasible, infeasible)
}

/// view から [trial_idx, value] の点列を計算する
pub fn compute_history_points(
    view: &StudyView,
    obj_names: &[String],
    obj_idx: usize,
    mode: &HistoryMode,
    window_size: usize,
    is_minimize: bool,
) -> Vec<[f64; 2]> {
    let values: Vec<f64> = obj_names
        .get(obj_idx)
        .and_then(|name| view.numeric_column(name))
        .map(|col| col.to_vec())
        .unwrap_or_default();

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

    // TASK-2126 tests
    #[test]
    fn best_line_toggle() {
        let mut show_best_line = false;
        show_best_line = !show_best_line;
        assert!(show_best_line);
        show_best_line = !show_best_line;
        assert!(!show_best_line);
    }

    #[test]
    fn best_line_plot_points() {
        let history = [(0u32, 1.0_f64), (3, 0.8), (7, 0.5)];
        let points: Vec<[f64; 2]> = history
            .iter()
            .enumerate()
            .map(|(i, &(_id, val))| [i as f64, val])
            .collect();
        assert_eq!(points[0], [0.0, 1.0]);
        assert_eq!(points[1], [1.0, 0.8]);
        assert_eq!(points[2], [2.0, 0.5]);
    }

    #[test]
    fn log_scale_toggle() {
        let mut log_scale = false;
        log_scale = !log_scale;
        assert!(log_scale);
    }

    // ── constraint-aware visualization (TASK-2349) ──────────────────

    #[test]
    fn tc_cav_opt_history_show_infeasible_default_true() {
        let chart = OptimizationHistoryChart::default();
        assert!(chart.show_infeasible);
    }

    #[test]
    fn tc_cav_partition_history_no_constraints_all_feasible() {
        let values = vec![1.0, 2.0, 3.0];
        let (f, inf) = partition_history_by_feasibility(&values, None);
        assert_eq!(f.len(), 3);
        assert!(inf.is_empty());
    }

    #[test]
    fn tc_cav_partition_history_mixed() {
        let values = vec![1.0, 2.0, 3.0];
        let is_feasible = vec![1.0_f64, 0.0, 1.0]; // idx 1 = infeasible
        let (f, inf) = partition_history_by_feasibility(&values, Some(&is_feasible));
        assert_eq!(f.len(), 2);
        assert_eq!(inf.len(), 1);
        assert_eq!(inf[0][0], 1.0); // trial_idx=1
        assert_eq!(inf[0][1], 2.0); // value=2.0
    }

    #[test]
    fn tc_cav_partition_history_all_infeasible() {
        let values = vec![1.0, 2.0];
        let is_feasible = vec![0.0_f64, 0.0];
        let (f, inf) = partition_history_by_feasibility(&values, Some(&is_feasible));
        assert!(f.is_empty());
        assert_eq!(inf.len(), 2);
    }

    #[test]
    fn best_line_none_history_no_panic() {
        // best_history = None の場合、Best ライン描画コードはスキップされる
        let history: Option<&[(u32, f64)]> = None;
        let rendered = history.map(|h| h.len());
        assert_eq!(rendered, None);
    }
}
