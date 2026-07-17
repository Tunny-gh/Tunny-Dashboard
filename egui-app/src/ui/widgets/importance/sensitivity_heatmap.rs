use crate::state::app_state::HeatmapMatrix;
use crate::theme::chart_colors::{COLOR_CHART_TEXT, COLOR_GRID_STROKE};
use crate::theme::color_compute::{diverging_colormap, sequential_colormap};
use crate::ui::widgets::importance_chart::ImportanceMetric;

/// Sensitivity heatmap widget. Shares the same `ImportanceMetric` methods with ImportanceChart.
/// Since the computation results are aggregated in `AppState::sensitivity_heatmap_cache`,
/// this only holds item-specific UI state (selected method, computing flag, compute request).
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SensitivityHeatmap {
    pub metric: ImportanceMetric,
    #[serde(skip)]
    pub computing: bool,
    /// Whether to fit the model using feasible solutions only (UI shown only for constrained studies)
    pub feasible_only: bool,
    /// The compute request consumed by poll_chart (target method, feasible_only).
    #[serde(skip)]
    pub pending_compute: Option<(ImportanceMetric, bool)>,
}

impl SensitivityHeatmap {
    /// Adopts the computing state from the global widget.
    /// Since results are aggregated in `AppState::sensitivity_heatmap_cache`, each item on
    /// the canvas (independent WidgetStates) only needs the computing flag reflected.
    /// The method selection is item-specific and is preserved.
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
    }

    /// Draws the sensitivity heatmap. `current` is the computed matrix for the currently
    /// selected method (if any).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        current: Option<&HeatmapMatrix>,
        has_constraints: bool,
    ) {
        // Control row: Run button + method selection + spinner
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                self.pending_compute = Some((self.metric, self.feasible_only));
                self.computing = true;
            }

            egui::ComboBox::from_id_salt("sensitivity_heatmap_metric")
                .selected_text(self.metric.label())
                .show_ui(ui, |ui| {
                    // Same family-based grouping as ImportanceChart, to make each method's character clear.
                    ui.label(group_header("── Correlation / Linear ──"));
                    for m in [ImportanceMetric::Spearman, ImportanceMetric::Ridge] {
                        ui.selectable_value(&mut self.metric, m, m.label());
                    }

                    ui.separator();
                    ui.label(group_header("── Tree-based ──"));
                    for m in [
                        ImportanceMetric::RfAnova,
                        ImportanceMetric::Mdi,
                        ImportanceMetric::Shap,
                        ImportanceMetric::Permutation,
                    ] {
                        ui.selectable_value(&mut self.metric, m, m.label());
                    }

                    ui.separator();
                    ui.label(group_header("── Global Sensitivity ──"));
                    for m in [ImportanceMetric::SobolFirst, ImportanceMetric::SobolTotal] {
                        ui.selectable_value(&mut self.metric, m, m.label());
                    }
                });

            // Feasible-solution filter (constrained studies only)
            if has_constraints {
                ui.toggle_value(&mut self.feasible_only, "Feasible only")
                    .on_hover_text("Fit the model using feasible trials only");
            }

            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }
        });

        // Low-cost methods (Spearman / Ridge) automatically issue a compute request if not
        // yet computed. High-cost methods require the Run button (to avoid unintended heavy
        // computation).
        if current.is_none()
            && !self.computing
            && self.pending_compute.is_none()
            && !self.metric.is_expensive()
        {
            self.pending_compute = Some((self.metric, self.feasible_only));
            self.computing = true;
        }

        let Some(matrix) = current else {
            ui.centered_and_justified(|ui| {
                if self.computing {
                    ui.add(egui::Spinner::new());
                } else if self.metric.is_expensive() {
                    ui.label(egui::RichText::new("Press Run to compute this metric.").weak());
                } else {
                    ui.label(egui::RichText::new("No sensitivity data.").weak());
                }
            });
            return;
        };

        if !matrix.is_well_formed() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        }

        draw_matrix(ui, matrix);
    }
}

/// Combo box family heading (same weak color / small size as ImportanceChart).
fn group_header(text: &str) -> egui::RichText {
    egui::RichText::new(text).weak().small()
}

fn draw_matrix(ui: &mut egui::Ui, matrix: &HeatmapMatrix) {
    let n_params = matrix.param_names.len();
    let n_objs = matrix.objective_names.len();

    // Non-negative metrics are normalized by the max value per objective (column), making
    // relative parameter comparisons within a column easier to read.
    let col_max: Vec<f64> = (0..n_objs)
        .map(|j| {
            matrix
                .values
                .iter()
                .map(|row| row[j].abs())
                .fold(0.0_f64, f64::max)
        })
        .collect();

    let header_w = 80.0_f32;
    let header_h = 20.0_f32;
    let available = ui.available_rect_before_wrap();
    let cell_w = (available.width() - header_w) / n_objs as f32;
    let cell_h = (available.height() - header_h) / n_params as f32;

    let painter = ui.painter();
    let text_color = ui.visuals().text_color();

    // Column headers (objective function names)
    for (j, obj_name) in matrix.objective_names.iter().enumerate() {
        let x = available.min.x + header_w + j as f32 * cell_w;
        let rect =
            egui::Rect::from_min_size(egui::pos2(x, available.min.y), egui::vec2(cell_w, header_h));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            obj_name,
            egui::FontId::proportional(10.0),
            text_color,
        );
    }

    // Row headers (parameter names) + cell grid
    for (i, param_name) in matrix.param_names.iter().enumerate() {
        let y = available.min.y + header_h + i as f32 * cell_h;

        let row_header_rect =
            egui::Rect::from_min_size(egui::pos2(available.min.x, y), egui::vec2(header_w, cell_h));
        painter.text(
            row_header_rect.center(),
            egui::Align2::CENTER_CENTER,
            param_name,
            egui::FontId::proportional(10.0),
            text_color,
        );

        for (j, &val) in matrix.values[i].iter().enumerate() {
            let x = available.min.x + header_w + j as f32 * cell_w;
            let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_w, cell_h));
            let color = if matrix.signed {
                // Signed: displayed diverging as-is, assuming [-1,1] (out-of-range values are clamped).
                diverging_colormap(val)
            } else {
                // Non-negative: normalized by the column max and displayed sequentially.
                let denom = col_max[j];
                let t = if denom > 0.0 { val.abs() / denom } else { 0.0 };
                sequential_colormap(t)
            };
            painter.rect_filled(cell_rect, 0.0, color);
            painter.rect_stroke(
                cell_rect,
                0.0,
                egui::Stroke::new(0.5, COLOR_GRID_STROKE()),
                egui::StrokeKind::Inside,
            );
            painter.text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{val:.2}"),
                egui::FontId::proportional(9.0),
                COLOR_CHART_TEXT(),
            );
        }
    }

    ui.allocate_rect(available, egui::Sense::hover());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitivity_heatmap_default() {
        let hm = SensitivityHeatmap::default();
        assert!(!hm.computing);
        assert_eq!(hm.metric, ImportanceMetric::Spearman);
        assert!(!hm.feasible_only);
        assert!(hm.pending_compute.is_none());
    }

    #[test]
    fn adopt_compute_state_copies_computing_flag() {
        let global = SensitivityHeatmap {
            computing: true,
            ..Default::default()
        };
        let mut item = SensitivityHeatmap {
            metric: ImportanceMetric::Ridge, // item-specific selection is preserved
            ..Default::default()
        };
        item.adopt_compute_state(&global);
        assert!(item.computing);
        assert_eq!(item.metric, ImportanceMetric::Ridge);
    }
}
