//! `heatmap` (correlation heatmap).

use std::fmt::Write as _;

use crate::report::theme;

use super::primitives::*;

/// Max display character count for row labels (excess is truncated with
/// `…` + `<title>`).
const HEATMAP_MAX_ROW_CHARS: usize = 24;
/// Max display character count for column labels. This bound keeps the
/// projected height of the rotated labels (= top margin) finite; excess
/// is truncated to `…` + `<title>`.
const HEATMAP_MAX_COL_CHARS: usize = 22;

/// Draws a correlation heatmap (diverging color scheme, value range
/// `[-1, 1]`).
///
/// Shape is `matrix[row][col]`. Cells have a 2px gap, and values are
/// quantized into a diverging 5+5+neutral scale
/// ([`theme::diverging_bin`]) for fill color; only cells with
/// `|value| > 0.5` get a direct numeric label (ink switches between
/// black/white based on cell lightness via [`theme::diverging_ink_var`]).
/// A discrete color legend showing the quantization steps is drawn on the
/// right.
///
/// Width is given by the `width` argument; height is auto-determined from
/// the row count (about 28px per row). Assumes
/// `row_labels.len() == matrix.len()` and
/// `col_labels.len() == matrix[0].len()`.
///
/// Row labels are truncated to 24 characters, column labels to 22
/// (appending `…`, with the full label kept in `<title>`); the left
/// margin is sized dynamically from the display label width, and the top
/// margin from the vertical projected height of the column labels rotated
/// by -40°.
pub fn heatmap(
    matrix: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    width: f64,
) -> String {
    const GAP: f64 = 2.0;
    const CELL_H: f64 = 28.0;
    const LEGEND_W: f64 = 90.0;
    /// Vertical projected height per character of a column label rotated
    /// by -40° (`CHAR_W × sin(40°) ≈ 7.0 × 0.643`).
    const COL_LABEL_RISE: f64 = 4.5;

    let rows = matrix.len();
    let cols = matrix.first().map(Vec::len).unwrap_or(0);
    if rows == 0 || cols == 0 {
        return empty_message(width, 40.0);
    }

    // Determine margins dynamically from the max display (post-truncation)
    // label character count.
    let max_row_chars = row_labels
        .iter()
        .map(|s| truncate_label(s, HEATMAP_MAX_ROW_CHARS).chars().count())
        .max()
        .unwrap_or(0);
    let max_col_chars = col_labels
        .iter()
        .map(|s| truncate_label(s, HEATMAP_MAX_COL_CHARS).chars().count())
        .max()
        .unwrap_or(0);
    let m = Margins {
        top: 24.0 + max_col_chars as f64 * COL_LABEL_RISE,
        right: LEGEND_W + 16.0,
        bottom: 8.0,
        left: 12.0 + max_row_chars as f64 * CHAR_W + 8.0,
    };

    let height = m.top + m.bottom + rows as f64 * CELL_H + (rows.saturating_sub(1)) as f64 * GAP;
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = rows as f64 * CELL_H + (rows.saturating_sub(1)) as f64 * GAP;
    let cell_w = ((plot_w - GAP * (cols.saturating_sub(1)) as f64) / cols as f64).max(0.5);

    let mut body = String::new();

    heatmap_col_labels(&mut body, col_labels, cols, &m, cell_w, GAP);
    heatmap_cells(
        &mut body, matrix, row_labels, col_labels, rows, cols, &m, cell_w, CELL_H, GAP,
    );
    heatmap_legend(&mut body, width, &m, plot_h, LEGEND_W);

    svg_wrap(width, height, &body)
}

/// Writes the heatmap's column labels (rotate(-40°), truncation +
/// `<title>` fallback).
fn heatmap_col_labels(
    body: &mut String,
    col_labels: &[String],
    cols: usize,
    m: &Margins,
    cell_w: f64,
    gap: f64,
) {
    for (c, label) in col_labels.iter().enumerate().take(cols) {
        let cx = m.left + c as f64 * (cell_w + gap) + cell_w / 2.0;
        let cy = m.top - 8.0;
        let display = truncate_label(label, HEATMAP_MAX_COL_CHARS);
        // Only keep the full label in <title> when truncation occurred.
        let title = if display == *label {
            String::new()
        } else {
            format!("<title>{}</title>", escape_xml(label))
        };
        let _ = writeln!(
            body,
            "<text x=\"{}\" y=\"{}\" transform=\"rotate(-40 {} {})\" text-anchor=\"start\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\">{title}{}</text>",
            coord(cx),
            coord(cy),
            coord(cx),
            coord(cy),
            escape_xml(&display),
            fs = FONT_SIZE,
            muted = theme::VAR_INK_MUTED
        );
    }
}

