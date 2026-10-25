use super::comparison::{objective_color, paint_scores, ObjectiveImportance};
use crate::state::app_state::{SensitivityResult, SobolResult};
use crate::theme::chart_colors::{COLOR_FIT_HIGH, COLOR_FIT_LOW, COLOR_FIT_MID};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum ImportanceMetric {
    #[default]
    Spearman,
    Ridge,
    RfAnova,
    Mdi,
    SobolFirst,
    SobolTotal,
    Shap,
    Permutation,
    Ard,
}

impl ImportanceMetric {
    pub fn label(&self) -> &'static str {
        match self {
            ImportanceMetric::Spearman => "Spearman",
            ImportanceMetric::Ridge => "Ridge",
            ImportanceMetric::RfAnova => "RF-Anova",
            ImportanceMetric::Mdi => "MDI",
            ImportanceMetric::SobolFirst => "Sobol First",
            ImportanceMetric::SobolTotal => "Sobol Total",
            ImportanceMetric::Shap => "SHAP",
            ImportanceMetric::Permutation => "Permutation",
            ImportanceMetric::Ard => "ARD",
        }
    }

    pub fn is_sobol(&self) -> bool {
        matches!(
            self,
            ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal
        )
    }

    /// Stable numeric key for use in caches — decoupled from display labels.
    pub fn cache_id(&self) -> u8 {
        match self {
            ImportanceMetric::Spearman => 0,
            ImportanceMetric::Ridge => 1,
            ImportanceMetric::RfAnova => 2,
            ImportanceMetric::Mdi => 3,
            ImportanceMetric::SobolFirst => 4,
            ImportanceMetric::SobolTotal => 5,
            ImportanceMetric::Shap => 6,
            ImportanceMetric::Permutation => 7,
            ImportanceMetric::Ard => 8,
        }
    }

    /// Whether values can have a sign (negative correlations / negative
    /// coefficients are possible). Only Spearman / Ridge are signed;
    /// tree-based and Sobol metrics are non-negative. Used to switch between
    /// diverging/sequential colormaps in the Sensitivity Heatmap.
    pub fn is_signed(&self) -> bool {
        matches!(self, ImportanceMetric::Spearman | ImportanceMetric::Ridge)
    }

    /// Whether this metric is computationally expensive, i.e. involves model
    /// training (tree-based) or Sobol sampling. Only correlation/linear
    /// metrics (Spearman, Ridge) are cheap.
    pub fn is_expensive(&self) -> bool {
        !matches!(self, ImportanceMetric::Spearman | ImportanceMetric::Ridge)
    }
}

/// Sample count for Sobol index estimation (shared by ImportanceChart /
/// Sensitivity Heatmap).
pub const SOBOL_SAMPLE_COUNT: usize = 1024;

pub const ALL_OBJECTIVES: usize = usize::MAX;
pub type ImportanceKey = (u8, usize, bool);

/// Constructs the corresponding core sensitivity metric from an
/// `ImportanceMetric`. Returns `None` for Sobol, since it uses the dedicated
/// `compute_sobol_from_df` path. Lets ImportanceChart and Sensitivity Heatmap
/// share the same mapping table.
pub fn core_sensitivity_metric(
    metric: ImportanceMetric,
) -> Option<Box<dyn tdashboard_analysis::sensitivity::SensitivityMetric>> {
    use tdashboard_analysis::sensitivity::{
        MdiMetric, PermutationMetric, RfAnovaMetric, RidgeMetric, ShapMetric, SpearmanMetric,
    };
    Some(match metric {
        ImportanceMetric::Spearman => Box::new(SpearmanMetric),
        ImportanceMetric::Ridge => Box::new(RidgeMetric),
        ImportanceMetric::RfAnova => Box::new(RfAnovaMetric),
        ImportanceMetric::Mdi => Box::new(MdiMetric),
        ImportanceMetric::Shap => Box::new(ShapMetric),
        ImportanceMetric::Permutation => Box::new(PermutationMetric),
        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal | ImportanceMetric::Ard => {
            return None
        }
    })
}

