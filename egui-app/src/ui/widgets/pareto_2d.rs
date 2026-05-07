use crate::state::app_state::{AppState, TrialRow};
use crate::theme::chart_colors::{
    COLOR_NON_PARETO, COLOR_NON_PARETO_DIM, COLOR_PARETO, COLOR_PARETO_DIM,
};
use crate::theme::color_compute::compute_point_alpha;

type PartitionedPoints = (Vec<[f64; 2]>, Vec<[f64; 2]>, Option<[f64; 2]>);

/// ダウンサンプリングインデックスでトライアルをフィルタリングする
/// indices が Some の場合はそのインデックスのトライアルのみ、None の場合は全件を返す
pub fn filter_by_downsample_indices<'a>(
    trial_rows: &'a [TrialRow],
    indices: Option<&[u32]>,
) -> Vec<&'a TrialRow> {
    match indices {
        Some(idx) => idx
            .iter()
            .filter_map(|&i| trial_rows.get(i as usize))
            .collect(),
        None => trial_rows.iter().collect(),
    }
}

/// Pareto ランクに応じたマーカー半径を返す（ランク0が最大）
pub fn pareto_marker_radius(pareto_rank: u32) -> f32 {
    if pareto_rank == 0 {
        5.0
    } else {
        2.5
    }
}

/// 2D Pareto 散布図ウィジェット（egui_plot ベース）
pub struct ParetoScatter2D {
    pub x_axis: String,
    pub y_axis: String,
    pub use_downsample: bool,
    display_rows_cache: Option<Vec<TrialRow>>,
    cache_key: (usize, usize), // (trial_count, downsample_count)
}

impl Default for ParetoScatter2D {
    fn default() -> Self {
        Self {
            x_axis: "obj0".to_string(),
            y_axis: "obj1".to_string(),
            use_downsample: true,
            display_rows_cache: None,
            cache_key: (0, 0),
        }
    }
}

impl ParetoScatter2D {
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        let Some(ctx) = &app_state.current_study else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a study");
            });
            return;
        };

        let obj_names = ctx.meta.objective_names.clone();
        let downsample_indices = if self.use_downsample {
            app_state.downsample_cache.scatter.clone()
        } else {
            None
        };
        let ds_len = downsample_indices.as_ref().map_or(0, |v| v.len());
        let cache_key = (ctx.trial_rows.len(), ds_len);
        if self.display_rows_cache.is_none() || self.cache_key != cache_key {
            let display_rows: Vec<TrialRow> =
                filter_by_downsample_indices(&ctx.trial_rows, downsample_indices.as_deref())
                    .into_iter()
                    .cloned()
                    .collect();
            self.display_rows_cache = Some(display_rows);
            self.cache_key = cache_key;
        }
        let trial_rows = self.display_rows_cache.as_ref().unwrap();
        let selected = app_state.selected_indices.clone();
        let highlighted = app_state.highlighted_trial;

        // 軸割り当て ComboBox
        ui.horizontal(|ui| {
            ui.label("X Axis:");
            egui::ComboBox::from_id_salt("x_axis_combo")
                .selected_text(&self.x_axis)
                .show_ui(ui, |ui| {
                    for name in &obj_names {
                        ui.selectable_value(&mut self.x_axis, name.clone(), name);
                    }
                });
            ui.label("Y Axis:");
            egui::ComboBox::from_id_salt("y_axis_combo")
                .selected_text(&self.y_axis)
                .show_ui(ui, |ui| {
                    for name in &obj_names {
                        ui.selectable_value(&mut self.y_axis, name.clone(), name);
                    }
                });
        });

        let x_idx = obj_names
            .iter()
            .position(|n| n == &self.x_axis)
            .unwrap_or(0);
        let y_idx = obj_names
            .iter()
            .position(|n| n == &self.y_axis)
            .unwrap_or(1);

        // パレートフロント(rank==0)と非パレートに分類
        let mut pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut pareto_pts_dim: Vec<[f64; 2]> = Vec::new();
        let mut non_pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut non_pareto_pts_dim: Vec<[f64; 2]> = Vec::new();
        let mut highlight_pt: Option<[f64; 2]> = None;

        for row in trial_rows {
            let x = row.objectives.get(x_idx).copied().unwrap_or(0.0);
            let y = row.objectives.get(y_idx).copied().unwrap_or(0.0);
            let pt = [x, y];

            if highlighted == Some(row.trial_id) {
                highlight_pt = Some(pt);
                continue;
            }

            let is_selected = compute_point_alpha(row.trial_id, &selected) == 255;
            if row.pareto_rank == 0 {
                if is_selected {
                    pareto_pts.push(pt);
                } else {
                    pareto_pts_dim.push(pt);
                }
            } else if is_selected {
                non_pareto_pts.push(pt);
            } else {
                non_pareto_pts_dim.push(pt);
            }
        }

        egui_plot::Plot::new("pareto_2d_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                // 非パレート（青点）
                if !non_pareto_pts_dim.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(non_pareto_pts_dim)
                            .name("Others")
                            .color(COLOR_NON_PARETO_DIM)
                            .radius(2.5),
                    );
                }
                if !non_pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(non_pareto_pts)
                            .name("Others")
                            .color(COLOR_NON_PARETO)
                            .radius(2.5),
                    );
                }
                // パレートフロント（赤丸 + 赤線）
                if !pareto_pts_dim.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(pareto_pts_dim)
                            .name("Pareto Front")
                            .color(COLOR_PARETO_DIM)
                            .radius(4.0),
                    );
                }
                if !pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(pareto_pts)
                            .name("Pareto Front")
                            .color(COLOR_PARETO)
                            .radius(4.0),
                    );
                }
                // ハイライト点
                if let Some(pt) = highlight_pt {
                    plot_ui.points(
                        egui_plot::Points::new(vec![pt])
                            .name("Highlighted")
                            .color(egui::Color32::YELLOW)
                            .radius(8.0),
                    );
                }
            });
    }
}

