use crate::state::messages::{PdpResult, PdpResult1d};
use crate::state::types::StudyView;
use crate::theme::chart_colors::{
    COLOR_ICE_LINE, COLOR_PARETO, COLOR_PDP_CI, COLOR_PDP_CI_LEGEND, COLOR_PDP_LINE,
};
use std::collections::HashMap;

/// 1D PDP 計算リクエスト（show() がセットし chart_registry が消費する）
pub struct PdpComputeRequest {
    pub param: String,
    pub objective: String,
    pub n_grid: usize,
    pub model_type: String,
}

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
    RandomForest,
}

impl ModelType {
    pub const ALL: [ModelType; 4] = [
        ModelType::Ridge,
        ModelType::Kriging,
        ModelType::SparseKriging,
        ModelType::RandomForest,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ModelType::Ridge => "Ridge",
            ModelType::Kriging => "Kriging",
            ModelType::SparseKriging => "Sparse Kriging",
            ModelType::RandomForest => "Random Forest (LightGBM)",
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ModelType::Ridge => "ridge",
            ModelType::Kriging => "kriging",
            ModelType::SparseKriging => "sparse_kriging",
            ModelType::RandomForest => "random_forest",
        }
    }
}

/// PDP キャッシュキーを生成する
pub fn cache_key(param: &str, objective: &str, model_type_str: &str) -> String {
    format!("{}:{}:{}", param, objective, model_type_str)
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
pub fn compute_band_polygon(x_vals: &[f64], y_upper: &[f64], y_lower: &[f64]) -> Vec<[f64; 2]> {
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

/// view + 選択インデックスから観測データを抽出する（テスト可能な純粋関数）
///
/// `selected_indices` が空の場合は全試行を対象とする（filter_rows_for_display と同様の規則）。
/// `selected_indices` / `pinned` のどちらかに trial_id が含まれる行のみを抽出する。
/// NaN / Inf の値はスキップする。
pub fn extract_observed(
    view: &StudyView,
    obj_names: &[String],
    param_name: &str,
    obj_idx: usize,
    selected_indices: &[u32],
    pinned: &[u32],
) -> Vec<[f64; 2]> {
    let param_col = view.numeric_column(param_name);
    let obj_col = obj_names
        .get(obj_idx)
        .and_then(|name| view.numeric_column(name));

    let (Some(params), Some(objs)) = (param_col, obj_col) else {
        return vec![];
    };

    let use_filter = !selected_indices.is_empty();
    let selected_set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
    let pinned_set: std::collections::HashSet<u32> = pinned.iter().copied().collect();

    (0..view.row_count())
        .filter_map(|i| {
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            if use_filter && !selected_set.contains(&trial_id) && !pinned_set.contains(&trial_id) {
                return None;
            }
            let x = params.get(i).copied()?;
            let y = objs.get(i).copied()?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            Some([x, y])
        })
        .collect()
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
    pub show_observed: bool,
    pub pending_compute: Option<PdpComputeRequest>,
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
            show_observed: false,
            pending_compute: None,
        }
    }
}

impl PdpChart {
    /// キャッシュを確認して結果を返す。キャッシュミスの場合は None を返す
    pub fn try_cache(&self, objective: &str) -> Option<&PdpResult1d> {
        let key = cache_key(&self.selected_param, objective, self.model_type.to_str());
        self.cache.get(&key)
    }

    /// キャッシュに結果を挿入する
    pub fn insert_cache(
        &mut self,
        param: &str,
        objective: &str,
        model_type_str: &str,
        result: PdpResult1d,
    ) {
        let key = cache_key(param, objective, model_type_str);
        self.cache.insert(key, result);
    }
}