/// Sensitivity analysis bar chart widget
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ImportanceChart {
    pub metric: ImportanceMetric,
    #[serde(skip)]
    pub computing: bool,
    pub objective_index: usize,
    /// Whether to fit the model using feasible trials only (UI shown only for
    /// constrained studies)
    pub feasible_only: bool,
    /// Compute request (method, objective idx, feasible_only). Consumed by poll_chart.
    #[serde(skip)]
    pub pending_compute: Option<(ImportanceMetric, usize, bool)>,
    #[serde(skip)]
    pub queue: std::collections::VecDeque<(ImportanceMetric, usize, bool)>,
    #[serde(skip)]
    pub active: Option<(ImportanceMetric, usize, bool)>,
    #[serde(skip)]
    pub errors: std::collections::HashMap<ImportanceKey, String>,
}

impl Default for ImportanceChart {
    fn default() -> Self {
        Self {
            metric: ImportanceMetric::Spearman,
            computing: false,
            objective_index: ALL_OBJECTIVES,
            feasible_only: false,
            pending_compute: None,
            queue: Default::default(),
            active: None,
            errors: Default::default(),
        }
    }
}

impl ImportanceChart {
    pub fn normalize_objective(&mut self, count: usize) {
        if count == 1 {
            self.objective_index = 0;
        } else if self.objective_index != ALL_OBJECTIVES && self.objective_index >= count {
            self.objective_index = ALL_OBJECTIVES;
        }
    }

    pub fn objective_indices(&self, count: usize) -> Vec<usize> {
        if self.objective_index == ALL_OBJECTIVES {
            (0..count).collect()
        } else if self.objective_index < count {
            vec![self.objective_index]
        } else {
            vec![0; usize::from(count > 0)]
        }
    }

    pub fn invalidate_input(&mut self) {
        self.computing = false;
        self.pending_compute = None;
        self.queue.clear();
        self.active = None;
        self.errors.clear();
    }

    pub fn finish_objective(&mut self, error: Option<String>) {
        if let Some((metric, objective, feasible)) = self.active.take() {
            let key = (metric.cache_id(), objective, feasible);
            if let Some(error) = error {
                self.errors.insert(key, error);
            } else {
                self.errors.remove(&key);
            }
        }
        self.computing = !self.queue.is_empty();
    }

