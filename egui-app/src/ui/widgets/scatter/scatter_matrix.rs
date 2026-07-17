use crate::theme::chart_colors::{
    COLOR_CHART_TEXT, COLOR_GRID_STROKE, COLOR_INFEASIBLE, COLOR_SCATTER_DOT,
};
use crate::theme::color_compute::correlation_color;
use crate::ui::widgets::common::range_math::value_range;

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

/// Pure function that resolves the objective name used for coloring.
/// - If `selected` is present in `obj_names`, returns that name.
/// - If `None` or the name doesn't exist, returns the first element (`obj_names[0]`).
/// - If `obj_names` is empty, returns `None`.
pub fn resolve_color_objective<'a>(
    selected: &Option<String>,
    obj_names: &'a [String],
) -> Option<&'a str> {
    if obj_names.is_empty() {
        return None;
    }
    if let Some(name) = selected {
        if let Some(found) = obj_names.iter().find(|n| *n == name) {
            return Some(found.as_str());
        }
    }
    Some(obj_names[0].as_str())
}

/// Builds feasible / infeasible index lists from feasibility.
/// For a Study without constraints (feas.has_constraints() == false), all entries
/// are treated as feasible.
pub fn split_feasibility_indices(
    n: usize,
    feas: tunny_core::dataframe::Feasibility<'_>,
) -> (Vec<u32>, Vec<u32>) {
    let (f_idx, inf_idx) = feas.partition_indices(n);
    let feasible: Vec<u32> = f_idx.into_iter().map(|i| i as u32).collect();
    let infeasible: Vec<u32> = inf_idx.into_iter().map(|i| i as u32).collect();
    (feasible, infeasible)
}

/// Evenly downsamples an index list to at most `cap` entries.
/// If already at or below `cap`, returns a plain copy; otherwise downsamples with
/// an evenly spaced stride, reducing the point count while preserving the overall
/// distribution shape.
pub fn downsample_indices_to_cap(indices: &[u32], cap: usize) -> Vec<u32> {
    if cap == 0 {
        return Vec::new();
    }
    if indices.len() <= cap {
        return indices.to_vec();
    }
    // Round the stride up so the result never exceeds cap.
    let step = indices.len().div_ceil(cap);
    indices.iter().step_by(step).copied().collect()
}

/// Converts data coordinates to screen coordinates.
pub fn data_to_screen(
    x: f64,
    y: f64,
    x_range: (f64, f64),
    y_range: (f64, f64),
    cell_rect: egui::Rect,
) -> egui::Pos2 {
    let (x_min, x_max) = x_range;
    let (y_min, y_max) = y_range;
    let tx = if (x_max - x_min).abs() < f64::EPSILON {
        0.5
    } else {
        ((x - x_min) / (x_max - x_min)).clamp(0.0, 1.0)
    } as f32;
    let ty = if (y_max - y_min).abs() < f64::EPSILON {
        0.5
    } else {
        1.0 - ((y - y_min) / (y_max - y_min)).clamp(0.0, 1.0)
    } as f32;
    egui::pos2(
        cell_rect.left() + tx * cell_rect.width(),
        cell_rect.top() + ty * cell_rect.height(),
    )
}

/// Computes histogram bin counts.
pub fn compute_histogram(data: &[f64], n_bins: usize) -> Vec<usize> {
    if data.is_empty() || n_bins == 0 {
        return vec![0; n_bins];
    }
    // `data` is guaranteed non-empty by the emptiness check above.
    let (v_min, v_max) = value_range(data.iter().cloned()).unwrap();
    if (v_max - v_min).abs() < f64::EPSILON {
        let mut bins = vec![0usize; n_bins];
        bins[n_bins / 2] = data.len();
        return bins;
    }
    let mut bins = vec![0usize; n_bins];
    for &v in data {
        let idx = ((v - v_min) / (v_max - v_min) * n_bins as f64) as usize;
        let idx = idx.min(n_bins - 1);
        bins[idx] += 1;
    }
    bins
}

/// Computes the Pearson correlation coefficient.
///
/// Delegates the actual computation to `tunny_core::math::stats::pearson_correlation`.
/// For cell display purposes, however, degenerate cases (fewer than 2 elements, or
/// near-zero variance) return 0.0 instead of NaN, and the result is clamped to
/// [-1, 1] to account for floating-point error.
pub fn compute_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    let r = tunny_core::math::stats::pearson_correlation(&x[..n], &y[..n]);
    if r.is_nan() {
        0.0
    } else {
        r.clamp(-1.0, 1.0)
    }
}

/// Returns the min/max of a column (used for coordinate transforms in scatter cells).
/// The `f64::min`/`f64::max` reduction ignores NaN and propagates Inf (preserving
/// the previous behavior).
pub fn col_min_max(data: &[f64]) -> (f64, f64) {
    value_range(data.iter().cloned()).unwrap_or((f64::INFINITY, f64::NEG_INFINITY))
}

