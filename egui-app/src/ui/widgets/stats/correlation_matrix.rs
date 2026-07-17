use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_CHART_TEXT, COLOR_GRID_STROKE};
use crate::theme::color_compute::diverging_colormap;
use tunny_core::statistics::{compute_correlation_matrix, CorrelationMatrix, CorrelationMethod};

fn method_label(method: CorrelationMethod) -> &'static str {
    match method {
        CorrelationMethod::Pearson => "Pearson",
        CorrelationMethod::Spearman => "Spearman",
    }
}

/// Discriminant used for the cache key.
fn method_disc(method: CorrelationMethod) -> u8 {
    match method {
        CorrelationMethod::Pearson => 0,
        CorrelationMethod::Spearman => 1,
    }
}

/// (study_name, method_disc, include_params, include_objectives, row_count)
type CorrCacheKey = (String, u8, bool, bool, usize);

/// Correlation matrix heatmap widget for parameters and objective functions.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CorrelationMatrixChart {
    pub method: CorrelationMethod,
    pub include_params: bool,
    pub include_objectives: bool,
    #[serde(skip)]
    cache: Option<(CorrCacheKey, CorrelationMatrix)>,
}

impl Default for CorrelationMatrixChart {
    fn default() -> Self {
        Self {
            method: CorrelationMethod::Pearson,
            include_params: true,
            include_objectives: true,
            cache: None,
        }
    }
}

impl CorrelationMatrixChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        study_name: &str,
    ) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("correlation_matrix_method_combo")
                .selected_text(method_label(self.method))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.method,
                        CorrelationMethod::Pearson,
                        method_label(CorrelationMethod::Pearson),
                    );
                    ui.selectable_value(
                        &mut self.method,
                        CorrelationMethod::Spearman,
                        method_label(CorrelationMethod::Spearman),
                    );
                });
            ui.toggle_value(&mut self.include_params, "Parameters");
            ui.toggle_value(&mut self.include_objectives, "Objectives");
        });

        if !self.include_params && !self.include_objectives {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Select at least one column group.").weak());
            });
            return;
        }

        let mut names: Vec<&String> = Vec::new();
        if self.include_params {
            names.extend(param_names.iter());
        }
        if self.include_objectives {
            names.extend(obj_names.iter());
        }
        let columns: Vec<&String> = names
            .into_iter()
            .filter(|n| view.numeric_column(n).is_some())
            .collect();

        if columns.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        }

        let key: CorrCacheKey = (
            study_name.to_string(),
            method_disc(self.method),
            self.include_params,
            self.include_objectives,
            view.row_count(),
        );
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            let cols: Vec<(String, Vec<f64>)> = columns
                .iter()
                .map(|name| {
                    (
                        (*name).clone(),
                        view.numeric_column(name).unwrap_or(&[]).to_vec(),
                    )
                })
                .collect();
            self.cache = compute_correlation_matrix(&cols, self.method).map(|m| (key, m));
        }

        let Some((_, matrix)) = &self.cache else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        };

        draw_matrix(ui, matrix);
    }
}