    /// Pulls in the global widget's compute execution state (computing/pending).
    /// Results are aggregated into `app_state.importance_cache` /
    /// `sobol_cache`, so each canvas item (an independent WidgetState) only
    /// needs the execution state applied. Metric and objective selections are
    /// per-item, so they are preserved.
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
    }

    pub fn show_results(
        &mut self,
        ui: &mut egui::Ui,
        results: &[ObjectiveImportance],
        obj_names: &[String],
        has_constraints: bool,
    ) {
        self.normalize_objective(obj_names.len());
        let previous = (self.metric, self.objective_index, self.feasible_only);
        // Controls stay available while the queued request finishes on its captured settings.
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.computing, egui::Button::new("Run"))
                .clicked()
            {
                self.pending_compute =
                    Some((self.metric, self.objective_index, self.feasible_only));
                self.computing = true;
            }

            egui::ComboBox::from_id_salt("importance_metric")
                .selected_text(self.metric.label())
                .show_ui(ui, |ui| {
                    ui.label(group_header("── Correlation / Linear ──"));
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::Spearman,
                        ImportanceMetric::Spearman.label(),
                    );
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::Ridge,
                        ImportanceMetric::Ridge.label(),
                    );

                    ui.separator();
                    ui.label(group_header("── Tree-based ──"));
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::RfAnova,
                        ImportanceMetric::RfAnova.label(),
                    );
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::Mdi,
                        ImportanceMetric::Mdi.label(),
                    );
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::Shap,
                        ImportanceMetric::Shap.label(),
                    );
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::Permutation,
                        ImportanceMetric::Permutation.label(),
                    );

                    ui.separator();
                    ui.label(group_header("── Global Sensitivity ──"));
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::SobolFirst,
                        ImportanceMetric::SobolFirst.label(),
                    );
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::SobolTotal,
                        ImportanceMetric::SobolTotal.label(),
                    );

                    ui.separator();
                    ui.label(group_header("── Surrogate (GP) ──"));
                    ui.selectable_value(
                        &mut self.metric,
                        ImportanceMetric::Ard,
                        ImportanceMetric::Ard.label(),
                    );
                });

            if obj_names.len() > 1 {
                egui::ComboBox::from_id_salt("importance_objective")
                    .selected_text(
                        obj_names
                            .get(self.objective_index)
                            .map(|s| s.as_str())
                            .unwrap_or("ALL"),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.objective_index, ALL_OBJECTIVES, "ALL");
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.objective_index, i, name);
                        }
                    });
            }

            // Feasible-only filter (constrained studies only)
            if has_constraints {
                ui.toggle_value(&mut self.feasible_only, "Feasible only")
                    .on_hover_text("Fit the model using feasible trials only");
            }

            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }
        });
        if previous != (self.metric, self.objective_index, self.feasible_only) {
            // Results were selected before the controls changed this frame.
            return;
        }
        if self.computing {
            return;
        }
        let all = self.objective_index == ALL_OBJECTIVES && obj_names.len() > 1;
        if all {
            ui.horizontal_wrapped(|ui| {
                for result in results {
                    let font = egui::TextStyle::Body.resolve(ui.style());
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        "■ ",
                        0.0,
                        egui::TextFormat::simple(font.clone(), objective_color(result.index)),
                    );
                    job.append(
                        &result.name,
                        0.0,
                        egui::TextFormat::simple(font.clone(), ui.visuals().text_color()),
                    );
                    if let Some(r2) = result.r_squared {
                        let (label, color) = fit_label(self.metric, r2);
                        job.append(
                            &format!("  {label}"),
                            0.0,
                            egui::TextFormat::simple(font, color),
                        );
                    }
                    // Lay out the complete entry before allocation so wrapping
                    // moves its color, name and fit together to the next row.
                    job.wrap.max_width = ui.available_width();
                    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                    ui.label(galley);
                }
            });
        }
        for result in results {
            let has_status = result.error.is_some()
                || (result.scores.is_empty() && result.unsupported.is_empty())
                || (!result.scores.is_empty() && result.scores.iter().all(|(_, s)| !s.is_finite()));
            if !all || has_status {
                ui.horizontal_wrapped(|ui| {
                    if let Some(error) = &result.error {
                        ui.label(format!("{}: Failed / unavailable: {error}", result.name));
                    } else if result.scores.is_empty() && result.unsupported.is_empty() {
                        ui.label(format!(
                            "{}: Unavailable (run the computation first)",
                            result.name
                        ));
                    } else if !result.scores.is_empty()
                        && result.scores.iter().all(|(_, s)| !s.is_finite())
                    {
                        ui.label(format!("{}: Unavailable (no finite scores)", result.name));
                    }
                    if let Some(r2) = result.r_squared.filter(|_| !all) {
                        let (label, color) = fit_label(self.metric, r2);
                        ui.colored_label(color, label);
                    }
                });
            }
            if all && !result.unsupported.is_empty() {
                ui.label(format!(
                    "{}: Unsupported (categorical): {}",
                    result.name,
                    result.unsupported.join(", ")
                ));
                if result.scores.is_empty() {
                    ui.label(format!(
                        "{}: No supported numerical parameters",
                        result.name
                    ));
                }
            } else {
                show_unsupported(ui, &result.unsupported, result.scores.is_empty());
            }
        }
        if all && results.iter().any(ObjectiveImportance::is_partial) {
            ui.label("Partial CSV export: only successful available scores are exported; failed, unavailable and unsupported results are omitted.");
        }
        paint_scores(ui, results, all);
    }
}

fn fit_label(metric: ImportanceMetric, r2: f64) -> (String, egui::Color32) {
    let (color, warning) = if r2 < 0.5 {
        (COLOR_FIT_LOW(), " (low fit)")
    } else if r2 < 0.8 {
        (COLOR_FIT_MID(), "")
    } else {
        (COLOR_FIT_HIGH(), "")
    };
    let scope = if metric == ImportanceMetric::Ridge {
        " (numerical-only)"
    } else {
        ""
    };
    (format!("R²{scope} = {r2:.3}{warning}"), color)
}

