use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_INFEASIBLE, COLOR_NON_PARETO_DIM, COLOR_UNSELECTED_POINT};
use crate::theme::color_compute::compute_point_alpha;
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::common::cluster_controls::ClusterControls;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};

mod cache;
mod matrix;
mod settings;
#[cfg(test)]
mod tests;

use matrix::compute_obj_axes_2d;

pub use cache::{validate_cluster_request, ClusterCacheKey, ClusterComputeRequest};
pub use matrix::{build_cluster_matrix, ClusterMatrix};
pub use settings::{ClusterSpace, KMeansInitStrategy, KSelectionMode};

/// Cluster scatter plot widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClusterScatter {
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    /// Upper bound of k explored in Elbow (auto) mode.
    pub elbow_max_k: usize,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub pending_compute: Option<ClusterComputeRequest>,
    #[serde(skip)]
    pub last_error: Option<crate::state::messages::ClusterUiError>,
    /// Trial detail modal opened by clicking a point.
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
    #[serde(skip)]
    cached_points: Option<Vec<[f32; 2]>>,
    #[serde(skip)]
    cache_key: (usize, usize, usize), // (df_ptr, trial_count, n_clusters)
}

impl Default for ClusterScatter {
    fn default() -> Self {
        Self {
            k: 3,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
            computing: false,
            pending_compute: None,
            last_error: None,
            detail_modal: TrialDetailModal::new(),
            cached_points: None,
            cache_key: (0, 0, 0),
        }
    }
}

impl ClusterScatter {
    /// Returns the cache key for the current settings.
    pub fn cache_key(&self) -> ClusterCacheKey {
        ClusterCacheKey::new(
            self.target_space,
            self.k_mode,
            self.k,
            self.init_strategy,
            self.elbow_max_k,
        )
    }

