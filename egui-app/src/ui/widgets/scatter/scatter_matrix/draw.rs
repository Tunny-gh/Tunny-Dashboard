//! Cell-drawing helpers for the scatter matrix: coordinate transforms and the
//! egui `Painter` calls for scatter/histogram/correlation cells, plus the
//! feasible-point color computation that feeds the scatter cells.

use crate::theme::chart_colors::{COLOR_CHART_TEXT, COLOR_SCATTER_DOT};
use crate::theme::color_compute::correlation_color;

use super::stats::{col_min_max, resolve_color_objective};

/// Converts data coordinates to screen coordinates.
pub(super) fn data_to_screen(
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

/// Computes the colors of feasible points to draw in the scatter matrix (same
/// ordering as `feasible_draw`). If there's no objective or the column can't be
/// retrieved, all points get `COLOR_SCATTER_DOT`. Since only the downsampled points
/// are actually drawn, the color array is only computed for that many points.
pub(super) fn compute_feasible_point_colors(
    view: &crate::state::app_state::StudyView,
    color_objective: &Option<String>,
    obj_names: &[String],
    feas: tunny_core::dataframe::Feasibility<'_>,
    cmap: &crate::theme::colormap::ColorMap,
    feasible_draw: &[u32],
) -> Vec<egui::Color32> {
    use super::super::parallel_coords::{feasible_color_range, normalize_value};
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
pub(super) fn draw_scatter_cell(
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
pub(super) fn draw_histogram_bars(painter: &egui::Painter, cell_rect: egui::Rect, bins: &[usize]) {
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
pub(super) fn draw_correlation_cell(painter: &egui::Painter, cell_rect: egui::Rect, corr: f64) {
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