fn group_header(text: &str) -> egui::RichText {
    egui::RichText::new(text).weak().small()
}

pub(super) fn show_unsupported(ui: &mut egui::Ui, names: &[String], no_numeric: bool) {
    for name in names {
        ui.label(format!("{name}: Unsupported (categorical)"));
    }
    if no_numeric && !names.is_empty() {
        ui.label("No supported numerical parameters");
    }
}

/// Retrieves R² for RfAnova / Mdi / Shap / Permutation (first objective's value, for display)
pub(super) fn extract_tree_r_squared(
    result: &SensitivityResult,
    metric: &ImportanceMetric,
) -> Option<f64> {
    match metric {
        ImportanceMetric::RfAnova => result.rf_anova.as_ref()?.r_squared.first().copied(),
        ImportanceMetric::Mdi => result.mdi.as_ref()?.r_squared.first().copied(),
        ImportanceMetric::Shap => result.shap.as_ref()?.r_squared.first().copied(),
        ImportanceMetric::Permutation => result.permutation.as_ref()?.r_squared.first().copied(),
        _ => None,
    }
}

fn extract_tree_importance(
    result: &SensitivityResult,
    metric: &ImportanceMetric,
    obj_idx: usize,
) -> Option<Vec<f64>> {
    let importances = match metric {
        ImportanceMetric::RfAnova => result.rf_anova.as_ref()?.importances.clone(),
        ImportanceMetric::Mdi => result.mdi.as_ref()?.importances.clone(),
        ImportanceMetric::Shap => result.shap.as_ref()?.importances.clone(),
        ImportanceMetric::Permutation => result.permutation.as_ref()?.importances.clone(),
        _ => return None,
    };
    Some(
        importances
            .iter()
            .map(|param_imp| param_imp.get(obj_idx).copied().unwrap_or(f64::NAN).abs())
            .collect(),
    )
}

/// Returns importance scores from a SensitivityResult, sorted in descending order
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
        ImportanceMetric::RfAnova
        | ImportanceMetric::Mdi
        | ImportanceMetric::Shap
        | ImportanceMetric::Permutation => {
            extract_tree_importance(result, metric, obj_idx).unwrap_or_default()
        }
        ImportanceMetric::Ard => result
            .ard
            .as_ref()
            .map(|a| a.importances.clone())
            .unwrap_or_default(),
        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => return vec![],
    };

    if raw_scores.is_empty() {
        return vec![];
    }
    let mut pairs: Vec<(String, f64)> = result
        .param_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            (
                name.clone(),
                raw_scores.get(i).copied().unwrap_or(f64::NAN).abs(),
            )
        })
        .collect();

    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs
}