    /// Draws the cluster scatter plot.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        cluster_result: Option<&crate::state::app_state::ClusterResult>,
        param_names: &[String],
        obj_names: &[String],
        colormap: &ColorMap,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
        selected_indices: &[u32],
    ) {
        let n_trials = view.row_count();
        // The clustering target is the Pareto front (pareto_rank == 0).
        // The upper bound of k and whether clustering can run are determined by the
        // front's point count.
        let pareto_count = view.pareto_rank.iter().filter(|&&r| r == 0).count();
        if pareto_count < 2 {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("At least 2 Pareto-front solutions are required.").weak(),
                );
            });
            return;
        }

        self.show_header(ui, pareto_count);

        // While a selection filter is active, make it explicit that clusters are
        // computed over the whole front.
        if !selected_indices.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Highlighting selection. Clusters are computed over the full Pareto front.",
                )
                .small()
                .weak(),
            );
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Running clustering...");
            });
            return;
        }

        if let Some(err) = &self.last_error {
            ui.label(egui::RichText::new(&err.user_message).color(ERROR_COLOR()));
            if let Some(detail) = &err.detail_for_dev {
                ui.label(egui::RichText::new(detail).small().weak());
            }
            if err.retryable && ui.button("Retry").clicked() {
                self.try_queue_compute(pareto_count);
            }
            ui.separator();
        }

        let Some(cr) = cluster_result else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Clustering has not been run yet.").weak());
            });
            return;
        };

        if cr.labels.len() != n_trials {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Cluster result is inconsistent. Please run again.")
                        .color(ERROR_COLOR()),
                );
            });
            return;
        }

        // Check/update the cache (objective-axis coordinates).
        // Include the Arc identity of df in the key to prevent stale drawing when
        // switching to a different Study with the same dimensionality (M-6).
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;
        let new_key = (df_ptr, n_trials, cr.n_clusters);
        if self.cached_points.is_none() || self.cache_key != new_key {
            self.cached_points = Some(compute_obj_axes_2d(view, obj_names));
            self.cache_key = new_key;
        }
        let plot_points = self.cached_points.as_ref().unwrap();

        let feas = view.feasibility();

        // Candidates for point-click hit testing (trial_id, row index, coordinates).
        let hit_candidates: Vec<(u32, usize, [f64; 2])> = plot_points
            .iter()
            .enumerate()
            .map(|(i, &[x, y])| {
                let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
                (trial_id, i, [x as f64, y as f64])
            })
            .collect();

        // Place k clusters evenly on [0, 1] and sample colors from the colormap.
        // E.g. k=2 → t=0.0, 1.0 (both ends); k=3 → t=0.0, 0.5, 1.0.
        let n_clusters = cr.n_clusters.max(1);
        let cluster_color = |label: i32| -> egui::Color32 {
            colormap.sample_categorical(label.max(0) as usize, n_clusters)
        };

        // Only the Pareto front is clustered. Aggregate coordinates per cluster;
        // solutions outside the target (label < 0) go to "Others", and infeasible
        // ones are collected separately. When a selection filter (e.g. PCP brush)
        // is active, unselected points are grouped in gray and drawn behind
        // everything else. Clustering itself is still computed over the whole
        // front — the branching here only affects display emphasis.
        let mut cluster_points: BTreeMap<i32, Vec<[f64; 2]>> = BTreeMap::new();
        let mut unselected_pts: Vec<[f64; 2]> = Vec::new();
        let mut infeasible_pts: Vec<[f64; 2]> = Vec::new();
        let mut other_pts: Vec<[f64; 2]> = Vec::new();
        for (i, &[x, y]) in plot_points.iter().enumerate() {
            if !feas.is_feasible(i) {
                infeasible_pts.push([x as f64, y as f64]);
                continue;
            }
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            let selected = compute_point_alpha(trial_id, selected_indices) == 255;
            // Points outside the selection filter are grouped in gray, regardless of
            // whether they're a cluster point or a dominated solution (label < 0).
            if !selected {
                unselected_pts.push([x as f64, y as f64]);
                continue;
            }
            let label = cr.labels.get(i).copied().unwrap_or(-1);
            if label < 0 {
                // Solution outside the Pareto front (not a clustering target).
                other_pts.push([x as f64, y as f64]);
            } else {
                cluster_points
                    .entry(label)
                    .or_default()
                    .push([x as f64, y as f64]);
            }
        }

        let x_label = obj_names.first().map(|s| s.as_str()).unwrap_or("Obj 1");
        let y_label = obj_names.get(1).map(|s| s.as_str()).unwrap_or("Obj 2");
        let mut clicked_detail: Option<(u32, usize)> = None;
        egui_plot::Plot::new("cluster_scatter")
            .unified_nav()
            .x_axis_label(x_label)
            .y_axis_label(y_label)
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                // Detect the target for opening the detail modal via point click.
                let resp = plot_ui.response();
                if resp.clicked_by(egui::PointerButton::Primary) {
                    clicked_detail = resp.interact_pointer_pos().and_then(|pos| {
                        hit_test_nearest(plot_ui, &hit_candidates, pos, HIT_THRESHOLD)
                    });
                }
                // Draw infeasible points at the very back.
                if !infeasible_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("", infeasible_pts)
                            .color(COLOR_INFEASIBLE())
                            .radius(3.0)
                            .name("Infeasible"),
                    );
                }
                // Draw non-Pareto-front points (outside the clustering target) in a
                // dim color behind everything else.
                if !other_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("", other_pts)
                            .color(COLOR_NON_PARETO_DIM())
                            .radius(2.0)
                            .name("Others"),
                    );
                }
                // Draw unselected cluster points first in gray to clearly
                // distinguish them from selected points. Cluster colors are not
                // preserved (the hues would be confusing), so they're grouped under
                // "Others (unselected)".
                if !unselected_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("", unselected_pts)
                            .color(COLOR_UNSELECTED_POINT())
                            .radius(2.0)
                            .name("Others (unselected)"),
                    );
                }
                for (label, pts) in cluster_points {
                    let color = cluster_color(label);
                    let points = egui_plot::Points::new("", pts)
                        .color(color)
                        .radius(3.0)
                        .name(format!("Cluster {}", label));
                    plot_ui.points(points);
                }
            });

        // Clicking a point opens the trial detail modal (scatter info = cluster number).
        if let Some((trial_id, row)) = clicked_detail {
            let label = cr.labels.get(row).copied().unwrap_or(-1);
            let cluster_str = if label < 0 {
                "Unclustered".to_string()
            } else {
                label.to_string()
            };
            let mut context = vec![("Cluster".to_string(), cluster_str)];
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            context.push(("Pareto Rank".to_string(), rank.to_string()));
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        self.detail_modal
            .show(ui, view, param_names, obj_names, artifact_map);
    }

    /// Assembles a bundle of mutable references to the settings/running-state fields
    /// (for delegating to shared logic).
    fn controls(&mut self) -> ClusterControls<'_> {
        ClusterControls {
            k: &mut self.k,
            target_space: &mut self.target_space,
            k_mode: &mut self.k_mode,
            init_strategy: &mut self.init_strategy,
            elbow_max_k: &mut self.elbow_max_k,
            computing: &mut self.computing,
            pending_compute: &mut self.pending_compute,
            last_error: &mut self.last_error,
        }
    }

    fn show_header(&mut self, ui: &mut egui::Ui, trial_count: usize) {
        // For 2D, the spinner is shown separately on the body side (inside show), so it's
        // not shown here.
        self.controls()
            .show_controls(ui, trial_count, "cluster_scatter", false);
    }

    fn try_queue_compute(&mut self, trial_count: usize) {
        self.controls().try_queue_compute(trial_count);
    }

    pub fn set_error(&mut self, err: crate::state::messages::ClusterUiError) {
        self.computing = false;
        self.last_error = Some(err);
    }

    pub fn clear_runtime_state(&mut self) {
        self.computing = false;
        self.pending_compute = None;
        self.last_error = None;
    }

    /// Pulls in the shared clustering running state (computing / pending / error).
    /// Since the clustering result is aggregated into `app_state.cluster_cache`, the
    /// completion state must also be reflected into every canvas item (an independent
    /// WidgetStates). Keeps display caches (cached_points, etc.) as-is since they're
    /// item-specific.
    pub fn adopt_runtime_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.pending_compute = src.pending_compute.clone();
        self.last_error = src.last_error.clone();
    }
}
