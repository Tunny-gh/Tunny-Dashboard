use crate::state::app_state::{Direction, TrialRow};

const COLOR_PARETO: egui::Color32 = egui::Color32::from_rgb(220, 50, 50); // Red
const COLOR_NON_PARETO: egui::Color32 = egui::Color32::from_rgb(100, 149, 237);

/// パラメータ vs 目的関数の Slice 散布図ウィジェット
///
/// X 軸に選択したパラメータ値、Y 軸に選択した目的関数値をプロットし、
/// パレート最適（pareto_rank == 0）のトライアルをアクセントカラーで強調する。
#[derive(Default)]
pub struct SliceChart {
    pub selected_param_idx: usize,
    pub selected_obj_idx: usize,
}

impl SliceChart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        trial_rows: &[TrialRow],
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
    ) {
        if trial_rows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No trial data.").weak());
            });
            return;
        }

        if param_names.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No parameters.").weak());
            });
            return;
        }

        if obj_names.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No objectives.").weak());
            });
            return;
        }

        // インデックスのクランプ（データ更新後の範囲外アクセスを防ぐ）
        self.selected_param_idx = self.selected_param_idx.min(param_names.len() - 1);
        self.selected_obj_idx = self.selected_obj_idx.min(obj_names.len() - 1);

        // ツールバー: パラメータ・目的関数選択 ComboBox
        ui.horizontal(|ui| {
            ui.label("Parameter:");
            egui::ComboBox::from_id_salt("slice_param_combo")
                .selected_text(&param_names[self.selected_param_idx])
                .show_ui(ui, |ui| {
                    for (i, name) in param_names.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_param_idx, i, name);
                    }
                });

            ui.label("Objective:");
            egui::ComboBox::from_id_salt("slice_obj_combo")
                .selected_text(&obj_names[self.selected_obj_idx])
                .show_ui(ui, |ui| {
                    for (i, name) in obj_names.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_obj_idx, i, name);
                    }
                });
        });

        let param_name = &param_names[self.selected_param_idx];
        let obj_idx = self.selected_obj_idx;

        let is_single = directions.len() == 1;
        let minimize = is_single && matches!(directions.first(), Some(Direction::Minimize));
        let (highlighted_pts, normal_pts) =
            compute_plot_points(trial_rows, param_name, obj_idx, is_single, minimize);

        let (highlight_label, normal_label) = if is_single {
            ("Best", "Trials")
        } else {
            ("Pareto", "Non-Pareto")
        };

        egui_plot::Plot::new("slice_chart_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                if !normal_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(normal_pts)
                            .name(normal_label)
                            .color(COLOR_NON_PARETO)
                            .radius(1.5),
                    );
                }
                if !highlighted_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(highlighted_pts)
                            .name(highlight_label)
                            .color(COLOR_PARETO)
                            .radius(3.0),
                    );
                }
            });
    }
}