/// Computes the colors of feasible points to draw in the scatter matrix (same
/// ordering as `feasible_draw`). If there's no objective or the column can't be
/// retrieved, all points get `COLOR_SCATTER_DOT`. Since only the downsampled points
/// are actually drawn, the color array is only computed for that many points.
fn compute_feasible_point_colors(
    view: &crate::state::app_state::StudyView,
    color_objective: &Option<String>,
    obj_names: &[String],
    feas: tunny_core::dataframe::Feasibility<'_>,
    cmap: &crate::theme::colormap::ColorMap,
    feasible_draw: &[u32],
) -> Vec<egui::Color32> {
    use super::parallel_coords::{feasible_color_range, normalize_value};
    let Some(name) = resolve_color_objective(color_objective, obj_names) else {
        return vec![COLOR_SCATTER_DOT(); feasible_draw.len()];
    };
    let Some(col) = view.numeric_column(name) else {
        return vec![COLOR_SCATTER_DOT(); feasible_draw.len()];
    };
    let (col_min, col_max) = col_min_max(col);
    let (mn, mx) = feasible_color_range(col, feas, (col_min, col_max));
    feasible_draw
        .iter()
        .map(|&i| {
            let v = col.get(i as usize).copied().unwrap_or(f64::NAN);
            if v.is_finite() {
                cmap.interpolate(normalize_value(v, mn, mx))
            } else {
                COLOR_SCATTER_DOT()
            }
        })
        .collect()
}

/// Draws a scatter cell with the painter.
/// `colors` doesn't need to cover every trial — only pass as many entries as the
/// index sequence actually drawn (the order of `downsample_indices` if present,
/// otherwise 0..x_data.len()); computing only the downsampled point count at the
/// call site keeps the per-frame cost down.
/// `x_range`/`y_range` should be the column's precomputed min/max, passed in to
/// avoid a per-frame reduction (H-4).
#[allow(clippy::too_many_arguments)]
pub fn draw_scatter_cell(
    painter: &egui::Painter,
    cell_rect: egui::Rect,
    x_data: &[f64],
    y_data: &[f64],
    x_range: (f64, f64),
    y_range: (f64, f64),
    colors: &[egui::Color32],
    downsample_indices: Option<&[u32]>,
) {
    let (x_min, x_max) = x_range;
    let (y_min, y_max) = y_range;

    let indices: Box<dyn Iterator<Item = usize>> = if let Some(ds) = downsample_indices {
        Box::new(ds.iter().map(|&i| i as usize))
    } else {
        Box::new(0..x_data.len())
    };

    for (k, i) in indices.enumerate() {
        if i >= x_data.len() || i >= y_data.len() {
            continue;
        }
        let pos = data_to_screen(
            x_data[i],
            y_data[i],
            (x_min, x_max),
            (y_min, y_max),
            cell_rect,
        );
        let color = colors.get(k).copied().unwrap_or(COLOR_SCATTER_DOT());
        painter.circle_filled(pos, 1.6, color);
    }
}

/// Draws precomputed histogram bins as a bar chart with the painter.
/// Bin computation (`compute_histogram`) is cached by the caller (H-4).
pub fn draw_histogram_bars(painter: &egui::Painter, cell_rect: egui::Rect, bins: &[usize]) {
    let n_bins = bins.len().max(1);
    let max_count = *bins.iter().max().unwrap_or(&1).max(&1);
    let bar_width = cell_rect.width() / n_bins as f32;

    for (i, &count) in bins.iter().enumerate() {
        let bar_height = (count as f32 / max_count as f32) * cell_rect.height();
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(
                cell_rect.left() + i as f32 * bar_width,
                cell_rect.bottom() - bar_height,
            ),
            egui::vec2(bar_width - 1.0, bar_height),
        );
        painter.rect_filled(bar_rect, 0.0, COLOR_SCATTER_DOT());
    }
}

