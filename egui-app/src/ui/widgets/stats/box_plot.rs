use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY};
use crate::ui::widgets::common::axis_labels::{draw_plot_x_labels, plot_x_label_band};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::distribution::{DistributionCache, DistributionSelection};
use tunny_core::statistics::BoxPlotStats;

/// Width assumed for the y axis on the very first frame, before the plot has reported
/// its actual frame. From the second frame on, the measured width is used instead.
const Y_AXIS_WIDTH_GUESS: f32 = 56.0;

/// Floor on the plot height, so a long rotated label band cannot collapse the boxes.
const MIN_PLOT_HEIGHT: f32 = 80.0;

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BoxPlotChart {
    pub selection: DistributionSelection,
    #[serde(skip)]
    cache: DistributionCache<Vec<(String, BoxPlotStats)>>,
}

impl BoxPlotChart {
    pub fn show(&mut self, ui: &mut egui::Ui, view: &StudyView) {
        self.selection.controls(ui, view);
        let stats = self.cache.get(view, &self.selection, |p| p.boxes());
        if stats.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No finite data.").weak());
            });
            return;
        }
        let labels: Vec<String> = stats.iter().map(|(name, _)| name.clone()).collect();
        let boxes: Vec<egui_plot::BoxElem> = stats
            .iter()
            .enumerate()
            .map(|(i, (name, s))| {
                let spread =
                    egui_plot::BoxSpread::new(s.whisker_low, s.q1, s.median, s.q3, s.whisker_high);
                egui_plot::BoxElem::new(i as f64, spread).name(name.clone())
            })
            .collect();
        let mut outlier_pts: Vec<[f64; 2]> = Vec::new();
        for (i, (_, s)) in stats.iter().enumerate() {
            for &v in &s.outliers {
                outlier_pts.push([i as f64, v]);
            }
        }
        let box_plot = egui_plot::BoxPlot::new("Box Plot", boxes).color(COLOR_BAR_PRIMARY());

        // egui_plot derives its own x tick spacing from the available width and offers
        // no way to rotate tick labels, so with more than a couple of groups it drops
        // most of the names. Hide its x axis and paint every label into a band reserved
        // below the plot instead, slanting them once they no longer fit side by side.
        // The band has to be sized before the plot is laid out, so the "do the names
        // still fit horizontally" test runs on the width measured last frame. It is
        // kept in egui's per-`Ui` memory rather than on `self`, because the maximize
        // modal draws the same widget state on top of the canvas cell within one frame
        // and the two have different widths.
        let avail = ui.available_size();
        let width_memo_id = ui.id().with("box_plot_x_label_band_width");
        let plot_width = ui
            .data(|d| d.get_temp::<f32>(width_memo_id))
            .filter(|w| *w > 0.0)
            .unwrap_or_else(|| (avail.x - Y_AXIS_WIDTH_GUESS).max(1.0));
        let plan = plot_x_label_band(ui, &labels, plot_width);
        let plot_height = (avail.y - plan.height).max(MIN_PLOT_HEIGHT);
        let resp = egui_plot::Plot::new("box_plot_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .show_axes([false, true])
            .height(plot_height)
            .y_axis_label(
                self.selection
                    .value
                    .as_ref()
                    .map(|v| v.label())
                    .unwrap_or_default(),
            )
            .x_axis_label(self.selection.group_by.as_deref().unwrap_or(""))
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                plot_ui.box_plot(box_plot);
                if !outlier_pts.is_empty() {
                    let pts: egui_plot::PlotPoints = outlier_pts.into();
                    plot_ui.points(
                        egui_plot::Points::new("Outliers", pts)
                            .shape(egui_plot::MarkerShape::Circle)
                            .radius(3.0)
                            .color(COLOR_BAR_NEGATIVE()),
                    );
                }
            });
        let measured_width = resp.transform.frame().width();
        ui.data_mut(|d| d.insert_temp(width_memo_id, measured_width));
        let (band, _) =
            ui.allocate_exact_size(egui::vec2(avail.x, plan.height), egui::Sense::hover());
        draw_plot_x_labels(ui, band, &resp.transform, &labels, &plan);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::distribution::DistributionValue;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow};

    #[test]
    fn value_combo_updates_statistics_in_same_frame_and_same_count_refreshes() {
        let view = |scale: f64| {
            let rows = [1.0, 2.0].map(|y| TrialRow {
                objective_values: vec![y * scale, y * scale * 10.0],
                ..Default::default()
            });
            StudyView::new(
                Arc::new(DataFrame::from_trials(
                    &rows,
                    &[],
                    &["obj0".into(), "obj1".into()],
                    &[],
                    &[],
                    0,
                )),
                vec![],
            )
        };
        let old = view(1.0);
        let new = view(100.0);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut chart = BoxPlotChart::default();
        let input = |node: Option<egui::accesskit::NodeId>| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 600.0),
            )),
            events: node
                .map(|target_node| {
                    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::Click,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        target_node,
                        data: None,
                    })
                })
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let first = ctx.run_ui(input(None), |ui| chart.show(ui, &old));
        let combo = first
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, n)| {
                n.role() == egui::accesskit::Role::ComboBox && n.value() == Some("Objective: obj0")
            })
            .unwrap()
            .0;
        let menu = ctx.run_ui(input(Some(combo)), |ui| chart.show(ui, &old));
        let choice = menu
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, n)| {
                n.role() == egui::accesskit::Role::Button && n.label() == Some("Objective: obj1")
            })
            .unwrap()
            .0;
        let _ = ctx.run_ui(input(Some(choice)), |ui| chart.show(ui, &old));
        assert_eq!(
            chart.selection.value,
            Some(DistributionValue::Objective("obj1".into()))
        );
        let stats = chart.cache.get(&old, &chart.selection, |_| {
            panic!("renderer must already have populated current selection")
        });
        assert_eq!(stats[0].1.median, 15.0);
        let _ = ctx.run_ui(input(None), |ui| chart.show(ui, &new));
        let stats = chart.cache.get(&new, &chart.selection, |_| {
            panic!("renderer must have refreshed the snapshot")
        });
        assert_eq!(stats[0].1.median, 1500.0);
    }
}
