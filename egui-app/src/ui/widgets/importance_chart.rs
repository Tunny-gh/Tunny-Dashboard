use crate::state::app_state::{SensitivityResult, SobolResult};

#[derive(Debug, Clone, PartialEq)]
pub enum ImportanceMetric {
    Spearman,
    Ridge,
    RfAnova,
    SobolFirst,
    SobolTotal,
}

impl ImportanceMetric {
    pub fn label(&self) -> &'static str {
        match self {
            ImportanceMetric::Spearman => "Spearman",
            ImportanceMetric::Ridge => "Ridge",
            ImportanceMetric::RfAnova => "RF-Anova",
            ImportanceMetric::SobolFirst => "Sobol First",
            ImportanceMetric::SobolTotal => "Sobol Total",
        }
    }

    pub fn is_sobol(&self) -> bool {
        matches!(self, ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal)
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
        // Run ボタン + メトリクスコンボボックス + 目的関数コンボボックス + spinner + R²（右端）
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                self.pending_compute = Some(self.metric.clone());
                self.computing = true;
            }

            egui::ComboBox::from_id_salt("importance_metric")
                .selected_text(self.metric.label())
                .show_ui(ui, |ui| {
                    for metric in [
                        ImportanceMetric::Spearman,
                        ImportanceMetric::Ridge,
                        ImportanceMetric::RfAnova,
                        ImportanceMetric::SobolFirst,
                        ImportanceMetric::SobolTotal,
                    ] {
                        let label = metric.label();
                        ui.selectable_value(&mut self.metric, metric, label);
                    }
                });

            if obj_names.len() > 1 {
                egui::ComboBox::from_id_salt("importance_objective")
                    .selected_text(
                        obj_names
                            .get(self.objective_index)
                            .map(|s| s.as_str())
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.objective_index, i, name);
                        }
                    });
            }

            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }

            // R² を右端に表示（Spearman 以外）
            let r2_opt: Option<f64> = match self.metric {
                ImportanceMetric::Spearman => None,
                ImportanceMetric::Ridge => sensitivity
                    .and_then(|r| r.ridge.get(self.objective_index))
                    .map(|ridge| ridge.r_squared),
                ImportanceMetric::RfAnova => sensitivity
                    .and_then(|r| r.rf_anova.as_ref())
                    .and_then(|rf| rf.r_squared.get(self.objective_index))
                    .copied(),
                ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => sobol
                    .and_then(|s| s.r_squared.get(self.objective_index))
                    .copied(),
            };
            if let Some(r2) = r2_opt {
                let (color, warning) = if r2 < 0.5 {
                    (egui::Color32::from_rgb(220, 80, 80), " (low fit)")
                } else if r2 < 0.8 {
                    (egui::Color32::from_rgb(200, 160, 0), "")
                } else {
                    (egui::Color32::from_rgb(60, 180, 60), "")
                };
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label(
                            egui::RichText::new(format!("R² = {r2:.3}{warning}"))
                                .color(color),
                        );
                    },
                );
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
            ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => {
                let Some(sobol_result) = sobol else {
                    ui.label("No Sobol data (start the computation first)");
                    return;
                };
                compute_sorted_sobol(sobol_result, self.objective_index, &self.metric)
            }
        };

        if scores.is_empty() {
            ui.label("No data");
            return;
        }

        let max_score = scores.iter().map(|(_, s)| *s).fold(0.0_f64, f64::max);
        let label_width = 150.0_f32;
        let bar_height = 20.0_f32;
        let bar_gap = 4.0_f32;
        let value_text_width = 50.0_f32;

        let bar_color = egui::Color32::from_rgb(0x0c, 0x0c, 0x6a);
        egui::ScrollArea::vertical().show(ui, |ui| {
                    let available_width =
                        ui.available_width() - label_width - value_text_width - 8.0;
                    let bar_max_width = available_width.max(50.0);

                    for (name, score) in &scores {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [label_width, bar_height],
                                egui::Label::new(
                                    egui::RichText::new(name).text_style(egui::TextStyle::Body),
                                )
                                .truncate(),
                            );

                            let bar_width = if max_score > 0.0 {
                                (score / max_score * bar_max_width as f64) as f32
                            } else {
                                0.0
                            };

                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(bar_max_width, bar_height - bar_gap),
                                egui::Sense::hover(),
                            );
                            if ui.is_rect_visible(rect) {
                                let bar_rect = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(bar_width, rect.height()),
                                );
                                ui.painter().rect_filled(bar_rect, 2.0, bar_color);
                            }

                            ui.label(format!("{score:.3}"));
                        });
                    }
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
        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => return vec![],
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

/// SobolResult から重要度スコアを降順でソートして返す。
/// metric に応じて一次指数（SobolFirst）または全効果指数（SobolTotal）を使用する。
pub fn compute_sorted_sobol(
    result: &SobolResult,
    obj_idx: usize,
    metric: &ImportanceMetric,
) -> Vec<(String, f64)> {
    let scores_opt = match metric {
        ImportanceMetric::SobolFirst => result.first_order.get(obj_idx),
        ImportanceMetric::SobolTotal => result.total_effect.get(obj_idx),
        _ => return vec![],
    };
    let Some(scores) = scores_opt else {
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
            rf_anova: Some(RfAnovaResult { importances, r_squared: vec![0.8] }),
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
        assert!(!ImportanceMetric::SobolFirst.label().is_empty());
        assert!(!ImportanceMetric::SobolTotal.label().is_empty());
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
    fn sorted_importance_sobol_returns_empty_from_sensitivity() {
        let result = make_result(&["x"], vec![0.5]);
        // SobolFirst/SobolTotal are handled by the Sobol branch, not compute_sorted_importance
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::SobolFirst, 0);
        assert!(sorted.is_empty());
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::SobolTotal, 0);
        assert!(sorted.is_empty());
    }

    #[test]
    fn sorted_sobol_first_and_total_use_correct_data() {
        use crate::state::app_state::SobolResult;
        let result = SobolResult {
            param_names: vec!["p0".into(), "p1".into()],
            objective_names: vec!["obj0".into()],
            first_order: vec![vec![0.6, 0.2]],  // [obj][param]
            total_effect: vec![vec![0.8, 0.3]],
            r_squared: vec![0.9],
        };
        let first = compute_sorted_sobol(&result, 0, &ImportanceMetric::SobolFirst);
        assert_eq!(first[0].0, "p0");
        assert!((first[0].1 - 0.6).abs() < 1e-9);

        let total = compute_sorted_sobol(&result, 0, &ImportanceMetric::SobolTotal);
        assert_eq!(total[0].0, "p0");
        assert!((total[0].1 - 0.8).abs() < 1e-9);
    }
}
