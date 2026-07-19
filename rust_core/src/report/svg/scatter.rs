//! `scatter_chart` (Pareto front).

use std::fmt::Write as _;

use crate::report::theme;

use super::primitives::*;

/// One point of a [`scatter_chart`].
#[derive(Debug, Clone, Copy)]
pub struct ScatterPoint {
    /// Trial number (shown in tooltip).
    pub trial_number: i64,
    /// X coordinate (first objective value).
    pub x: f64,
    /// Y coordinate (second objective value).
    pub y: f64,
    /// Whether this point satisfies all constraints (always `true` for
    /// unconstrained studies). Points with `false` get `[infeasible]`
    /// appended to their tooltip.
    pub feasible: bool,
}

/// Draws a Pareto scatter plot.
///
/// `background` is all COMPLETE points (muted, semi-transparent, r=4);
/// `front` is the non-dominated points (`series-1`, r=5). `front` is
/// internally sorted by ascending X and then connected with a staircase
/// line (1.5px) — no staircase line is drawn if `front` has fewer than two
/// points. Objective names are passed in as axis labels. When both the
/// front and dominated series are present, a small legend is shown in the
/// upper-right of the plot (the rule that two or more series require a
/// legend).
pub fn scatter_chart(
    background: &[ScatterPoint],
    front: &[ScatterPoint],
    x_label: &str,
    y_label: &str,
    width: f64,
    height: f64,
) -> String {
    if background.is_empty() && front.is_empty() {
        return empty_message(width, height);
    }

    let all_x: Vec<f64> = background.iter().chain(front.iter()).map(|p| p.x).collect();
    let all_y: Vec<f64> = background.iter().chain(front.iter()).map(|p| p.y).collect();
    let x_min_raw = all_x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max_raw = all_x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let y_min_raw = all_y.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max_raw = all_y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let x_ticks = nice_ticks(x_min_raw, x_max_raw, 5);
    let y_ticks = nice_ticks(y_min_raw, y_max_raw, 5);
    let x_min = x_ticks[0];
    let x_max = x_ticks[x_ticks.len() - 1];
    let y_min = y_ticks[0];
    let y_max = y_ticks[y_ticks.len() - 1];

    // Left margin = space for the rotated Y-axis title (16px) + max tick
    // label width + padding. Top margin reserves one line for the legend
    // (a legend is required because there are two series, front and
    // dominated; the shared rule of no legend for a single series applies
    // to the other charts).
    let max_y_tick_chars = y_ticks
        .iter()
        .map(|t| fmt_sig4(*t).chars().count())
        .max()
        .unwrap_or(1);
    let has_legend = !background.is_empty() && !front.is_empty();
    let m = Margins {
        top: if has_legend { 28.0 } else { 14.0 },
        right: 16.0,
        bottom: 44.0,
        left: 16.0 + max_y_tick_chars as f64 * CHAR_W + 12.0,
    };
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = (height - m.top - m.bottom).max(1.0);

    let sx = |v: f64| -> f64 {
        if (x_max - x_min).abs() < f64::EPSILON {
            m.left + plot_w / 2.0
        } else {
            m.left + (v - x_min) / (x_max - x_min) * plot_w
        }
    };
    let sy = |v: f64| -> f64 {
        if (y_max - y_min).abs() < f64::EPSILON {
            m.top + plot_h / 2.0
        } else {
            m.top + plot_h - (v - y_min) / (y_max - y_min) * plot_h
        }
    };

    let mut body = String::new();

    scatter_frame(
        &mut body, &m, plot_w, plot_h, height, &x_ticks, &y_ticks, &sx, &sy, x_label, y_label,
    );
    scatter_points(&mut body, background, front, &sx, &sy);
    if has_legend {
        scatter_legend(&mut body, &m, plot_w);
    }

    svg_wrap(width, height, &body)
}