impl PdpChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        param_names: &[String],
        obj_names: &[String],
        view: &StudyView,
        selected_indices: &[u32],
        pinned: &[u32],
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
                    for model in ModelType::ALL {
                        let selected = self.model_type == model;
                        if ui.selectable_label(selected, model.label()).clicked() {
                            self.model_type = model;
                        }
                    }
                });
            // 観測データ表示トグル
            ui.separator();
            ui.toggle_value(&mut self.show_observed, "Show data");

            // Run ボタン
            ui.separator();
            let can_run =
                !self.selected_param.is_empty() && !obj_names.is_empty() && !self.computing;
            if ui
                .add_enabled(can_run, egui::Button::new("Run PDP"))
                .clicked()
            {
                if let Some(obj_name) = obj_names.get(self.selected_objective) {
                    // キャッシュヒットの場合は再計算せずにキャッシュから結果を取得
                    let cache_key_str =
                        cache_key(&self.selected_param, obj_name, self.model_type.to_str());
                    if let Some(cached) = self.cache.get(&cache_key_str) {
                        self.result = Some(PdpResult::OneDim(cached.clone()));
                    } else {
                        let n_grid = match self.model_type {
                            ModelType::Ridge => 50,
                            ModelType::RandomForest => 30,
                            _ => 30, // Kriging is O(N²×grid); 30 keeps debug builds fast
                        };
                        self.pending_compute = Some(PdpComputeRequest {
                            param: self.selected_param.clone(),
                            objective: obj_name.clone(),
                            n_grid,
                            model_type: self.model_type.to_str().to_string(),
                        });
                    }
                }
            }
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

        // 観測データを事前計算（show_observed == false のときはゼロコスト）
        let observed = if self.show_observed {
            extract_observed(
                view,
                obj_names,
                &self.selected_param,
                self.selected_objective,
                selected_indices,
                pinned,
            )
        } else {
            vec![]
        };

        match result {
            PdpResult::OneDim(r) => self.show_1d(ui, r, &observed),
            PdpResult::TwoDim(_) => {
                ui.label("2D PDP will be implemented in TASK-2017");
            }
        }
    }

    fn show_1d(&self, ui: &mut egui::Ui, result: &PdpResult1d, observed: &[[f64; 2]]) {
        // R² 表示
        if let Some(r2) = result.r2 {
            ui.label(format!("R²: {:.2} ({})", r2, r2_quality(r2)));
        }

        egui_plot::Plot::new("pdp_1d_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                // 信頼区間バンド（グリッド区間ごとに凸四辺形を描画）
                // egui_plot::Polygon はファン三角分割を使うため一枚の非凸ポリゴンでは
                // 描画が崩れる。区間ごとの凸四辺形に分割することで正確に描画できる。
                if let (Some(upper), Some(lower)) = (&result.y_upper, &result.y_lower) {
                    let fill = COLOR_PDP_CI;
                    let xs = &result.x_values;
                    let n = xs.len();
                    for i in 0..n.saturating_sub(1) {
                        let quad = vec![
                            [xs[i], upper[i]],
                            [xs[i + 1], upper[i + 1]],
                            [xs[i + 1], lower[i + 1]],
                            [xs[i], lower[i]],
                        ];
                        plot_ui.polygon(
                            egui_plot::Polygon::new(egui_plot::PlotPoints::new(quad))
                                .fill_color(fill)
                                .stroke(egui::Stroke::NONE),
                        );
                    }
                }
                // 凡例エントリ（透明な点でラベルのみ表示）
                if result.y_upper.is_some() {
                    plot_ui.points(
                        egui_plot::Points::new(vec![[f64::NAN, f64::NAN]])
                            .name("95% CI")
                            .color(COLOR_PDP_CI_LEGEND)
                            .radius(6.0),
                    );
                }

                // ICE ライン
                for ice in &result.ice_lines {
                    let pts: egui_plot::PlotPoints = result
                        .x_values
                        .iter()
                        .zip(ice.iter())
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    plot_ui.line(egui_plot::Line::new(pts).width(0.5).color(COLOR_ICE_LINE));
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
                        .color(COLOR_PDP_LINE),
                );

                // 観測データ散布図（最前面）
                if self.show_observed && !observed.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(observed.to_vec())
                            .name("Observed")
                            .color(COLOR_PARETO)
                            .radius(4.0),
                    );
                }
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

    // TASK-2062 tests

    #[test]
    fn pdp_chart_default_show_observed_false() {
        let chart = PdpChart::default();
        assert!(!chart.show_observed);
    }

    fn make_view_xobj(x_vals: &[f64], y_vals: &[f64]) -> (StudyView, Vec<String>) {
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
        let n = x_vals.len();
        let obj_names = vec!["obj0".to_string()];
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                param_display: [("x".to_string(), x_vals[i])].into(),
                param_category_label: HashMap::new(),
                objective_values: vec![y_vals[i]],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &["x".to_string()], &obj_names, &[], &[], 0);
        (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
    }

    #[test]
    fn extract_observed_normal() {
        let (view, obj_names) = make_view_xobj(&[1.5], &[2.0]);
        let pts = extract_observed(&view, &obj_names, "x", 0, &[], &[]);
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0], [1.5, 2.0]);
    }

    #[test]
    fn extract_observed_missing_param() {
        let (view, obj_names) = make_view_xobj(&[1.5], &[2.0]);
        // "y" は view に存在しない → 空
        let pts = extract_observed(&view, &obj_names, "y", 0, &[], &[]);
        assert!(pts.is_empty());
    }

    #[test]
    fn extract_observed_out_of_range_obj() {
        let (view, obj_names) = make_view_xobj(&[1.5], &[2.0]);
        // obj_idx=5 は範囲外 → 空
        let pts = extract_observed(&view, &obj_names, "x", 5, &[], &[]);
        assert!(pts.is_empty());
    }

    // TASK-2025 tests

    #[test]
    fn cache_key_same_inputs_produce_same_key() {
        let k1 = cache_key("x", "obj0", "Ridge");
        let k2 = cache_key("x", "obj0", "Ridge");
        assert_eq!(k1, k2);
    }

    #[test]
    fn cache_key_different_model_produces_different_key() {
        let k1 = cache_key("x", "obj0", "Ridge");
        let k2 = cache_key("x", "obj0", "Kriging");
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_different_param_produces_different_key() {
        let k1 = cache_key("x", "obj0", "Ridge");
        let k2 = cache_key("y", "obj0", "Ridge");
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_hit_returns_result() {
        let mut chart = PdpChart {
            selected_param: "x".to_string(),
            selected_objective: 0,
            ..Default::default()
        };
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
        chart.insert_cache("x", "obj0", "ridge", result);
        assert!(chart.try_cache("obj0").is_some());
    }

    #[test]
    fn cache_miss_returns_none() {
        let chart = PdpChart::default();
        assert!(chart.try_cache("obj0").is_none());
    }

    // ── TASK-2237: PDP observed overlay 選択連動テスト ──────────

    #[test]
    fn pdp_overlay_uses_filtered_rows_when_selection_exists() {
        let (view, obj_names) = make_view_xobj(&[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]);
        let selected = vec![0u32, 1];
        let pts = extract_observed(&view, &obj_names, "x", 0, &selected, &[]);
        assert_eq!(pts.len(), 2);
        let xs: Vec<f64> = pts.iter().map(|p| p[0]).collect();
        assert!(xs.contains(&1.0));
        assert!(xs.contains(&2.0));
        assert!(!xs.contains(&3.0));
    }

    #[test]
    fn pdp_overlay_falls_back_to_all_rows_without_selection() {
        let (view, obj_names) = make_view_xobj(&[1.0, 2.0], &[2.0, 3.0]);
        let pts = extract_observed(&view, &obj_names, "x", 0, &[], &[]);
        assert_eq!(pts.len(), 2, "all rows returned when no selection");
    }

    #[test]
    fn pinned_row_remains_in_observed_overlay() {
        let (view, obj_names) = make_view_xobj(&[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]);
        let pts = extract_observed(&view, &obj_names, "x", 0, &[0], &[2]);
        let xs: Vec<f64> = pts.iter().map(|p| p[0]).collect();
        assert!(xs.contains(&1.0), "selected row must be visible");
        assert!(xs.contains(&3.0), "pinned row must remain visible");
        assert!(!xs.contains(&2.0), "unselected unpinned row must be hidden");
    }
}
