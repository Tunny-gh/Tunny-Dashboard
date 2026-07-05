use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::{COLOR_NON_PARETO, COLOR_PARETO};
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, show_hover_tooltip, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};

/// パラメータ vs 目的関数の Slice 散布図ウィジェット
///
/// X 軸に選択したパラメータ値、Y 軸に選択した目的関数値をプロットし、
/// パレート最適（pareto_rank == 0）のトライアルをアクセントカラーで強調する。
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SliceChart {
    pub selected_param_idx: usize,
    pub selected_obj_idx: usize,
    /// Y 軸（目的関数）対数スケール切替
    pub log_scale: bool,
    /// 点クリックで開くトライアル詳細モーダル。
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
}

impl SliceChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
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

            // Y 軸対数スケールトグル（最適化履歴チャートと同じ挙動）
            ui.separator();
            if ui.selectable_label(self.log_scale, "Log Scale").clicked() {
                self.log_scale = !self.log_scale;
            }
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

        // 対数スケール時は Y 値を log10 変換して描画する。正値のみ変換し、
        // 0 以下の値はそのまま渡す（log10 不能なため）。
        let log_scale = self.log_scale;
        let apply_log = |[x, y]: [f64; 2]| -> [f64; 2] {
            [x, if log_scale && y > 0.0 { y.log10() } else { y }]
        };

        // 点クリック判定用の候補（trial_id, 行 index, プロット座標）。
        // プロット座標は描画と一致させるため対数変換後の Y を使う。
        let hit_candidates =
            compute_hit_candidates(view, param_name, obj_names, obj_idx, log_scale);

        let mut plot =
            egui_plot::Plot::new("slice_chart_plot").legend(egui_plot::Legend::default());
        if log_scale {
            plot = crate::ui::widgets::common::log_scale::apply_log_y_axis(plot);
        }

        let mut clicked_detail: Option<(u32, usize)> = None;
        // マウスホバー中の点（trial_id, 行 index）。ツールチップ表示に使う。
        let mut hovered_detail: Option<(u32, usize)> = None;
        plot.show(ui, |plot_ui| {
            // 点クリックで詳細モーダルを開く対象を検出する。
            let resp = plot_ui.response();
            if resp.clicked_by(egui::PointerButton::Primary) {
                clicked_detail = resp
                    .interact_pointer_pos()
                    .and_then(|pos| hit_test_nearest(plot_ui, &hit_candidates, pos, HIT_THRESHOLD));
            }
            // ホバー中の点を検出する。
            if let Some(pos) = resp.hover_pos() {
                hovered_detail = hit_test_nearest(plot_ui, &hit_candidates, pos, HIT_THRESHOLD);
            }
            if !normal_pts.is_empty() {
                let pts: egui_plot::PlotPoints = normal_pts.into_iter().map(apply_log).collect();
                plot_ui.points(
                    egui_plot::Points::new(normal_label, pts)
                        .color(COLOR_NON_PARETO)
                        .radius(1.5),
                );
            }
            if !highlighted_pts.is_empty() {
                let pts: egui_plot::PlotPoints =
                    highlighted_pts.into_iter().map(apply_log).collect();
                plot_ui.points(
                    egui_plot::Points::new(highlight_label, pts)
                        .color(COLOR_PARETO)
                        .radius(3.0),
                );
            }
        });

        // ホバー中の点があれば、ポインタ位置に概要ツールチップを表示する。
        if let Some((_, row)) = hovered_detail {
            let trial_number = view.df.get_trial_number(row).unwrap_or(row as u32);
            let fmt = |v: Option<f64>| v.map(|x| format!("{x:.4}")).unwrap_or_else(|| "—".into());
            let param_val = view
                .numeric_column(param_name)
                .and_then(|c| c.get(row).copied());
            let obj_val = obj_names
                .get(obj_idx)
                .and_then(|name| view.numeric_column(name))
                .and_then(|c| c.get(row).copied());
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            let rows = vec![
                (param_name.clone(), fmt(param_val)),
                (
                    obj_names.get(obj_idx).cloned().unwrap_or_default(),
                    fmt(obj_val),
                ),
                ("Pareto Rank".to_string(), rank.to_string()),
            ];
            show_hover_tooltip(ui, "slice_hover_tooltip", trial_number, &rows);
        }

        // 点クリックでトライアル詳細モーダルを開く（散布図情報 = Pareto ランク）。
        if let Some((trial_id, row)) = clicked_detail {
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            let context = vec![("Pareto Rank".to_string(), rank.to_string())];
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        self.detail_modal
            .show(ui, view, param_names, obj_names, artifact_map);
    }
}

/// 点クリック判定用の候補（trial_id, 行 index, プロット座標 [x, y]）を返す。
/// `log_scale` が有効なときは Y を log10 変換し、描画位置と一致させる。
/// NaN/Inf の点はスキップする。
fn compute_hit_candidates(
    view: &StudyView,
    param_name: &str,
    obj_names: &[String],
    obj_idx: usize,
    log_scale: bool,
) -> Vec<(u32, usize, [f64; 2])> {
    let param_col = view.numeric_column(param_name);
    let obj_col = obj_names
        .get(obj_idx)
        .and_then(|name| view.numeric_column(name));
    let (Some(params), Some(objs)) = (param_col, obj_col) else {
        return Vec::new();
    };
    (0..view.row_count())
        .filter_map(|i| {
            let x = params.get(i).copied()?;
            let y = objs.get(i).copied()?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            let y_plot = if log_scale && y > 0.0 { y.log10() } else { y };
            Some((trial_id, i, [x, y_plot]))
        })
        .collect()
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
                trial_number: i as u32,
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
                trial_number: 0,
                param_display: HashMap::from([("x".to_string(), 1.0)]),
                param_category_label: HashMap::new(),
                objective_values: vec![], // 目的関数なし → NaN
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            CoreRow {
                trial_id: 1,
                trial_number: 1,
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
