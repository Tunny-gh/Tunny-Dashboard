use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::messages::PdpResult2d;
use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_CONTOUR;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::heatmap::draw_colorbar_simple;
use crate::ui::widgets::pdp_chart::{classify_observed, ModelType, ObservedKind};
use crate::ui::widgets::scatter_3d::{
    axis_segments_3d, draw_3d_axis_labels, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    show_hover_and_click_detail, ArcballCamera,
};
use crate::ui::widgets::trial_detail_modal::{axis_row, TrialDetailModal};

mod math;
mod mesh;
mod observed;
#[cfg(test)]
mod tests;

use math::{axis_range_of, check_params_different};
pub(crate) use math::{band_grids, value_range_of};
pub(crate) use mesh::draw_surface_mesh;
pub(crate) use observed::extract_observed_3d;

/// 2D grid values (rows = param1, columns = param2)
pub(crate) type Grid = Vec<Vec<f64>>;
/// (lower, upper) grids for the 95% CI band
pub(crate) type BandGrids = (Grid, Grid);

/// Pending 2D PDP computation request, placed by show() and consumed by the chart cell body.
pub struct Pdp2dComputeRequest {
    pub param1: String,
    pub param2: String,
    pub objective: String,
    pub n_grid: usize,
    pub model_type: String,
    /// Whether to fit the model using only feasible trials (is_feasible > 0.5)
    pub feasible_only: bool,
}

/// PDP 2D widget state
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PdpChart2DState {
    pub selected_param1: String,
    pub selected_param2: String,
    pub selected_objective: usize,
    pub selected_model: ModelType,
    #[serde(skip)]
    pub result: Option<PdpResult2d>,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub pending_compute: Option<Pdp2dComputeRequest>,
    pub camera: ArcballCamera,
    /// Whether to overlay uncertainty (±1.96σ = 95% CI) as a translucent band for GP-family models
    pub show_uncertainty: bool,
    /// Whether to overlay observed data (sampled points) on the surface
    pub show_observed: bool,
    /// Whether to fit the model using only feasible trials (UI shown only for constrained studies)
    pub feasible_only: bool,
    /// Trial detail modal opened by clicking an observed point
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
}

impl Default for PdpChart2DState {
    fn default() -> Self {
        Self {
            selected_param1: String::new(),
            selected_param2: String::new(),
            selected_objective: 0,
            selected_model: ModelType::Ridge,
            result: None,
            computing: false,
            pending_compute: None,
            camera: ArcballCamera::isometric_default(),
            show_uncertainty: true,
            show_observed: false,
            feasible_only: false,
            detail_modal: TrialDetailModal::new(),
        }
    }
}

