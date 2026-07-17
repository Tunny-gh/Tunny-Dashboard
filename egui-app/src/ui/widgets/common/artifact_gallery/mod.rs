use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::AppState;
use crate::theme::colormap_name::colormap_from_name;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::cluster_scatter::{
    ClusterCacheKey, ClusterComputeRequest, ClusterSpace, KMeansInitStrategy, KSelectionMode,
};
use crate::ui::widgets::common::cluster_controls::ClusterControls;
use crate::ui::widgets::mcdm_chart::McdmControls;
use crate::ui::widgets::trial_detail_modal::{TrialDetailModal, TrialDetailTarget};

mod card;
mod sections;

use card::{image_uri, render_card_grid};
use sections::{
    artifact_trials_with_index, build_objective_labels, cluster_color, cluster_sections,
    filter_ids_by_selection, mcdm_ordered, paginate, restrict_to_current_study,
};

/// Number of artifact cards shown per page (All mode).
/// Limits how many `egui::Image`s are created at once, keeping texture generation cost down.
const PAGE_SIZE: usize = 12;

/// Thumbnail display size (large/medium/small). Holds the world-space side length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThumbSize {
    /// Small (default).
    Small,
    /// Medium.
    Medium,
    /// Large.
    Large,
}

impl ThumbSize {
    fn label(&self) -> &'static str {
        match self {
            ThumbSize::Small => "Small",
            ThumbSize::Medium => "Medium",
            ThumbSize::Large => "Large",
        }
    }

    /// Thumbnail side length (world-space coordinates).
    fn size(&self) -> f32 {
        match self {
            ThumbSize::Small => 140.0,
            ThumbSize::Medium => 220.0,
            ThumbSize::Large => 320.0,
        }
    }
}

/// Display mode for the artifact gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArtifactViewMode {
    /// Paginated display of all artifacts (no configuration needed).
    All,
    /// Grouped display by clustering result.
    Cluster,
    /// Display ordered by MCDM ranking.
    Mcdm,
}

impl ArtifactViewMode {
    fn label(&self) -> &'static str {
        match self {
            ArtifactViewMode::All => "All",
            ArtifactViewMode::Cluster => "By Cluster",
            ArtifactViewMode::Mcdm => "By MCDM Rank",
        }
    }
}

/// Artifact gallery widget.
///
/// Displays `app_state.artifact_map` (trial_id -> file path) in association with
/// clustering / MCDM results. This widget owns its own cluster / MCDM settings, and the
/// computed results are shared and cached per settings key via `cluster_cache` /
/// `mcdm_cache`.
///
/// This means switching the settings shows the corresponding results, and placing multiple
/// galleries with different settings lets you compare "settings A vs settings B" (the same
/// comparison style as the other Cluster/MCDM widgets).
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ArtifactGallery {
    pub mode: ArtifactViewMode,
    pub page: usize,
    pub thumb_size: ThumbSize,
    /// Which artifact index (0-based) to display when a trial has multiple artifacts.
    pub artifact_index: usize,
    /// Trial detail modal opened by clicking a card (shared with the scatter plots, etc.).
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
    /// List of `file://` URIs for which an image texture was generated in the most recent
    /// frame (used to free them when leaving the page).
    #[serde(skip)]
    displayed_uris: Vec<String>,
    /// The most recently rendered (view mode, page). When this changes, the previous
    /// page's textures are freed.
    #[serde(skip)]
    displayed_key: Option<(ArtifactViewMode, usize)>,
    // ── Cluster settings (same layout as ClusterTable) ──────────────
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    /// Upper bound of k explored in Elbow (automatic) mode.
    pub elbow_max_k: usize,
    #[serde(skip)]
    pub cluster_computing: bool,
    #[serde(skip)]
    pub cluster_pending: Option<ClusterComputeRequest>,
    #[serde(skip)]
    pub cluster_error: Option<crate::state::messages::ClusterUiError>,
    // ── MCDM settings (reuses the existing McdmControls) ──────────────
    pub mcdm: McdmControls,
}

