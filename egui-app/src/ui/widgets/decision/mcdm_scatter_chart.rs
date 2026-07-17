//! MCDM Scatter Chart Widget

use std::collections::{HashMap, HashSet};

use crate::io::artifacts::ArtifactEntry;
use crate::state::results::{McdmMethod, McdmResult};
use crate::state::types::{ColormapName, StudyView};
use crate::theme::chart_colors::{COLOR_EMPTY_STATE, COLOR_MCDM_NONE, COLOR_UNSELECTED_POINT};
use crate::theme::color_compute::{key_to_color32, point_alpha_in_set, rgba_key};
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::mcdm_chart::McdmControls;
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};
use egui::Color32;

/// Axis identifier constants (shared by `get_axis_options` and `extract_axis_values`)
const AXIS_VIKOR_Q: &str = "VIKOR_Q";
const AXIS_VIKOR_S: &str = "VIKOR_S";
const AXIS_VIKOR_R: &str = "VIKOR_R";
const AXIS_TOPSIS_SCORE: &str = "TOPSIS_Score";
const AXIS_PHI_PLUS: &str = "Phi+";
const AXIS_PHI_MINUS: &str = "Phi-";
const AXIS_PHI_NET: &str = "Phi_Net";

/// Axis selection option
#[derive(Clone, Debug)]
pub(crate) struct AxisOption {
    pub id: String,
    pub label: String,
}

/// Scatter plot computation metadata
#[derive(Clone, Debug)]
pub(crate) struct ScatterMetadata {
    pub total_trials: usize,
    pub compute_time_ms: f64,
}

/// Precomputed display batches (independent of the selection filter · M-17).
///
/// The previous implementation rebuilt a `HashMap` of "color -> points" plus a luminance
/// sort every frame, but this classification doesn't depend on the selection filter, so
/// it is now computed once when the cache is rebuilt and kept around. Only the dimming
/// caused by the selection filter (PCP brush, etc.) is applied lightly at render time via
/// a `HashSet` (M-16).
struct DisplayBatches {
    /// Color batches sorted by ascending luminance (ranked feasible points).
    /// Each point is `(trial_id, [x, y])`. `trial_id` is used for selection filter checks.
    color_batches: Vec<(Color32, BatchPoints)>,
    /// Unranked (COLOR_MCDM_NONE) feasible points.
    none_pts: BatchPoints,
}

/// List of points in a display batch. Each point is `(trial_id, [x, y])`.
type BatchPoints = Vec<(u32, [f64; 2])>;

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

// ──────────────────────────────────────────────────────────────
// Scatter plot rendering
// ──────────────────────────────────────────────────────────────

/// Builds the precomputed display batches (`DisplayBatches`).
///
/// The `HashMap` classification by color and luminance sorting are done once here, and
/// are not recomputed except when the cache is rebuilt (M-17). Independent of the
/// selection filter.
fn build_display_batches(points: &[ScatterPoint]) -> DisplayBatches {
    let mut none_pts: BatchPoints = Vec::new();
    // color -> coordinate list (also keeps the u32 luminance value for sorting)
    let mut color_groups: HashMap<[u8; 4], (BatchPoints, u32)> = HashMap::new();

    for &(x, y, color, trial_id) in points {
        if color == COLOR_MCDM_NONE() {
            none_pts.push((trial_id, [x, y]));
        } else {
            let key = rgba_key(color);
            let lum = color.r() as u32 + color.g() as u32 + color.b() as u32;
            let entry = color_groups.entry(key).or_insert((Vec::new(), lum));
            entry.0.push((trial_id, [x, y]));
        }
    }

    // Sort by luminance (draw dark-to-light, so lighter points end up on top)
    let mut sorted: Vec<_> = color_groups.into_iter().collect();
    sorted.sort_by_key(|(_, (_, lum))| *lum);
    let color_batches = sorted
        .into_iter()
        .map(|(key, (pts, _))| (key_to_color32(key), pts))
        .collect();

    DisplayBatches {
        color_batches,
        none_pts,
    }
}