/// Draws a precomputed correlation coefficient `corr` as a cell with the painter.
/// Correlation computation (`compute_correlation`) is cached by the caller (H-4).
pub fn draw_correlation_cell(painter: &egui::Painter, cell_rect: egui::Rect, corr: f64) {
    let bg_color = correlation_color(corr);
    painter.rect_filled(cell_rect, 0.0, bg_color);
    painter.text(
        cell_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.2}", corr),
        egui::FontId::proportional(12.0),
        COLOR_CHART_TEXT(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── resolve_color_objective ──────────────────────────────────────

    #[test]
    fn resolve_color_objective_none_returns_first() {
        let names = vec!["obj0".to_string(), "obj1".to_string()];
        assert_eq!(resolve_color_objective(&None, &names), Some("obj0"));
    }

    #[test]
    fn resolve_color_objective_existing_name_returns_it() {
        let names = vec!["obj0".to_string(), "obj1".to_string()];
        assert_eq!(
            resolve_color_objective(&Some("obj1".to_string()), &names),
            Some("obj1")
        );
    }

    #[test]
    fn resolve_color_objective_unknown_name_falls_back_to_first() {
        let names = vec!["obj0".to_string(), "obj1".to_string()];
        assert_eq!(
            resolve_color_objective(&Some("unknown".to_string()), &names),
            Some("obj0")
        );
    }

    #[test]
    fn resolve_color_objective_empty_names_returns_none() {
        assert_eq!(resolve_color_objective(&None, &[]), None);
        assert_eq!(
            resolve_color_objective(&Some("obj0".to_string()), &[]),
            None
        );
    }

    // ── constraint-aware visualization (TASK-2350) ──────────────────

    #[test]
    fn tc_cav_scatter_matrix_show_infeasible_default_true() {
        let sm = ScatterMatrix::default();
        assert!(sm.show_infeasible);
    }

    #[test]
    fn tc_cav_split_feasibility_no_constraints_all_feasible() {
        use tunny_core::dataframe::Feasibility;
        let feas = Feasibility::from_column(None);
        let (f, inf) = split_feasibility_indices(3, feas);
        assert_eq!(f, vec![0, 1, 2]);
        assert!(inf.is_empty());
    }

    #[test]
    fn tc_cav_split_feasibility_mixed() {
        use tunny_core::dataframe::Feasibility;
        let col = vec![1.0_f64, 0.0, 1.0];
        let feas = Feasibility::from_column(Some(&col));
        let (f, inf) = split_feasibility_indices(3, feas);
        assert_eq!(f, vec![0, 2]);
        assert_eq!(inf, vec![1]);
    }

    #[test]
    fn tc_cav_split_feasibility_all_infeasible() {
        use tunny_core::dataframe::Feasibility;
        let col = vec![0.0_f64, 0.0];
        let feas = Feasibility::from_column(Some(&col));
        let (f, inf) = split_feasibility_indices(2, feas);
        assert!(f.is_empty());
        assert_eq!(inf, vec![0, 1]);
    }

    // TASK-2019 tests

    #[test]
    fn scatter_matrix_default_mode() {
        let sm = ScatterMatrix::default();
        assert_eq!(sm.mode, MatrixMode::ParamsVsParams);
        assert_eq!(sm.sort, AxisSort::Alphabetical);
        assert!(sm.selected_cell.is_none());
    }

    #[test]
    fn downsample_cap_keeps_all_when_under_cap() {
        let idx: Vec<u32> = (0..100).collect();
        let out = downsample_indices_to_cap(&idx, 4000);
        assert_eq!(out, idx);
    }

    #[test]
    fn downsample_cap_limits_when_over_cap() {
        let idx: Vec<u32> = (0..100_000).collect();
        let out = downsample_indices_to_cap(&idx, 4000);
        assert!(out.len() <= 4000, "got {}", out.len());
        assert!(!out.is_empty());
        // The first element is preserved, and downsampling keeps ascending order.
        assert_eq!(out[0], 0);
        assert!(out.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn downsample_cap_zero_is_empty() {
        let idx: Vec<u32> = (0..10).collect();
        assert!(downsample_indices_to_cap(&idx, 0).is_empty());
    }

    #[test]
    fn compute_histogram_bins_count() {
        let data = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let bins = compute_histogram(&data, 5);
        assert_eq!(bins.len(), 5);
        let total: usize = bins.iter().sum();
        assert_eq!(total, data.len());
    }

    #[test]
    fn compute_histogram_all_in_same_bin() {
        let data = vec![5.0; 10];
        let bins = compute_histogram(&data, 4);
        let total: usize = bins.iter().sum();
        assert_eq!(total, 10);
    }

    #[test]
    fn compute_histogram_empty_data() {
        let bins = compute_histogram(&[], 5);
        assert_eq!(bins.len(), 5);
        assert!(bins.iter().all(|&b| b == 0));
    }

    #[test]
    fn compute_correlation_perfect_positive() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y = x.clone();
        let corr = compute_correlation(&x, &y);
        assert!((corr - 1.0).abs() < 1e-9);
    }

    #[test]
    fn compute_correlation_perfect_negative() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&v| -v).collect();
        let corr = compute_correlation(&x, &y);
        assert!((corr + 1.0).abs() < 1e-9);
    }

    #[test]
    fn compute_correlation_range_bounded() {
        let x = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        let y = vec![2.0, 1.0, 4.0, 3.0, 5.0];
        let corr = compute_correlation(&x, &y);
        assert!((-1.0..=1.0).contains(&corr));
    }

    #[test]
    fn data_to_screen_min_maps_to_left_bottom() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let pos = data_to_screen(0.0, 0.0, (0.0, 1.0), (0.0, 1.0), rect);
        assert!((pos.x - 0.0).abs() < 1e-3);
        assert!((pos.y - 100.0).abs() < 1e-3); // y is inverted
    }

    #[test]
    fn data_to_screen_max_maps_to_right_top() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let pos = data_to_screen(1.0, 1.0, (0.0, 1.0), (0.0, 1.0), rect);
        assert!((pos.x - 100.0).abs() < 1e-3);
        assert!((pos.y - 0.0).abs() < 1e-3); // y is inverted
    }
}
