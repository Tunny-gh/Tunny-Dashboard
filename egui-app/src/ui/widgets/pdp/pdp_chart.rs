use crate::state::messages::PdpResult1d;
use crate::state::types::StudyView;
use crate::theme::chart_colors::{
    COLOR_ICE_LINE, COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_PDP_CI,
    COLOR_PDP_CI_LEGEND, COLOR_PDP_LINE,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use std::collections::HashMap;

/// Classification of an observed point (colored using the same scheme as the scatter
/// chart family). Shared by the observed-data overlays of 1D / 2D PDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedKind {
    /// Pareto front (pareto_rank == 0) -> red
    Pareto,
    /// Non-Pareto feasible solution -> blue
    NonPareto,
    /// Infeasible solution -> gray
    Infeasible,
}

impl ObservedKind {
    pub fn color(self) -> egui::Color32 {
        match self {
            ObservedKind::Pareto => COLOR_PARETO(),
            ObservedKind::NonPareto => COLOR_NON_PARETO(),
            ObservedKind::Infeasible => COLOR_INFEASIBLE(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ObservedKind::Pareto => "Pareto",
            ObservedKind::NonPareto => "Non-Pareto",
            ObservedKind::Infeasible => "Infeasible",
        }
    }

    pub const ALL: [ObservedKind; 3] = [
        ObservedKind::Pareto,
        ObservedKind::NonPareto,
        ObservedKind::Infeasible,
    ];
}

/// Returns the observed-point classification from feasibility and Pareto rank (same
/// rule as the other scatter plots)
pub fn classify_observed(feasible: bool, pareto_rank: u32) -> ObservedKind {
    if !feasible {
        ObservedKind::Infeasible
    } else if pareto_rank == 0 {
        ObservedKind::Pareto
    } else {
        ObservedKind::NonPareto
    }
}

/// 1D PDP computation request (set by show() and consumed by chart_registry)
pub struct PdpComputeRequest {
    pub param: String,
    pub objective: String,
    pub n_grid: usize,
    pub model_type: String,
    /// Whether to fit the model using only feasible trials (is_feasible > 0.5)
    pub feasible_only: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PdpMode {
    OneDim,
    TwoDim,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ModelType {
    Ridge,
    GpFitc,
    GpVfe,
    GpMoe,
    RandomForest,
}

impl ModelType {
    pub const ALL: [ModelType; 5] = [
        ModelType::Ridge,
        ModelType::GpFitc,
        ModelType::GpVfe,
        ModelType::GpMoe,
        ModelType::RandomForest,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ModelType::Ridge => "Ridge",
            ModelType::GpFitc => "GP-FITC",
            ModelType::GpVfe => "GP-VFE",
            ModelType::GpMoe => "GP-MOE",
            ModelType::RandomForest => "Random Forest (LightGBM)",
        }
    }

    /// Cache key string — must match the `model_type` strings accepted by
    /// `pdp/api.rs`: "ridge", "gp_fitc", "gp_vfe", "gp_moe", "random_forest".
    pub fn to_str(&self) -> &'static str {
        match self {
            ModelType::Ridge => "ridge",
            ModelType::GpFitc => "gp_fitc",
            ModelType::GpVfe => "gp_vfe",
            ModelType::GpMoe => "gp_moe",
            ModelType::RandomForest => "random_forest",
        }
    }
}

/// Generates a PDP cache key
pub fn cache_key(
    param: &str,
    objective: &str,
    model_type_str: &str,
    feasible_only: bool,
) -> String {
    format!(
        "{}:{}:{}:{}",
        param, objective, model_type_str, feasible_only
    )
}

/// Returns a quality classification for the R² value
pub fn r2_quality(r2: f64) -> &'static str {
    if r2 > 0.8 {
        "Good"
    } else if r2 > 0.6 {
        "Fair"
    } else {
        "Poor"
    }
}

