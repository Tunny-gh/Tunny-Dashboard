//! MCDM Scatter Chart Widget

mod hit_test;
mod points;
mod render;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::results::{McdmMethod, McdmResult};
use crate::state::types::{ColormapName, StudyView};
use crate::theme::chart_colors::COLOR_EMPTY_STATE;
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::mcdm_chart::McdmControls;
use crate::ui::widgets::trial_detail_modal::{TrialDetailModal, TrialDetailTarget};

use hit_test::compute_hit_candidates;
use points::compute_scatter_points;
pub(crate) use points::{
    build_rank_map, extract_axis_values, fallback_axis_id, get_axis_options, mcdm_rank_color,
    ScatterMetadata,
};
use render::{build_display_batches, render_scatter_plot, DisplayBatches};

/// FNV-like hash of `ranked_indices()`, shared by the 2D/3D scatter plots (H-3).
///
/// The previous 2D implementation keyed the cache only on the bit pattern of
/// `primary_scores()[0]` plus the point count. If row 0 was outside the Pareto front
/// (`expand_scores` always maps it to 0.0), changing the weights and re-running could
/// keep the same key, silently leaving the old rank colors displayed. Using a hash of the
/// entire `ranked_indices()`, as the 3D version does, guarantees detection whenever the
/// ranking changes.
pub(crate) fn ranked_hash(result: &McdmResult) -> u64 {
    result.ranked_indices().iter().fold(0u64, |acc, &x| {
        acc.wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(x as u64 + 1)
    })
}

/// Cache key
#[derive(Clone, PartialEq, Eq)]
struct CacheKey {
    trial_count: usize,
    x_axis: String,
    y_axis: String,
    colormap_name: ColormapName,
    top_n: usize,
    /// MCDM method (detects method switches)
    result_method: McdmMethod,
    /// Hash of ranked_indices (detects weight changes / ranking changes)
    ranked_indices_hash: u64,
}

/// MCDM scatter plot widget
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmScatterChart {
    /// MCDM configuration and execution state (method / weights / Run, etc.)
    pub controls: McdmControls,
    /// X-axis identifier
    pub x_axis: String,
    /// Y-axis identifier
    pub y_axis: String,
    /// Trial detail modal opened by clicking a point.
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
    // --- Internal cache state ---
    /// Display batches classified by color and sorted by luminance (avoids rebuilding
    /// every frame).
    #[serde(skip)]
    display_batches: Option<DisplayBatches>,
    #[serde(skip)]
    infeasible_cache: Option<Vec<(f64, f64)>>,
    /// Candidates for point-click hit testing (trial_id, row index, coordinates).
    /// Updated with the same key as `display_rows_cache`.
    #[serde(skip)]
    hit_candidates: Option<Vec<(u32, usize, [f64; 2])>>,
    #[serde(skip)]
    metadata: Option<ScatterMetadata>,
    #[serde(skip)]
    error_message: Option<String>,
    #[serde(skip)]
    cache_key: Option<CacheKey>,
}

impl Default for McdmScatterChart {
    fn default() -> Self {
        Self {
            controls: McdmControls::default(),
            x_axis: "Objective0".to_string(),
            y_axis: "Objective1".to_string(),
            detail_modal: TrialDetailModal::new(),
            display_batches: None,
            infeasible_cache: None,
            hit_candidates: None,
            metadata: None,
            error_message: None,
            cache_key: None,
        }
    }
}

impl McdmScatterChart {
    /// Adopts the MCDM execution state from the global widget (for canvas items).
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    /// Builds a cache key from the current settings
    fn make_cache_key(
        &self,
        trial_count: usize,
        result: &McdmResult,
        colormap_name: &ColormapName,
        top_n: usize,
    ) -> CacheKey {
        CacheKey {
            trial_count,
            x_axis: self.x_axis.clone(),
            y_axis: self.y_axis.clone(),
            colormap_name: colormap_name.clone(),
            top_n,
            result_method: result.method(),
            ranked_indices_hash: ranked_hash(result),
        }
    }

    /// Checks whether the cache is stale
    fn is_cache_stale(
        &self,
        trial_count: usize,
        result: &McdmResult,
        colormap_name: &ColormapName,
        top_n: usize,
    ) -> bool {
        match &self.cache_key {
            None => true,
            Some(key) => {
                key.trial_count != trial_count
                    || key.x_axis != self.x_axis
                    || key.y_axis != self.y_axis
                    || key.colormap_name != *colormap_name
                    || key.top_n != top_n
                    || key.result_method != result.method()
                    || key.ranked_indices_hash != ranked_hash(result)
            }
        }
    }

