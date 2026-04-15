use crate::state::app_state::{SensitivityResult, SobolResult};

#[derive(Debug, Clone, PartialEq)]
pub enum ImportanceMetric {
    Spearman,
    Ridge,
    RfAnova,
    Sobol,
}

impl ImportanceMetric {
    pub fn label(&self) -> &'static str {
        match self {
            ImportanceMetric::Spearman => "Spearman",
            ImportanceMetric::Ridge => "Ridge (beta)",
            ImportanceMetric::RfAnova => "RF-Anova",
            ImportanceMetric::Sobol => "Sobol",
        }
    }
}

/// 感度分析バーチャートウィジェット
pub struct ImportanceChart {
    pub metric: ImportanceMetric,
    pub computing: bool,
    pub objective_index: usize,
    pub pending_compute: Option<ImportanceMetric>,
}

impl Default for ImportanceChart {
    fn default() -> Self {
        Self {
            metric: ImportanceMetric::Spearman,
            computing: false,
            objective_index: 0,
            pending_compute: None,
        }
    }
}

impl ImportanceChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        sensitivity: Option<&SensitivityResult>,
        sobol: Option<&SobolResult>,
        obj_names: &[String],
    ) {
        // メトリクス切り替え
        ui.horizontal(|ui| {
            for metric in [
                ImportanceMetric::Spearman,
                ImportanceMetric::Ridge,
                ImportanceMetric::RfAnova,
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
                    if ui
                        .selectable_label(self.objective_index == i, name)
                        .clicked()
                    {
                        self.objective_index = i;
                    }
                }
            });
        }

        // Run ボタン + spinner
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                self.pending_compute = Some(self.metric.clone());
                self.computing = true;
            }
            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }
        });

        if self.computing {
            return;
        }

        let scores = match self.metric {
            ImportanceMetric::Spearman | ImportanceMetric::Ridge | ImportanceMetric::RfAnova => {
                let Some(result) = sensitivity else {
                    ui.label("No sensitivity data (start the computation first)");
                    return;
                };
                compute_sorted_importance(result, &self.metric, self.objective_index)
            }
            ImportanceMetric::Sobol => {
                let Some(sobol_result) = sobol else {
                    ui.label("No Sobol data (start the computation first)");
                    return;
                };
                compute_sorted_sobol(sobol_result, self.objective_index)
            }
        };

        if scores.is_empty() {
            ui.label("No data");
            return;
        }

        egui_plot::Plot::new("importance_chart_plot").show(ui, |plot_ui| {
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
    metric: &ImportanceMetric,
    obj_idx: usize,
) -> Vec<(String, f64)> {
    let raw_scores: Vec<f64> = match metric {
        ImportanceMetric::Spearman => {
            let Some(scores) = result.spearman.get(obj_idx) else {
                return vec![];
            };
            scores.clone()
        }
        ImportanceMetric::Ridge => {
            let Some(ridge) = result.ridge.get(obj_idx) else {
                return vec![];
            };
            ridge.beta.iter().map(|b| b.abs()).collect()
        }
        ImportanceMetric::RfAnova => {
            let Some(ref rf) = result.rf_anova else {
                return vec![];
            };
            rf.importances
                .iter()
                .map(|param_imp| param_imp.get(obj_idx).copied().unwrap_or(0.0).abs())
                .collect()
        }
        ImportanceMetric::Sobol => return vec![],
    };

    let mut pairs: Vec<(String, f64)> = result
        .param_names
        .iter()
        .zip(raw_scores.iter())
        .map(|(name, &score)| (name.clone(), score.abs()))
        .collect();

    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs
}

/// SobolResult から重要度スコア（first_order）を降順でソートして返す
pub fn compute_sorted_sobol(result: &SobolResult, obj_idx: usize) -> Vec<(String, f64)> {
    let Some(scores) = result.first_order.get(obj_idx) else {
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
    use crate::state::app_state::{RfAnovaResult, RidgeResult, SensitivityResult};

    fn make_result(params: &[&str], scores: Vec<f64>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            objective_names: vec!["obj0".to_string()],
            spearman: vec![scores],
            ridge: vec![],
            rf_anova: None,
        }
    }

    fn make_result_with_ridge(params: &[&str], beta: Vec<f64>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            objective_names: vec!["obj0".to_string()],
            spearman: vec![vec![0.5; params.len()]],
            ridge: vec![RidgeResult {
                beta,
                r_squared: 0.8,
            }],
            rf_anova: None,
        }
    }

    fn make_result_with_rf_anova(params: &[&str], importances: Vec<Vec<f64>>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            objective_names: vec!["obj0".to_string()],
            spearman: vec![vec![0.5; params.len()]],
            ridge: vec![],
            rf_anova: Some(RfAnovaResult { importances }),
        }
    }

    #[test]
    fn sorted_importance_descending_order() {
        let result = make_result(&["x", "y", "z"], vec![0.3, 0.8, 0.1]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Spearman, 0);
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].0, "y");
        assert_eq!(sorted[1].0, "x");
        assert_eq!(sorted[2].0, "z");
    }

    #[test]
    fn sorted_importance_uses_abs_value() {
        let result = make_result(&["x", "y"], vec![-0.9, 0.3]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Spearman, 0);
        assert_eq!(sorted[0].0, "x");
        assert!((sorted[0].1 - 0.9).abs() < 1e-9);
    }

    #[test]
    fn sorted_importance_invalid_obj_idx_returns_empty() {
        let result = make_result(&["x"], vec![0.5]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Spearman, 99);
        assert!(sorted.is_empty());
    }

    #[test]
    fn importance_metric_labels_not_empty() {
        assert!(!ImportanceMetric::Spearman.label().is_empty());
        assert!(!ImportanceMetric::Ridge.label().is_empty());
        assert!(!ImportanceMetric::RfAnova.label().is_empty());
        assert!(!ImportanceMetric::Sobol.label().is_empty());
    }

    #[test]
    fn importance_chart_default() {
        let chart = ImportanceChart::default();
        assert_eq!(chart.metric, ImportanceMetric::Spearman);
        assert!(!chart.computing);
        assert_eq!(chart.objective_index, 0);
        assert!(chart.pending_compute.is_none());
    }

    #[test]
    fn sorted_importance_ridge_uses_abs_beta() {
        let result = make_result_with_ridge(&["a", "b"], vec![-0.5, 0.9]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Ridge, 0);
        assert_eq!(sorted[0].0, "b");
        assert!((sorted[0].1 - 0.9).abs() < 1e-9);
        assert_eq!(sorted[1].0, "a");
        assert!((sorted[1].1 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn sorted_importance_rf_anova_descending() {
        // 2 params, 1 objective: param0 importance=0.3, param1 importance=0.7
        let result = make_result_with_rf_anova(&["p0", "p1"], vec![vec![0.3], vec![0.7]]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::RfAnova, 0);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].0, "p1");
        assert!((sorted[0].1 - 0.7).abs() < 1e-9);
    }

    #[test]
    fn sorted_importance_sobol_returns_empty() {
        let result = make_result(&["x"], vec![0.5]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Sobol, 0);
        assert!(sorted.is_empty());
    }
}
