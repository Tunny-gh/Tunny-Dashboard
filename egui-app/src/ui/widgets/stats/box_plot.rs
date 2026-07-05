use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use tunny_core::statistics::{compute_boxplot, BoxPlotStats};

/// 箱ひげ図の対象列グループ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BoxPlotSource {
    #[default]
    Objectives,
    Parameters,
}

impl BoxPlotSource {
    fn label(self) -> &'static str {
        match self {
            BoxPlotSource::Objectives => "Objectives",
            BoxPlotSource::Parameters => "Parameters",
        }
    }

    fn disc(self) -> u8 {
        match self {
            BoxPlotSource::Objectives => 0,
            BoxPlotSource::Parameters => 1,
        }
    }
}

/// (study_name, source_disc, normalize, row_count)
type BoxCacheKey = (String, u8, bool, usize);

/// 複数列の箱ひげ図を並べて表示するウィジェット。
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BoxPlotChart {
    pub source: BoxPlotSource,
    /// 表示用に各列を min-max 正規化するか（[0,1]。統計値そのものは変更しない）。
    pub normalize: bool,
    #[serde(skip)]
    cache: Option<(BoxCacheKey, Vec<(String, BoxPlotStats)>)>,
}

impl BoxPlotChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        study_name: &str,
    ) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("box_plot_source_combo")
                .selected_text(self.source.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.source,
                        BoxPlotSource::Objectives,
                        BoxPlotSource::Objectives.label(),
                    );
                    ui.selectable_value(
                        &mut self.source,
                        BoxPlotSource::Parameters,
                        BoxPlotSource::Parameters.label(),
                    );
                });

            ui.toggle_value(&mut self.normalize, "Normalize [0,1]")
                .on_hover_text("Min-max normalize each column for display");
        });

        let names: &[String] = match self.source {
            BoxPlotSource::Objectives => obj_names,
            BoxPlotSource::Parameters => param_names,
        };
        let columns: Vec<&String> = names
            .iter()
            .filter(|n| view.numeric_column(n).is_some())
            .collect();

        if columns.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        }

        let key: BoxCacheKey = (
            study_name.to_string(),
            self.source.disc(),
            self.normalize,
            view.row_count(),
        );
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            let normalize = self.normalize;
            let stats: Vec<(String, BoxPlotStats)> = columns
                .iter()
                .filter_map(|name| {
                    let raw = view.numeric_column(name)?;
                    let values = if normalize {
                        normalize_minmax(raw)
                    } else {
                        raw.to_vec()
                    };
                    compute_boxplot(&values).map(|s| ((*name).clone(), s))
                })
                .collect();
            self.cache = Some((key, stats));
        }

        let stats = &self.cache.as_ref().expect("cache just populated above").1;
        if stats.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        }

        let labels: Vec<String> = stats.iter().map(|(name, _)| name.clone()).collect();
        let boxes: Vec<egui_plot::BoxElem> = stats
            .iter()
            .enumerate()
            .map(|(i, (name, s))| {
                let spread =
                    egui_plot::BoxSpread::new(s.whisker_low, s.q1, s.median, s.q3, s.whisker_high);
                egui_plot::BoxElem::new(i as f64, spread).name(name.clone())
            })
            .collect();
        let mut outlier_pts: Vec<[f64; 2]> = Vec::new();
        for (i, (_, s)) in stats.iter().enumerate() {
            for &v in &s.outliers {
                outlier_pts.push([i as f64, v]);
            }
        }

        let box_plot = egui_plot::BoxPlot::new("Box Plot", boxes).color(COLOR_BAR_PRIMARY);

        egui_plot::Plot::new("box_plot_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .x_axis_formatter(move |mark, _range| {
                let idx = mark.value.round();
                if (mark.value - idx).abs() < 1e-6 && idx >= 0.0 && (idx as usize) < labels.len() {
                    labels[idx as usize].clone()
                } else {
                    String::new()
                }
            })
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                plot_ui.box_plot(box_plot);
                if !outlier_pts.is_empty() {
                    let pts: egui_plot::PlotPoints = outlier_pts.into();
                    plot_ui.points(
                        egui_plot::Points::new("Outliers", pts)
                            .shape(egui_plot::MarkerShape::Circle)
                            .radius(3.0)
                            .color(COLOR_BAR_NEGATIVE),
                    );
                }
            });
    }
}

/// 各列を独立に min-max 正規化する（表示専用）。定数列（min == max）は全値 0.0 に潰す。
/// 非有限値はそのまま素通しし、後段の `compute_boxplot` で除外させる。
/// CSV エクスポートでも同じ正規化を再現するため crate 内に公開する。
pub(crate) fn normalize_minmax(values: &[f64]) -> Vec<f64> {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return values.to_vec();
    }
    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                v
            } else if max > min {
                (v - min) / (max - min)
            } else {
                0.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_plot_chart_default_values() {
        let chart = BoxPlotChart::default();
        assert_eq!(chart.source, BoxPlotSource::Objectives);
        assert!(!chart.normalize);
        assert!(chart.cache.is_none());
    }

    #[test]
    fn cache_key_changes_with_source() {
        let key_a: BoxCacheKey = ("s".into(), BoxPlotSource::Objectives.disc(), false, 5);
        let key_b: BoxCacheKey = ("s".into(), BoxPlotSource::Parameters.disc(), false, 5);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_changes_with_normalize() {
        let key_a: BoxCacheKey = ("s".into(), 0, false, 5);
        let key_b: BoxCacheKey = ("s".into(), 0, true, 5);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn normalize_minmax_scales_to_unit_range() {
        let normalized = normalize_minmax(&[0.0, 5.0, 10.0]);
        assert_eq!(normalized, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn normalize_minmax_constant_column_collapses_to_zero() {
        let normalized = normalize_minmax(&[3.0, 3.0, 3.0]);
        assert_eq!(normalized, vec![0.0, 0.0, 0.0]);
    }
}
