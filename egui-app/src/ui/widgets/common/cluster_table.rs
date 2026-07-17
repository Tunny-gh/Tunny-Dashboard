use std::collections::BTreeMap;

use crate::state::app_state::AppState;
use crate::state::results::ClusterResult;
use crate::theme::chart_colors::COLOR_LINK;
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::cluster_scatter::{
    ClusterCacheKey, ClusterComputeRequest, ClusterSpace, KMeansInitStrategy, KSelectionMode,
};
use crate::ui::widgets::common::cluster_controls::ClusterControls;

/// The cluster assignment table widget.
/// Lists the clustering result (which cluster each trial belongs to).
/// Rows can be highlighted by clicking and pinned via 📌 (the same feel as TrialTable).
///
/// Like 2D / 3D, it has its own clustering settings (k / target space / mode / Init),
/// and results are shared/cached per settings key in `app_state.cluster_cache`.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClusterTable {
    /// Whether to also show solutions outside the clustering target (non-Pareto-front)
    pub show_unclustered: bool,
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    /// Upper bound of k searched in Elbow (automatic) mode.
    pub elbow_max_k: usize,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub pending_compute: Option<ClusterComputeRequest>,
    #[serde(skip)]
    pub last_error: Option<crate::state::messages::ClusterUiError>,
}

impl Default for ClusterTable {
    fn default() -> Self {
        Self {
            show_unclustered: false,
            k: 3,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
            computing: false,
            pending_compute: None,
            last_error: None,
        }
    }
}

impl ClusterTable {
    /// Returns the cache key corresponding to the current settings.
    pub fn cache_key(&self) -> ClusterCacheKey {
        ClusterCacheKey::new(
            self.target_space,
            self.k_mode,
            self.k,
            self.init_strategy,
            self.elbow_max_k,
        )
    }

    /// Renders the table
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState, colormap: &ColorMap) {
        let Some(study_ctx) = app_state.current_study.as_ref() else {
            ui.centered_and_justified(|ui| {
                ui.label("Open a journal file");
            });
            return;
        };

        let view = &study_ctx.view;
        let n = view.row_count();
        // The clustering target is determined by the count of Pareto-front solutions
        // (pareto_rank == 0).
        let pareto_count = view.pareto_rank.iter().filter(|&&r| r == 0).count();

        self.show_controls(ui, pareto_count);

        if let Some(err) = self.last_error.clone() {
            ui.label(egui::RichText::new(&err.user_message).color(ERROR_COLOR()));
            if let Some(detail) = &err.detail_for_dev {
                ui.label(egui::RichText::new(detail).small().weak());
            }
            if err.retryable && ui.button("Retry").clicked() {
                self.try_queue_compute(pareto_count);
            }
            ui.separator();
        }

        if self.computing {
            return;
        }

        let key = self.cache_key();
        let Some(cr) = app_state.cluster_cache.get(&key) else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Clustering has not been run yet.").weak());
            });
            return;
        };

        if cr.labels.len() != n {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Cluster result is inconsistent. Please run clustering again.",
                    )
                    .color(ERROR_COLOR()),
                );
            });
            return;
        }

        // Aggregate counts per cluster (label < 0 means unclustered)
        let counts = cluster_counts(&cr.labels);

        self.show_header(ui, cr, &counts);

        // Determine the row indices to display (cluster order -> trial order)
        let visible = visible_indices(&cr.labels, self.show_unclustered);
        if visible.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No clustered trials to display.").weak());
            });
            return;
        }

        let param_names = study_ctx.meta.param_names.clone();
        let obj_names = study_ctx.meta.objective_names.clone();
        let param_cols = view.numeric_columns(&param_names);
        let obj_cols = view.numeric_columns(&obj_names);
        let trial_ids = &view.trial_ids;
        let pareto_rank = &view.pareto_rank;

        let pinned = app_state.pinned_trials.clone();
        let highlighted = app_state.highlighted_trial;

        let n_clusters = cr.n_clusters.max(1);
        let cluster_color = |label: i32| -> egui::Color32 {
            if label < 0 {
                return crate::theme::TEXT_SECONDARY();
            }
            let t = if n_clusters == 1 {
                0.5
            } else {
                label as f32 / (n_clusters - 1) as f32
            };
            colormap.interpolate(t)
        };

        use egui_extras::{Column, TableBuilder};

        let mut clicked_trial: Option<u32> = None;
        let mut pin_toggled: Option<u32> = None;

        // Expand parameters and objectives into one column each, allowing horizontal
        // scrolling. egui_extras's Table has no built-in horizontal scroll, so wrap
        // fixed-width columns in a horizontal ScrollArea and display every column
        // individually instead of cramming them into one cell.
        egui::ScrollArea::horizontal().show(ui, |ui| {
            // Emphasize the stripe color to make it easy to tell even/odd rows apart.
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG();
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::exact(30.0)) // Pin column
                .column(Column::initial(70.0).at_least(50.0)) // Cluster
                .column(Column::initial(70.0).at_least(50.0)) // Trial ID
                .columns(Column::initial(90.0).at_least(50.0), param_names.len()) // per variable
                .columns(Column::initial(90.0).at_least(50.0), obj_names.len()) // per objective
                .column(Column::initial(90.0).at_least(50.0)) // Pareto Rank
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("📌");
                    });
                    header.col(|ui| {
                        ui.strong("Cluster");
                    });
                    header.col(|ui| {
                        ui.strong("Trial ID");
                    });
                    for name in &param_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    for name in &obj_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    header.col(|ui| {
                        ui.strong("Pareto Rank");
                    });
                })
                .body(|body| {
                    body.rows(18.0, visible.len(), |mut row| {
                        let idx = visible[row.index()];
                        let trial_id = trial_ids.get(idx).copied().unwrap_or(idx as u32);
                        let label = cr.labels.get(idx).copied().unwrap_or(-1);
                        let rank = pareto_rank.get(idx).copied().unwrap_or(0);
                        let is_highlighted = highlighted == Some(trial_id);
                        let is_pinned = pinned.contains(&trial_id);

                        row.col(|ui| {
                            let pin_label = if is_pinned { "📌" } else { "·" };
                            if ui.small_button(pin_label).clicked() {
                                pin_toggled = Some(trial_id);
                            }
                        });
                        row.col(|ui| {
                            let text = if label < 0 {
                                "—".to_string()
                            } else {
                                label.to_string()
                            };
                            let color = cluster_color(label);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(10.0, 10.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2.0, color);
                                ui.label(text);
                            });
                        });
                        row.col(|ui| {
                            let res = ui.selectable_label(is_highlighted, trial_id.to_string());
                            if res.clicked() {
                                clicked_trial = Some(trial_id);
                            }
                            if is_highlighted {
                                ui.painter().rect_filled(res.rect, 0.0, COLOR_LINK());
                            }
                        });
                        for col in &param_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.3}", v));
                            });
                        }
                        for col in &obj_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.4}", v));
                            });
                        }
                        row.col(|ui| {
                            ui.label(rank.to_string());
                        });
                    });
                });
        });

        if let Some(trial_id) = clicked_trial {
            app_state.set_highlight(trial_id);
        }
        if let Some(trial_id) = pin_toggled {
            let _ = app_state.toggle_pinned_trial(trial_id);
        }
    }

    fn show_header(
        &mut self,
        ui: &mut egui::Ui,
        cr: &ClusterResult,
        counts: &BTreeMap<i32, usize>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(format!("k = {}", cr.n_clusters)).strong());
            ui.separator();
            for (&label, &count) in counts {
                if label < 0 {
                    continue;
                }
                ui.label(format!("Cluster {label}: {count}"));
            }
            if let Some(&unclustered) = counts.get(&-1) {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Unclustered: {unclustered}"))
                        .color(crate::theme::TEXT_SECONDARY()),
                );
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_unclustered, "Show Unclustered");
        });
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

    /// Renders the clustering settings UI (k / mode / space / Init / Run).
    /// Same feel as 2D's ClusterScatter::show_header.
    fn show_controls(&mut self, ui: &mut egui::Ui, pareto_count: usize) {
        self.controls()
            .show_controls(ui, pareto_count, "cluster_table", true);
    }

    fn try_queue_compute(&mut self, pareto_count: usize) {
        self.controls().try_queue_compute(pareto_count);
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
    /// Since compute results are aggregated into `app_state.cluster_cache`, reflect the
    /// completion state into every canvas item (an independent WidgetStates) too. Keeps
    /// display settings as-is.
    pub fn adopt_runtime_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.pending_compute = src.pending_compute.clone();
        self.last_error = src.last_error.clone();
    }
}