    /// Renders the widget
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        mcdm_result: Option<&McdmResult>,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        colormap: &ColorMap,
        colormap_name: &ColormapName,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
        selected_indices: &[u32],
    ) {
        if !self.controls.show_controls(ui, obj_names, "mcdm_scatter") {
            return;
        }
        if self.controls.computing {
            return;
        }
        let top_n = self.controls.top_n.value();

        let Some(result) = mcdm_result else {
            ui.centered_and_justified(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE(), "Press Run to compute the MCDM ranking");
            });
            return;
        };

        let options = get_axis_options(result, obj_names);

        // Update the default axis if it's invalid
        if !options.iter().any(|o| o.id == self.x_axis) {
            if let Some(first) = options.first() {
                self.x_axis = first.id.clone();
            }
        }
        if !options.iter().any(|o| o.id == self.y_axis) {
            if options.len() > 1 {
                self.y_axis = options[1].id.clone();
            } else if let Some(first) = options.first() {
                self.y_axis = first.id.clone();
            }
        }

        ui.horizontal(|ui| {
            ui.label("X:");
            egui::ComboBox::from_id_salt("mcdm_scatter_x_axis")
                .selected_text(&self.x_axis)
                .show_ui(ui, |ui| {
                    for opt in &options {
                        ui.selectable_value(&mut self.x_axis, opt.id.clone(), &opt.label);
                    }
                });

            ui.label("Y:");
            egui::ComboBox::from_id_salt("mcdm_scatter_y_axis")
                .selected_text(&self.y_axis)
                .show_ui(ui, |ui| {
                    for opt in &options {
                        ui.selectable_value(&mut self.y_axis, opt.id.clone(), &opt.label);
                    }
                });
        });

        let n_trials = view.row_count();

        // Recompute if the cache is stale
        if self.is_cache_stale(n_trials, result, colormap_name, top_n) {
            let new_key = self.make_cache_key(n_trials, result, colormap_name, top_n);
            let start = std::time::Instant::now();

            match compute_scatter_points(
                result,
                view,
                obj_names,
                &self.x_axis,
                &self.y_axis,
                colormap,
                top_n,
            ) {
                Ok((points, infeasible, mut meta)) => {
                    meta.compute_time_ms = start.elapsed().as_secs_f64() * 1000.0;
                    // Color classification and luminance sorting are done once here;
                    // subsequent frames only apply the selection filter (M-17).
                    self.display_batches = Some(build_display_batches(&points));
                    self.infeasible_cache = Some(infeasible);
                    self.hit_candidates = Some(compute_hit_candidates(
                        result,
                        view,
                        obj_names,
                        &self.x_axis,
                        &self.y_axis,
                    ));
                    self.cache_key = Some(new_key);
                    self.metadata = Some(meta);
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(e);
                    self.display_batches = None;
                    self.infeasible_cache = None;
                    self.hit_candidates = None;
                    self.cache_key = None;
                }
            }
        }

        if let Some(ref error) = self.error_message {
            ui.colored_label(ERROR_COLOR(), error);
            return;
        }

        let empty = vec![];
        let infeasible = self.infeasible_cache.as_deref().unwrap_or(&empty);
        let no_candidates = vec![];
        let candidates = self.hit_candidates.as_deref().unwrap_or(&no_candidates);
        let mut clicked_detail: Option<(u32, usize)> = None;
        if let Some(ref batches) = self.display_batches {
            clicked_detail = render_scatter_plot(
                ui,
                batches,
                infeasible,
                candidates,
                &self.x_axis,
                &self.y_axis,
                colormap,
                top_n,
                selected_indices,
            );
        }

        // Open the trial detail modal on point click (scatter info = MCDM rank/score).
        if let Some((trial_id, row)) = clicked_detail {
            let rank_map = build_rank_map(result.ranked_indices(), view.row_count());
            let rank = rank_map.get(row).copied().unwrap_or(usize::MAX);
            let rank_str = if rank == usize::MAX {
                "—".to_string()
            } else {
                (rank + 1).to_string()
            };
            let score = result.primary_scores().get(row).copied();
            let mut context = vec![("MCDM Rank".to_string(), rank_str)];
            context.push((
                "Score".to_string(),
                score
                    .map(|s| format!("{s:.4}"))
                    .unwrap_or_else(|| "—".to_string()),
            ));
            // VIKOR: also flag points belonging to the compromise solution set (C1/C2) in
            // the modal.
            if let McdmResult::Vikor(vr) = result {
                if vr.compromise_indices.contains(&row) {
                    context.push(("VIKOR Compromise".to_string(), "★ Yes (C1/C2)".to_string()));
                }
            }
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }
        self.detail_modal
            .show(ui, view, param_names, obj_names, artifact_map);

        ui.separator();
        // While a selection filter is active, clarify that scores are computed over the
        // full Pareto front.
        if !selected_indices.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Highlighting selection. Scores are computed over the full Pareto front.",
                )
                .small()
                .weak(),
            );
        }
        // VIKOR: highlight the compromise solution set (solutions satisfying Opricovic &
        // Tzeng's acceptance conditions C1/C2). If C1 doesn't hold there may be multiple
        // solutions, so display them as a list of trial numbers.
        if let McdmResult::Vikor(vr) = result {
            if !vr.compromise_indices.is_empty() {
                let labels: Vec<String> = vr
                    .compromise_indices
                    .iter()
                    .map(|&row| {
                        view.df
                            .get_trial_number(row)
                            .map(|n| format!("#{n}"))
                            .unwrap_or_else(|| format!("row {row}"))
                    })
                    .collect();
                ui.label(
                    egui::RichText::new(format!(
                        "★ VIKOR compromise set (C1/C2): {}",
                        labels.join(", ")
                    ))
                    .small()
                    .strong(),
                );
            }
        }
        if let Some(ref meta) = self.metadata {
            ui.label(
                egui::RichText::new(format!(
                    "Rendering {} points ({:.1}ms)",
                    meta.total_trials, meta.compute_time_ms
                ))
                .small(),
            );
        }
    }
}