/// Returns importance scores from a SobolResult, sorted in descending order.
/// Uses the first-order index (SobolFirst) or total-effect index
/// (SobolTotal) depending on the metric.
pub fn compute_sorted_sobol(
    result: &SobolResult,
    obj_idx: usize,
    metric: &ImportanceMetric,
) -> Vec<(String, f64)> {
    let data = match metric {
        ImportanceMetric::SobolFirst => &result.first_order,
        ImportanceMetric::SobolTotal => &result.total_effect,
        _ => return vec![],
    };

    let mut pairs: Vec<(String, f64)> = result
        .param_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let score = data
                .get(i)
                .and_then(|p| p.get(obj_idx))
                .copied()
                .unwrap_or(f64::NAN);
            (name.clone(), score.abs())
        })
        .collect();

    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{MdiResult, RfAnovaResult, RidgeResult, SensitivityResult};

    #[test]
    fn categorical_ui_is_explicit_unranked_and_r_squared_is_numerical_only() {
        for mixed in [false, true] {
            for metric in [ImportanceMetric::Spearman, ImportanceMetric::Ridge] {
                let mut result = make_result_with_ridge(&["x"], vec![0.8]);
                result.unsupported_categorical = vec!["cat".into()];
                if !mixed {
                    result.param_names.clear();
                    result.spearman.clear();
                    result.ridge.clear();
                }
                let sorted = compute_sorted_importance(&result, &metric, 0);
                assert_eq!(sorted.len(), usize::from(mixed));
                assert!(sorted.iter().all(|(name, _)| name == "x"));
                let ctx = egui::Context::default();
                let mut chart = ImportanceChart {
                    metric,
                    ..Default::default()
                };
                let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    chart.show_results(
                        ui,
                        &[ObjectiveImportance::new(
                            0,
                            "obj".into(),
                            metric,
                            Some(&result),
                            None,
                            None,
                        )],
                        &["obj".into()],
                        false,
                    );
                });
                let text = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::epaint::Shape::Text(t) => Some(t.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(text.contains("cat: Unsupported (categorical)"), "{text}");
                assert_eq!(text.contains("No supported numerical parameters"), !mixed);
                assert_eq!(
                    text.contains("R² (numerical-only)"),
                    mixed && metric == ImportanceMetric::Ridge
                );
            }
        }
    }

    #[test]
    fn cache_ids_are_distinct() {
        let metrics = [
            ImportanceMetric::Spearman,
            ImportanceMetric::Ridge,
            ImportanceMetric::RfAnova,
            ImportanceMetric::Mdi,
            ImportanceMetric::Shap,
            ImportanceMetric::Permutation,
            ImportanceMetric::SobolFirst,
            ImportanceMetric::SobolTotal,
            ImportanceMetric::Ard,
        ];
        let ids: Vec<u8> = metrics.iter().map(|m| m.cache_id()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "cache ids must be unique");
    }

    #[test]
    fn metric_signedness() {
        assert!(ImportanceMetric::Spearman.is_signed());
        assert!(ImportanceMetric::Ridge.is_signed());
        assert!(!ImportanceMetric::RfAnova.is_signed());
        assert!(!ImportanceMetric::SobolTotal.is_signed());
    }

    #[test]
    fn metric_expensiveness() {
        // Only correlation/linear metrics are cheap.
        assert!(!ImportanceMetric::Spearman.is_expensive());
        assert!(!ImportanceMetric::Ridge.is_expensive());
        assert!(ImportanceMetric::RfAnova.is_expensive());
        assert!(ImportanceMetric::Mdi.is_expensive());
        assert!(ImportanceMetric::Shap.is_expensive());
        assert!(ImportanceMetric::Permutation.is_expensive());
        assert!(ImportanceMetric::SobolFirst.is_expensive());
        assert!(ImportanceMetric::SobolTotal.is_expensive());
    }

    #[test]
    fn core_metric_none_only_for_sobol() {
        assert!(core_sensitivity_metric(ImportanceMetric::Spearman).is_some());
        assert!(core_sensitivity_metric(ImportanceMetric::Ridge).is_some());
        assert!(core_sensitivity_metric(ImportanceMetric::RfAnova).is_some());
        assert!(core_sensitivity_metric(ImportanceMetric::Mdi).is_some());
        assert!(core_sensitivity_metric(ImportanceMetric::Shap).is_some());
        assert!(core_sensitivity_metric(ImportanceMetric::Permutation).is_some());
        assert!(core_sensitivity_metric(ImportanceMetric::SobolFirst).is_none());
        assert!(core_sensitivity_metric(ImportanceMetric::SobolTotal).is_none());
    }

    #[test]
    fn adopt_compute_state_clears_computing_and_preserves_selection() {
        let mut item = ImportanceChart {
            computing: true,
            metric: ImportanceMetric::Shap,
            objective_index: 3,
            ..Default::default()
        };
        let global = ImportanceChart::default(); // computing=false
        item.adopt_compute_state(&global);
        assert!(!item.computing);
        // Results are aggregated on the app_state side, so selections remain per-item.
        assert_eq!(item.metric, ImportanceMetric::Shap);
        assert_eq!(item.objective_index, 3);
    }

    fn make_result(params: &[&str], scores: Vec<f64>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            unsupported_categorical: vec![],
            spearman: vec![scores],
            ridge: vec![],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
            ard: None,
        }
    }

    fn make_result_with_ridge(params: &[&str], beta: Vec<f64>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            unsupported_categorical: vec![],
            spearman: vec![vec![0.5; params.len()]],
            ridge: vec![RidgeResult {
                beta,
                r_squared: 0.8,
            }],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
            ard: None,
        }
    }

    fn make_result_with_rf_anova(params: &[&str], importances: Vec<Vec<f64>>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            unsupported_categorical: vec![],
            spearman: vec![vec![0.5; params.len()]],
            ridge: vec![],
            rf_anova: Some(RfAnovaResult {
                importances,
                r_squared: vec![0.8],
            }),
            mdi: None,
            shap: None,
            permutation: None,
            ard: None,
        }
    }

    fn make_result_with_mdi(params: &[&str], importances: Vec<Vec<f64>>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
            unsupported_categorical: vec![],
            spearman: vec![vec![0.5; params.len()]],
            ridge: vec![],
            rf_anova: None,
            mdi: Some(MdiResult {
                importances,
                r_squared: vec![0.8],
            }),
            shap: None,
            permutation: None,
            ard: None,
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
        assert_eq!(chart.objective_index, ALL_OBJECTIVES);
        assert!(!chart.feasible_only);
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
    fn sorted_importance_mdi_descending() {
        // 2 params, 1 objective: param0 importance=0.2, param1 importance=0.8
        let result = make_result_with_mdi(&["p0", "p1"], vec![vec![0.2], vec![0.8]]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Mdi, 0);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].0, "p1");
        assert!((sorted[0].1 - 0.8).abs() < 1e-9);
    }

    #[test]
    fn importance_metric_mdi_label() {
        assert_eq!(ImportanceMetric::Mdi.label(), "MDI");
        assert!(!ImportanceMetric::Mdi.is_sobol());
    }

    #[test]
    fn sorted_importance_mdi_no_data_returns_empty() {
        let result = make_result(&["x"], vec![0.5]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Mdi, 0);
        assert!(sorted.is_empty());
    }

    #[test]
    fn sorted_sobol_first_and_total_use_correct_data() {
        use crate::state::app_state::SobolResult;
        let result = SobolResult {
            param_names: vec!["p0".into(), "p1".into()],
            first_order: vec![vec![0.6], vec![0.2]], // [param][objective]
            total_effect: vec![vec![0.8], vec![0.3]],
            r_squared: vec![0.9],
        };
        let first = compute_sorted_sobol(&result, 0, &ImportanceMetric::SobolFirst);
        assert_eq!(first[0].0, "p0");
        assert!((first[0].1 - 0.6).abs() < 1e-9);
        assert_eq!(first[1], ("p1".into(), 0.2));

        let total = compute_sorted_sobol(&result, 0, &ImportanceMetric::SobolTotal);
        assert_eq!(total[0].0, "p0");
        assert!((total[0].1 - 0.8).abs() < 1e-9);
        assert_eq!(total[1], ("p1".into(), 0.3));
    }

    #[test]
    fn test_permutation_is_not_sobol() {
        assert!(!ImportanceMetric::Permutation.is_sobol());
    }

    #[test]
    fn test_permutation_label() {
        assert_eq!(ImportanceMetric::Permutation.label(), "Permutation");
    }

    #[test]
    fn sorted_importance_permutation_descending() {
        use crate::state::app_state::PermutationResult;
        let result = SensitivityResult {
            param_names: vec!["p0".to_string(), "p1".to_string()],
            unsupported_categorical: vec![],
            spearman: vec![],
            ridge: vec![],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: Some(PermutationResult {
                importances: vec![vec![0.2], vec![0.8]],
                r_squared: vec![0.85],
            }),
            ard: None,
        };
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Permutation, 0);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].0, "p1");
        assert!((sorted[0].1 - 0.8).abs() < 1e-9);
    }

    #[test]
    fn sorted_importance_permutation_no_data_returns_empty() {
        let result = make_result(&["x"], vec![0.5]);
        let sorted = compute_sorted_importance(&result, &ImportanceMetric::Permutation, 0);
        assert!(sorted.is_empty());
    }
}
