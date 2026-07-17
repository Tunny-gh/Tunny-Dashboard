use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_BAR_ACCENT, COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use tunny_core::statistics::{
    compute_histogram, fit_all, quantile, BinRule, FitDistribution, FittedDistribution, Histogram,
};

/// The histogram's bin-splitting rule (for the UI; `Manual` holds the count in a separate field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum HistBinRule {
    #[default]
    Sturges,
    Scott,
    FreedmanDiaconis,
    Manual,
}

impl HistBinRule {
    fn label(self) -> &'static str {
        match self {
            HistBinRule::Sturges => "Sturges",
            HistBinRule::Scott => "Scott",
            HistBinRule::FreedmanDiaconis => "Freedman-Diaconis",
            HistBinRule::Manual => "Manual",
        }
    }

    /// The discriminant used for the cache key.
    fn disc(self) -> u8 {
        match self {
            HistBinRule::Sturges => 0,
            HistBinRule::Scott => 1,
            HistBinRule::FreedmanDiaconis => 2,
            HistBinRule::Manual => 3,
        }
    }

    /// Exposed within the crate so the same bin splitting can be reproduced for CSV export.
    pub(crate) fn to_core(self, manual_bins: usize) -> BinRule {
        match self {
            HistBinRule::Sturges => BinRule::Sturges,
            HistBinRule::Scott => BinRule::Scott,
            HistBinRule::FreedmanDiaconis => BinRule::FreedmanDiaconis,
            HistBinRule::Manual => BinRule::Manual(manual_bins),
        }
    }
}

/// The distribution fit selection overlaid on the histogram.
/// `Auto` chooses the applicable distribution with the lowest AIC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum HistFit {
    #[default]
    None,
    Auto,
    Normal,
    LogNormal,
    Weibull,
}

impl HistFit {
    fn label(self) -> &'static str {
        match self {
            HistFit::None => "No fit",
            HistFit::Auto => "Auto (AIC)",
            HistFit::Normal => "Normal",
            HistFit::LogNormal => "Log-normal",
            HistFit::Weibull => "Weibull",
        }
    }
}

/// (study_name, col, rule_disc, manual_bins, row_count)
type HistCacheKey = (String, String, u8, usize, usize);

/// The full set of cached computation results. Since mean/median involve sorting,
/// they're cached alongside the histogram to avoid recomputing every frame.
/// `fits` is the applicable distribution fits (ascending AIC).
struct HistComputed {
    hist: Histogram,
    mean: f64,
    median: f64,
    fits: Vec<FittedDistribution>,
}

/// Histogram widget. Displays the distribution of a single column.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HistogramChart {
    /// Falls back to the first objective function if empty, or if the column name
    /// doesn't exist in the current Study.
    pub selected_col: String,
    pub rule: HistBinRule,
    pub manual_bins: usize,
    pub fit: HistFit,
    #[serde(skip)]
    cache: Option<(HistCacheKey, HistComputed)>,
}

impl Default for HistogramChart {
    fn default() -> Self {
        Self {
            selected_col: String::new(),
            rule: HistBinRule::default(),
            manual_bins: 20,
            fit: HistFit::default(),
            cache: None,
        }
    }
}

