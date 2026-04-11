use crate::render::colormap::compute_point_alpha;
use crate::state::app_state::AppState;

/// 2D Pareto 散布図ウィジェット（egui_plot ベース）
pub struct ParetoScatter2D {
    pub x_axis: String,
    pub y_axis: String,
}

impl Default for ParetoScatter2D {
    fn default() -> Self {
        Self {
            x_axis: "obj0".to_string(),
            y_axis: "obj1".to_string(),
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
        let trial_rows = ctx.trial_rows.clone();
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

        let x_idx = obj_names.iter().position(|n| n == &self.x_axis).unwrap_or(0);
        let y_idx = obj_names.iter().position(|n| n == &self.y_axis).unwrap_or(1);

        // 選択・非選択点の分離
        let (selected_pts, unselected_pts, highlight_pt) =
            partition_points(&trial_rows, &selected, highlighted, x_idx, y_idx);

        // Plot 描画
        egui_plot::Plot::new("pareto_2d_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                if !unselected_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(unselected_pts)
                            .name("Unselected")
                            .color(egui::Color32::from_rgba_unmultiplied(100, 100, 200, 50))
                            .radius(3.0),
                    );
                }
                if !selected_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(selected_pts)
                            .name("Selected")
                            .color(egui::Color32::from_rgb(50, 100, 255))
                            .radius(5.0),
                    );
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

/// TrialRow リストから選択・非選択・ハイライト点を分離する
pub fn partition_points(
    trial_rows: &[crate::state::app_state::TrialRow],
    selected_indices: &[u32],
    highlighted: Option<u32>,
    x_idx: usize,
    y_idx: usize,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>, Option<[f64; 2]>) {
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
        let rows = vec![
            make_trial(0, vec![1.0, 2.0]),
            make_trial(5, vec![9.0, 8.0]),
        ];
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
    }
}