/// Extracts observed data ([param, objective], classification) from the view + selected
/// indices (a testable pure function)
///
/// If `selected_indices` is empty, all trials are targeted.
/// Only rows whose trial_id is included in `selected_indices` or `pinned` are extracted.
/// NaN / Inf values are skipped.
/// Classification follows the same rule as the other scatter plots (pareto_rank == 0 ->
/// Pareto, is_feasible <= 0.5 -> Infeasible).
pub fn extract_observed(
    view: &StudyView,
    obj_names: &[String],
    param_name: &str,
    obj_idx: usize,
    selected_indices: &[u32],
    pinned: &[u32],
) -> Vec<([f64; 2], ObservedKind)> {
    let param_col = view.numeric_column(param_name);
    let obj_col = obj_names
        .get(obj_idx)
        .and_then(|name| view.numeric_column(name));

    let (Some(params), Some(objs)) = (param_col, obj_col) else {
        return vec![];
    };
    let feas = view.feasibility();

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
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            Some(([x, y], classify_observed(feas.is_feasible(i), rank)))
        })
        .collect()
}

/// PDP chart widget
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PdpChart {
    pub mode: PdpMode,
    pub selected_param: String,
    pub selected_objective: usize,
    pub model_type: ModelType,
    #[serde(skip)]
    pub result: Option<PdpResult1d>,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub cache: HashMap<String, PdpResult1d>,
    pub show_observed: bool,
    /// Whether to fit the model using only feasible trials (UI shown only for constrained studies)
    pub feasible_only: bool,
    #[serde(skip)]
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
            feasible_only: false,
            pending_compute: None,
        }
    }
}