/// Renders the scatter plot and returns `(trial_id, row index)` if a point was clicked.
#[allow(clippy::too_many_arguments)]
fn render_scatter_plot(
    ui: &mut egui::Ui,
    batches: &DisplayBatches,
    infeasible: &[(f64, f64)],
    hit_candidates: &[(u32, usize, [f64; 2])],
    x_label: &str,
    y_label: &str,
    colormap: &ColorMap,
    top_n: usize,
    selected_indices: &[u32],
) -> Option<(u32, usize)> {
    // When a selection filter (PCP brush, etc.) is active, points outside the selection
    // are dimmed and drawn together in the back. Scores/colors remain based on the full
    // front; branching here only affects visual emphasis.
    // Only dimming via the selection set (HashSet) is applied to the precomputed batches
    // (M-16).
    let selected: HashSet<u32> = selected_indices.iter().copied().collect();
    let mut dim_pts: Vec<[f64; 2]> = Vec::new();
    let mut none_pts: Vec<[f64; 2]> = Vec::new();
    for &(trial_id, pt) in &batches.none_pts {
        if point_alpha_in_set(trial_id, &selected) != 255 {
            dim_pts.push(pt);
        } else {
            none_pts.push(pt);
        }
    }
    // Keep ascending luminance order while routing unselected points into dim_pts.
    let mut color_draw: Vec<(Color32, Vec<[f64; 2]>)> =
        Vec::with_capacity(batches.color_batches.len());
    for (color, pts) in &batches.color_batches {
        let mut drawn: Vec<[f64; 2]> = Vec::with_capacity(pts.len());
        for &(trial_id, pt) in pts {
            if point_alpha_in_set(trial_id, &selected) != 255 {
                dim_pts.push(pt);
            } else {
                drawn.push(pt);
            }
        }
        if !drawn.is_empty() {
            color_draw.push((*color, drawn));
        }
    }

    // Representative colors for the legend
    let best_color = colormap.interpolate(1.0);
    let worst_color = if top_n > 1 {
        colormap.interpolate(0.0)
    } else {
        best_color
    };
    // Always draw since visibility can be toggled from the legend
    let has_infeasible = !infeasible.is_empty();

    let mut clicked_detail: Option<(u32, usize)> = None;
    egui_plot::Plot::new("mcdm_scatter_plot")
        .unified_nav()
        .x_axis_label(x_label)
        .y_axis_label(y_label)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            // Detect the target for opening the detail modal on point click.
            let resp = plot_ui.response();
            if resp.clicked_by(egui::PointerButton::Primary) {
                clicked_detail = resp
                    .interact_pointer_pos()
                    .and_then(|pos| hit_test_nearest(plot_ui, hit_candidates, pos, HIT_THRESHOLD));
            }
            // Draw infeasible solutions in the back
            if has_infeasible {
                let pts: Vec<[f64; 2]> = infeasible.iter().map(|&(x, y)| [x, y]).collect();
                plot_ui.points(
                    egui_plot::Points::new("Infeasible", pts)
                        .color(crate::theme::chart_colors::COLOR_INFEASIBLE())
                        .radius(3.0),
                );
            }
            // Outside the selection filter (gray, drawn in back; grouped under
            // "Others (unselected)" in the legend)
            if !dim_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Others (unselected)", dim_pts)
                        .color(COLOR_UNSELECTED_POINT())
                        .radius(2.5),
                );
            }
            // Unranked (gray)
            if !none_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Others", none_pts)
                        .color(COLOR_MCDM_NONE())
                        .radius(3.0),
                );
            }
            // Ranked: dark (lower rank) to light (higher rank)
            for (color, pts) in color_draw {
                plot_ui.points(egui_plot::Points::new("", pts).color(color).radius(4.0));
            }
            // Legend-only entries (no data, name only)
            plot_ui.points(
                egui_plot::Points::new("Rank 1 (Best)", Vec::<[f64; 2]>::new())
                    .color(best_color)
                    .radius(5.0),
            );
            if top_n > 1 {
                plot_ui.points(
                    egui_plot::Points::new(format!("Rank {top_n}"), Vec::<[f64; 2]>::new())
                        .color(worst_color)
                        .radius(5.0),
                );
            }
        });
    clicked_detail
}