/// Aggregates the count per cluster (key: label, value: count. -1 means unclustered).
fn cluster_counts(labels: &[i32]) -> BTreeMap<i32, usize> {
    let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
    for &label in labels {
        *counts.entry(label).or_insert(0) += 1;
    }
    counts
}

/// Returns the row indices to display, in "cluster order -> trial order."
/// If `show_unclustered` is false, rows with label < 0 are excluded.
fn visible_indices(labels: &[i32], show_unclustered: bool) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..labels.len())
        .filter(|&i| {
            let label = labels[i];
            show_unclustered || label >= 0
        })
        .collect();
    // Group unclustered (-1) at the end, so use (sort_label, index) as the sort key.
    indices.sort_by_key(|&i| {
        let label = labels[i];
        let sort_label = if label < 0 { i32::MAX } else { label };
        (sort_label, i)
    });
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_table_default_hides_unclustered() {
        let table = ClusterTable::default();
        assert!(!table.show_unclustered);
    }

    #[test]
    fn cluster_counts_aggregates_per_label() {
        let labels = vec![0, 1, 0, 2, 1, 0, -1];
        let counts = cluster_counts(&labels);
        assert_eq!(counts.get(&0), Some(&3));
        assert_eq!(counts.get(&1), Some(&2));
        assert_eq!(counts.get(&2), Some(&1));
        assert_eq!(counts.get(&-1), Some(&1));
    }

    #[test]
    fn visible_indices_excludes_unclustered_by_default() {
        let labels = vec![0, -1, 1, -1, 0];
        let visible = visible_indices(&labels, false);
        // Indices 1, 3 with -1 are excluded
        assert_eq!(visible, vec![0, 4, 2]);
    }

    #[test]
    fn visible_indices_includes_unclustered_when_requested() {
        let labels = vec![0, -1, 1, -1, 0];
        let visible = visible_indices(&labels, true);
        // Cluster order (0,0,1) is followed by unclustered (-1,-1)
        assert_eq!(visible, vec![0, 4, 2, 1, 3]);
    }

    #[test]
    fn visible_indices_sorts_by_cluster_then_trial() {
        let labels = vec![2, 0, 1, 0, 2];
        let visible = visible_indices(&labels, false);
        assert_eq!(visible, vec![1, 3, 2, 0, 4]);
    }

    #[test]
    fn visible_indices_empty_when_all_unclustered_and_hidden() {
        let labels = vec![-1, -1, -1];
        let visible = visible_indices(&labels, false);
        assert!(visible.is_empty());
    }
}
