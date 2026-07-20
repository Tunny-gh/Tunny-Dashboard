use crate::theme::chart_colors::{COLOR_GRID_STROKE, COLOR_INFEASIBLE};

mod draw;
mod stats;
#[cfg(test)]
mod tests;

use draw::{
    compute_feasible_point_colors, draw_correlation_cell, draw_histogram_bars, draw_scatter_cell,
};
pub use stats::downsample_indices_to_cap;
use stats::{
    col_min_max, compute_correlation, compute_histogram, resolve_color_objective,
    split_feasibility_indices,
};

/// Maximum number of points drawn per scatter cell. Trials beyond this limit are
/// downsampled evenly. Since drawing cost scales with cell count (lower triangle)
/// times point count, capping the point count keeps the UI responsive.
pub const MAX_SCATTER_POINTS: usize = 1500;

/// Number of histogram bins for diagonal cells.
const HIST_BINS: usize = 10;

/// Display mode for the scatter matrix.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MatrixMode {
    ParamsVsParams,
    ParamsVsObjectives,
}

/// Axis sort order for the scatter matrix.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AxisSort {
    Alphabetical,
    Correlation,
}

/// Overall state of the scatter matrix.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ScatterMatrix {
    pub mode: MatrixMode,
    pub sort: AxisSort,
    #[serde(skip)]
    pub selected_cell: Option<(usize, usize)>,
    /// Whether to show infeasible solutions (only meaningful for a Study with constraints).
    pub show_infeasible: bool,
    /// Objective name used to color points (`None` falls back to the first objective).
    pub color_objective: Option<String>,
    /// Cache of feasible/infeasible split + downsampled indices ((feasible, infeasible)).
    #[serde(skip)]
    downsample_cache: Option<(Vec<u32>, Vec<u32>)>,
    #[serde(skip)]
    downsample_cache_key: Option<(usize, usize, bool)>, // (df_ptr, trial_count, has_constraints)
    /// Cache of cell statistics (column range / histogram / correlation) and point colors (H-4).
    #[serde(skip)]
    stats_cache: Option<MatrixStatsCache>,
    /// Cache of pre-laid-out row/column label Galleys (recomputed only when the axis name list changes).
    #[serde(skip)]
    label_galleys_cache: Option<Vec<std::sync::Arc<egui::Galley>>>,
    #[serde(skip)]
    label_galleys_cache_key: Option<Vec<String>>,
}

/// Cache of cell statistics (column range / histogram / correlation) and point colors (H-4).
///
/// The cell drawing loop used to recompute histograms, correlation coefficients, and
/// min/max for every column and every trial on each frame (O(n_axes² × trial_count)).
/// With tens of thousands of trials and a dozen-plus parameters, this was the main
/// cause of dropped frames. As long as the DataFrame identity, display mode, coloring
/// objective, and colormap don't change, these are computed once and cached, so each
/// frame only needs to draw.
struct MatrixStatsCache {
    key: MatrixStatsKey,
    /// Min/max of each column (used for coordinate transforms in scatter cells; same
    /// reduction as `draw_scatter_cell`).
    col_ranges: Vec<(f64, f64)>,
    /// Histogram for diagonal cells (per column, `HIST_BINS` bins).
    histograms: Vec<Vec<usize>>,
    /// Correlation coefficients for upper-triangle cells. Accessed via `row * n + col`
    /// (only valid for row < col).
    correlations: Vec<f64>,
    /// Colors of feasible points to draw (same ordering as `feasible_draw`).
    point_colors: Vec<egui::Color32>,
}

/// Invalidation key for `MatrixStatsCache`. If any field changes, all cell statistics
/// are recomputed.
#[derive(PartialEq)]
struct MatrixStatsKey {
    /// Arc identity of the DataFrame (prevents mixing up a different Study or a post-update state).
    df_ptr: usize,
    mode: MatrixMode,
    color_objective: Option<String>,
    cmap_fp: u64,
    trial_count: usize,
    has_constraints: bool,
}

