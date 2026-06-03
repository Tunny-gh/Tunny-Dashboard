use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::{COLOR_NON_PARETO, COLOR_PARETO};

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
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
    ) {
        if view.row_count() == 0 {
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
            compute_plot_points(view, param_name, obj_names, obj_idx, is_single, minimize);

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

/// view から Slice チャートの描画点を計算する（テスト可能な純粋関数）
///
/// - `param_name` に一致するパラメータを X 軸、`obj_names[obj_idx]` の目的関数を Y 軸とした点列を返す
/// - `single_objective=true` のとき: ベスト値（minimize なら最小、maximize なら最大）のトライアルを
///   1番目のタプルに、それ以外を2番目に分類する
/// - `single_objective=false` のとき: pareto_rank == 0 を1番目、それ以外を2番目に分類する
/// - `param_name` が view に存在しない、または `obj_idx` が範囲外のとき空を返す
/// - NaN/Inf の値はスキップする
pub fn compute_plot_points(
    view: &StudyView,
    param_name: &str,
    obj_names: &[String],
    obj_idx: usize,
    single_objective: bool,
    minimize: bool,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let param_col = view.numeric_column(param_name);
    let obj_col = obj_names
        .get(obj_idx)
        .and_then(|name| view.numeric_column(name));

    let (Some(params), Some(objs)) = (param_col, obj_col) else {
        return (vec![], vec![]);
    };

    // 有効な点を収集する（NaN/Inf はスキップ）
    let valid: Vec<(f64, f64, u32)> = (0..view.row_count())
        .filter_map(|i| {
            let x = params.get(i).copied()?;
            let y = objs.get(i).copied()?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            Some((x, y, rank))
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
    use std::collections::HashMap;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

    fn make_view(param_vals: &[f64], obj_vals: &[f64], pareto_ranks: Vec<u32>) -> StudyView {
        let n = param_vals.len();
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                param_display: HashMap::from([("x".to_string(), param_vals[i])]),
                param_category_label: HashMap::new(),
                objective_values: vec![obj_vals[i]],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );
        StudyView::new(Arc::new(df), pareto_ranks)
    }

    fn make_empty_view() -> StudyView {
        let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        StudyView::new(Arc::new(df), vec![])
    }

    #[test]
    fn test_compute_plot_points_empty() {
        let view = make_empty_view();
        let obj_names = vec!["obj0".to_string()];
        let (highlighted, normal) = compute_plot_points(&view, "x", &obj_names, 0, false, true);
        assert!(highlighted.is_empty());
        assert!(normal.is_empty());
    }

    #[test]
    fn test_compute_plot_points_pareto_classification() {
        let view = make_view(&[1.0, 3.0, 5.0], &[2.0, 4.0, 6.0], vec![0, 1, 0]);
        let obj_names = vec!["obj0".to_string()];
        let (pareto, non_pareto) = compute_plot_points(&view, "x", &obj_names, 0, false, true);
        assert_eq!(pareto.len(), 2);
        assert_eq!(non_pareto.len(), 1);
        assert_eq!(pareto[0], [1.0, 2.0]);
        assert_eq!(non_pareto[0], [3.0, 4.0]);
    }

    #[test]
    fn test_compute_plot_points_single_obj_minimize() {
        let view = make_view(&[1.0, 2.0, 3.0], &[5.0, 3.0, 7.0], vec![0; 3]);
        let obj_names = vec!["obj0".to_string()];
        let (best, others) = compute_plot_points(&view, "x", &obj_names, 0, true, true);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0], [2.0, 3.0]);
        assert_eq!(others.len(), 2);
    }

    #[test]
    fn test_compute_plot_points_single_obj_maximize() {
        let view = make_view(&[1.0, 2.0, 3.0], &[5.0, 9.0, 7.0], vec![0; 3]);
        let obj_names = vec!["obj0".to_string()];
        let (best, others) = compute_plot_points(&view, "x", &obj_names, 0, true, false);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0], [2.0, 9.0]);
        assert_eq!(others.len(), 2);
    }

    #[test]
    fn test_compute_plot_points_unknown_param_returns_empty() {
        let view = make_view(&[1.0, 3.0], &[2.0, 4.0], vec![0, 1]);
        let obj_names = vec!["obj0".to_string()];
        // "y" は view に存在しない → 空
        let (highlighted, normal) = compute_plot_points(&view, "y", &obj_names, 0, false, true);
        assert!(highlighted.is_empty());
        assert!(normal.is_empty());
    }

    #[test]
    fn test_compute_plot_points_obj_idx_out_of_bounds() {
        let view = make_view(&[1.0], &[2.0], vec![0]);
        let obj_names = vec!["obj0".to_string()];
        // obj_idx=1 だが obj_names は長さ 1 → 空
        let (highlighted, normal) = compute_plot_points(&view, "x", &obj_names, 1, false, true);
        assert!(highlighted.is_empty());
        assert!(normal.is_empty());
    }

    #[test]
    fn test_compute_plot_points_skips_nan_obj() {
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
        // obj_vals に NaN を含む行はスキップされる
        let core_rows: Vec<CoreRow> = vec![
            CoreRow {
                trial_id: 0,
                param_display: HashMap::from([("x".to_string(), 1.0)]),
                param_category_label: HashMap::new(),
                objective_values: vec![], // 目的関数なし → NaN
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            CoreRow {
                trial_id: 1,
                param_display: HashMap::from([("x".to_string(), 3.0)]),
                param_category_label: HashMap::new(),
                objective_values: vec![4.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );
        let view = StudyView::new(Arc::new(df), vec![0, 1]);
        let obj_names = vec!["obj0".to_string()];
        let (highlighted, normal) = compute_plot_points(&view, "x", &obj_names, 0, false, true);
        assert!(highlighted.is_empty());
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0], [3.0, 4.0]);
    }
}
