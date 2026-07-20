//! `hbar_chart` (parameter importance).

use std::fmt::Write as _;

use crate::report::theme;

use super::primitives::*;

/// One bar of an [`hbar_chart`].
#[derive(Debug, Clone)]
pub struct HBarItem {
    /// Category name (e.g. parameter name).
    pub label: String,
    /// Value (importance score).
    pub value: f64,
}

/// Draws a horizontal bar chart (height auto-determined from item count).
///
/// Bars use a single `series-1` color (ranking is not color-coded). Bar
/// height 20px, rounded corners `rx=2`, 8px gap between bars. The value is
/// labeled directly at the right end of the bar, and the category name is
/// shown on the left (truncated with `…` beyond 24 characters, with the
/// full label preserved in `<title>`).
pub fn hbar_chart(items: &[HBarItem], width: f64) -> String {
    const BAR_H: f64 = 20.0;
    const GAP: f64 = 8.0;
    const MAX_LABEL_CHARS: usize = 24;

    if items.is_empty() {
        return empty_message(width, 40.0);
    }

    let n = items.len();
    let top = 8.0;
    let bottom = 24.0;
    let height = top + bottom + n as f64 * BAR_H + (n.saturating_sub(1)) as f64 * GAP;

    let max_val = items
        .iter()
        .map(|it| it.value)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0);
    let value_ticks = nice_ticks(0.0, max_val, 4);
    let x_max = value_ticks[value_ticks.len() - 1].max(f64::EPSILON);

    let max_label_len = items
        .iter()
        .map(|it| truncate_label(&it.label, MAX_LABEL_CHARS).chars().count())
        .max()
        .unwrap_or(0);
    let max_value_len = items
        .iter()
        .map(|it| fmt_sig4(it.value).chars().count())
        .max()
        .unwrap_or(1);

    // The left margin is dynamically sized as
    // max display label chars (after truncation) × CHAR_W + padding
    // (since labels are placed at m.left - 8 with an end anchor, we need
    // the full label width + anchor padding + left-edge padding).
    let m = Margins {
        top,
        right: 16.0 + max_value_len as f64 * CHAR_W,
        bottom,
        left: 12.0 + max_label_len as f64 * CHAR_W + 8.0,
    };
    let plot_w = (width - m.left - m.right).max(1.0);

    let mut body = String::new();

    for t in &value_ticks {
        let x = m.left + (t / x_max) * plot_w;
        hairline(&mut body, x, m.top, x, height - m.bottom);
        text_muted(
            &mut body,
            x,
            height - m.bottom + 16.0,
            "middle",
            &fmt_sig4(*t),
            true,
        );
    }

    for (i, item) in items.iter().enumerate() {
        let y_top = m.top + i as f64 * (BAR_H + GAP);
        let y_mid = y_top + BAR_H / 2.0 + 4.0;
        let bar_w = ((item.value.max(0.0) / x_max) * plot_w).max(0.0);

        let full_label_escaped = escape_xml(&item.label);
        let display_label = truncate_label(&item.label, MAX_LABEL_CHARS);
        let _ = writeln!(
            body,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\"><title>{full_label_escaped}</title>{}</text>",
            coord(m.left - 8.0),
            coord(y_mid),
            escape_xml(&display_label),
            fs = FONT_SIZE,
            muted = theme::VAR_INK_MUTED
        );

        let title = escape_xml(&format!("{}: {}", item.label, fmt_sig4(item.value)));
        let _ = writeln!(
            body,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"var({series1})\"><title>{title}</title></rect>",
            coord(m.left),
            coord(y_top),
            coord(bar_w),
            coord(BAR_H),
            series1 = theme::VAR_SERIES[0]
        );

        text_secondary(
            &mut body,
            m.left + bar_w + 6.0,
            y_mid,
            "start",
            &fmt_sig4(item.value),
        );
    }

    svg_wrap(width, height, &body)
}