/// Computes candidates for hit testing (trial_id, row index, coordinates).
/// Only covers points with finite values drawn in the scatter plot (feasible or
/// infeasible).
fn compute_hit_candidates(
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
    x_axis: &str,
    y_axis: &str,
) -> Vec<(u32, usize, [f64; 2])> {
    let (Ok(x_vals), Ok(y_vals)) = (
        extract_axis_values(x_axis, mcdm_result, view, obj_names),
        extract_axis_values(y_axis, mcdm_result, view, obj_names),
    ) else {
        return Vec::new();
    };
    (0..view.row_count())
        .filter_map(|i| {
            let x = x_vals.get(i).copied()?;
            let y = y_vals.get(i).copied()?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            Some((trial_id, i, [x, y]))
        })
        .collect()
}

// ──────────────────────────────────────────────────────────────
// Axis option generation
// ──────────────────────────────────────────────────────────────

/// Generates the available axis options from the MCDM result
pub(crate) fn get_axis_options(mcdm_result: &McdmResult, obj_names: &[String]) -> Vec<AxisOption> {
    let mut options = Vec::with_capacity(obj_names.len() + 5);

    // Objective function options
    for (i, name) in obj_names.iter().enumerate() {
        options.push(AxisOption {
            id: format!("Objective{}", i),
            label: format!("Objective {} ({})", i, name),
        });
    }

    // Score options per MCDM method
    match mcdm_result {
        McdmResult::Vikor(_) => {
            for (id, label) in [
                (AXIS_VIKOR_Q, "VIKOR Q Score"),
                (AXIS_VIKOR_S, "VIKOR S Value"),
                (AXIS_VIKOR_R, "VIKOR R Value"),
            ] {
                options.push(AxisOption {
                    id: id.to_string(),
                    label: label.to_string(),
                });
            }
        }
        McdmResult::Topsis(_) => {
            options.push(AxisOption {
                id: AXIS_TOPSIS_SCORE.to_string(),
                label: "TOPSIS Score".to_string(),
            });
        }
        McdmResult::PrometheeI(_) | McdmResult::PrometheeII(_) => {
            for (id, label) in [
                (AXIS_PHI_PLUS, "Phi+ (Positive Flow)"),
                (AXIS_PHI_MINUS, "Phi- (Negative Flow)"),
                (AXIS_PHI_NET, "Phi Net"),
            ] {
                options.push(AxisOption {
                    id: id.to_string(),
                    label: label.to_string(),
                });
            }
        }
    }

    options
}

// ──────────────────────────────────────────────────────────────
// Axis value extraction
// ──────────────────────────────────────────────────────────────