/// trial_rows から Slice チャートの描画点を計算する（テスト可能な純粋関数）
///
/// - `param_name` に一致するパラメータを X 軸、`obj_idx` 番目の目的関数を Y 軸とした点列を返す
/// - `single_objective=true` のとき: ベスト値（minimize なら最小、maximize なら最大）のトライアルを
///   1番目のタプルに、それ以外を2番目に分類する
/// - `single_objective=false` のとき: pareto_rank == 0 を1番目、それ以外を2番目に分類する
/// - パラメータが存在しない、または `objectives[obj_idx]` が存在しないトライアルはスキップする
pub fn compute_plot_points(
    trial_rows: &[TrialRow],
    param_name: &str,
    obj_idx: usize,
    single_objective: bool,
    minimize: bool,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    // まず有効な点を収集する
    let valid: Vec<(f64, f64, u32)> = trial_rows
        .iter()
        .filter_map(|row| {
            let x = row.params.get(param_name).copied()?;
            let y = row.objectives.get(obj_idx).copied()?;
            Some((x, y, row.pareto_rank))
        })
        .collect();

    if single_objective {
        let best_y = if minimize {
            valid
                .iter()
                .map(|(_, y, _)| *y)
                .fold(f64::INFINITY, f64::min)
        } else {
            valid
                .iter()
                .map(|(_, y, _)| *y)
                .fold(f64::NEG_INFINITY, f64::max)
        };

        let mut highlighted: Vec<[f64; 2]> = Vec::new();
        let mut normal: Vec<[f64; 2]> = Vec::new();
        for (x, y, _) in &valid {
            if *y == best_y {
                highlighted.push([*x, *y]);
            } else {
                normal.push([*x, *y]);
            }
        }
        (highlighted, normal)
    } else {
        let mut pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut non_pareto_pts: Vec<[f64; 2]> = Vec::new();
        for (x, y, rank) in &valid {
            if *rank == 0 {
                pareto_pts.push([*x, *y]);
            } else {
                non_pareto_pts.push([*x, *y]);
            }
        }
        (pareto_pts, non_pareto_pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{TrialRow, TrialState};
    use std::collections::HashMap;

    fn make_trial(
        trial_id: u32,
        param_val: Option<f64>,
        obj_val: Option<f64>,
        pareto_rank: u32,
    ) -> TrialRow {
        let mut params = HashMap::new();
        if let Some(v) = param_val {
            params.insert("x".to_string(), v);
        }
        let objectives = if let Some(v) = obj_val {
            vec![v]
        } else {
            vec![]
        };
        TrialRow {
            trial_id,
            trial_number: trial_id,
            params,
            objectives,
            pareto_rank,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }
    }

    #[test]
    fn test_compute_plot_points_empty() {
        let (highlighted, normal) = compute_plot_points(&[], "x", 0, false, true);
        assert!(highlighted.is_empty());
        assert!(normal.is_empty());
    }

    #[test]
    fn test_compute_plot_points_pareto_classification() {
        let rows = vec![
            make_trial(0, Some(1.0), Some(2.0), 0), // pareto
            make_trial(1, Some(3.0), Some(4.0), 1), // non-pareto
            make_trial(2, Some(5.0), Some(6.0), 0), // pareto
        ];
        let (pareto, non_pareto) = compute_plot_points(&rows, "x", 0, false, true);
        assert_eq!(pareto.len(), 2);
        assert_eq!(non_pareto.len(), 1);
        assert_eq!(pareto[0], [1.0, 2.0]);
        assert_eq!(non_pareto[0], [3.0, 4.0]);
    }

    #[test]
    fn test_compute_plot_points_single_obj_minimize() {
        let rows = vec![
            make_trial(0, Some(1.0), Some(5.0), 0),
            make_trial(1, Some(2.0), Some(3.0), 0), // best (minimize)
            make_trial(2, Some(3.0), Some(7.0), 0),
        ];
        let (best, others) = compute_plot_points(&rows, "x", 0, true, true);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0], [2.0, 3.0]);
        assert_eq!(others.len(), 2);
    }

    #[test]
    fn test_compute_plot_points_single_obj_maximize() {
        let rows = vec![
            make_trial(0, Some(1.0), Some(5.0), 0),
            make_trial(1, Some(2.0), Some(9.0), 0), // best (maximize)
            make_trial(2, Some(3.0), Some(7.0), 0),
        ];
        let (best, others) = compute_plot_points(&rows, "x", 0, true, false);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0], [2.0, 9.0]);
        assert_eq!(others.len(), 2);
    }

    #[test]
    fn test_compute_plot_points_skips_missing_param() {
        let rows = vec![
            make_trial(0, None, Some(2.0), 0),      // param なし → スキップ
            make_trial(1, Some(3.0), Some(4.0), 1), // 通常
        ];
        let (highlighted, normal) = compute_plot_points(&rows, "x", 0, false, true);
        assert!(highlighted.is_empty());
        assert_eq!(normal.len(), 1);
    }

    #[test]
    fn test_compute_plot_points_skips_missing_obj() {
        let rows = vec![
            make_trial(0, Some(1.0), None, 0),      // obj なし → スキップ
            make_trial(1, Some(3.0), Some(4.0), 1), // 通常
        ];
        let (highlighted, normal) = compute_plot_points(&rows, "x", 0, false, true);
        assert!(highlighted.is_empty());
        assert_eq!(normal.len(), 1);
    }

    #[test]
    fn test_compute_plot_points_obj_idx_out_of_bounds() {
        let rows = vec![make_trial(0, Some(1.0), Some(2.0), 0)];
        // obj_idx=1 だが objectives は長さ 1 → スキップ
        let (highlighted, normal) = compute_plot_points(&rows, "x", 1, false, true);
        assert!(highlighted.is_empty());
        assert!(normal.is_empty());
    }
}