impl Default for ScatterMatrix {
    fn default() -> Self {
        Self {
            mode: MatrixMode::ParamsVsParams,
            sort: AxisSort::Alphabetical,
            selected_cell: None,
            show_infeasible: true,
            color_objective: None,
            downsample_cache: None,
            downsample_cache_key: None,
            stats_cache: None,
            label_galleys_cache: None,
            label_galleys_cache_key: None,
        }
    }
}

impl ScatterMatrix {
    /// Draws the scatter matrix.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &crate::state::app_state::StudyView,
        param_names: &[String],
        obj_names: &[String],
        cmap: &crate::theme::colormap::ColorMap,
    ) {
        let trial_count = view.row_count();
        if trial_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No trial data.").weak());
            });
            return;
        }

        let all_names: Vec<String> = param_names
            .iter()
            .chain(obj_names.iter())
            .cloned()
            .collect();
        let n = all_names.len();
        if n == 0 {
            return;
        }

        // Borrow each axis's column slice from view (no copy - MEM-003).
        let cols: Vec<&[f64]> = all_names
            .iter()
            .map(|name| view.numeric_column(name).unwrap_or(&[]))
            .collect();

        let feas = view.feasibility();
        let has_constraints = feas.has_constraints();
        // Arc identity of the DataFrame. Used as part of the cache key to prevent
        // mixing up a different Study or a post-update state.
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;

        // Control row: "Show Infeasible" toggle (only for a Study with constraints)
        // and the "Color by" dropdown.
        ui.horizontal(|ui| {
            if has_constraints {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
            if !obj_names.is_empty() {
                // Resolved objective name used for coloring.
                let current_obj =
                    resolve_color_objective(&self.color_objective, obj_names).unwrap_or("");
                ui.label("Color by:");
                egui::ComboBox::from_id_salt("scatter_matrix_color_obj")
                    .selected_text(current_obj)
                    .show_ui(ui, |ui| {
                        for name in obj_names {
                            if ui
                                .selectable_label(*name == current_obj, name.as_str())
                                .clicked()
                            {
                                self.color_objective = Some(name.clone());
                            }
                        }
                    });
            }
        });

        let show_infeasible = self.show_infeasible;

        // Drawing performance: cap the number of points shown per cell.
        // Reuse the same downsample indices across all scatter cells (no per-cell recompute).
        // The feasible/infeasible split + downsampling is recomputed only when trial_count
        // or the presence of constraints changes.
        let ds_key = (df_ptr, trial_count, has_constraints);
        if self.downsample_cache.is_none() || self.downsample_cache_key != Some(ds_key) {
            let (feasible_indices, infeasible_indices) =
                split_feasibility_indices(trial_count, feas);
            let feasible_draw = downsample_indices_to_cap(&feasible_indices, MAX_SCATTER_POINTS);
            let infeasible_draw =
                downsample_indices_to_cap(&infeasible_indices, MAX_SCATTER_POINTS);
            self.downsample_cache = Some((feasible_draw, infeasible_draw));
            self.downsample_cache_key = Some(ds_key);
        }

        // Compute cell statistics (column range / histogram / correlation) and point
        // colors once, keyed on DataFrame identity, display mode, coloring objective,
        // and colormap, and cache them (H-4). As long as these don't change, the
        // subsequent cell drawing loop only needs to draw.
        let stats_key = MatrixStatsKey {
            df_ptr,
            mode: self.mode.clone(),
            color_objective: self.color_objective.clone(),
            cmap_fp: super::rank_plot::cmap_fingerprint(cmap),
            trial_count,
            has_constraints,
        };
        if self.stats_cache.as_ref().map(|c| &c.key) != Some(&stats_key) {
            // Column ranges (min/max via the same reduction as `draw_scatter_cell`).
            let col_ranges: Vec<(f64, f64)> = cols.iter().map(|c| col_min_max(c)).collect();
            // Histograms for diagonal cells.
            let histograms: Vec<Vec<usize>> = cols
                .iter()
                .map(|c| compute_histogram(c, HIST_BINS))
                .collect();
            // Correlation coefficients for upper-triangle cells (only computed for row < col).
            let mut correlations = vec![0.0f64; n * n];
            for row in 0..n {
                for col in (row + 1)..n {
                    correlations[row * n + col] = compute_correlation(cols[row], cols[col]);
                }
            }
            // Colors of feasible points to draw (only for the downsampled point count).
            let point_colors = {
                let feasible_draw = &self.downsample_cache.as_ref().unwrap().0;
                compute_feasible_point_colors(
                    view,
                    &self.color_objective,
                    obj_names,
                    feas,
                    cmap,
                    feasible_draw,
                )
            };
            self.stats_cache = Some(MatrixStatsCache {
                key: stats_key,
                col_ranges,
                histograms,
                correlations,
                point_colors,
            });
        }
        let stats = self.stats_cache.as_ref().unwrap();
        let (feasible_draw, infeasible_draw) = self.downsample_cache.as_ref().unwrap();

        // Pre-layout row/column labels and measure their sizes.
        // The layout is not recomputed each frame unless the axis name list changes.
        let outer = ui.available_rect_before_wrap();
        let painter = ui.painter().clone();
        let label_color = ui.visuals().text_color();
        let label_font = egui::FontId::proportional(10.0);
        if self.label_galleys_cache.is_none()
            || self.label_galleys_cache_key.as_deref() != Some(&all_names[..])
        {
            let galleys: Vec<std::sync::Arc<egui::Galley>> = all_names
                .iter()
                .map(|name| painter.layout_no_wrap(name.clone(), label_font.clone(), label_color))
                .collect();
            self.label_galleys_cache = Some(galleys);
            self.label_galleys_cache_key = Some(all_names.clone());
        }
        let label_galleys = self.label_galleys_cache.as_ref().unwrap();
        let max_label_w = label_galleys
            .iter()
            .map(|g| g.size().x)
            .fold(0.0_f32, f32::max);
        let label_h = label_galleys.first().map(|g| g.size().y).unwrap_or(12.0);

        let label_angle = std::f32::consts::FRAC_PI_4; // 45°

        // Estimate the height of one cell; rotate row labels 45° if they don't fit the row.
        let cell_h_est = outer.height() / n as f32;
        let rotate_rows = label_h > cell_h_est - 2.0 || max_label_w > outer.width() * 0.25;
        // Reserved width for row labels (left edge). When rotated, use the diagonal
        // extent instead (capped at 110px).
        let row_label_w = if rotate_rows {
            (max_label_w * label_angle.cos() + label_h * label_angle.sin()).min(110.0) + 6.0
        } else {
            (max_label_w + 8.0).min(outer.width() * 0.25)
        };
        // Estimate one cell's width from the grid width; rotate column labels 45° if
        // they don't fit.
        let grid_w_est = outer.width() - row_label_w;
        let cell_w_est = grid_w_est / n as f32;
        let rotate_cols = max_label_w > cell_w_est - 4.0;
        let col_label_h = if rotate_cols {
            (max_label_w * label_angle.sin() + label_h * label_angle.cos()).min(110.0) + 6.0
        } else {
            label_h + 6.0
        };

        let available = egui::Rect::from_min_max(
            egui::pos2(outer.min.x + row_label_w, outer.min.y + col_label_h),
            outer.max,
        );
        let cell_w = available.width() / n as f32;
        let cell_h = available.height() / n as f32;

        // Draw axis names in the column header (top edge) and row header (left edge).
        for (idx, galley) in label_galleys.iter().enumerate() {
            let col_center_x = available.min.x + (idx as f32 + 0.5) * cell_w;
            let size = galley.size();
            if rotate_cols {
                // Align the lowest point of the "/"-shaped label rotated -45°
                // (counterclockwise) just above the column center / grid top edge
                // (same technique as PCP, D-12 shared helper).
                let applied = -label_angle;
                let lowest = super::rotated_label_corners(size, applied).lowest;
                let anchor = egui::pos2(col_center_x, available.min.y - 2.0);
                let pos = anchor - egui::vec2(lowest.0, lowest.1);
                painter.add(
                    egui::epaint::TextShape::new(pos, galley.clone(), label_color)
                        .with_angle(applied),
                );
            } else {
                painter.galley(
                    egui::pos2(col_center_x - size.x * 0.5, available.min.y - label_h - 2.0),
                    galley.clone(),
                    label_color,
                );
            }

            let row_center_y = available.min.y + (idx as f32 + 0.5) * cell_h;
            if rotate_rows {
                // Align the right edge (the corner with max rx) of the label rotated
                // -45° just to the left of the row center / grid left edge (D-12 shared helper).
                let applied = -label_angle;
                let corners = super::rotated_label_corners(size, applied);
                let right = corners.rightmost;
                let (min_ry, max_ry) = corners.ry_range;
                // Align the right edge to (available.min.x - gap) and the rotated
                // vertical center to row_center_y.
                let anchor = egui::pos2(available.min.x - 4.0, row_center_y);
                let center_ry = (min_ry + max_ry) * 0.5;
                let pos = anchor - egui::vec2(right.0, center_ry);
                painter.add(
                    egui::epaint::TextShape::new(pos, galley.clone(), label_color)
                        .with_angle(applied),
                );
            } else {
                painter.galley(
                    egui::pos2(available.min.x - size.x - 4.0, row_center_y - size.y * 0.5),
                    galley.clone(),
                    label_color,
                );
            }
        }
        // Point colors are already cached (stats.point_colors). Only the downsampled
        // feasible_draw/infeasible_draw are actually drawn, so the color array only
        // needs to hold that many entries (draw_scatter_cell indexes colors in the
        // same order as downsample_indices). Infeasible points are a single flat
        // color, so build them cheaply every frame to track theme color changes.
        let infeasible_colors: Vec<egui::Color32> = vec![COLOR_INFEASIBLE(); infeasible_draw.len()];

        for row in 0..n {
            for col in 0..n {
                let min = available.min + egui::vec2(col as f32 * cell_w, row as f32 * cell_h);
                let cell_rect = egui::Rect::from_min_size(min, egui::vec2(cell_w, cell_h));

                if row == col {
                    draw_histogram_bars(&painter, cell_rect, &stats.histograms[row]);
                } else if col > row {
                    // Upper triangle: correlation coefficient (drawn from the cached value).
                    draw_correlation_cell(&painter, cell_rect, stats.correlations[row * n + col]);
                } else {
                    // Lower triangle: scatter plot (drawn using downsampled indices +
                    // cached column ranges).
                    if has_constraints && show_infeasible && !infeasible_draw.is_empty() {
                        // Draw infeasible points behind the feasible ones.
                        draw_scatter_cell(
                            &painter,
                            cell_rect,
                            cols[col],
                            cols[row],
                            stats.col_ranges[col],
                            stats.col_ranges[row],
                            &infeasible_colors,
                            Some(infeasible_draw),
                        );
                    }
                    // Draw feasible points (all points when there are no constraints) in front.
                    draw_scatter_cell(
                        &painter,
                        cell_rect,
                        cols[col],
                        cols[row],
                        stats.col_ranges[col],
                        stats.col_ranges[row],
                        &stats.point_colors,
                        Some(feasible_draw),
                    );
                }

                // Draw a border on each cell to make the cell boundary explicit
                // (so the plot extent is clear even with dense scatter plots).
                painter.rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(1.0, COLOR_GRID_STROKE()),
                    egui::StrokeKind::Inside,
                );
            }
        }

        ui.allocate_rect(outer, egui::Sense::hover());
    }
}
