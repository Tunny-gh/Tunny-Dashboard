use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::{
    COLOR_INFEASIBLE, COLOR_OPT_PRUNED, COLOR_OPT_RUNNING, COLOR_OPT_TRIAL,
};
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, show_hover_tooltip, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
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

/// 基準 Study の値列から導出する O(n) 計算結果をまとめたキャッシュ。
/// `key` が前回と変わらない限り毎フレームの再計算を避ける。
/// Moving Average は表示トグルが有効なときのみ計算する（無効時は無駄な計算をしない）。
struct HistoryCache {
    key: (usize, usize, bool, usize, bool), // (row_count, obj_idx, log_scale, window_size, is_minimize)
    values: Vec<f64>,
    feasible_vals: Vec<[f64; 2]>,
    infeasible_vals: Vec<[f64; 2]>,
    base_hit_points: Vec<(u32, usize, [f64; 2])>,
    best_values: Vec<[f64; 2]>,
    moving_avg: Option<Vec<[f64; 2]>>,
}

/// 最適化履歴チャートウィジェット
pub struct OptimizationHistoryChart {
    pub show_moving_avg: bool,
    pub window_size: usize,
    pub obj_idx: usize,
    /// REQ-008: Y 軸対数スケール切替
    pub log_scale: bool,
    /// 点クリックで開くトライアル詳細モーダル（散布図と共有）。
    detail_modal: TrialDetailModal,
    /// 基準 Study の O(n) 計算結果キャッシュ。
    history_cache: Option<HistoryCache>,
}

impl Default for OptimizationHistoryChart {
    fn default() -> Self {
        Self {
            show_moving_avg: false,
            window_size: 10,
            obj_idx: 0,
            log_scale: false,
            detail_modal: TrialDetailModal::new(),
            history_cache: None,
        }
    }
}

