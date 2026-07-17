//! Response surface 3D viewer.
//!
//! Same two-stage structure as `robustness.rs`: the surrogate fit runs
//! asynchronously (via poll_chart), while slice evaluation from the fitted
//! model (`tunny_core::surrogate_opt::surface_slice_at`) is on the order of
//! milliseconds, so it runs synchronously in the render pass and is cached.
//! Displays, as a 3D mesh, a slice through the 2-parameter plane passing
//! through the anchor point (Best trial / a pinned trial) — this is not a PDP
//! marginalization, but a "raw cross-section" with the other parameters fixed
//! at the anchor values. Anchor point selection uses `anchor::CenterChoice`,
//! shared with `robustness.rs`.
//!
//! Drawing reuses the shared surface mesh rendering from `pdp_2d.rs` (`draw_surface_mesh` etc.).

use std::collections::HashMap;
use std::sync::Arc;

use tunny_core::surrogate_opt::{
    SurfaceSlice, SurrogateModelKind, TrainedSurrogate, MIN_TRIALS_FOR_SURROGATE_OPT,
};

use super::anchor::{center_label, resolve_center, CenterChoice};
use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::COLOR_CONTOUR;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::pdp_2d::{
    band_grids, draw_surface_mesh, extract_observed_3d, value_range_of,
};
use crate::ui::widgets::pdp_chart::classify_observed;
use crate::ui::widgets::scatter_3d::{
    axis_segments_3d, draw_3d_axis_labels, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    show_hover_and_click_detail, ArcballCamera,
};
use crate::ui::widgets::trial_detail_modal::{axis_row, TrialDetailModal};

// Model choices (combo display order). Uses the single source of truth shared
// by all 3 widgets (`super::MODEL_CHOICES`).
use super::MODEL_CHOICES;

/// Grid resolution choices.
const GRID_CHOICES: [usize; 3] = [20, 30, 50];

/// Slice evaluation for GP-family models is expensive, scaling with the
/// square of the grid point count (50^2 = 2500 point predictions). To avoid
/// blocking the UI during synchronous execution in the render pass, GP-family
/// models cap the grid resolution at this value. Ridge / LightGBM are cheap
/// and are not limited.
const GP_GRID_CAP: usize = 30;

/// Whether this is a GP (Gaussian process) family model. The group of models
/// with high response-surface slice computation cost.
fn is_gp_kind(kind: SurrogateModelKind) -> bool {
    matches!(
        kind,
        SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe | SurrogateModelKind::GpMoe
    )
}

/// Computation request for the fit stage. Consumed by poll_chart.
pub struct ResponseSurfaceFitRequest {
    pub objective_index: usize,
    pub model: SurrogateModelKind,
}

/// Cache key: (fit generation ID, x_idx, y_idx, bit representation of anchor, n_grid).
/// The first element used to be `Arc::as_ptr`, but if the same address is
/// reused after deallocation, results from a different model could be
/// displayed incorrectly (ABA problem). Avoided by replacing it with a
/// generation ID (`ResponseSurfaceChart::fit_generation`) that increments
/// monotonically whenever a fit is adopted.
type SliceCacheKey = (u64, usize, usize, Vec<u64>, usize);

/// Cache key for anchor resolution results: (fit generation ID, anchor
/// choice, DataFrame identity). Since center-point resolution
/// (`resolve_center`) is an O(N) scan over all trials, this avoids
/// re-scanning on frames where the input hasn't changed.
type AnchorCacheKey = (u64, CenterChoice, usize);

/// UI state for the response surface 3D widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ResponseSurfaceChart {
    pub selected_objective: usize,
    pub model: SurrogateModelKind,
    pub anchor: CenterChoice,
    pub param_x: String,
    pub param_y: String,
    pub n_grid: usize,
    /// Toggle for showing the +-1.96 sigma band, only meaningful for GP-family models.
    pub show_uncertainty: bool,
    pub show_observed: bool,
    pub camera: ArcballCamera,

    #[serde(skip)]
    pub trained: Option<Arc<TrainedSurrogate>>,
    #[serde(skip)]
    pub fitting: bool,
    #[serde(skip)]
    pub fit_error: Option<String>,
    #[serde(skip)]
    pub pending_fit: Option<ResponseSurfaceFitRequest>,
    #[serde(skip)]
    cache: Option<(SliceCacheKey, SurfaceSlice)>,
    /// Cache of anchor resolution results (avoids the per-frame O(N) scan).
    #[serde(skip)]
    anchor_cache: Option<(AnchorCacheKey, Vec<f64>)>,
    /// Generation ID that increments monotonically whenever a fit is adopted. Used to replace `Arc::as_ptr` in the cache key.
    #[serde(skip)]
    fit_generation: u64,
    /// The trained model's Arc pointer observed on the most recent frame (used to detect changes for generation ID updates).
    #[serde(skip)]
    fit_ptr: usize,
    /// Trial detail modal opened by clicking an observed point.
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
}