impl Default for ArtifactGallery {
    fn default() -> Self {
        Self {
            mode: ArtifactViewMode::All,
            page: 0,
            thumb_size: ThumbSize::Small,
            artifact_index: 0,
            detail_modal: TrialDetailModal::new(),
            k: 3,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
            cluster_computing: false,
            cluster_pending: None,
            cluster_error: None,
            mcdm: McdmControls::default(),
            displayed_uris: Vec::new(),
            displayed_key: None,
        }
    }
}

impl ArtifactGallery {
    /// Cache key corresponding to the current cluster settings.
    pub fn cluster_cache_key(&self) -> ClusterCacheKey {
        ClusterCacheKey::new(
            self.target_space,
            self.k_mode,
            self.k,
            self.init_strategy,
            self.elbow_max_k,
        )
    }

    /// Assembles the bundle of mutable references to the settings/execution-state fields
    /// (for delegating to shared logic).
    fn cluster_controls(&mut self) -> ClusterControls<'_> {
        ClusterControls {
            k: &mut self.k,
            target_space: &mut self.target_space,
            k_mode: &mut self.k_mode,
            init_strategy: &mut self.init_strategy,
            elbow_max_k: &mut self.elbow_max_k,
            computing: &mut self.cluster_computing,
            pending_compute: &mut self.cluster_pending,
            last_error: &mut self.cluster_error,
        }
    }

    /// Queues a clustering run (equivalent to ClusterTable).
    fn try_queue_cluster_compute(&mut self, pareto_count: usize) {
        self.cluster_controls().try_queue_compute(pareto_count);
    }

    pub fn set_cluster_error(&mut self, err: crate::state::messages::ClusterUiError) {
        self.cluster_computing = false;
        self.cluster_error = Some(err);
    }

    pub fn clear_cluster_runtime(&mut self) {
        self.cluster_computing = false;
        self.cluster_pending = None;
        self.cluster_error = None;
    }

    /// Adopts the shared clustering execution state (computing / pending / error).
    /// Used to reflect completion status into each canvas item (display settings are kept
    /// as-is).
    pub fn adopt_cluster_runtime(&mut self, src: &Self) {
        self.cluster_computing = src.cluster_computing;
        self.cluster_pending = src.cluster_pending.clone();
        self.cluster_error = src.cluster_error.clone();
    }

    /// Renders the gallery.
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        if app_state.current_study.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Open a journal file");
            });
            return;
        }

        if app_state.artifact_map.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(
                        "No artifacts loaded. Use the \"Artifacts\" button to select a folder.",
                    )
                    .weak(),
                );
            });
            return;
        }

        // Maximum number of artifacts per trial (used as the range for the index selector).
        let max_artifacts = app_state
            .artifact_map
            .values()
            .map(|v| v.len())
            .max()
            .unwrap_or(1)
            .max(1);
        if self.artifact_index >= max_artifacts {
            self.artifact_index = max_artifacts - 1;
        }

        // Mode selector + artifact index selector.
        ui.horizontal(|ui| {
            ui.label("View:");
            egui::ComboBox::from_id_salt("artifact_gallery_mode")
                .selected_text(self.mode.label())
                .show_ui(ui, |ui| {
                    for m in [
                        ArtifactViewMode::All,
                        ArtifactViewMode::Cluster,
                        ArtifactViewMode::Mcdm,
                    ] {
                        if ui.selectable_value(&mut self.mode, m, m.label()).clicked() {
                            self.page = 0;
                        }
                    }
                });

            ui.separator();
            ui.label("Size:");
            egui::ComboBox::from_id_salt("artifact_gallery_thumb_size")
                .selected_text(self.thumb_size.label())
                .show_ui(ui, |ui| {
                    for s in [ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large] {
                        ui.selectable_value(&mut self.thumb_size, s, s.label());
                    }
                });

            // Only shown when a trial has more than one artifact.
            if max_artifacts > 1 {
                ui.separator();
                ui.label("Artifact #:");
                ui.add(egui::DragValue::new(&mut self.artifact_index).range(0..=max_artifacts - 1))
                    .on_hover_text(
                        "Which artifact to show per trial (0-based). \
                     Trials without this index are skipped.",
                    );
                ui.label(format!("of up to {max_artifacts}"));
            }
        });
        ui.separator();

        let artifact_index = self.artifact_index;

        // Precompute each trial's objective value labels (shown on cards as a hint for
        // judging quality).
        let obj_by_trial = build_objective_labels(app_state);

        // Inside the canvas's Area, `available_width` is effectively unbounded and
        // `horizontal_wrapped` won't wrap, so the widget's own width is fixed here and used
        // for the column count calculation.
        let content_w = ui.available_width();

        match self.mode {
            ArtifactViewMode::All => {
                self.show_all(ui, app_state, content_w, artifact_index, &obj_by_trial)
            }
            ArtifactViewMode::Cluster => {
                self.show_cluster(ui, app_state, content_w, artifact_index, &obj_by_trial)
            }
            ArtifactViewMode::Mcdm => {
                self.show_mcdm(ui, app_state, content_w, artifact_index, &obj_by_trial)
            }
        }

        // Render the trial detail modal opened by a card click (same content as the
        // scatter plots, etc.).
        if self.detail_modal.is_open() {
            if let Some(ctx) = app_state.current_study.as_ref() {
                self.detail_modal.show(
                    ui,
                    &ctx.view,
                    ctx.view.param_names(),
                    ctx.view.objective_names(),
                    &app_state.artifact_map,
                );
            }
        }
    }

    /// When the page / mode changes, frees the image textures generated for the previous
    /// page (preventing VRAM accumulation while paging), and records the URIs displayed in
    /// the current frame.
    fn refresh_displayed_textures(
        &mut self,
        ctx: &egui::Context,
        key: (ArtifactViewMode, usize),
        uris: Vec<String>,
    ) {
        if self.displayed_key != Some(key) {
            for uri in self.displayed_uris.drain(..) {
                ctx.forget_image(&uri);
            }
            self.displayed_key = Some(key);
        }
        self.displayed_uris = uris;
    }

    /// Applies the outcome of rendering the card grid (highlight request / detail modal
    /// request).
    fn apply_card_outcome(
        &mut self,
        app_state: &mut AppState,
        clicked: Option<u32>,
        detail: Option<TrialDetailTarget>,
    ) {
        if let Some(trial_id) = clicked {
            app_state.set_highlight(trial_id);
        }
        if let Some(target) = detail {
            self.detail_modal.open(target);
        }
    }

    /// All mode: paginated display of every trial with an artifact (each trial shows the
    /// selected index only).
    fn show_all(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
        obj_by_trial: &HashMap<u32, String>,
    ) {
        // `artifact_map` includes trials from the whole Journal (all Studies), so first
        // restrict to trials belonging to the current Study (`trial_id` is unique across
        // the whole Journal, so artifacts from other Studies can be mixed in). Then apply
        // the selection filter (PCP brush, etc.) on top (empty = all).
        let study_trials = restrict_to_current_study(
            artifact_trials_with_index(&app_state.artifact_map, artifact_index),
            app_state,
        );
        let trials = filter_ids_by_selection(study_trials, &app_state.selected_indices);
        let total_pages = trials.len().div_ceil(PAGE_SIZE).max(1);
        if self.page >= total_pages {
            self.page = total_pages - 1;
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.page > 0, egui::Button::new("◀ Prev"))
                .clicked()
            {
                self.page -= 1;
            }
            ui.label(format!("Page {}/{}", self.page + 1, total_pages));
            if ui
                .add_enabled(self.page + 1 < total_pages, egui::Button::new("Next ▶"))
                .clicked()
            {
                self.page += 1;
            }
            ui.separator();
            ui.label(format!("{} trials with artifacts", trials.len()));
            if !app_state.selected_indices.is_empty() {
                ui.separator();
                ui.label(egui::RichText::new("filtered by selection").small().weak());
            }
        });

        let page_trials = paginate(&trials, self.page, PAGE_SIZE);
        let thumb = self.thumb_size.size();
        let mut cards: Vec<(u32, String, &ArtifactEntry)> = Vec::new();
        for &trial_id in page_trials {
            if let Some(entry) = app_state
                .artifact_map
                .get(&trial_id)
                .and_then(|entries| entries.get(artifact_index))
            {
                cards.push((trial_id, String::new(), entry));
            }
        }
        // URIs of the image textures generated for this page. Used to decide what to free
        // when leaving the page.
        let page_uris: Vec<String> = cards.iter().filter_map(|(_, _, e)| image_uri(e)).collect();
        let mut clicked: Option<u32> = None;
        let mut detail: Option<TrialDetailTarget> = None;
        let ctx = ui.ctx().clone();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (h, p) =
                    render_card_grid(ui, app_state, content_w, thumb, &cards, obj_by_trial);
                clicked = h;
                detail = p;
            });
        self.refresh_displayed_textures(&ctx, (ArtifactViewMode::All, self.page), page_uris);
        self.apply_card_outcome(app_state, clicked, detail);
    }

    /// Cluster mode: settings UI + per-cluster sections.
    fn show_cluster(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
        obj_by_trial: &HashMap<u32, String>,
    ) {
        let pareto_count = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.pareto_rank.iter().filter(|&&r| r == 0).count())
            .unwrap_or(0);

        self.show_cluster_controls(ui, pareto_count);

        if let Some(err) = self.cluster_error.clone() {
            ui.label(egui::RichText::new(&err.user_message).color(ERROR_COLOR()));
            if err.retryable && ui.button("Retry").clicked() {
                self.try_queue_cluster_compute(pareto_count);
            }
            ui.separator();
        }

        if self.cluster_computing {
            return;
        }

        let key = self.cluster_cache_key();
        let Some(cr) = app_state.cluster_cache.get(&key) else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No clustering result for this setting. Press Run.").weak(),
                );
            });
            return;
        };

        let cmap = colormap_from_name(&app_state.selected_colormap);
        let trial_ids: &[u32] = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.trial_ids.as_slice())
            .unwrap_or(&[]);
        let sections = cluster_sections(cr, trial_ids, &app_state.artifact_map, artifact_index);
        if sections.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No clustered trials have artifacts.").weak());
            });
            return;
        }

        let thumb = self.thumb_size.size();
        let n_clusters = cr.n_clusters.max(1);

        // Flatten every cluster's members in section order and paginate them in PAGE_SIZE
        // chunks, same as All mode. Previously this rendered every cluster's every member
        // image unvirtualized every frame, which caused a large hang plus resident VRAM at
        // hundreds of images when switching views (M-15).
        let flat: Vec<(i32, u32, &ArtifactEntry)> = sections
            .iter()
            .flat_map(|(label, members)| members.iter().map(move |(tid, e)| (*label, *tid, *e)))
            .collect();
        let total_pages = flat.len().div_ceil(PAGE_SIZE).max(1);
        if self.page >= total_pages {
            self.page = total_pages - 1;
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.page > 0, egui::Button::new("◀ Prev"))
                .clicked()
            {
                self.page -= 1;
            }
            ui.label(format!("Page {}/{}", self.page + 1, total_pages));
            if ui
                .add_enabled(self.page + 1 < total_pages, egui::Button::new("Next ▶"))
                .clicked()
            {
                self.page += 1;
            }
            ui.separator();
            ui.label(format!("{} clustered trials with artifacts", flat.len()));
        });

        let page_items = paginate(&flat, self.page, PAGE_SIZE);
        // URIs of the image textures generated for this page (used to decide what to free
        // when leaving the page).
        let page_uris: Vec<String> = page_items
            .iter()
            .filter_map(|(_, _, e)| image_uri(e))
            .collect();
        let ctx = ui.ctx().clone();
        // Determine the column count using the width minus the header's indentation.
        let w = (content_w - 24.0).max(thumb);

        let mut clicked: Option<u32> = None;
        let mut detail: Option<TrialDetailTarget> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Group consecutive entries of the same cluster within the page, and render
                // a section heading + grid for each.
                let mut i = 0;
                while i < page_items.len() {
                    let label = page_items[i].0;
                    let start = i;
                    while i < page_items.len() && page_items[i].0 == label {
                        i += 1;
                    }
                    let group = &page_items[start..i];
                    let color = cluster_color(label, n_clusters, &cmap);
                    let badge = if label < 0 {
                        "·".to_string()
                    } else {
                        format!("C{label}")
                    };
                    let title = if label < 0 {
                        format!("Unclustered ({} shown)", group.len())
                    } else {
                        format!("Cluster {label} ({} shown)", group.len())
                    };
                    let cards: Vec<(u32, String, &ArtifactEntry)> = group
                        .iter()
                        .map(|(_, tid, e)| (*tid, badge.clone(), *e))
                        .collect();
                    egui::CollapsingHeader::new(egui::RichText::new(title).color(color))
                        .id_salt(("artifact_cluster_section", label))
                        .default_open(true)
                        .show(ui, |ui| {
                            let (h, p) =
                                render_card_grid(ui, app_state, w, thumb, &cards, obj_by_trial);
                            if h.is_some() {
                                clicked = h;
                            }
                            if p.is_some() {
                                detail = p;
                            }
                        });
                }
            });
        self.refresh_displayed_textures(&ctx, (ArtifactViewMode::Cluster, self.page), page_uris);
        self.apply_card_outcome(app_state, clicked, detail);
    }

    /// MCDM mode: settings UI + ranking-order display.
    fn show_mcdm(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        content_w: f32,
        artifact_index: usize,
        obj_by_trial: &HashMap<u32, String>,
    ) {
        let obj_names = app_state
            .current_study
            .as_ref()
            .map(|c| c.meta.objective_names.clone())
            .unwrap_or_default();

        if !self
            .mcdm
            .show_controls(ui, &obj_names, "artifact_gallery_mcdm")
        {
            return;
        }
        if self.mcdm.computing {
            return;
        }

        let key = self.mcdm.cache_key();
        let Some(result) = app_state.mcdm_cache.get(&key) else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No MCDM result for this setting. Press Run.").weak());
            });
            return;
        };

        let trial_ids: &[u32] = app_state
            .current_study
            .as_ref()
            .map(|c| c.view.trial_ids.as_slice())
            .unwrap_or(&[]);
        let ordered = mcdm_ordered(
            result,
            trial_ids,
            &app_state.artifact_map,
            artifact_index,
            self.mcdm.top_n.value(),
        );
        if ordered.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No ranked trials have artifacts.").weak());
            });
            return;
        }

        let thumb = self.thumb_size.size();
        let mut cards: Vec<(u32, String, &ArtifactEntry)> = Vec::new();
        for entry in &ordered {
            let badge = format!("#{} ({:.3})", entry.rank, entry.score);
            cards.push((entry.trial_id, badge, entry.entry));
        }
        // Mcdm mode is not paginated (top_n already limits the count for the user).
        // Only record the currently displayed URIs so textures can be freed on mode exit.
        let page_uris: Vec<String> = cards.iter().filter_map(|(_, _, e)| image_uri(e)).collect();
        let ctx = ui.ctx().clone();
        let mut clicked: Option<u32> = None;
        let mut detail: Option<TrialDetailTarget> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (h, p) =
                    render_card_grid(ui, app_state, content_w, thumb, &cards, obj_by_trial);
                clicked = h;
                detail = p;
            });
        self.refresh_displayed_textures(&ctx, (ArtifactViewMode::Mcdm, 0), page_uris);
        self.apply_card_outcome(app_state, clicked, detail);
    }

    /// Clustering settings UI (same feel as ClusterTable's `show_controls`).
    fn show_cluster_controls(&mut self, ui: &mut egui::Ui, pareto_count: usize) {
        self.cluster_controls()
            .show_controls(ui, pareto_count, "artifact_gallery", true);
        ui.separator();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adopt_cluster_runtime_clears_stuck_computing() {
        let mut item = ArtifactGallery {
            cluster_computing: true,
            ..Default::default()
        };
        let global = ArtifactGallery::default();
        item.adopt_cluster_runtime(&global);
        assert!(!item.cluster_computing);
    }
}
