use crate::state::app_state::{SensitivityResult, SobolResult};
use crate::theme::chart_colors::{
    COLOR_FIT_HIGH, COLOR_FIT_LOW, COLOR_FIT_MID, COLOR_IMPORTANCE_BAR,
};

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

    /// 値が符号を持つ（負の相関・負の係数があり得る）か。Spearman / Ridge のみ符号付きで、
    /// 木ベース・Sobol は非負。Sensitivity Heatmap の発散/逐次カラーマップ切替に使う。
    pub fn is_signed(&self) -> bool {
        matches!(self, ImportanceMetric::Spearman | ImportanceMetric::Ridge)
    }

    /// モデル訓練（木ベース）や Sobol サンプリングを伴い計算コストが高いか。
    /// 低コストなのは相関/線形（Spearman・Ridge）のみ。
    pub fn is_expensive(&self) -> bool {
        !matches!(self, ImportanceMetric::Spearman | ImportanceMetric::Ridge)
    }
}

/// Sobol 指数推定のサンプル数（ImportanceChart / Sensitivity Heatmap 共通）。
pub const SOBOL_SAMPLE_COUNT: usize = 1024;

/// `ImportanceMetric` から対応するコア感度メトリクスを生成する。
/// Sobol は専用の `compute_sobol_from_df` 経路を使うため `None` を返す。
/// ImportanceChart と Sensitivity Heatmap が同じ対応表を共有するためのヘルパー。
pub fn core_sensitivity_metric(
    metric: ImportanceMetric,
) -> Option<Box<dyn tunny_core::sensitivity::SensitivityMetric>> {
    use tunny_core::sensitivity::{
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

/// 感度分析バーチャートウィジェット
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ImportanceChart {
    pub metric: ImportanceMetric,
    #[serde(skip)]
    pub computing: bool,
    pub objective_index: usize,
    /// 実行可能解のみでモデルをフィットするか（制約付きスタディのみ UI 表示）
    pub feasible_only: bool,
    /// 計算要求 (手法, 目的 idx, feasible_only)。poll_chart が消費する。
    #[serde(skip)]
    pub pending_compute: Option<(ImportanceMetric, usize, bool)>,
}

impl Default for ImportanceChart {
    fn default() -> Self {
        Self {
            metric: ImportanceMetric::Spearman,
            computing: false,
            objective_index: 0,
            feasible_only: false,
            pending_compute: None,
        }
    }
}

impl ImportanceChart {
    /// グローバル widget の計算実行状態（computing/pending）を取り込む。
    /// 結果は `app_state.importance_cache` / `sobol_cache` に集約されるため、
    /// キャンバスの各アイテム（独立した WidgetStates）には実行状態のみ反映すればよい。
    /// メトリクス・目的関数の選択はアイテム固有なので維持する。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        sensitivity: Option<&SensitivityResult>,
        sobol: Option<&SobolResult>,
        obj_names: &[String],
        has_constraints: bool,
    ) {
        // Run ボタン + メトリクスコンボボックス + 目的関数コンボボックス + spinner + R²（右端）
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
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
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.objective_index, i, name);
                        }
                    });
            }

            // 実行可能解フィルタ（制約付きスタディのみ）
            if has_constraints {
                ui.toggle_value(&mut self.feasible_only, "Feasible only")
                    .on_hover_text("Fit the model using feasible trials only");
            }

            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }

            // R² を右端に表示（Spearman 以外）
            let r2_opt: Option<f64> = match self.metric {
                ImportanceMetric::Spearman => None,
                ImportanceMetric::Ridge => sensitivity
                    .and_then(|r| r.ridge.first())
                    .map(|ridge| ridge.r_squared),
                ImportanceMetric::RfAnova
                | ImportanceMetric::Mdi
                | ImportanceMetric::Shap
                | ImportanceMetric::Permutation => {
                    sensitivity.and_then(|r| extract_tree_r_squared(r, &self.metric))
                }
                ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => {
                    sobol.and_then(|s| s.r_squared.first()).copied()
                }
                ImportanceMetric::Ard => sensitivity
                    .and_then(|r| r.ard.as_ref())
                    .map(|a| a.r_squared),
            };
            if let Some(r2) = r2_opt {
                let (color, warning) = if r2 < 0.5 {
                    (COLOR_FIT_LOW, " (low fit)")
                } else if r2 < 0.8 {
                    (COLOR_FIT_MID, "")
                } else {
                    (COLOR_FIT_HIGH, "")
                };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("R² = {r2:.3}{warning}")).color(color));
                });
            }
        });

        if self.computing {
            return;
        }

        let scores = match self.metric {
            ImportanceMetric::Spearman
            | ImportanceMetric::Ridge
            | ImportanceMetric::RfAnova
            | ImportanceMetric::Mdi
            | ImportanceMetric::Shap
            | ImportanceMetric::Permutation
            | ImportanceMetric::Ard => {
                let Some(result) = sensitivity else {
                    ui.label("No sensitivity data (start the computation first)");
                    return;
                };
                compute_sorted_importance(result, &self.metric, 0)
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

        let bar_color = COLOR_IMPORTANCE_BAR;
        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width() - label_width - value_text_width - 8.0;
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

fn group_header(text: &str) -> egui::RichText {
    egui::RichText::new(text).weak().small()
}

/// RfAnova / Mdi / Shap / Permutation の R² を取得する（表示用の最初の目的関数値）
fn extract_tree_r_squared(result: &SensitivityResult, metric: &ImportanceMetric) -> Option<f64> {
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
            .map(|param_imp| param_imp.get(obj_idx).copied().unwrap_or(0.0).abs())
            .collect(),
    )
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
    let data = match metric {
        ImportanceMetric::SobolFirst => &result.first_order,
        ImportanceMetric::SobolTotal => &result.total_effect,
        _ => return vec![],
    };

    let mut pairs: Vec<(String, f64)> = result
        .param_names
        .iter()
        .zip(data.iter())
        .map(|(name, param_scores)| {
            let score = param_scores.get(obj_idx).copied().unwrap_or(0.0);
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
        // 低コストは相関/線形のみ。
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
        // 結果は app_state 側に集約されるため、選択はアイテム固有で維持される。
        assert_eq!(item.metric, ImportanceMetric::Shap);
        assert_eq!(item.objective_index, 3);
    }

    fn make_result(params: &[&str], scores: Vec<f64>) -> SensitivityResult {
        SensitivityResult {
            param_names: params.iter().map(|s| s.to_string()).collect(),
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
        assert_eq!(chart.objective_index, 0);
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
            first_order: vec![vec![0.6, 0.2]], // [obj][param]
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

    #[test]
    fn test_permutation_cache_id() {
        assert_eq!(ImportanceMetric::Permutation.cache_id(), 7);
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
