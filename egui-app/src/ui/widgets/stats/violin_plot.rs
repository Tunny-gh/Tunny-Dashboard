use std::ops::RangeInclusive;
use std::sync::Arc;

use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_BAR_ACCENT, COLOR_BAR_PRIMARY};
use crate::ui::widgets::common::axis_labels::{draw_plot_x_labels, plot_x_label_band};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::distribution::{DistributionCache, DistributionSelection, ViolinData};
use tunny_core::statistics::ViolinCurve;

/// Width assumed for the y axis on the very first frame, before the plot has reported
/// its actual frame. From the second frame on, the measured width is used instead.
const Y_AXIS_WIDTH_GUESS: f32 = 56.0;

/// Floor on the plot height, so a long rotated label band cannot collapse the violins.
const MIN_PLOT_HEIGHT: f32 = 80.0;

/// Half-width of a violin at its widest point (in x-axis units). Every violin is
/// scaled to this same maximum width.
const HALF_WIDTH: f64 = 0.4;

/// Half-width of the median marker segment (in x-axis units).
const MEDIAN_HALF_WIDTH: f64 = 0.2;

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ViolinPlotChart {
    pub selection: DistributionSelection,
    #[serde(skip)]
    cache: DistributionCache<ViolinData>,
}

impl ViolinPlotChart {
    pub fn show(&mut self, ui: &mut egui::Ui, view: &StudyView) {
        self.selection.controls(ui, view);
        let (curves, skipped) = self.cache.get(view, &self.selection, |p| p.violins());
        if !skipped.is_empty() {
            ui.label(
                egui::RichText::new(format!("{} group(s) skipped.", skipped.len()))
                    .small()
                    .weak(),
            );
            for (label, reason) in skipped {
                ui.label(
                    egui::RichText::new(format!("{label}: {reason}."))
                        .small()
                        .weak(),
                );
            }
        }
        if curves.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Need at least 2 finite, non-identical values to estimate a distribution.",
                    )
                    .weak(),
                );
            });
            return;
        }
        let labels: Vec<String> = curves.iter().map(|(name, _)| name.clone()).collect();
        // Each group is placed at its integer index; reserve a band below the plot
        // for rotated/slanted names, exactly like the box plot does.
        let avail = ui.available_size();
        let width_memo_id = ui.id().with("violin_plot_x_label_band_width");
        let plot_width = ui
            .data(|d| d.get_temp::<f32>(width_memo_id))
            .filter(|w| *w > 0.0)
            .unwrap_or_else(|| (avail.x - Y_AXIS_WIDTH_GUESS).max(1.0));
        let plan = plot_x_label_band(ui, &labels, plot_width);
        let plot_height = (avail.y - plan.height).max(MIN_PLOT_HEIGHT);
        let resp = egui_plot::Plot::new("violin_plot_plot")
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
                plot_ui.add(ViolinItem::new(
                    curves,
                    COLOR_BAR_PRIMARY(),
                    COLOR_BAR_ACCENT(),
                ));
            });
        let measured_width = resp.transform.frame().width();
        ui.data_mut(|d| d.insert_temp(width_memo_id, measured_width));
        let (band, _) =
            ui.allocate_exact_size(egui::vec2(avail.x, plan.height), egui::Sense::hover());
        draw_plot_x_labels(ui, band, &resp.transform, &labels, &plan);
    }
}

/// A custom [`egui_plot::PlotItem`] that draws the violin fills as triangle-strip
/// meshes. `egui_plot::Polygon`/`FilledArea` are not usable here: `Polygon` fills via
/// `Shape::convex_polygon` (which breaks on the non-convex, multi-modal outline) and
/// `FilledArea` only fills between two x-indexed lines, not between the left/right
/// edges of a violin.
struct ViolinItem<'a> {
    base: egui_plot::PlotItemBase,
    violins: &'a [(String, ViolinCurve)],
    fill: egui::Color32,
    median_color: egui::Color32,
    bounds: egui_plot::PlotBounds,
}

impl<'a> ViolinItem<'a> {
    fn new(
        violins: &'a [(String, ViolinCurve)],
        fill: egui::Color32,
        median_color: egui::Color32,
    ) -> Self {
        let mut bounds = egui_plot::PlotBounds::NOTHING;
        for (i, (_, curve)) in violins.iter().enumerate() {
            let center = i as f64;
            let scale = density_scale(curve);
            for (&g, &d) in curve.grid.iter().zip(curve.density.iter()) {
                let w = d * scale;
                bounds.extend_with(&egui_plot::PlotPoint::new(center + w, g));
                bounds.extend_with(&egui_plot::PlotPoint::new(center - w, g));
            }
            bounds.extend_with(&egui_plot::PlotPoint::new(
                center - MEDIAN_HALF_WIDTH,
                curve.median,
            ));
            bounds.extend_with(&egui_plot::PlotPoint::new(
                center + MEDIAN_HALF_WIDTH,
                curve.median,
            ));
        }
        Self {
            base: egui_plot::PlotItemBase::new("Violin".to_string()),
            violins,
            fill,
            median_color,
            bounds,
        }
    }
}