impl PdpChart {
    /// Adopts the computing state, result, and cache from the global widget.
    /// The PDP result is held on the widget side (result/cache), so unless it is also
    /// reflected onto each canvas item (independent WidgetStates), it stays stuck at
    /// "No PDP data" even after completion. The parameter, objective, model, etc.
    /// selections are item-specific and are preserved.
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.result = src.result.clone();
        self.cache = src.cache.clone();
    }

    /// Inserts a result into the cache
    pub fn insert_cache(
        &mut self,
        param: &str,
        objective: &str,
        model_type_str: &str,
        feasible_only: bool,
        result: PdpResult1d,
    ) {
        let key = cache_key(param, objective, model_type_str, feasible_only);
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
        // Parameter selection
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
            // Model selection
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
            // Toggle to show observed data
            ui.separator();
            ui.toggle_value(&mut self.show_observed, "Show data");

            // Feasible-only filter (constrained studies only)
            if view.feasibility().has_constraints() {
                ui.toggle_value(&mut self.feasible_only, "Feasible only")
                    .on_hover_text("Fit the model using feasible trials only");
            }

            // Run button
            ui.separator();
            let can_run =
                !self.selected_param.is_empty() && !obj_names.is_empty() && !self.computing;
            if ui
                .add_enabled(can_run, egui::Button::new("Run PDP"))
                .clicked()
            {
                if let Some(obj_name) = obj_names.get(self.selected_objective) {
                    // On a cache hit, fetch the result from the cache instead of recomputing
                    let cache_key_str = cache_key(
                        &self.selected_param,
                        obj_name,
                        self.model_type.to_str(),
                        self.feasible_only,
                    );
                    if let Some(cached) = self.cache.get(&cache_key_str) {
                        self.result = Some(cached.clone());
                    } else {
                        let n_grid = match self.model_type {
                            ModelType::Ridge => 50,
                            ModelType::RandomForest => 30,
                            _ => 30, // GP methods are O(N²×grid); 30 keeps debug builds fast
                        };
                        self.pending_compute = Some(PdpComputeRequest {
                            param: self.selected_param.clone(),
                            objective: obj_name.clone(),
                            n_grid,
                            model_type: self.model_type.to_str().to_string(),
                            feasible_only: self.feasible_only,
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

        // Pre-compute observed data (zero cost when show_observed == false)
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

        self.show_1d(ui, result, &observed);
    }

    fn show_1d(
        &self,
        ui: &mut egui::Ui,
        result: &PdpResult1d,
        observed: &[([f64; 2], ObservedKind)],
    ) {
        // Display R²
        if let Some(r2) = result.r2 {
            ui.label(format!("R²: {:.2} ({})", r2, r2_quality(r2)));
        }

        egui_plot::Plot::new("pdp_1d_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                // Confidence-interval band (drawn as a convex quad per grid interval).
                // egui_plot::Polygon uses fan triangulation, so a single non-convex
                // polygon would render incorrectly; splitting into a convex quad per
                // interval renders it accurately.
                if let (Some(upper), Some(lower)) = (&result.y_upper, &result.y_lower) {
                    let fill = COLOR_PDP_CI();
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
                            egui_plot::Polygon::new("", egui_plot::PlotPoints::new(quad))
                                .fill_color(fill)
                                .stroke(egui::Stroke::NONE),
                        );
                    }
                }
                // Legend entry (a transparent point used to show only the label)
                if result.y_upper.is_some() {
                    plot_ui.points(
                        egui_plot::Points::new("95% CI", vec![[f64::NAN, f64::NAN]])
                            .color(COLOR_PDP_CI_LEGEND())
                            .radius(6.0),
                    );
                }

                // ICE lines
                for ice in &result.ice_lines {
                    let pts: egui_plot::PlotPoints = result
                        .x_values
                        .iter()
                        .zip(ice.iter())
                        .map(|(&x, &y)| [x, y])
                        .collect();
                    plot_ui.line(
                        egui_plot::Line::new("", pts)
                            .width(0.5)
                            .color(COLOR_ICE_LINE()),
                    );
                }

                // PDP mean curve
                let main_pts: egui_plot::PlotPoints = result
                    .x_values
                    .iter()
                    .zip(result.y_values.iter())
                    .map(|(&x, &y)| [x, y])
                    .collect();
                plot_ui.line(
                    egui_plot::Line::new("PDP", main_pts)
                        .width(2.0)
                        .color(COLOR_PDP_LINE()),
                );

                // Observed-data scatter (frontmost). Drawn per classification with the
                // same coloring as the other scatter plots
                if self.show_observed && !observed.is_empty() {
                    for kind in ObservedKind::ALL {
                        let pts: Vec<[f64; 2]> = observed
                            .iter()
                            .filter(|(_, k)| *k == kind)
                            .map(|(p, _)| *p)
                            .collect();
                        if pts.is_empty() {
                            continue;
                        }
                        plot_ui.points(
                            egui_plot::Points::new(kind.label(), pts)
                                .color(kind.color())
                                .radius(4.0),
                        );
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adopt_compute_state_propagates_result_and_clears_computing() {
        // A canvas item left with computing=true from Run should clear its spinner and
        // draw the result once it adopts the global side's completed result
        // (regression guard against the "not drawn" bug).
        let mut item = PdpChart {
            computing: true,
            selected_param: "x1".to_string(),
            selected_objective: 2,
            ..Default::default()
        };
        let mut global = PdpChart {
            computing: false,
            ..Default::default()
        };
        global.insert_cache(
            "x0",
            "obj0",
            ModelType::Ridge.to_str(),
            false,
            PdpResult1d {
                x_values: vec![0.0, 1.0],
                y_values: vec![1.0, 2.0],
                y_upper: None,
                y_lower: None,
                ice_lines: vec![],
                r2: Some(0.9),
                param_name: "x0".to_string(),
            },
        );
        global.result = Some(global.cache.values().next().unwrap().clone());

        item.adopt_compute_state(&global);

        assert!(!item.computing);
        assert!(item.result.is_some());
        assert_eq!(item.cache.len(), 1);
        // Item-specific selections are preserved.
        assert_eq!(item.selected_param, "x1");
        assert_eq!(item.selected_objective, 2);
    }

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
    fn pdp_chart_default() {
        let chart = PdpChart::default();
        assert_eq!(chart.mode, PdpMode::OneDim);
        assert_eq!(chart.model_type, ModelType::Ridge);
        assert!(!chart.computing);
        assert!(chart.result.is_none());
        assert!(chart.cache.is_empty());
        assert!(!chart.feasible_only);
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
                trial_number: i as u32,
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
        assert_eq!(pts[0].0, [1.5, 2.0]);
    }

    #[test]
    fn extract_observed_missing_param() {
        let (view, obj_names) = make_view_xobj(&[1.5], &[2.0]);
        // "y" does not exist in the view -> empty
        let pts = extract_observed(&view, &obj_names, "y", 0, &[], &[]);
        assert!(pts.is_empty());
    }

    #[test]
    fn extract_observed_out_of_range_obj() {
        let (view, obj_names) = make_view_xobj(&[1.5], &[2.0]);
        // obj_idx=5 is out of range -> empty
        let pts = extract_observed(&view, &obj_names, "x", 5, &[], &[]);
        assert!(pts.is_empty());
    }

    #[test]
    fn classify_observed_matches_scatter_rules() {
        assert_eq!(classify_observed(true, 0), ObservedKind::Pareto);
        assert_eq!(classify_observed(true, 1), ObservedKind::NonPareto);
        // Infeasible regardless of rank
        assert_eq!(classify_observed(false, 0), ObservedKind::Infeasible);
        assert_eq!(classify_observed(false, 3), ObservedKind::Infeasible);
    }

    #[test]
    fn observed_kind_colors_match_scatter_palette() {
        assert_eq!(ObservedKind::Pareto.color(), COLOR_PARETO());
        assert_eq!(ObservedKind::NonPareto.color(), COLOR_NON_PARETO());
        assert_eq!(ObservedKind::Infeasible.color(), COLOR_INFEASIBLE());
    }

    // TASK-2025 tests

    #[test]
    fn cache_key_same_inputs_produce_same_key() {
        let k1 = cache_key("x", "obj0", "Ridge", false);
        let k2 = cache_key("x", "obj0", "Ridge", false);
        assert_eq!(k1, k2);
    }

    #[test]
    fn cache_key_different_model_produces_different_key() {
        let k1 = cache_key("x", "obj0", "ridge", false);
        let k2 = cache_key("x", "obj0", "gp_fitc", false);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_different_param_produces_different_key() {
        let k1 = cache_key("x", "obj0", "Ridge", false);
        let k2 = cache_key("y", "obj0", "Ridge", false);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_different_feasible_flag_produces_different_key() {
        // Toggling feasible_only produces a distinct cache entry (prevents stale hits)
        let k1 = cache_key("x", "obj0", "Ridge", false);
        let k2 = cache_key("x", "obj0", "Ridge", true);
        assert_ne!(k1, k2);
    }

    // ── TASK-2237: PDP observed overlay selection-linkage tests ──────────

    #[test]
    fn pdp_overlay_uses_filtered_rows_when_selection_exists() {
        let (view, obj_names) = make_view_xobj(&[1.0, 2.0, 3.0], &[2.0, 3.0, 4.0]);
        let selected = vec![0u32, 1];
        let pts = extract_observed(&view, &obj_names, "x", 0, &selected, &[]);
        assert_eq!(pts.len(), 2);
        let xs: Vec<f64> = pts.iter().map(|(p, _)| p[0]).collect();
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
        let xs: Vec<f64> = pts.iter().map(|(p, _)| p[0]).collect();
        assert!(xs.contains(&1.0), "selected row must be visible");
        assert!(xs.contains(&3.0), "pinned row must remain visible");
        assert!(!xs.contains(&2.0), "unselected unpinned row must be hidden");
    }
}