impl PdpChart2DState {
    /// Adopts the computing state and result from the global widget.
    /// The 2D PDP result is held on the widget side (`result`), so it must also be
    /// reflected onto each canvas item (independent WidgetStates). The parameter,
    /// objective, and model selections are preserved.
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.result = src.result.clone();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        param_names: &[String],
        obj_names: &[String],
        cmap: ColorMap,
        view: &StudyView,
        selected_indices: &[u32],
        pinned: &[u32],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        // Row 1: Parameter 1 + Parameter 2
        ui.horizontal(|ui| {
            ui.label("Parameter 1:");
            egui::ComboBox::from_id_salt("pdp2d_p1")
                .selected_text(&self.selected_param1)
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.selected_param1, name.clone(), name);
                    }
                });
            ui.label("Parameter 2:");
            egui::ComboBox::from_id_salt("pdp2d_p2")
                .selected_text(&self.selected_param2)
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.selected_param2, name.clone(), name);
                    }
                });
        });

        // Row 2: Objective + Model selector
        ui.horizontal(|ui| {
            ui.label("Objective:");
            let obj_text = obj_names
                .get(self.selected_objective)
                .map(|s| s.as_str())
                .unwrap_or("—");
            egui::ComboBox::from_id_salt("pdp2d_obj")
                .selected_text(obj_text)
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

            ui.label("Model:");
            egui::ComboBox::from_id_salt("pdp2d_model")
                .selected_text(self.selected_model.label())
                .show_ui(ui, |ui| {
                    for model in ModelType::ALL {
                        let selected = self.selected_model == model;
                        if ui.selectable_label(selected, model.label()).clicked() {
                            self.selected_model = model;
                        }
                    }
                });

            // Toggle to show observed data (same interaction as 1D PDP)
            ui.separator();
            ui.toggle_value(&mut self.show_observed, "Show data");

            // Feasible-only filter (constrained studies only)
            if view.feasibility().has_constraints() {
                ui.toggle_value(&mut self.feasible_only, "Feasible only")
                    .on_hover_text("Fit the model using feasible trials only");
            }
        });

        // Warning for identical parameter selection
        if !self.selected_param1.is_empty() && self.selected_param1 == self.selected_param2 {
            ui.colored_label(COLOR_CONTOUR(), "Warning: the same parameter is selected");
        }

        // Run button — only enabled when params are different and objectives exist
        let can_run = check_params_different(&self.selected_param1, &self.selected_param2)
            && !obj_names.is_empty()
            && !self.computing;
        if ui
            .add_enabled(can_run, egui::Button::new("Run 2D PDP"))
            .clicked()
        {
            if let Some(obj_name) = obj_names.get(self.selected_objective) {
                let n_grid = match self.selected_model {
                    ModelType::RandomForest => 30,
                    _ => 20,
                };
                self.pending_compute = Some(Pdp2dComputeRequest {
                    param1: self.selected_param1.clone(),
                    param2: self.selected_param2.clone(),
                    objective: obj_name.clone(),
                    n_grid,
                    model_type: self.selected_model.to_str().to_string(),
                    feasible_only: self.feasible_only,
                });
            }
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing 2D PDP...");
            });
            return;
        }

        if self.result.is_none() {
            ui.label("No 2D PDP data");
            return;
        }

        // Toggle for the uncertainty band (GP-family models only; mutably borrow self
        // before taking an immutable borrow of result)
        let has_uncertainty = self
            .result
            .as_ref()
            .is_some_and(|r| r.uncertainties.is_some());
        if has_uncertainty {
            ui.checkbox(&mut self.show_uncertainty, "95% CI (±1.96σ)");
        }

        let camera = &mut self.camera;
        let detail_modal = &mut self.detail_modal;
        let result = self.result.as_ref().unwrap();
        let values: &[Vec<f64>] = &result.z_values;
        let value_label = result.objective_name.clone();

        if values.len() < 2 || values[0].len() < 2 {
            ui.label("Not enough grid data for 3D surface");
            return;
        }

        // Uncertainty band: upper/lower surfaces at Mean ± 1.96σ (drawn translucent, overlaid)
        let bands: Option<BandGrids> = if self.show_uncertainty && has_uncertainty {
            result
                .uncertainties
                .as_ref()
                .map(|unc| band_grids(values, unc))
        } else {
            None
        };

        // Observed data: (row index, [param1, param2, objective], classification)
        let observed: Vec<(usize, [f64; 3], ObservedKind)> = if self.show_observed {
            extract_observed_3d(
                view,
                &result.param1_name,
                &result.param2_name,
                &result.objective_name,
                selected_indices,
                pinned,
            )
        } else {
            vec![]
        };

        // Color is normalized over the Mean value range (the colorbar uses this range too)
        let (c_min, c_max) = value_range_of(values);
        // Extend the vertical-axis geometry range so the bands and observed points fit too
        let (mut v_min, mut v_max) = (c_min, c_max);
        if let Some((lower, upper)) = &bands {
            let (l_min, _) = value_range_of(lower);
            let (_, u_max) = value_range_of(upper);
            v_min = v_min.min(l_min);
            v_max = v_max.max(u_max);
        }
        for (_, p, _) in &observed {
            v_min = v_min.min(p[2]);
            v_max = v_max.max(p[2]);
        }
        let (x_min, x_max) = axis_range_of(&result.x_values);
        let (y_min, y_max) = axis_range_of(&result.y_values);

        // Project observed points into clip space (X = param1, Y(vertical) = objective value, Z = param2)
        let observed_clip: Vec<([f32; 3], egui::Color32)> = observed
            .iter()
            .map(|&(_, [p1, p2, ov], kind)| {
                (
                    [
                        normalize_to_clip(p1, x_min, x_max),
                        normalize_to_clip(ov, v_min, v_max),
                        normalize_to_clip(p2, y_min, y_max),
                    ],
                    kind.color(),
                )
            })
            .collect();

        // Column references for the hover tooltip / click detail (observed points are real trials)
        let p1_col = view.numeric_column(&result.param1_name);
        let p2_col = view.numeric_column(&result.param2_name);
        let obj_col = view.numeric_column(&result.objective_name);
        let feas = view.feasibility();

        // Canvas (reserve margin on the right for the colorbar: bar + numeric ticks +
        // vertical title. Uses the same width as COLORBAR_RESERVE in observed_contour.rs)
        let avail = ui.available_size();
        let canvas_size = egui::vec2((avail.x - 96.0).max(120.0), avail.y.max(160.0));
        ui.allocate_ui(canvas_size, |ui| {
            ui.set_min_size(canvas_size);
            let (painter, rect, project, click_pos, hover_pos) = setup_3d_canvas(ui, camera);
            draw_3d_grid(&painter, &project);
            // Axis lines are subdivided and depth-sorted together with the surface so
            // their front/back relationship with the faces is reflected correctly
            draw_surface_mesh(
                &painter,
                &project,
                values,
                (v_min, v_max),
                (c_min, c_max),
                &cmap,
                bands.as_ref().map(|(lower, upper)| (lower, upper)),
                &observed_clip,
                &axis_segments_3d(24),
            );
            // Axis labels are always drawn on top for readability.
            // X = param1, Y(vertical) = objective value, Z = param2
            draw_3d_axis_labels(
                &painter,
                &project,
                [&result.param1_name, &value_label, &result.param2_name],
                [(x_min, x_max), (v_min, v_max), (y_min, y_max)],
            );

            // The colorbar is overlaid to the right of the canvas (color = Mean value range).
            // Uses the same shared drawing routine as heatmap/contour (observed_contour.rs).
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(rect.right() + 6.0, rect.top()),
                egui::vec2(14.0, rect.height()),
            );
            draw_colorbar_simple(ui, bar_rect, c_min, c_max, cmap.clone(), Some(&value_label));

            // Hover tooltip / click detail for observed points (same interaction as the
            // other 3D scatter plots). When "Show data" is off, `observed` is empty so
            // nothing happens.
            let candidates: Vec<(u32, usize, egui::Pos2)> = observed
                .iter()
                .zip(observed_clip.iter())
                .filter_map(|(&(row, _, _), &(clip, _))| {
                    let (pos, _) = project(clip);
                    if !pos.x.is_finite() || !pos.y.is_finite() {
                        return None;
                    }
                    let trial_id = view.trial_ids.get(row).copied().unwrap_or(row as u32);
                    Some((trial_id, row, pos))
                })
                .collect();
            show_hover_and_click_detail(
                ui,
                view,
                &candidates,
                hover_pos,
                click_pos,
                "pdp2d_hover_tooltip",
                &mut *detail_modal,
                |row| {
                    vec![
                        axis_row(&result.param1_name, p1_col, row),
                        axis_row(&result.param2_name, p2_col, row),
                        axis_row(&result.objective_name, obj_col, row),
                    ]
                },
                |row| {
                    let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
                    vec![(
                        "Status".to_string(),
                        classify_observed(feas.is_feasible(row), rank)
                            .label()
                            .to_string(),
                    )]
                },
            );
        });

        // Draw the trial detail modal opened by a click.
        if detail_modal.is_open() {
            detail_modal.show(ui, view, param_names, obj_names, artifact_map);
        }
    }
}