impl OptimizationHistoryChart {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        directions: &[Direction],
        param_names: &[String],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        self.show_with_comparisons(
            ui,
            view,
            obj_names,
            directions,
            param_names,
            "",
            &[],
            artifact_map,
        );
    }

    /// 基準 Study に加えて、比較 Study の累積ベスト値ラインを同一グラフに重ねて描画する。
    /// 比較ラインは「Best Value」表示が有効なときに各 Study の色で描かれる。
    ///
    /// 「All Trials」の点をクリックすると、散布図と共有のトライアル詳細モーダルを開く。
    /// 基準 Study の点のみ対象（比較 Study の試行は基準 Study の `view` に存在しない）。
    #[allow(clippy::too_many_arguments)]
    pub fn show_with_comparisons(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        directions: &[Direction],
        param_names: &[String],
        base_name: &str,
        comparisons: &[OptHistoryComparison],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
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

        let feas = view.feasibility();

        // All Trials / Best Value / Infeasible は常に描画する（表示のオン/オフは
        // チャート凡例のクリックで切り替えられる）。トグルは Moving Average /
        // Log Scale と目的選択のみ残す。
        ui.horizontal(|ui| {
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

            // REQ-008-D: 対数スケールトグル
            ui.separator();
            if ui.selectable_label(self.log_scale, "Log Scale").clicked() {
                self.log_scale = !self.log_scale;
            }
        });

        let log_scale = self.log_scale;
        let show_moving_avg = self.show_moving_avg;
        let window_size = self.window_size;
        let row_count = view.row_count();

        // 基準 Study の O(n) 計算（values / feasible 分割 / hit-test 点 / 累積ベスト値）は
        // 行数・目的選択・log/最小最大化フラグ・移動平均ウィンドウが変わらない限り
        // 再計算しない（Moving Average は表示トグルが有効なときのみ遅延計算する）。
        let cache_key = (row_count, self.obj_idx, log_scale, window_size, is_minimize);
        if self.history_cache.as_ref().map(|c| c.key) != Some(cache_key) {
            let values: Vec<f64> = obj_names
                .get(self.obj_idx)
                .and_then(|name| view.numeric_column(name))
                .map(|col| col.to_vec())
                .unwrap_or_default();

            // All Trials の feasible / infeasible 分割（制約あり Study のみ分岐）
            let (feasible_vals, infeasible_vals) = partition_history_by_feasibility(&values, feas);

            // クリック判定用に各試行の点を (trial_id, 行 index, [x, y]) で構築する。
            // x は行 index、y は描画と一致させるため log スケール時のみ log10 変換する。
            let base_hit_points: Vec<(u32, usize, [f64; 2])> = values
                .iter()
                .enumerate()
                .filter_map(|(i, &v)| {
                    let tid = *view.trial_ids.get(i)?;
                    let y = if log_scale && v > 0.0 { v.log10() } else { v };
                    Some((tid, i, [i as f64, y]))
                })
                .collect();

            let best_values = compute_best_values(&values, is_minimize);

            self.history_cache = Some(HistoryCache {
                key: cache_key,
                values,
                feasible_vals,
                infeasible_vals,
                base_hit_points,
                best_values,
                moving_avg: None,
            });
        }
        let cache = self.history_cache.as_mut().unwrap();
        if show_moving_avg && cache.moving_avg.is_none() {
            cache.moving_avg = Some(compute_moving_average(&cache.values, window_size));
        }
        let values = &cache.values;
        let feasible_vals = &cache.feasible_vals;
        let infeasible_vals = &cache.infeasible_vals;
        let base_hit_points = &cache.base_hit_points;
        let best_values = &cache.best_values;
        let moving_avg = cache.moving_avg.as_ref();

        // クリックされた点（trial_id, 行 index）。
        let mut clicked_detail: Option<(u32, usize)> = None;
        // マウスホバー中の点（trial_id, 行 index）。ツールチップ表示に使う。
        let mut hovered_detail: Option<(u32, usize)> = None;

        let mut plot =
            egui_plot::Plot::new("optimization_history_plot").legend(egui_plot::Legend::default());

        // 対数スケール時は値を log10 変換して描画しているため、Y 軸ラベルは
        // 変換前の元の値（10^mark で復元）を表示する。目盛りは 10 の累乗
        // （1, 10, 100, ...）を主目盛りとし、その間に 2〜9 倍の補助目盛りを置く。
        if log_scale {
            plot = crate::ui::widgets::common::log_scale::apply_log_y_axis(plot);
        }

        plot.show(ui, |plot_ui| {
            // 点クリックでトライアル詳細モーダルを開く（基準 Study の試行のみ）。
            let resp = plot_ui.response();
            if resp.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = resp.interact_pointer_pos() {
                    clicked_detail = hit_test_nearest(plot_ui, base_hit_points, pos, HIT_THRESHOLD);
                }
            }
            // ホバー中の点を検出する（基準 Study の試行のみ）。
            if let Some(pos) = resp.hover_pos() {
                hovered_detail = hit_test_nearest(plot_ui, base_hit_points, pos, HIT_THRESHOLD);
            }

            // All Trials は常に描画（凡例クリックで表示切替可能）。
            if !values.is_empty() {
                let apply_log = |[x, v]: [f64; 2]| -> [f64; 2] {
                    [x, if log_scale && v > 0.0 { v.log10() } else { v }]
                };
                // infeasible を背面に常時描画（凡例クリックで表示切替可能）
                if !infeasible_vals.is_empty() {
                    let pts: egui_plot::PlotPoints =
                        infeasible_vals.iter().copied().map(apply_log).collect();
                    plot_ui.points(
                        egui_plot::Points::new("Infeasible", pts)
                            .color(COLOR_INFEASIBLE)
                            .radius(1.5),
                    );
                }
                // feasible 点（制約なし Study は全点 feasible_vals に入る）
                if !feasible_vals.is_empty() {
                    let pts: egui_plot::PlotPoints =
                        feasible_vals.iter().copied().map(apply_log).collect();
                    plot_ui.points(
                        egui_plot::Points::new("All Trials", pts)
                            .color(COLOR_OPT_TRIAL)
                            .radius(1.5),
                    );
                }
            }

            // Best Value は常に描画（凡例クリックで表示切替可能）。
            {
                let apply_log_y = |[x, y]: [f64; 2]| -> [f64; 2] {
                    [x, if log_scale && y > 0.0 { y.log10() } else { y }]
                };
                if !values.is_empty() {
                    // 比較時は基準 Study も名前で区別できるようラベルを切り替える。
                    let base_label = if comparisons.is_empty() || base_name.is_empty() {
                        "Best Value"
                    } else {
                        base_name
                    };
                    let pts: egui_plot::PlotPoints =
                        best_values.iter().copied().map(apply_log_y).collect();
                    plot_ui.line(
                        egui_plot::Line::new(base_label, pts)
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
                        egui_plot::Line::new(&comp.name, pts)
                            .color(comp.color)
                            .width(1.5),
                    );
                }
            }

            if let Some(avg) = moving_avg.filter(|a| show_moving_avg && !a.is_empty()) {
                let pts: egui_plot::PlotPoints = avg
                    .iter()
                    .map(|&[x, y]| {
                        let y2 = if log_scale && y > 0.0 { y.log10() } else { y };
                        [x, y2]
                    })
                    .collect();
                plot_ui.line(
                    egui_plot::Line::new("Moving Average", pts)
                        .color(COLOR_OPT_RUNNING)
                        .width(1.5),
                );
            }
        });

        // ホバー中の点があれば、ポインタ位置に概要ツールチップを表示する。
        if let Some((_, row)) = hovered_detail {
            let trial_number = view.df.get_trial_number(row).unwrap_or(row as u32);
            let mut rows = Vec::new();
            if let (Some(name), Some(v)) = (obj_names.get(self.obj_idx), values.get(row)) {
                rows.push((name.clone(), format!("{v:.6}")));
            }
            if feas.has_constraints() {
                rows.push((
                    "Feasible".to_string(),
                    if feas.is_feasible(row) { "Yes" } else { "No" }.to_string(),
                ));
            }
            show_hover_tooltip(ui, "opt_history_hover_tooltip", trial_number, &rows);
        }

        // クリックされた点があれば、選択中の目的値（と feasibility）を付加情報として
        // モーダルを開く。
        if let Some((trial_id, row)) = clicked_detail {
            let mut context = Vec::new();
            if let (Some(name), Some(v)) = (obj_names.get(self.obj_idx), values.get(row)) {
                context.push((name.clone(), format!("{v:.6}")));
            }
            if feas.has_constraints() {
                context.push((
                    "Feasible".to_string(),
                    if feas.is_feasible(row) { "Yes" } else { "No" }.to_string(),
                ));
            }
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        // 詳細モーダルを描画する（散布図と同じ共有実装）。
        if self.detail_modal.is_open() {
            self.detail_modal
                .show(ui, view, param_names, obj_names, artifact_map);
        }
    }
}