/// Draws the k x k correlation matrix heatmap directly with the painter
/// (same style as sensitivity_heatmap). When a column header doesn't fit
/// within the cell width, it's rotated 45 degrees to avoid overlap, the
/// same as ScatterMatrix.
fn draw_matrix(ui: &mut egui::Ui, matrix: &CorrelationMatrix) {
    let n = matrix.labels.len();
    if n == 0 {
        return;
    }

    let text_color = ui.visuals().text_color();
    let font = egui::FontId::proportional(10.0);

    // Lay out labels first and measure the max width (used for the rotation decision and header dimensions).
    let label_galleys: Vec<std::sync::Arc<egui::Galley>> = matrix
        .labels
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap(l.clone(), font.clone(), text_color)
        })
        .collect();
    let max_label_w = label_galleys
        .iter()
        .map(|g| g.size().x)
        .fold(0.0_f32, f32::max);
    let label_h = label_galleys.first().map(|g| g.size().y).unwrap_or(12.0);

    let available = ui.available_rect_before_wrap();

    // Row header width matches the actual label width (the previous fixed
    // 80px caused long names to overlap).
    let header_w = (max_label_w + 8.0).clamp(60.0, available.width() * 0.3);

    // Rotate column labels 45 degrees if they don't fit the cell width (same test/dimensions as ScatterMatrix).
    let label_angle = std::f32::consts::FRAC_PI_4;
    let cell_w_est = (available.width() - header_w) / n as f32;
    let rotate_cols = max_label_w > cell_w_est - 4.0;
    let header_h = if rotate_cols {
        (max_label_w * label_angle.sin() + label_h * label_angle.cos()).min(110.0) + 6.0
    } else {
        label_h + 6.0
    };

    let cell_w = (available.width() - header_w) / n as f32;
    let cell_h = (available.height() - header_h) / n as f32;

    let painter = ui.painter();

    // Column headers
    for (j, galley) in label_galleys.iter().enumerate() {
        let col_center_x = available.min.x + header_w + (j as f32 + 0.5) * cell_w;
        let size = galley.size();
        if rotate_cols {
            // Align the bottom edge of the "/"-shaped label, rotated -45
            // degrees (counterclockwise), just above each column's center
            // at the grid's top edge (same technique as ScatterMatrix).
            let applied = -label_angle;
            let (sa, ca) = (applied.sin(), applied.cos());
            let corners = [(0.0, 0.0), (size.x, 0.0), (0.0, size.y), (size.x, size.y)];
            let mut lowest = (0.0_f32, f32::MIN);
            for (px, py) in corners {
                let rx = px * ca - py * sa;
                let ry = px * sa + py * ca;
                if ry > lowest.1 {
                    lowest = (rx, ry);
                }
            }
            let anchor = egui::pos2(col_center_x, available.min.y + header_h - 2.0);
            let pos = anchor - egui::vec2(lowest.0, lowest.1);
            painter.add(
                egui::epaint::TextShape::new(pos, galley.clone(), text_color).with_angle(applied),
            );
        } else {
            painter.galley(
                egui::pos2(
                    col_center_x - size.x * 0.5,
                    available.min.y + (header_h - label_h) * 0.5,
                ),
                galley.clone(),
                text_color,
            );
        }
    }

    // Row headers + cell grid
    for (i, label) in matrix.labels.iter().enumerate() {
        let y = available.min.y + header_h + i as f32 * cell_h;

        let row_header_rect =
            egui::Rect::from_min_size(egui::pos2(available.min.x, y), egui::vec2(header_w, cell_h));
        painter.text(
            row_header_rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.0),
            text_color,
        );

        for (j, &val) in matrix.values[i].iter().enumerate() {
            let x = available.min.x + header_w + j as f32 * cell_w;
            let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_w, cell_h));
            let (color, text) = if val.is_nan() {
                (COLOR_GRID_STROKE(), "\u{2013}".to_string())
            } else {
                (diverging_colormap(val), format!("{val:.2}"))
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
                text,
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
    fn correlation_matrix_chart_default_values() {
        let chart = CorrelationMatrixChart::default();
        assert_eq!(chart.method, CorrelationMethod::Pearson);
        assert!(chart.include_params);
        assert!(chart.include_objectives);
        assert!(chart.cache.is_none());
    }

    #[test]
    fn cache_key_changes_with_method() {
        let key_a: CorrCacheKey = (
            "s".into(),
            method_disc(CorrelationMethod::Pearson),
            true,
            true,
            5,
        );
        let key_b: CorrCacheKey = (
            "s".into(),
            method_disc(CorrelationMethod::Spearman),
            true,
            true,
            5,
        );
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_changes_with_column_group_toggles() {
        let key_a: CorrCacheKey = ("s".into(), 0, true, true, 5);
        let key_b: CorrCacheKey = ("s".into(), 0, false, true, 5);
        assert_ne!(key_a, key_b);
    }
}
