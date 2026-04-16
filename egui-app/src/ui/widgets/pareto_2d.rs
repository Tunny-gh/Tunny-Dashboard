use std::collections::HashMap;

use crate::render::colormap::compute_point_alpha;
use crate::state::app_state::{AppState, TrialRow};

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
}

impl Default for ParetoScatter2D {
    fn default() -> Self {
        Self {
            x_axis: "obj0".to_string(),
            y_axis: "obj1".to_string(),
            use_downsample: true,
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
        let display_rows: Vec<crate::state::app_state::TrialRow> =
            filter_by_downsample_indices(&ctx.trial_rows, downsample_indices.as_deref())
                .into_iter()
                .cloned()
                .collect();
        let trial_rows = display_rows;
        let selected = app_state.selected_indices.clone();
        let highlighted = app_state.highlighted_trial;
        let chart_colors = app_state.chart_colors.clone();
        let trial_id_to_color_idx: HashMap<u32, usize> = ctx
            .trial_rows
            .iter()
            .enumerate()
            .map(|(i, r)| (r.trial_id, i))
            .collect();

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

        let default_color = egui::Color32::from_rgb(100, 100, 200);

        // 色別グループ化（selected / unselected / highlighted）
        let mut selected_groups: HashMap<egui::Color32, Vec<[f64; 2]>> = HashMap::new();
        let mut unselected_groups: HashMap<egui::Color32, Vec<[f64; 2]>> = HashMap::new();
        let mut highlight_pt: Option<[f64; 2]> = None;

        for row in &trial_rows {
            let x = row.objectives.get(x_idx).copied().unwrap_or(0.0);
            let y = row.objectives.get(y_idx).copied().unwrap_or(0.0);
            let pt = [x, y];

            if let Some(h) = highlighted {
                if row.trial_id == h {
                    highlight_pt = Some(pt);
                    continue;
                }
            }

            let base_color = trial_id_to_color_idx
                .get(&row.trial_id)
                .and_then(|&idx| chart_colors.get(idx))
                .copied()
                .unwrap_or(default_color);

            let alpha = compute_point_alpha(row.trial_id, &selected);
            if alpha == 255 {
                selected_groups.entry(base_color).or_default().push(pt);
            } else {
                let dimmed = apply_alpha(base_color, alpha);
                unselected_groups.entry(dimmed).or_default().push(pt);
            }
        }

        // Plot 描画
        egui_plot::Plot::new("pareto_2d_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                for (color, points) in &unselected_groups {
                    if !points.is_empty() {
                        plot_ui.points(
                            egui_plot::Points::new(points.clone())
                                .name("Unselected")
                                .color(*color)
                                .radius(3.0),
                        );
                    }
                }
                for (color, points) in &selected_groups {
                    if !points.is_empty() {
                        plot_ui.points(
                            egui_plot::Points::new(points.clone())
                                .name("Selected")
                                .color(*color)
                                .radius(5.0),
                        );
                    }
                }
                if let Some(pt) = highlight_pt {
                    plot_ui.points(
                        egui_plot::Points::new(vec![pt])
                            .name("Highlighted")
                            .color(egui::Color32::RED)
                            .radius(8.0),
                    );
                }
            });
    }
}

/// Color32 の RGB を維持しつつ alpha 値を設定する
fn apply_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    let [r, g, b, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
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
    fn pareto_marker_radius_nonzero_rank_same() {
        assert_eq!(pareto_marker_radius(1), pareto_marker_radius(2));
    }
}