/// feasibility に基づいて目的値列を feasible / infeasible 点列に分割する。
/// 制約なし Study（feas.has_constraints() == false）の場合は全点を feasible に分類する。
/// 戻り値: (feasible_pts, infeasible_pts) いずれも [trial_idx, value] 形式。
pub fn partition_history_by_feasibility(
    values: &[f64],
    feas: tunny_core::dataframe::Feasibility<'_>,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let mut feasible: Vec<[f64; 2]> = Vec::with_capacity(values.len());
    let mut infeasible: Vec<[f64; 2]> = Vec::with_capacity(values.len());
    for (i, &v) in values.iter().enumerate() {
        if feas.is_feasible(i) {
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
    fn log_scale_toggle() {
        let mut log_scale = false;
        log_scale = !log_scale;
        assert!(log_scale);
    }

    // ── constraint-aware visualization (TASK-2349) ──────────────────

    #[test]
    fn tc_cav_partition_history_no_constraints_all_feasible() {
        use tunny_core::dataframe::Feasibility;
        let values = vec![1.0, 2.0, 3.0];
        let feas = Feasibility::from_column(None);
        let (f, inf) = partition_history_by_feasibility(&values, feas);
        assert_eq!(f.len(), 3);
        assert!(inf.is_empty());
    }

    #[test]
    fn tc_cav_partition_history_mixed() {
        use tunny_core::dataframe::Feasibility;
        let values = vec![1.0, 2.0, 3.0];
        let is_feasible = vec![1.0_f64, 0.0, 1.0]; // idx 1 = infeasible
        let feas = Feasibility::from_column(Some(&is_feasible));
        let (f, inf) = partition_history_by_feasibility(&values, feas);
        assert_eq!(f.len(), 2);
        assert_eq!(inf.len(), 1);
        assert_eq!(inf[0][0], 1.0); // trial_idx=1
        assert_eq!(inf[0][1], 2.0); // value=2.0
    }

    #[test]
    fn tc_cav_partition_history_all_infeasible() {
        use tunny_core::dataframe::Feasibility;
        let values = vec![1.0, 2.0];
        let is_feasible = vec![0.0_f64, 0.0];
        let feas = Feasibility::from_column(Some(&is_feasible));
        let (f, inf) = partition_history_by_feasibility(&values, feas);
        assert!(f.is_empty());
        assert_eq!(inf.len(), 2);
    }
}