impl Default for ResponseSurfaceChart {
    fn default() -> Self {
        Self {
            selected_objective: 0,
            model: SurrogateModelKind::GpFitc,
            anchor: CenterChoice::default(),
            param_x: String::new(),
            param_y: String::new(),
            n_grid: 30,
            show_uncertainty: true,
            show_observed: true,
            camera: ArcballCamera::isometric_default(),
            trained: None,
            fitting: false,
            fit_error: None,
            pending_fit: None,
            cache: None,
            anchor_cache: None,
            fit_generation: 0,
            fit_ptr: 0,
            detail_modal: TrialDetailModal::new(),
        }
    }
}

impl ResponseSurfaceChart {
    /// Pulls in the global widget's computation state, result, and error
    /// (called from `ComputeSyncKind::ResponseSurfaceFit`; same convention as `robustness.rs`).
    pub fn adopt_compute_state(&mut self, global: &Self) {
        self.trained = global.trained.clone();
        self.fitting = global.fitting;
        self.fit_error = global.fit_error.clone();
    }

    /// The most recent slice evaluation result (cache). Referenced by CSV export, etc.
    pub fn cached_slice(&self) -> Option<&SurfaceSlice> {
        self.cache.as_ref().map(|(_, s)| s)
    }
}

fn cache_key(
    fit_generation: u64,
    x_idx: usize,
    y_idx: usize,
    anchor: &[f64],
    n_grid: usize,
) -> SliceCacheKey {
    (
        fit_generation,
        x_idx,
        y_idx,
        anchor.iter().map(|v| v.to_bits()).collect(),
        n_grid,
    )
}

impl ResponseSurfaceChart {
    /// `obj_names` / `directions` are all objectives of the current Study (for resolving Best trial).
    /// `param_names` is the list of numeric parameters (candidates for the X/Y combos).
    /// `pinned_trials` are pinned trial_ids (candidates for the Anchor combo).
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        trial_count: usize,
        pinned_trials: &[u32],
        cmap: &ColorMap,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        if obj_names.is_empty() {
            ui.label("No objectives available.");
            return;
        }
        if self.selected_objective >= obj_names.len() {
            self.selected_objective = 0;
        }

        ui.horizontal(|ui| {
            ui.label("Objective:");
            egui::ComboBox::from_id_salt("response_surface_obj")
                .selected_text(obj_names[self.selected_objective].as_str())
                .show_ui(ui, |ui| {
                    for (i, name) in obj_names.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_objective, i, name);
                    }
                });

