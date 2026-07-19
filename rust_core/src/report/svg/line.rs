//! `line_chart` (best-so-far / HV history).

use std::fmt::Write as _;

use crate::report::theme;

use super::primitives::*;

/// One point of a [`line_chart`]. X = trial number, Y = value.
#[derive(Debug, Clone, Copy)]
pub struct LinePoint {
    /// Trial number.
    pub trial_number: i64,
    /// Value (best-so-far or HV).
    pub value: f64,
}

/// Draws a line chart of the convergence curve (best-so-far / HV history).
///
/// Single series, no fill, 2px stroke. `improvement_marks` is a set of
/// indices into `points`; only best-update points get a 4px marker (the
/// caller passes in the already-determined values — this function does
/// not judge that itself). The final point always shows a direct label
/// (its value); if the final point is not in `improvement_marks`, a
/// marker is added for it too. The X axis uses integer (trial-number)
/// ticks.
pub fn line_chart(
    points: &[LinePoint],
    improvement_marks: &[usize],
    width: f64,
    height: f64,
) -> String {
    if points.is_empty() {
        return empty_message(width, height);
    }

    let x_min = points[0].trial_number as f64;
    let x_max = points[points.len() - 1].trial_number as f64;
    let y_min_raw = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let y_max_raw = points
        .iter()
        .map(|p| p.value)
        .fold(f64::NEG_INFINITY, f64::max);

    let y_ticks = nice_ticks(y_min_raw, y_max_raw, 5);
    let y_min = y_ticks[0];
    let y_max = y_ticks[y_ticks.len() - 1];
    let x_ticks = nice_ticks_integer(x_min, x_max, 6);

    let last_idx = points.len() - 1;
    let final_label = fmt_sig4(points[last_idx].value);

    let max_y_tick_chars = y_ticks
        .iter()
        .map(|t| fmt_sig4(*t).chars().count())
        .max()
        .unwrap_or(1);
    let m = Margins {
        top: 14.0,
        right: 20.0 + final_label.chars().count() as f64 * CHAR_W,
        bottom: 30.0,
        left: 16.0 + max_y_tick_chars as f64 * CHAR_W,
    };
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = (height - m.top - m.bottom).max(1.0);

    let sx = |tn: f64| -> f64 {
        if (x_max - x_min).abs() < f64::EPSILON {
            m.left + plot_w / 2.0
        } else {
            m.left + (tn - x_min) / (x_max - x_min) * plot_w
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

    for t in &y_ticks {
        let y = sy(*t);
        hairline(&mut body, m.left, y, m.left + plot_w, y);
        text_muted(&mut body, m.left - 8.0, y + 3.0, "end", &fmt_sig4(*t), true);
    }
    axis_line(
        &mut body,
        m.left,
        m.top + plot_h,
        m.left + plot_w,
        m.top + plot_h,
    );
    for t in &x_ticks {
        let x = sx(*t as f64);
        text_muted(
            &mut body,
            x,
            m.top + plot_h + 18.0,
            "middle",
            &t.to_string(),
            true,
        );
    }

    let path_points = points
        .iter()
        .map(|p| {
            format!(
                "{},{}",
                coord(sx(p.trial_number as f64)),
                coord(sy(p.value))
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let _ = writeln!(
        body,
        "<polyline points=\"{path_points}\" fill=\"none\" stroke=\"var({series1})\" stroke-width=\"2\" />",
        series1 = theme::VAR_SERIES[0]
    );

    let marker = |idx: usize, body: &mut String| {
        let p = &points[idx];
        let cx = sx(p.trial_number as f64);
        let cy = sy(p.value);
        let title = escape_xml(&format!(
            "trial #{} = {}",
            p.trial_number,
            fmt_sig4(p.value)
        ));
        let _ = writeln!(
            body,
            "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({series1})\"><title>{title}</title></circle>",
            coord(cx),
            coord(cy),
            series1 = theme::VAR_SERIES[0]
        );
    };

    for &idx in improvement_marks {
        if idx < points.len() {
            marker(idx, &mut body);
        }
    }
    if !improvement_marks.contains(&last_idx) {
        marker(last_idx, &mut body);
    }

    let last = &points[last_idx];
    text_secondary(
        &mut body,
        sx(last.trial_number as f64) + 8.0,
        sy(last.value) + 4.0,
        "start",
        &final_label,
    );

    svg_wrap(width, height, &body)
}