/// Writes the scatter plot's frame (grid lines, tick labels, axis lines,
/// axis titles).
#[allow(clippy::too_many_arguments)]
fn scatter_frame(
    body: &mut String,
    m: &Margins,
    plot_w: f64,
    plot_h: f64,
    height: f64,
    x_ticks: &[f64],
    y_ticks: &[f64],
    sx: &dyn Fn(f64) -> f64,
    sy: &dyn Fn(f64) -> f64,
    x_label: &str,
    y_label: &str,
) {
    for t in y_ticks {
        let y = sy(*t);
        hairline(body, m.left, y, m.left + plot_w, y);
        text_muted(body, m.left - 8.0, y + 3.0, "end", &fmt_sig4(*t), true);
    }
    for t in x_ticks {
        let x = sx(*t);
        hairline(body, x, m.top, x, m.top + plot_h);
        text_muted(
            body,
            x,
            m.top + plot_h + 18.0,
            "middle",
            &fmt_sig4(*t),
            true,
        );
    }
    axis_line(
        body,
        m.left,
        m.top + plot_h,
        m.left + plot_w,
        m.top + plot_h,
    );
    axis_line(body, m.left, m.top, m.left, m.top + plot_h);

    text_muted(
        body,
        m.left + plot_w / 2.0,
        height - 4.0,
        "middle",
        x_label,
        false,
    );
    // Y-axis title: placed vertically with rotate(-90) inside the left
    // margin (further left than the tick labels). Rotation center =
    // (14, vertical center of the plot).
    let ty = m.top + plot_h / 2.0;
    let _ = writeln!(
        body,
        "<text x=\"14\" y=\"{}\" transform=\"rotate(-90 14 {})\" text-anchor=\"middle\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\">{}</text>",
        coord(ty),
        coord(ty),
        escape_xml(y_label),
        fs = FONT_SIZE,
        muted = theme::VAR_INK_MUTED
    );
}

/// Writes the scatter plot's data marks (background points, front
/// staircase line, front points).
fn scatter_points(
    body: &mut String,
    background: &[ScatterPoint],
    front: &[ScatterPoint],
    sx: &dyn Fn(f64) -> f64,
    sy: &dyn Fn(f64) -> f64,
) {
    for p in background {
        let title = escape_xml(&scatter_title(p, false));
        let _ = writeln!(
            body,
            "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({muted})\" fill-opacity=\"0.4\"><title>{title}</title></circle>",
            coord(sx(p.x)),
            coord(sy(p.y)),
            muted = theme::VAR_INK_MUTED
        );
    }

    if front.len() >= 2 {
        let mut sorted_front: Vec<&ScatterPoint> = front.iter().collect();
        sorted_front.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let mut path = format!(
            "M {} {}",
            coord(sx(sorted_front[0].x)),
            coord(sy(sorted_front[0].y))
        );
        for w in sorted_front.windows(2) {
            let (prev, cur) = (w[0], w[1]);
            let _ = write!(
                path,
                " L {} {} L {} {}",
                coord(sx(cur.x)),
                coord(sy(prev.y)),
                coord(sx(cur.x)),
                coord(sy(cur.y))
            );
        }
        let _ = writeln!(
            body,
            "<path d=\"{path}\" fill=\"none\" stroke=\"var({series1})\" stroke-width=\"1.5\" />",
            series1 = theme::VAR_SERIES[0]
        );
    }

    for p in front {
        let title = escape_xml(&scatter_title(p, true));
        let _ = writeln!(
            body,
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"var({series1})\"><title>{title}</title></circle>",
            coord(sx(p.x)),
            coord(sy(p.y)),
            series1 = theme::VAR_SERIES[0]
        );
    }
}

/// Tooltip string for a scatter point (including front / infeasible
/// annotations).
fn scatter_title(p: &ScatterPoint, on_front: bool) -> String {
    let mut title = format!(
        "trial #{} ({}, {})",
        p.trial_number,
        fmt_sig4(p.x),
        fmt_sig4(p.y)
    );
    if on_front {
        title.push_str(" [front]");
    }
    if !p.feasible {
        title.push_str(" [infeasible]");
    }
    title
}

/// Legend (only when both the front and dominated series are present):
/// places "● Pareto front ● dominated" right-aligned inside the top
/// margin at the upper-right of the plot. Legend markers are not data
/// marks, so they get no `<title>`.
fn scatter_legend(body: &mut String, m: &Margins, plot_w: f64) {
    const LEGEND_FRONT: &str = "Pareto front";
    const LEGEND_BG: &str = "dominated";
    let w_front = LEGEND_FRONT.chars().count() as f64 * CHAR_W;
    let w_bg = LEGEND_BG.chars().count() as f64 * CHAR_W;
    let item_gap = 18.0;
    let marker_w = 12.0; // marker diameter + gap to text
    let total = marker_w + w_front + item_gap + marker_w + w_bg;
    let start_x = (m.left + plot_w - total).max(m.left);
    let cy = 10.0;
    let text_y = cy + 4.0;

    let _ = writeln!(
        body,
        "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({series1})\" />",
        coord(start_x + 4.0),
        coord(cy),
        series1 = theme::VAR_SERIES[0]
    );
    text_muted(
        body,
        start_x + marker_w,
        text_y,
        "start",
        LEGEND_FRONT,
        false,
    );
    let x2 = start_x + marker_w + w_front + item_gap;
    let _ = writeln!(
        body,
        "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({muted})\" fill-opacity=\"0.4\" />",
        coord(x2 + 4.0),
        coord(cy),
        muted = theme::VAR_INK_MUTED
    );
    text_muted(body, x2 + marker_w, text_y, "start", LEGEND_BG, false);
}
