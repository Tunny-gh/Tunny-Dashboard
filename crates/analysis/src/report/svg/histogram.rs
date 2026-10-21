//! `histogram` (objective value distribution).

use std::fmt::Write as _;

use crate::report::theme;

use super::primitives::*;

/// One bin of a [`histogram`].
#[derive(Debug, Clone, Copy)]
pub struct HistBin {
    /// Lower bin edge.
    pub lower: f64,
    /// Upper bin edge.
    pub upper: f64,
    /// Count.
    pub count: u64,
}

/// Draws a histogram (single sequential color, 2px gap between bins).
pub fn histogram(bins: &[HistBin], width: f64, height: f64) -> String {
    const GAP: f64 = 2.0;

    if bins.is_empty() {
        return empty_message(width, height);
    }

    let max_count = bins.iter().map(|b| b.count).max().unwrap_or(0);
    let y_ticks = nice_ticks(0.0, max_count as f64, 4);
    let y_max = y_ticks[y_ticks.len() - 1].max(1.0);

    let max_y_tick_chars = y_ticks
        .iter()
        .map(|t| format!("{}", t.round() as i64).chars().count())
        .max()
        .unwrap_or(1);
    let m = Margins {
        top: 12.0,
        right: 12.0,
        bottom: 28.0,
        left: 16.0 + max_y_tick_chars as f64 * CHAR_W,
    };
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = (height - m.top - m.bottom).max(1.0);

    let n = bins.len();
    let bin_w = ((plot_w - GAP * (n.saturating_sub(1)) as f64) / n as f64).max(0.5);

    let mut body = String::new();

    for t in &y_ticks {
        let y = m.top + plot_h - (t / y_max) * plot_h;
        hairline(&mut body, m.left, y, m.left + plot_w, y);
        text_muted(
            &mut body,
            m.left - 8.0,
            y + 3.0,
            "end",
            &format!("{}", t.round() as i64),
            true,
        );
    }

    for (i, bin) in bins.iter().enumerate() {
        let x = m.left + i as f64 * (bin_w + GAP);
        let bar_h = (bin.count as f64 / y_max) * plot_h;
        let y = m.top + plot_h - bar_h;
        let title = escape_xml(&format!(
            "[{}, {}): {}",
            fmt_sig4(bin.lower),
            fmt_sig4(bin.upper),
            bin.count
        ));
        let _ = writeln!(
            body,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var({seq400})\"><title>{title}</title></rect>",
            coord(x),
            coord(y),
            coord(bin_w),
            coord(bar_h),
            seq400 = theme::VAR_SEQ[3].1
        );
    }

    // X-axis representative ticks (first bin's lower edge / middle
    // boundary / last bin's upper edge).
    let first = bins[0].lower;
    let mid = bins[n / 2].lower;
    let last = bins[n - 1].upper;
    for (val, x) in [
        (first, m.left),
        (mid, m.left + (n / 2) as f64 * (bin_w + GAP)),
        (last, m.left + plot_w),
    ] {
        text_muted(&mut body, x, height - 8.0, "middle", &fmt_sig4(val), true);
    }

    svg_wrap(width, height, &body)
}
