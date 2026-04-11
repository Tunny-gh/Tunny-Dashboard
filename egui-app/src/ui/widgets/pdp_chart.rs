use crate::state::messages::{PdpResult, PdpResult1d};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum PdpMode {
    OneDim,
    TwoDim,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    Ridge,
    Kriging,
    SparseKriging,
}

impl ModelType {
    pub fn label(&self) -> &'static str {
        match self {
            ModelType::Ridge => "Ridge",
            ModelType::Kriging => "Kriging",
            ModelType::SparseKriging => "Sparse Kriging",
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ModelType::Ridge => "ridge",
            ModelType::Kriging => "kriging",
            ModelType::SparseKriging => "sparse_kriging",
        }
    }
}

/// PDP キャッシュキーを生成する
pub fn cache_key(param: &str, obj_idx: usize, model: &ModelType) -> String {
    format!("{}:{}:{}", param, obj_idx, model.to_str())
}

/// R² 値の品質分類を返す
pub fn r2_quality(r2: f64) -> &'static str {
    if r2 > 0.8 {
        "Good"
    } else if r2 > 0.6 {
        "Fair"
    } else {
        "Poor"
    }
}

/// 信頼区間バンドのポリゴン点列を構築する
/// 上限を左→右、下限を右→左の順で結合する
pub fn compute_band_polygon(
    x_vals: &[f64],
    y_upper: &[f64],
    y_lower: &[f64],
) -> Vec<[f64; 2]> {
    let upper: Vec<[f64; 2]> = x_vals
        .iter()
        .zip(y_upper.iter())
        .map(|(&x, &y)| [x, y])
        .collect();
    let lower: Vec<[f64; 2]> = x_vals
        .iter()
        .zip(y_lower.iter())
        .rev()
        .map(|(&x, &y)| [x, y])
        .collect();
    upper.into_iter().chain(lower).collect()
}

/// PDP チャートウィジェット
pub struct PdpChart {
    pub mode: PdpMode,
    pub selected_param: String,
    pub selected_objective: usize,
    pub model_type: ModelType,
    pub result: Option<PdpResult>,
    pub computing: bool,
    pub cache: HashMap<String, PdpResult1d>,
}

impl Default for PdpChart {
    fn default() -> Self {
        Self {
            mode: PdpMode::OneDim,
            selected_param: String::new(),
            selected_objective: 0,
            model_type: ModelType::Ridge,
            result: None,
            computing: false,
            cache: HashMap::new(),
        }
    }
}

impl PdpChart {
    /// キャッシュを確認して結果を返す。キャッシュミスの場合は None を返す
    pub fn try_cache(&self) -> Option<&PdpResult1d> {
        let key = cache_key(&self.selected_param, self.selected_objective, &self.model_type);
        self.cache.get(&key)
    }

    /// キャッシュに結果を挿入する
    pub fn insert_cache(&mut self, param: &str, obj_idx: usize, result: PdpResult1d) {
        let key = cache_key(param, obj_idx, &self.model_type);
        self.cache.insert(key, result);
    }
}

