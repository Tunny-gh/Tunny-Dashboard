//! Rendering of a cached robustness analysis result: histogram bar geometry
//! construction and the histogram + statistics summary plot.

use tunny_core::statistics::{compute_histogram, BinRule};
use tunny_core::surrogate_opt::RobustnessResult;

use crate::theme::chart_colors::{COLOR_BAR_ACCENT, COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};

/// A cached analysis result plus the histogram bar geometry
/// `(center, height, width)` built from it exactly once. Keeping this
/// alongside the result avoids recomputing `compute_histogram` and rebuilding
/// the Bar Vec every frame (no recomputation as long as the result is
/// unchanged).
pub(super) struct RobustnessRender {
    pub(super) result: RobustnessResult,
    pub(super) bars: Vec<(f64, f64, f64)>,
}

/// Builds the histogram bar geometry `(center, height, width)` from the output
/// samples. Returns an empty Vec if there are too few samples and
/// `compute_histogram` returns `None` (the caller treats this as a signal
/// that plotting is not possible).
pub(super) fn build_histogram_bars(samples: &[f64]) -> Vec<(f64, f64, f64)> {
    let Some(hist) = compute_histogram(samples, BinRule::Sturges) else {
        return Vec::new();
    };
    hist.bin_edges
        .windows(2)
        .zip(&hist.counts)
        .map(|(edge, &count)| {
            let raw_width = edge[1] - edge[0];
            let (center, width) = if raw_width > 0.0 {
                ((edge[0] + edge[1]) / 2.0, raw_width)
            } else {
                (edge[0], 1.0)
            };
            (center, count as f64, width)
        })
        .collect()
}

/// Renders the histogram plus statistics summary.
/// `lower_spec` / `upper_spec` draw LSL/USL vertical lines on the histogram
/// only when set. The bar geometry is cached (`render.bars`), so no re-binning
/// happens every frame.
pub(super) fn render_result(
    ui: &mut egui::Ui,
    render: &RobustnessRender,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) {
    let result = &render.result;
    if render.bars.is_empty() {
        ui.label(egui::RichText::new("Not enough samples to plot.").weak());
        return;
    }

    // Rebuild egui_plot::Bar from the cached bar geometry (center, height, width) — cheap.
    let bars: Vec<egui_plot::Bar> = render
        .bars
        .iter()
        .map(|&(center, height, width)| egui_plot::Bar::new(center, height).width(width))
        .collect();
    let chart = egui_plot::BarChart::new("Samples", bars).color(COLOR_BAR_PRIMARY());

    // Reserve height for the stats / success-rate label rows (below the
    // plot). Without this, the plot would expand to fill the entire available
    // height, pushing the labels outside the widget and out of view.
    let label_rows = 1
        + usize::from(result.feasibility_rate.is_some())
        + usize::from(result.success_rate.is_some())
        + usize::from(result.clipped_fraction > 0.0);
    let reserved = 8.0 + label_rows as f32 * 20.0;
    let plot_height = (ui.available_height() - reserved).max(120.0);

    egui_plot::Plot::new("robustness_histogram")
        .unified_nav()
        .legend(egui_plot::Legend::default())
        .x_axis_label("Output")
        .y_axis_label("Count")
        .height(plot_height)
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            plot_ui.bar_chart(chart);
            plot_ui.vline(
                egui_plot::VLine::new("Nominal", result.nominal)
                    .color(crate::theme::chart_colors::COLOR_GRID_STROKE())
                    .style(egui_plot::LineStyle::Dashed { length: 8.0 }),
            );
            plot_ui.vline(egui_plot::VLine::new("Mean", result.mean).color(COLOR_BAR_NEGATIVE()));
            plot_ui.vline(
                egui_plot::VLine::new("P5", result.p05)
                    .color(COLOR_BAR_ACCENT())
                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
            );
            plot_ui.vline(
                egui_plot::VLine::new("P95", result.p95)
                    .color(COLOR_BAR_ACCENT())
                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
            );
            if let Some(lsl) = lower_spec {
                plot_ui.vline(egui_plot::VLine::new("LSL", lsl).color(COLOR_BAR_NEGATIVE()));
            }
            if let Some(usl) = upper_spec {
                plot_ui.vline(egui_plot::VLine::new("USL", usl).color(COLOR_BAR_NEGATIVE()));
            }
        });

    ui.add_space(4.0);
    let shift = result.mean - result.nominal;
    ui.label(format!(
        "Mean {:.4} ± {:.4}   Shift {:+.4}   P5 {:.4} / P95 {:.4}",
        result.mean, result.std, shift, result.p05, result.p95
    ));
    if let Some(rate) = result.feasibility_rate {
        ui.label(format!("P(feasible) {:.1}%", rate * 100.0));
    }
    if let Some(rate) = result.success_rate {
        let sigma = result.sigma_level.unwrap_or(0.0);
        let color = if sigma >= 4.0 {
            crate::theme::chart_colors::COLOR_FIT_HIGH()
        } else if sigma >= 2.0 {
            crate::theme::chart_colors::COLOR_FIT_MID()
        } else {
            crate::theme::chart_colors::COLOR_FIT_LOW()
        };
        let mut line = format!("Success: {:.2}%  |  σ level: {:.2}σ", rate * 100.0, sigma);
        if let Some(cpk) = result.cpk {
            line.push_str(&format!("  |  Cpk: {cpk:.2}"));
        }
        ui.colored_label(color, line);
    }
    if result.clipped_fraction > 0.0 {
        ui.colored_label(
            egui::Color32::from_rgb(202, 138, 4), // amber-600
            format!("Clipped {:.1}%", result.clipped_fraction * 100.0),
        );
    }
}