/// Scales a curve's density so its peak reaches [`HALF_WIDTH`]. Returns 0 for a
/// degenerate (all-zero / non-finite) density.
fn density_scale(curve: &ViolinCurve) -> f64 {
    let max_density = curve
        .density
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .fold(0.0_f64, f64::max);
    if max_density > 0.0 {
        HALF_WIDTH / max_density
    } else {
        0.0
    }
}

impl egui_plot::PlotItem for ViolinItem<'_> {
    fn shapes(
        &self,
        _ui: &egui::Ui,
        transform: &egui_plot::PlotTransform,
        shapes: &mut Vec<egui::Shape>,
    ) {
        for (i, (_, curve)) in self.violins.iter().enumerate() {
            let n = curve.grid.len().min(curve.density.len());
            if n < 2 {
                continue;
            }
            let center = i as f64;
            let scale = density_scale(curve);
            if scale <= 0.0 {
                continue;
            }
            let right: Vec<egui::Pos2> = (0..n)
                .map(|j| {
                    transform.position_from_point(&egui_plot::PlotPoint::new(
                        center + curve.density[j] * scale,
                        curve.grid[j],
                    ))
                })
                .collect();
            let left: Vec<egui::Pos2> = (0..n)
                .map(|j| {
                    transform.position_from_point(&egui_plot::PlotPoint::new(
                        center - curve.density[j] * scale,
                        curve.grid[j],
                    ))
                })
                .collect();
            // Fill the region between the right and left edges. Vertices are laid out as
            // [right..., left...] and each consecutive pair forms a quad; this is the same
            // triangle-strip pattern `egui_plot`'s built-in FilledArea uses, and it handles
            // the non-convex (multi-modal) outline that `Shape::convex_polygon` cannot.
            let mut mesh = egui::Mesh::default();
            mesh.reserve_vertices(n * 2);
            mesh.reserve_triangles((n - 1) * 2);
            for &p in &right {
                mesh.colored_vertex(p, self.fill);
            }
            for &p in &left {
                mesh.colored_vertex(p, self.fill);
            }
            for j in 0..n - 1 {
                mesh.add_triangle(j as u32, (n + j) as u32, (j + 1) as u32);
                mesh.add_triangle((n + j) as u32, (n + j + 1) as u32, (j + 1) as u32);
            }
            shapes.push(egui::Shape::Mesh(Arc::new(mesh)));
            // Outline the violin: right edge top-to-bottom, then left edge bottom-to-top.
            let mut outline = right;
            outline.extend(left.iter().rev().copied());
            shapes.push(egui::Shape::closed_line(
                outline,
                egui::Stroke::new(1.0, self.fill),
            ));
            // Median marker: a short horizontal segment centered on the group.
            let y = curve.median;
            let a = transform
                .position_from_point(&egui_plot::PlotPoint::new(center - MEDIAN_HALF_WIDTH, y));
            let b = transform
                .position_from_point(&egui_plot::PlotPoint::new(center + MEDIAN_HALF_WIDTH, y));
            shapes.push(egui::Shape::line_segment(
                [a, b],
                egui::Stroke::new(2.0, self.median_color),
            ));
        }
    }
    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}
    fn color(&self) -> egui::Color32 {
        self.fill
    }
    fn geometry(&self) -> egui_plot::PlotGeometry<'_> {
        egui_plot::PlotGeometry::None
    }
    fn bounds(&self) -> egui_plot::PlotBounds {
        self.bounds
    }
    fn base(&self) -> &egui_plot::PlotItemBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut egui_plot::PlotItemBase {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn density_scale_gives_every_curve_the_same_max_width() {
        for values in [
            &[0.0, 1.0, 2.0, 3.0, 4.0][..],
            &[0.0, 0.1, 0.2, 5.0, 10.0][..],
        ] {
            let curve = tunny_core::statistics::compute_violin(
                values,
                super::super::distribution::GRID_POINTS,
            )
            .unwrap();
            let max = curve.density.iter().copied().fold(0.0_f64, f64::max);
            assert!((max * density_scale(&curve) - HALF_WIDTH).abs() < 1e-12);
        }
    }
}
