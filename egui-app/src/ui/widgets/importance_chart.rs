use crate::state::app_state::SensitivityResult;

#[derive(Debug, Clone, PartialEq)]
pub enum ImportanceMetric {
    Spearman,
    Ridge,
    Sobol,
}

impl ImportanceMetric {
    pub fn label(&self) -> &'static str {
        match self {
            ImportanceMetric::Spearman => "Spearman",
            ImportanceMetric::Ridge => "Ridge (beta)",
            ImportanceMetric::Sobol => "Sobol",
        }
    }
}

/// 感度分析バーチャートウィジェット
pub struct ImportanceChart {
    pub metric: ImportanceMetric,
    pub computing: bool,
    pub objective_index: usize,
}

impl Default for ImportanceChart {
    fn default() -> Self {
        Self {
            metric: ImportanceMetric::Spearman,
            computing: false,
            objective_index: 0,
        }
    }
}

impl ImportanceChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        sensitivity: Option<&SensitivityResult>,
        obj_names: &[String],
    ) {
        // メトリクス切り替え
        ui.horizontal(|ui| {
            for metric in [
                ImportanceMetric::Spearman,
                ImportanceMetric::Ridge,
                ImportanceMetric::Sobol,
            ] {
                let selected = self.metric == metric;
                if ui.selectable_label(selected, metric.label()).clicked() {
                    self.metric = metric;
                }
            }
        });

        // 目的関数タブ（複数目的の場合）
        if obj_names.len() > 1 {
            ui.horizontal(|ui| {
                for (i, name) in obj_names.iter().enumerate() {
                    if ui.selectable_label(self.objective_index == i, name).clicked() {
                        self.objective_index = i;
                    }
                }
            });
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing...");
            });
            return;
        }

        let Some(result) = sensitivity else {
            ui.label("No sensitivity data (start the computation first)");
            return;
        };

        let scores = compute_sorted_importance(result, self.objective_index);

        if scores.is_empty() {
            ui.label("No data");
            return;
        }

        egui_plot::Plot::new("importance_chart_plot")
            .show(ui, |plot_ui| {
                let bars: Vec<egui_plot::Bar> = scores
                    .iter()
                    .enumerate()
                    .map(|(i, (_, score))| {
                        egui_plot::Bar::new(i as f64, *score)
                            .width(0.8)
                            .fill(egui::Color32::from_rgb(70, 150, 250))
                    })
                    .collect();
                plot_ui.bar_chart(egui_plot::BarChart::new(bars));
            });
    }
}

/// SensitivityResult から重要度スコアを降順でソートして返す
pub fn compute_sorted_importance(
    result: &SensitivityResult,
    obj_idx: usize,
) -> Vec<(String, f64)> {
    let obj_scores = result.spearman.get(obj_idx);
    let Some(scores) = obj_scores else {
        return vec![];
    };

    let mut pairs: Vec<(String, f64)> = result
        .param_names
        .iter()
        .zip(scores.iter())
        .map(|(name, &score)| (name.clone(), score.abs()))
        .collect();

    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::SensitivityResult;

    fn make_result(params: &[&str], scores: Vec<f64>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            objective_names: vec!["obj0".to_string()],
            spearman: vec![scores],
        }
    }

    #[test]
    fn sorted_importance_descending_order() {
        let result = make_result(&["x", "y", "z"], vec![0.3, 0.8, 0.1]);
        let sorted = compute_sorted_importance(&result, 0);
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].0, "y");
        assert_eq!(sorted[1].0, "x");
        assert_eq!(sorted[2].0, "z");
    }

    #[test]
    fn sorted_importance_uses_abs_value() {
        // Negative correlation should rank high if magnitude is large
        let result = make_result(&["x", "y"], vec![-0.9, 0.3]);
        let sorted = compute_sorted_importance(&result, 0);
        assert_eq!(sorted[0].0, "x");
        assert!((sorted[0].1 - 0.9).abs() < 1e-9);
    }

    #[test]
    fn sorted_importance_invalid_obj_idx_returns_empty() {
        let result = make_result(&["x"], vec![0.5]);
        let sorted = compute_sorted_importance(&result, 99);
        assert!(sorted.is_empty());
    }

    #[test]
    fn importance_metric_labels_not_empty() {
        assert!(!ImportanceMetric::Spearman.label().is_empty());
        assert!(!ImportanceMetric::Ridge.label().is_empty());
        assert!(!ImportanceMetric::Sobol.label().is_empty());
    }

    #[test]
    fn importance_chart_default() {
        let chart = ImportanceChart::default();
        assert_eq!(chart.metric, ImportanceMetric::Spearman);
        assert!(!chart.computing);
        assert_eq!(chart.objective_index, 0);
    }
}