/// Writes the heatmap's row labels and cells (quantized fill + direct
/// label for highly-correlated cells).
#[allow(clippy::too_many_arguments)]
fn heatmap_cells(
    body: &mut String,
    matrix: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    rows: usize,
    cols: usize,
    m: &Margins,
    cell_w: f64,
    cell_h: f64,
    gap: f64,
) {
    for (r, row) in matrix.iter().enumerate().take(rows) {
        let y = m.top + r as f64 * (cell_h + gap);
        let row_label = row_labels.get(r).map(String::as_str).unwrap_or("");
        let display = truncate_label(row_label, HEATMAP_MAX_ROW_CHARS);
        let title = if display == row_label {
            String::new()
        } else {
            format!("<title>{}</title>", escape_xml(row_label))
        };
        let _ = writeln!(
            body,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\">{title}{}</text>",
            coord(m.left - 8.0),
            coord(y + cell_h / 2.0 + 4.0),
            escape_xml(&display),
            fs = FONT_SIZE,
            muted = theme::VAR_INK_MUTED
        );

        for (c, &value) in row.iter().enumerate().take(cols) {
            let x = m.left + c as f64 * (cell_w + gap);
            let clamped = value.clamp(-1.0, 1.0);
            let bin = theme::diverging_bin(clamped);
            let fill_var = theme::diverging_var(bin);
            let col_label = col_labels.get(c).map(String::as_str).unwrap_or("");
            let title = escape_xml(&format!("{row_label} × {col_label}: {}", fmt_sig4(value)));
            let _ = writeln!(
                body,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var({fill_var})\"><title>{title}</title></rect>",
                coord(x),
                coord(y),
                coord(cell_w),
                coord(cell_h)
            );

            if theme::diverging_show_label(value) {
                let ink_var = theme::diverging_ink_var(bin);
                let _ = writeln!(
                    body,
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({ink_var})\" style=\"font-variant-numeric: tabular-nums\">{}</text>",
                    coord(x + cell_w / 2.0),
                    coord(y + cell_h / 2.0 + 4.0),
                    escape_xml(&fmt_sig4(value)),
                    fs = FONT_SIZE
                );
            }
        }
    }
}

/// Writes the discrete color legend (11 steps from -5..=5 stacked
/// vertically).
fn heatmap_legend(body: &mut String, width: f64, m: &Margins, plot_h: f64, legend_w: f64) {
    let legend_x = width - legend_w + 8.0;
    let swatch_h = (plot_h / 11.0).max(6.0);
    for (i, bin) in (-5..=5).rev().enumerate() {
        let y = m.top + i as f64 * swatch_h;
        let fill_var = theme::diverging_var(bin);
        let range_desc = diverging_bin_range_desc(bin);
        let _ = writeln!(
            body,
            "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"{}\" fill=\"var({fill_var})\"><title>{}</title></rect>",
            coord(legend_x),
            coord(y),
            coord(swatch_h.max(1.0)),
            escape_xml(&range_desc)
        );
        if bin == 5 || bin == 0 || bin == -5 {
            let label = if bin == 5 {
                "1"
            } else if bin == -5 {
                "-1"
            } else {
                "0"
            };
            text_muted(
                body,
                legend_x + 18.0,
                y + swatch_h / 2.0 + 3.0,
                "start",
                label,
                true,
            );
        }
    }
}

/// Builds a string describing the value range a quantization step
/// represents, for the legend's `<title>`.
fn diverging_bin_range_desc(bin: i32) -> String {
    match bin {
        0 => "0".to_string(),
        b if b > 0 => {
            let lower = (b - 1) as f64 / 5.0;
            let upper = b as f64 / 5.0;
            format!("{lower} < ρ ≤ {upper}")
        }
        b => {
            let lower = b as f64 / 5.0;
            let upper = (b + 1) as f64 / 5.0;
            format!("{lower} ≤ ρ < {upper}")
        }
    }
}