/// Extracts each trial's value for the given axis identifier
pub(crate) fn extract_axis_values(
    axis_id: &str,
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
) -> Result<Vec<f64>, String> {
    // For the objective function "Objective{N}"
    if let Some(idx_str) = axis_id.strip_prefix("Objective") {
        let idx: usize = idx_str
            .parse()
            .map_err(|_| format!("Invalid objective index in axis: '{}'", axis_id))?;
        let obj_name = obj_names
            .get(idx)
            .ok_or_else(|| format!("Objective index {} out of range", idx))?;
        let values = view
            .numeric_column(obj_name)
            .map(|col| col.to_vec())
            .unwrap_or_else(|| vec![f64::NAN; view.row_count()]);
        return Ok(values);
    }

    // Score per MCDM method (independent of view)
    match mcdm_result {
        McdmResult::Vikor(r) => {
            if axis_id == AXIS_VIKOR_Q {
                Ok(r.q_values.clone())
            } else if axis_id == AXIS_VIKOR_S {
                Ok(r.s_values.clone())
            } else if axis_id == AXIS_VIKOR_R {
                Ok(r.r_values.clone())
            } else {
                Err(format!("Unknown axis '{}' for VIKOR result", axis_id))
            }
        }
        McdmResult::Topsis(r) => {
            if axis_id == AXIS_TOPSIS_SCORE {
                Ok(r.scores.clone())
            } else {
                Err(format!("Unknown axis '{}' for TOPSIS result", axis_id))
            }
        }
        McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
            if axis_id == AXIS_PHI_PLUS {
                Ok(r.phi_plus.clone())
            } else if axis_id == AXIS_PHI_MINUS {
                Ok(r.phi_minus.clone())
            } else if axis_id == AXIS_PHI_NET {
                Ok(r.phi_net.clone())
            } else {
                Err(format!("Unknown axis '{}' for PROMETHEE result", axis_id))
            }
        }
    }
}

/// Builds a `trial_idx -> rank` reverse lookup map (shared by the 2D/3D scatter plots ·
/// D-6). Since `ranked_indices[rank] = trial_idx`, a reverse lookup is needed.
pub(crate) fn build_rank_map(ranked_indices: &[u32], n_trials: usize) -> Vec<usize> {
    let mut rank_map = vec![usize::MAX; n_trials];
    for (rank, &trial_idx) in ranked_indices.iter().enumerate() {
        let idx = trial_idx as usize;
        if idx < n_trials {
            rank_map[idx] = rank;
        }
    }
    rank_map
}

/// MCDM rank -> scatter point color (shared by the 2D/3D scatter plots · D-6).
/// If `rank` (the value from `build_rank_map`; `usize::MAX` when outside the ranking) is
/// less than `colored_range`, returns a continuous colormap color (rank 0 = best -> t=1.0);
/// otherwise returns gray (`COLOR_MCDM_NONE`).
pub(crate) fn mcdm_rank_color(rank: usize, colored_range: usize, colormap: &ColorMap) -> Color32 {
    if rank == usize::MAX || rank >= colored_range {
        COLOR_MCDM_NONE()
    } else {
        let t = if colored_range > 1 {
            1.0 - rank as f32 / (colored_range - 1) as f32
        } else {
            1.0
        };
        colormap.interpolate(t)
    }
}

/// Returns the default axis ID to use when axis selection becomes invalid (shared by the
/// 2D/3D scatter plots · D-6). The `nth` option, or the first one if unavailable, or an
/// empty string if there are none at all.
pub(crate) fn fallback_axis_id(options: &[AxisOption], nth: usize) -> String {
    options
        .get(nth)
        .or_else(|| options.first())
        .map(|o| o.id.clone())
        .unwrap_or_default()
}

// ──────────────────────────────────────────────────────────────
// Scatter point computation
// ──────────────────────────────────────────────────────────────

/// A single scatter plot point: (x coordinate, y coordinate, color, trial_id).
/// `trial_id` is used to determine graying-out for the selection filter (PCP brush, etc.).
type ScatterPoint = (f64, f64, Color32, u32);
/// Return type alias for `compute_scatter_points`.
type ScatterPointsResult = (Vec<ScatterPoint>, Vec<(f64, f64)>, ScatterMetadata);