            ui.label("Model:");
            egui::ComboBox::from_id_salt("response_surface_model")
                .selected_text(super::surrogate_opt::model_label(self.model))
                .show_ui(ui, |ui| {
                    for kind in MODEL_CHOICES {
                        ui.selectable_value(
                            &mut self.model,
                            kind,
                            super::surrogate_opt::model_label(kind),
                        );
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("X:");
            egui::ComboBox::from_id_salt("response_surface_x")
                .selected_text(self.param_x.as_str())
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.param_x, name.clone(), name);
                    }
                });
            ui.label("Y:");
            egui::ComboBox::from_id_salt("response_surface_y")
                .selected_text(self.param_y.as_str())
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.param_y, name.clone(), name);
                    }
                });
            ui.label("Grid:");
            egui::ComboBox::from_id_salt("response_surface_grid")
                .selected_text(self.n_grid.to_string())
                .show_ui(ui, |ui| {
                    for n in GRID_CHOICES {
                        ui.selectable_value(&mut self.n_grid, n, n.to_string());
                    }
                });
        });

        if !self.param_x.is_empty() && self.param_x == self.param_y {
            ui.colored_label(COLOR_CONTOUR(), "Warning: X and Y must differ");
        }

        ui.horizontal(|ui| {
            ui.label("Anchor:");
            let anchor_text = center_label(self.anchor, view);
            egui::ComboBox::from_id_salt("response_surface_anchor")
                .selected_text(anchor_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.anchor, CenterChoice::BestTrial, "Best trial");
                    for &trial_id in pinned_trials {
                        let Some(row) = view.trial_ids.iter().position(|&t| t == trial_id) else {
                            continue;
                        };
                        let number = view.df.get_trial_number(row).unwrap_or(trial_id);
                        ui.selectable_value(
                            &mut self.anchor,
                            CenterChoice::Pinned(trial_id),
                            format!("Trial #{number}"),
                        );
                    }
                });
            ui.toggle_value(&mut self.show_observed, "Show data");
        });

        if trial_count < MIN_TRIALS_FOR_SURROGATE_OPT {
            ui.label(
                egui::RichText::new(format!(
                    "At least {} trials required (current: {})",
                    MIN_TRIALS_FOR_SURROGATE_OPT, trial_count
                ))
                .weak(),
            );
            return;
        }

        let can_fit = !self.fitting && self.pending_fit.is_none();
        if ui
            .add_enabled(can_fit, egui::Button::new("Fit Surrogate"))
            .clicked()
        {
            self.fit_error = None;
            self.fitting = true;
            self.pending_fit = Some(ResponseSurfaceFitRequest {
                objective_index: self.selected_objective,
                model: self.model,
            });
        }
        if self.fitting {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Fitting surrogate...");
            });
        }
        if let Some(err) = self.fit_error.clone() {
            ui.colored_label(egui::Color32::RED, err);
        }

        let Some(trained) = self.trained.clone() else {
            return;
        };

        // Detect a fit being adopted (the trained Arc was swapped to a different
        // model) and advance the generation ID. The cache key uses this
        // generation ID to avoid address reuse (ABA) with `Arc::as_ptr`.
        let trained_ptr = Arc::as_ptr(&trained) as usize;
        if trained_ptr != self.fit_ptr {
            self.fit_ptr = trained_ptr;
            self.fit_generation = self.fit_generation.wrapping_add(1);
        }

        if self.param_x.is_empty() || self.param_y.is_empty() || self.param_x == self.param_y {
            return;
        }
        let Some(x_idx) = trained.param_names.iter().position(|p| p == &self.param_x) else {
            ui.colored_label(
                egui::Color32::RED,
                "Selected X parameter is not part of the trained model.",
            );
            return;
        };
        let Some(y_idx) = trained.param_names.iter().position(|p| p == &self.param_y) else {
            ui.colored_label(
                egui::Color32::RED,
                "Selected Y parameter is not part of the trained model.",
            );
            return;
        };

        // Anchor resolution is an O(N) scan over all trials. Reuse the previous
        // result on frames where the input (generation, selection, DataFrame) hasn't changed.
        let anchor_key: AnchorCacheKey = (
            self.fit_generation,
            self.anchor,
            Arc::as_ptr(&view.df) as usize,
        );
        if self.anchor_cache.as_ref().map(|(k, _)| k) != Some(&anchor_key) {
            self.anchor_cache = resolve_center(&trained, self.anchor, view, obj_names, directions)
                .map(|a| (anchor_key, a));
        }
        let Some((_, anchor)) = self.anchor_cache.as_ref() else {
            ui.colored_label(
                egui::Color32::RED,
                "Could not resolve the anchor point for the trained parameters.",
            );
            return;
        };
        let anchor = anchor.clone();

        // GP-family models scale with the square of the grid point count, so
        // cap the slice resolution to limit UI blocking during synchronous
        // execution in the render pass (Ridge / LightGBM are unrestricted).
        let effective_grid = if is_gp_kind(trained.model_kind) {
            self.n_grid.min(GP_GRID_CAP)
        } else {
            self.n_grid
        };

        let key = cache_key(self.fit_generation, x_idx, y_idx, &anchor, effective_grid);
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            self.cache = tunny_core::surrogate_opt::surface_slice_at(
                &trained,
                &anchor,
                x_idx,
                y_idx,
                effective_grid,
            )
            .map(|s| (key, s));
        }

        if self.cache.is_none() {
            ui.colored_label(egui::Color32::RED, "Response surface evaluation failed.");
            return;
        }

        // Uncertainty band display toggle (Gaussian process models only; borrow self mutably before immutably borrowing cache).
        let has_uncertainty = self.cache.as_ref().is_some_and(|(_, s)| s.z_std.is_some());
        if has_uncertainty {
            ui.checkbox(&mut self.show_uncertainty, "95% CI (±1.96σ)");
        }

        let anchor_text = center_label(self.anchor, view);
        let show_uncertainty = self.show_uncertainty;
        let show_observed = self.show_observed;
        let param_x = self.param_x.clone();
        let param_y = self.param_y.clone();
        let objective_name = obj_names[self.selected_objective].clone();
        let camera = &mut self.camera;
        let detail_modal = &mut self.detail_modal;
        // `camera` (a mutable borrow of self.camera) and `slice` (an immutable
        // borrow of self.cache) are disjoint fields, so they can be borrowed
        // simultaneously (same pattern as pdp_2d.rs).
        let (_, slice) = self.cache.as_ref().expect("checked non-empty above");

        let (c_min, c_max) = value_range_of(&slice.z_values);
        let mut v_min = c_min;
        let mut v_max = c_max;

        let bands = if show_uncertainty {
            slice
                .z_std
                .as_ref()
                .map(|std_grid| band_grids(&slice.z_values, std_grid))
        } else {
            None
        };
        if let Some((lower, upper)) = &bands {
            let (l_min, _) = value_range_of(lower);
            let (_, u_max) = value_range_of(upper);
            v_min = v_min.min(l_min);
            v_max = v_max.max(u_max);
        }

        let observed = if show_observed {
            extract_observed_3d(
                view,
                &param_x,
                &param_y,
                &objective_name,
                &[],
                pinned_trials,
            )
        } else {
            vec![]
        };
        for (_, p, _) in &observed {
            v_min = v_min.min(p[2]);
            v_max = v_max.max(p[2]);
        }

        let (x_min, x_max) = value_range_of(std::slice::from_ref(&slice.x_values));
        let (y_min, y_max) = value_range_of(std::slice::from_ref(&slice.y_values));

        let observed_clip: Vec<([f32; 3], egui::Color32)> = observed
            .iter()
            .map(|&(_, [px, py, ov], kind)| {
                (
                    [
                        normalize_to_clip(px, x_min, x_max),
                        normalize_to_clip(ov, v_min, v_max),
                        normalize_to_clip(py, y_min, y_max),
                    ],
                    kind.color(),
                )
            })
            .collect();

        // Subtract the height of the single-line anchor caption below before
        // allocating the 3D canvas (prevents the caption from being clipped).
        let caption_h = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
        let avail = ui.available_size();
        let canvas_size = egui::vec2(
            (avail.x - 16.0).max(120.0),
            (avail.y - caption_h).max(160.0),
        );
        // Column references for the hover tooltip / click detail (observed points are real trials).
        let px_col = view.numeric_column(&param_x);
        let py_col = view.numeric_column(&param_y);
        let obj_col = view.numeric_column(&objective_name);
        let feas = view.feasibility();

        ui.allocate_ui(canvas_size, |ui| {
            ui.set_min_size(canvas_size);
            let (painter, _rect, project, click_pos, hover_pos) = setup_3d_canvas(ui, camera);
            draw_3d_grid(&painter, &project);
            draw_surface_mesh(
                &painter,
                &project,
                &slice.z_values,
                (v_min, v_max),
                (c_min, c_max),
                cmap,
                bands.as_ref().map(|(lower, upper)| (lower, upper)),
                &observed_clip,
                &axis_segments_3d(24),
            );
            draw_3d_axis_labels(
                &painter,
                &project,
                [&param_x, &objective_name, &param_y],
                [(x_min, x_max), (v_min, v_max), (y_min, y_max)],
            );

            // Hover tooltip / click detail for observed points (same interaction as other 3D scatter plots).
            // When "Show data" is off, observed is empty so nothing happens.
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
                "response_surface_hover_tooltip",
                &mut *detail_modal,
                |row| {
                    vec![
                        axis_row(&param_x, px_col, row),
                        axis_row(&param_y, py_col, row),
                        axis_row(&objective_name, obj_col, row),
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

        ui.label(
            egui::RichText::new(format!(
                "Slice through {anchor_text} (other parameters fixed)"
            ))
            .weak(),
        );

        // Draws the trial detail modal opened by a click.
        if detail_modal.is_open() {
            detail_modal.show(ui, view, param_names, obj_names, artifact_map);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_surface_chart_default_values() {
        let s = ResponseSurfaceChart::default();
        assert_eq!(s.selected_objective, 0);
        assert_eq!(s.anchor, CenterChoice::BestTrial);
        assert_eq!(s.n_grid, 30);
        assert!(s.show_uncertainty);
        assert!(s.show_observed);
        assert!(s.trained.is_none());
        assert!(!s.fitting);
        assert!(s.pending_fit.is_none());
        assert!(s.cached_slice().is_none());
        assert_ne!(s.camera.rotation, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn adopt_compute_state_propagates_and_keeps_selection() {
        let src = ResponseSurfaceChart {
            fitting: false,
            fit_error: Some("err".into()),
            ..Default::default()
        };
        let mut dst = ResponseSurfaceChart {
            fitting: true,
            selected_objective: 2,
            param_x: "x1".to_string(),
            ..Default::default()
        };
        dst.adopt_compute_state(&src);
        assert!(!dst.fitting);
        assert_eq!(dst.fit_error.as_deref(), Some("err"));
        // UI selections are preserved.
        assert_eq!(dst.selected_objective, 2);
        assert_eq!(dst.param_x, "x1");
    }
}