/// TrialRow リストから選択・非選択・ハイライト点を分離する
pub fn partition_points(
    trial_rows: &[crate::state::app_state::TrialRow],
    selected_indices: &[u32],
    highlighted: Option<u32>,
    x_idx: usize,
    y_idx: usize,
) -> PartitionedPoints {
    let mut selected_pts = vec![];
    let mut unselected_pts = vec![];
    let mut highlight_pt = None;

    for row in trial_rows {
        let x = row.objectives.get(x_idx).copied().unwrap_or(0.0);
        let y = row.objectives.get(y_idx).copied().unwrap_or(0.0);
        let pt = [x, y];

        if let Some(h) = highlighted {
            if row.trial_id == h {
                highlight_pt = Some(pt);
                continue;
            }
        }

        let alpha = compute_point_alpha(row.trial_id, selected_indices);
        if alpha == 255 {
            selected_pts.push(pt);
        } else {
            unselected_pts.push(pt);
        }
    }
    (selected_pts, unselected_pts, highlight_pt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{Direction, TrialRow, TrialState};
    use std::collections::HashMap;

    fn make_trial(id: u32, objs: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            params: HashMap::new(),
            objectives: objs,
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }
    }

    #[test]
    fn partition_empty_selected_all_go_to_selected() {
        let rows = vec![make_trial(0, vec![1.0, 2.0]), make_trial(1, vec![3.0, 4.0])];
        let (sel, unsel, hl) = partition_points(&rows, &[], None, 0, 1);
        assert_eq!(sel.len(), 2);
        assert_eq!(unsel.len(), 0);
        assert!(hl.is_none());
    }

    #[test]
    fn partition_with_selected_splits_correctly() {
        let rows = vec![
            make_trial(0, vec![1.0, 2.0]),
            make_trial(1, vec![3.0, 4.0]),
            make_trial(2, vec![5.0, 6.0]),
        ];
        let (sel, unsel, hl) = partition_points(&rows, &[0, 2], None, 0, 1);
        assert_eq!(sel.len(), 2);
        assert_eq!(unsel.len(), 1);
        assert!(hl.is_none());
        // 非選択は trial_id=1
        assert_eq!(unsel[0], [3.0, 4.0]);
    }

    #[test]
    fn partition_highlight_extracted_separately() {
        let rows = vec![make_trial(0, vec![1.0, 2.0]), make_trial(5, vec![9.0, 8.0])];
        let (sel, unsel, hl) = partition_points(&rows, &[], Some(5), 0, 1);
        assert_eq!(sel.len(), 1);
        assert_eq!(unsel.len(), 0);
        assert_eq!(hl, Some([9.0, 8.0]));
    }

    #[test]
    fn pareto_scatter_2d_default() {
        let widget = ParetoScatter2D::default();
        assert_eq!(widget.x_axis, "obj0");
        assert_eq!(widget.y_axis, "obj1");
        assert!(widget.use_downsample);
    }

    // TASK-2020 tests

    #[test]
    fn filter_by_downsample_none_returns_all() {
        let rows = vec![
            make_trial(0, vec![]),
            make_trial(1, vec![]),
            make_trial(2, vec![]),
        ];
        let result = filter_by_downsample_indices(&rows, None);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn filter_by_downsample_some_returns_subset() {
        let rows = vec![
            make_trial(0, vec![]),
            make_trial(1, vec![]),
            make_trial(2, vec![]),
        ];
        let indices = vec![0u32, 2u32];
        let result = filter_by_downsample_indices(&rows, Some(&indices));
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].trial_id, 0);
        assert_eq!(result[1].trial_id, 2);
    }

    #[test]
    fn pareto_marker_radius_rank0_is_larger() {
        let r0 = pareto_marker_radius(0);
        let r1 = pareto_marker_radius(1);
        assert!(r0 > r1);
    }

    #[test]
    fn pareto_marker_radius_non_front_rank_same() {
        assert_eq!(pareto_marker_radius(1), pareto_marker_radius(2));
    }
}