/// Computes the MCDM scatter plot points
/// - Extract axis values -> continuous coloring via colormap
/// - Return value: (feasible points, infeasible points, metadata)
pub(crate) fn compute_scatter_points(
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
    x_axis: &str,
    y_axis: &str,
    colormap: &ColorMap,
    top_n: usize,
) -> Result<ScatterPointsResult, String> {
    let n_trials = view.row_count();
    if n_trials == 0 {
        return Ok((
            vec![],
            vec![],
            ScatterMetadata {
                total_trials: 0,
                compute_time_ms: 0.0,
            },
        ));
    }

    let x_vals = extract_axis_values(x_axis, mcdm_result, view, obj_names)?;
    let y_vals = extract_axis_values(y_axis, mcdm_result, view, obj_names)?;
    let feas = view.feasibility();

    let ranked = mcdm_result.ranked_indices();
    let rank_map = build_rank_map(ranked, n_trials);
    // Assign color contours within the top_n range, ensuring at least 1
    let colored_range = top_n.max(1);

    let mut feasible_pts: Vec<ScatterPoint> = Vec::with_capacity(n_trials);
    let mut infeasible_pts: Vec<(f64, f64)> = Vec::new();

    for (i, &rank) in rank_map.iter().enumerate() {
        let x = match x_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };
        let y = match y_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };

        if !feas.is_feasible(i) {
            infeasible_pts.push((x, y));
            continue;
        }
        // Rank -> color (colormap within top_n, gray outside; shared with 3D · D-6)
        let color = mcdm_rank_color(rank, colored_range, colormap);
        let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
        feasible_pts.push((x, y, color, trial_id));
    }

    let total = feasible_pts.len() + infeasible_pts.len();
    Ok((
        feasible_pts,
        infeasible_pts,
        ScatterMetadata {
            total_trials: total,
            compute_time_ms: 0.0,
        },
    ))
}