impl HistogramChart {
    /// Draws a histogram for numeric columns (objective functions first, then parameters).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        study_name: &str,
    ) {
        let candidates: Vec<&String> = obj_names
            .iter()
            .chain(param_names.iter())
            .filter(|name| view.numeric_column(name).is_some())
            .collect();

        if candidates.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        }

        if self.selected_col.is_empty() || !candidates.iter().any(|c| **c == self.selected_col) {
            self.selected_col = candidates[0].clone();
        }

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("histogram_col_combo")
                .selected_text(self.selected_col.clone())
                .show_ui(ui, |ui| {
                    for name in &candidates {
                        ui.selectable_value(&mut self.selected_col, (*name).clone(), name.as_str());
                    }
                });

            egui::ComboBox::from_id_salt("histogram_rule_combo")
                .selected_text(self.rule.label())
                .show_ui(ui, |ui| {
                    for rule in [
                        HistBinRule::Sturges,
                        HistBinRule::Scott,
                        HistBinRule::FreedmanDiaconis,
                        HistBinRule::Manual,
                    ] {
                        ui.selectable_value(&mut self.rule, rule, rule.label());
                    }
                });

            if self.rule == HistBinRule::Manual {
                ui.add(egui::Slider::new(&mut self.manual_bins, 2..=100).text("Bins"));
            }

            egui::ComboBox::from_id_salt("histogram_fit_combo")
                .selected_text(self.fit.label())
                .show_ui(ui, |ui| {
                    for fit in [
                        HistFit::None,
                        HistFit::Auto,
                        HistFit::Normal,
                        HistFit::LogNormal,
                        HistFit::Weibull,
                    ] {
                        ui.selectable_value(&mut self.fit, fit, fit.label());
                    }
                });
        });

        let Some(values) = view.numeric_column(&self.selected_col) else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        };

        let key: HistCacheKey = (
            study_name.to_string(),
            self.selected_col.clone(),
            self.rule.disc(),
            self.manual_bins,
            view.row_count(),
        );
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            let core_rule = self.rule.to_core(self.manual_bins);
            self.cache = compute_histogram(values, core_rule).map(|hist| {
                let mut sorted: Vec<f64> =
                    values.iter().copied().filter(|v| v.is_finite()).collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
                let median = quantile(&sorted, 0.5);
                let fits = fit_all(&sorted);
                (
                    key,
                    HistComputed {
                        hist,
                        mean,
                        median,
                        fits,
                    },
                )
            });
        }

        let Some((_, computed)) = &self.cache else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        };
        let (hist, mean, median) = (&computed.hist, computed.mean, computed.median);

        // The selected distribution fit (Auto = lowest AIC = the first of fits).
        let selected_fit: Option<&FittedDistribution> = match self.fit {
            HistFit::None => None,
            HistFit::Auto => computed.fits.first(),
            HistFit::Normal => computed
                .fits
                .iter()
                .find(|f| f.dist == FitDistribution::Normal),
            HistFit::LogNormal => computed
                .fits
                .iter()
                .find(|f| f.dist == FitDistribution::LogNormal),
            HistFit::Weibull => computed
                .fits
                .iter()
                .find(|f| f.dist == FitDistribution::Weibull),
        };

        // An overlay curve converting the PDF to a count scale (n × bin width).
        let fit_line = selected_fit.map(|f| {
            let n_total: f64 = hist.counts.iter().map(|&c| c as f64).sum();
            let (first, last) = (
                *hist.bin_edges.first().unwrap_or(&0.0),
                *hist.bin_edges.last().unwrap_or(&1.0),
            );
            let bin_w = if hist.counts.is_empty() {
                1.0
            } else {
                (last - first) / hist.counts.len() as f64
            };
            const CURVE_POINTS: usize = 200;
            let pts: Vec<[f64; 2]> = (0..=CURVE_POINTS)
                .map(|i| {
                    let x = first + (last - first) * i as f64 / CURVE_POINTS as f64;
                    [x, f.pdf(x) * n_total * bin_w]
                })
                .collect();
            egui_plot::Line::new(format!("{} fit", f.dist.label()), pts)
                .color(egui::Color32::from_rgb(147, 51, 234))
                .width(2.0)
        });

        let bars: Vec<egui_plot::Bar> = hist
            .bin_edges
            .windows(2)
            .zip(&hist.counts)
            .map(|(edge, &count)| {
                let raw_width = edge[1] - edge[0];
                let (center, width) = if raw_width > 0.0 {
                    ((edge[0] + edge[1]) / 2.0, raw_width)
                } else {
                    (edge[0], 1.0)
                };
                egui_plot::Bar::new(center, count as f64).width(width)
            })
            .collect();
        let chart = egui_plot::BarChart::new("Count", bars).color(COLOR_BAR_PRIMARY());

        egui_plot::Plot::new("histogram_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .x_axis_label(&self.selected_col)
            .y_axis_label("Count")
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                plot_ui.bar_chart(chart);
                plot_ui.vline(
                    egui_plot::VLine::new("Mean", mean)
                        .color(COLOR_BAR_NEGATIVE())
                        .style(egui_plot::LineStyle::Dashed { length: 6.0 }),
                );
                plot_ui.vline(
                    egui_plot::VLine::new("Median", median)
                        .color(COLOR_BAR_ACCENT())
                        .style(egui_plot::LineStyle::Dashed { length: 6.0 }),
                );
                if let Some(line) = fit_line {
                    plot_ui.line(line);
                }
            });

        // Displays the fit's parameters, or a note that it's not applicable.
        match (self.fit, selected_fit) {
            (HistFit::None, _) => {}
            (_, Some(f)) => {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}   AIC {:.1}",
                        f.dist.label(),
                        f.param_text(),
                        f.aic
                    ))
                    .small(),
                );
            }
            (HistFit::Auto, None) => {
                ui.label(
                    egui::RichText::new("No distribution could be fitted.")
                        .small()
                        .weak(),
                );
            }
            (_, None) => {
                ui.label(
                    egui::RichText::new(
                        "Selected fit is not applicable (needs ≥3 finite, positive values).",
                    )
                    .small()
                    .weak(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_chart_default_values() {
        let chart = HistogramChart::default();
        assert_eq!(chart.selected_col, "");
        assert_eq!(chart.rule, HistBinRule::Sturges);
        assert_eq!(chart.manual_bins, 20);
        assert!(chart.cache.is_none());
    }

    #[test]
    fn cache_key_changes_with_rule() {
        let key_a: HistCacheKey = ("s".into(), "x".into(), HistBinRule::Sturges.disc(), 20, 10);
        let key_b: HistCacheKey = ("s".into(), "x".into(), HistBinRule::Scott.disc(), 20, 10);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_changes_with_manual_bins() {
        let key_a: HistCacheKey = ("s".into(), "x".into(), HistBinRule::Manual.disc(), 20, 10);
        let key_b: HistCacheKey = ("s".into(), "x".into(), HistBinRule::Manual.disc(), 30, 10);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_changes_with_row_count() {
        let key_a: HistCacheKey = ("s".into(), "x".into(), 0, 20, 10);
        let key_b: HistCacheKey = ("s".into(), "x".into(), 0, 20, 11);
        assert_ne!(key_a, key_b);
    }
}