impl PdpChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        param_names: &[String],
        obj_names: &[String],
    ) {
        // パラメータ選択
        ui.horizontal(|ui| {
            ui.label("Parameter:");
            egui::ComboBox::from_id_salt("pdp_param_combo")
                .selected_text(&self.selected_param)
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.selected_param, name.clone(), name);
                    }
                });
            ui.label("Objective:");
            if let Some(obj_name) = obj_names.get(self.selected_objective) {
                egui::ComboBox::from_id_salt("pdp_obj_combo")
                    .selected_text(obj_name)
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            if ui
                                .selectable_label(self.selected_objective == i, name)
                                .clicked()
                            {
                                self.selected_objective = i;
                            }
                        }
                    });
            }
            // モデル選択
            ui.label("Model:");
            egui::ComboBox::from_id_salt("pdp_model_combo")
                .selected_text(self.model_type.label())
                .show_ui(ui, |ui| {
                    for model in [ModelType::Ridge, ModelType::Kriging, ModelType::SparseKriging] {
                        let selected = self.model_type == model;
                        if ui.selectable_label(selected, model.label()).clicked() {
                            self.model_type = model;
                        }
                    }
                });
        });

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing PDP...");
            });
            return;
        }

        let Some(ref result) = self.result else {
            ui.label("No PDP data");
            return;
        };

        match result {
            PdpResult::OneDim(r) => self.show_1d(ui, r),
            PdpResult::TwoDim(_) => {
                ui.label("2D PDP will be implemented in TASK-2017");
            }
        }
    }

    fn show_1d(&self, ui: &mut egui::Ui, result: &PdpResult1d) {
        // R² 表示
        if let Some(r2) = result.r2 {
            ui.label(format!("R²: {:.2} ({})", r2, r2_quality(r2)));
        }

        egui_plot::Plot::new("pdp_1d_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                // 信頼区間バンド
                if let (Some(upper), Some(lower)) = (&result.y_upper, &result.y_lower) {
                    let band = compute_band_polygon(&result.x_values, upper, lower);
                    if !band.is_empty() {
                        plot_ui.polygon(
                            egui_plot::Polygon::new(
                                egui_plot::PlotPoints::new(band),
                            )
                            .fill_color(egui::Color32::from_rgba_unmultiplied(
                                100, 100, 255, 40,
                            )),
                        );
                    }
                }

                // ICE ライン
                for ice in &result.ice_lines {
                    let pts: egui_plot::PlotPoints = result
                        .x_values
                        .iter()
                        .zip(ice.iter())
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    plot_ui.line(
                        egui_plot::Line::new(pts)
                            .width(0.5)
                            .color(egui::Color32::from_rgba_unmultiplied(150, 150, 150, 60)),
                    );
                }

                // PDP 平均曲線
                let main_pts: egui_plot::PlotPoints = result
                    .x_values
                    .iter()
                    .zip(result.y_values.iter())
                    .map(|(&x, &y)| [x, y])
                    .collect();
                plot_ui.line(
                    egui_plot::Line::new(main_pts)
                        .name("PDP")
                        .width(2.0)
                        .color(egui::Color32::from_rgb(50, 100, 255)),
                );
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r2_quality_good_above_0_8() {
        assert_eq!(r2_quality(0.9), "Good");
        assert_eq!(r2_quality(0.81), "Good");
    }

    #[test]
    fn r2_quality_fair_between_0_6_and_0_8() {
        assert_eq!(r2_quality(0.7), "Fair");
        assert_eq!(r2_quality(0.61), "Fair");
    }

    #[test]
    fn r2_quality_poor_at_or_below_0_6() {
        assert_eq!(r2_quality(0.6), "Poor");
        assert_eq!(r2_quality(0.0), "Poor");
        assert_eq!(r2_quality(-0.5), "Poor");
    }

    #[test]
    fn band_polygon_upper_then_lower_reversed() {
        let x = vec![0.0, 1.0, 2.0];
        let upper = vec![3.0, 4.0, 5.0];
        let lower = vec![1.0, 2.0, 3.0];
        let pts = compute_band_polygon(&x, &upper, &lower);
        // 6 points total: 3 upper (l→r) + 3 lower (r→l)
        assert_eq!(pts.len(), 6);
        assert_eq!(pts[0], [0.0, 3.0]); // first upper
        assert_eq!(pts[2], [2.0, 5.0]); // last upper
        assert_eq!(pts[3], [2.0, 3.0]); // first lower (reversed: rightmost)
        assert_eq!(pts[5], [0.0, 1.0]); // last lower (leftmost)
    }

    #[test]
    fn band_polygon_upper_always_gte_lower_for_valid_input() {
        let x = vec![0.0, 0.5, 1.0];
        let upper = vec![2.0, 3.0, 4.0];
        let lower = vec![0.0, 1.0, 2.0];
        let pts = compute_band_polygon(&x, &upper, &lower);
        // Upper points (index 0..3) should have higher y than lower points (index 3..6 reversed)
        for i in 0..3 {
            assert!(pts[i][1] >= pts[5 - i][1]);
        }
    }

    #[test]
    fn pdp_chart_default() {
        let chart = PdpChart::default();
        assert_eq!(chart.mode, PdpMode::OneDim);
        assert_eq!(chart.model_type, ModelType::Ridge);
        assert!(!chart.computing);
        assert!(chart.result.is_none());
        assert!(chart.cache.is_empty());
    }

    // TASK-2025 tests

    #[test]
    fn cache_key_same_inputs_produce_same_key() {
        let k1 = cache_key("x", 0, &ModelType::Ridge);
        let k2 = cache_key("x", 0, &ModelType::Ridge);
        assert_eq!(k1, k2);
    }

    #[test]
    fn cache_key_different_model_produces_different_key() {
        let k1 = cache_key("x", 0, &ModelType::Ridge);
        let k2 = cache_key("x", 0, &ModelType::Kriging);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_different_param_produces_different_key() {
        let k1 = cache_key("x", 0, &ModelType::Ridge);
        let k2 = cache_key("y", 0, &ModelType::Ridge);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_hit_returns_result() {
        let mut chart = PdpChart::default();
        chart.selected_param = "x".to_string();
        chart.selected_objective = 0;
        let result = PdpResult1d {
            x_values: vec![0.0, 1.0],
            y_values: vec![0.5, 1.5],
            y_upper: None,
            y_lower: None,
            ice_lines: vec![],
            r2: None,
            param_name: "x".to_string(),
            objective_name: "obj0".to_string(),
        };
        chart.insert_cache("x", 0, result);
        assert!(chart.try_cache().is_some());
    }

    #[test]
    fn cache_miss_returns_none() {
        let chart = PdpChart::default();
        assert!(chart.try_cache().is_none());
    }
}