// ──────────────────────────────────────────────────────────────
// Unit tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::{PrometheeResult, TopsisResult, VikorResult};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

    // ── Test helpers ──────────────────────────────────────────

    fn make_view_with_objectives(objective_rows: &[Vec<f64>]) -> (StudyView, Vec<String>) {
        let n = objective_rows.len();
        if n == 0 {
            let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
            return (StudyView::new(Arc::new(df), vec![]), vec![]);
        }
        let n_obj = objective_rows[0].len();
        let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: objective_rows[i].clone(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
        (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
    }

    fn make_empty_view() -> StudyView {
        let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        StudyView::new(Arc::new(df), vec![])
    }

    fn make_vikor(n: usize) -> VikorResult {
        let values: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
        VikorResult {
            s_values: values.clone(),
            r_values: values.clone(),
            q_values: values.clone(),
            display_scores: values.iter().map(|v| 1.0 - v).collect(),
            ranked_indices: (0..n as u32).collect(),
            compromise_indices: if n > 0 { vec![0] } else { vec![] },
            duration_ms: 1.0,
        }
    }

    fn make_vikor_result(n: usize) -> McdmResult {
        McdmResult::Vikor(make_vikor(n))
    }

    fn make_topsis(n: usize) -> TopsisResult {
        TopsisResult {
            scores: (0..n).map(|i| i as f64 / n as f64).collect(),
            ranked_indices: (0..n as u32).rev().collect(),
            duration_ms: 1.0,
        }
    }

    fn make_promethee(n: usize) -> PrometheeResult {
        let v: Vec<f64> = (0..n).map(|i| i as f64 * 0.05).collect();
        PrometheeResult {
            phi_plus: v.clone(),
            phi_minus: v.iter().map(|x| 1.0 - x).collect(),
            phi_net: v.clone(),
            ranked_indices_i: (0..n as u32).collect(),
            ranked_indices_ii: (0..n as u32).collect(),
            incomparable_counts: vec![0; n],
            duration_ms: 1.0,
        }
    }

    // ── Struct / initialization tests ─────────────────────────────────────

    #[test]
    fn test_scatter_chart_default_values() {
        let chart = McdmScatterChart::default();
        assert_eq!(chart.x_axis, "Objective0");
        assert_eq!(chart.y_axis, "Objective1");
        assert!(chart.display_batches.is_none());
        assert!(chart.cache_key.is_none());
        assert!(chart.error_message.is_none());
    }

    #[test]
    fn test_cache_stale_when_no_key() {
        use crate::state::types::ColormapName;
        let chart = McdmScatterChart::default();
        assert!(chart.is_cache_stale(
            100,
            &McdmResult::Topsis(make_topsis(100)),
            &ColormapName::Viridis,
            10
        ));
    }

    #[test]
    fn test_cache_stale_when_trial_count_changes() {
        use crate::state::types::ColormapName;
        let cmap_name = ColormapName::Viridis;
        let mut chart = McdmScatterChart::default();
        let result = McdmResult::Topsis(make_topsis(100));
        chart.cache_key = Some(chart.make_cache_key(100, &result, &cmap_name, 10));
        assert!(chart.is_cache_stale(150, &result, &cmap_name, 10)); // 150 ≠ 100
    }

    #[test]
    fn test_cache_not_stale_same_key() {
        use crate::state::types::ColormapName;
        let cmap_name = ColormapName::Viridis;
        let mut chart = McdmScatterChart::default();
        let result = McdmResult::Topsis(make_topsis(100));
        chart.cache_key = Some(chart.make_cache_key(100, &result, &cmap_name, 10));
        assert!(!chart.is_cache_stale(100, &result, &cmap_name, 10));
    }

    // ── get_axis_options tests ──────────────────────────────────

    #[test]
    fn test_axis_options_vikor_has_scores() {
        let result = McdmResult::Vikor(make_vikor(3));
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let options = get_axis_options(&result, &obj_names);

        assert!(options.iter().any(|o| o.id == "Objective0"));
        assert!(options.iter().any(|o| o.id == "Objective1"));
        assert!(options.iter().any(|o| o.id == "VIKOR_Q"));
        assert!(options.iter().any(|o| o.id == "VIKOR_S"));
        assert!(options.iter().any(|o| o.id == "VIKOR_R"));
    }

    #[test]
    fn test_axis_options_topsis() {
        let result = McdmResult::Topsis(make_topsis(3));
        let options = get_axis_options(&result, &["obj".to_string()]);
        assert!(options.iter().any(|o| o.id == "TOPSIS_Score"));
        assert!(!options.iter().any(|o| o.id == "VIKOR_Q"));
    }

    #[test]
    fn test_axis_options_promethee() {
        let result = McdmResult::PrometheeI(make_promethee(3));
        let options = get_axis_options(&result, &[]);
        assert!(options.iter().any(|o| o.id == "Phi+"));
        assert!(options.iter().any(|o| o.id == "Phi-"));
        assert!(options.iter().any(|o| o.id == "Phi_Net"));
    }

    #[test]
    fn test_axis_options_empty_objectives() {
        let result = McdmResult::Topsis(make_topsis(3));
        let options = get_axis_options(&result, &[]);
        // Only TOPSIS_Score
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].id, "TOPSIS_Score");
    }

    // ── extract_axis_values tests ────────────────────────────────

    #[test]
    fn test_extract_objective0() {
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        let result = McdmResult::Vikor(make_vikor(2));
        let vals = extract_axis_values("Objective0", &result, &view, &obj_names).unwrap();
        assert_eq!(vals, vec![1.0, 3.0]);
    }

    #[test]
    fn test_extract_objective1() {
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        let result = McdmResult::Vikor(make_vikor(2));
        let vals = extract_axis_values("Objective1", &result, &view, &obj_names).unwrap();
        assert_eq!(vals, vec![2.0, 4.0]);
    }

    #[test]
    fn test_extract_vikor_q() {
        let vikor = make_vikor(3);
        let q = vikor.q_values.clone();
        let result = McdmResult::Vikor(vikor);
        let view = make_empty_view();
        let vals = extract_axis_values("VIKOR_Q", &result, &view, &[]).unwrap();
        assert_eq!(vals, q);
    }

    #[test]
    fn test_extract_topsis_score() {
        let topsis = make_topsis(3);
        let scores = topsis.scores.clone();
        let result = McdmResult::Topsis(topsis);
        let view = make_empty_view();
        let vals = extract_axis_values("TOPSIS_Score", &result, &view, &[]).unwrap();
        assert_eq!(vals, scores);
    }

    #[test]
    fn test_extract_phi_plus() {
        let promethee = make_promethee(3);
        let phi_plus = promethee.phi_plus.clone();
        let result = McdmResult::PrometheeI(promethee);
        let view = make_empty_view();
        let vals = extract_axis_values("Phi+", &result, &view, &[]).unwrap();
        assert_eq!(vals, phi_plus);
    }

    #[test]
    fn test_extract_unknown_axis_error() {
        let result = McdmResult::Vikor(make_vikor(3));
        let view = make_empty_view();
        let err = extract_axis_values("NonExistent", &result, &view, &[]);
        assert!(err.is_err());
    }

    #[test]
    fn test_extract_out_of_range_objective() {
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0]]);
        let result = McdmResult::Vikor(make_vikor(1));
        // obj_names is only ["obj0"]. Objective5 is out of range -> error
        let err = extract_axis_values("Objective5", &result, &view, &obj_names);
        assert!(err.is_err());
    }

    // ── build_rank_map tests ────────────────────────────────────

    #[test]
    fn test_build_rank_map_basic() {
        let ranked: Vec<u32> = vec![5, 2, 8];
        let map = build_rank_map(&ranked, 10);
        assert_eq!(map[5], 0);
        assert_eq!(map[2], 1);
        assert_eq!(map[8], 2);
        assert_eq!(map[0], usize::MAX); // outside the ranking
        assert_eq!(map[3], usize::MAX);
    }

    #[test]
    fn test_build_rank_map_all_trials() {
        let n = 5usize;
        let ranked: Vec<u32> = vec![4, 3, 2, 1, 0];
        let map = build_rank_map(&ranked, n);
        assert_eq!(map[4], 0); // trial 4 is rank 0 (best)
        assert_eq!(map[0], 4); // trial 0 is rank 4 (worst)
    }

    // ── compute_scatter_points integration tests ─────────────────────────

    #[test]
    fn test_compute_scatter_points_basic() {
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let n = 10;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (n - i) as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, meta) = compute_scatter_points(
            &result,
            &view,
            &obj_names,
            "Objective0",
            "Objective1",
            &cmap,
            n,
        )
        .unwrap();

        assert_eq!(points.len(), n);
        assert_eq!(meta.total_trials, n);
        assert!((points[0].0 - 0.0).abs() < 1e-10);
        assert!((points[0].1 - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_scatter_points_rank0_gets_best_color() {
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let n = 20;
        let top_n = 10_usize;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, i as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, _) = compute_scatter_points(
            &result,
            &view,
            &obj_names,
            "Objective0",
            "Objective1",
            &cmap,
            top_n,
        )
        .unwrap();

        // rank 0 (best) -> t=1.0 -> top end of the colormap
        let expected = cmap.interpolate(1.0);
        assert_eq!(points[0].2, expected);
        // Outside top_n (rank >= top_n) is gray
        assert_eq!(points[n - 1].2, COLOR_MCDM_NONE());
    }

    #[test]
    fn test_compute_scatter_points_empty_trials() {
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let vikor = make_vikor(0);
        let result = McdmResult::Vikor(vikor);
        let view = make_empty_view();
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, meta) =
            compute_scatter_points(&result, &view, &[], "Objective0", "Objective1", &cmap, 10)
                .unwrap();
        assert!(points.is_empty());
        assert_eq!(meta.total_trials, 0);
    }

    #[test]
    fn test_compute_scatter_points_vikor_axis() {
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let n = 5;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, _) =
            compute_scatter_points(&result, &view, &obj_names, "VIKOR_Q", "VIKOR_S", &cmap, n)
                .unwrap();

        assert_eq!(points.len(), n);
    }
}
