use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::ConvergenceHistory;
use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_CONVERGENCE_LINE;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};
use tunny_core::indicators::MoIndicator;

/// One indicator-history series (legend name + color + data).
pub struct ConvergenceSeries {
    pub name: String,
    pub color: egui::Color32,
    pub history: ConvergenceHistory,
}

/// A request to change the reference point spec. `render_chart` reflects it into
/// app_state.
#[derive(Debug, Clone, PartialEq)]
pub enum RefPointChange {
    /// Revert to auto-computation (nadir + 10% margin).
    Auto,
    /// Specify a reference point per objective, in the original objective value units.
    Manual(Vec<f64>),
}

/// Multi-objective convergence indicator chart widget (HV / IGD+ / ε-indicator / R2)
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ConvergenceChart {
    #[serde(skip)]
    pub history: Option<ConvergenceHistory>,
    #[serde(skip)]
    pub computing: bool,
    /// Legend name of the base Study (shown to distinguish it from comparison series).
    #[serde(skip)]
    pub base_name: String,
    /// Objective names (used as the per-objective heading for reference point labels).
    #[serde(skip)]
    pub objective_names: Vec<String>,
    /// Comparison Study series overlaid on the same graph.
    #[serde(skip)]
    pub comparisons: Vec<ConvergenceSeries>,
    /// Current reference point spec (in original objective value units). `None` means
    /// auto-computation. Mirrored from app_state and the origin of UI operations.
    pub ref_point_override: Option<Vec<f64>>,
    /// A request to change the reference point spec (`render_chart` `.take()`s it and
    /// reflects it into app_state).
    #[serde(skip)]
    pub pending_ref_point: Option<RefPointChange>,
    /// The currently displayed convergence indicator (set from app_state by
    /// render_chart every frame).
    pub indicator: MoIndicator,
    /// A request to change the indicator (`render_chart` `.take()`s it and reflects it
    /// into app_state).
    #[serde(skip)]
    pub pending_indicator: Option<MoIndicator>,
    /// Per-objective input buffer (holds values being edited under Manual until
    /// committed).
    #[serde(skip)]
    ref_point_buf: Vec<f64>,
    /// Trial detail modal opened by clicking a point (shared with the scatter plot).
    #[serde(skip)]
    detail_modal: TrialDetailModal,
}

impl Default for ConvergenceChart {
    fn default() -> Self {
        Self {
            history: None,
            computing: false,
            base_name: String::new(),
            objective_names: Vec::new(),
            comparisons: Vec::new(),
            ref_point_override: None,
            pending_ref_point: None,
            indicator: MoIndicator::Hypervolume,
            pending_indicator: None,
            ref_point_buf: Vec::new(),
            detail_modal: TrialDetailModal::new(),
        }
    }
}

impl ConvergenceChart {
    /// Pulls only the running flag from the global widget (the just-processed
    /// authoritative state). Indicator data is aggregated into
    /// `app_state.convergence_history` and reflected every frame at render time, so each
    /// canvas item (an independent WidgetStates) only needs `computing` synced.
    /// Without this, the item's computing never drops after compute finishes and the
    /// spinner keeps spinning.
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
    }

    /// Builds a point list of (x=index×step, y=value) using `history`'s sampling step.
    /// The X axis is the sampling-order index × step (0, step, 2*step, …). trial_id
    /// isn't used because it may start partway through and thus not start at 0.
    fn to_points(history: &ConvergenceHistory) -> Vec<[f64; 2]> {
        let step = history.sample_step.max(1);
        history
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| [(i * step) as f64, v])
            .collect()
    }

    /// Renders the reference point controls (only when multi-objective + HV is
    /// selected). Checking Auto reverts to auto-computation; unchecking it allows
    /// entering per-objective numeric fields. `pending_ref_point` is only set when a
    /// value is committed (focus lost / drag stopped), triggering recomputation, to
    /// avoid continuous recomputation mid-edit.
    fn show_ref_point_controls(&mut self, ui: &mut egui::Ui) {
        let n_obj = self.objective_names.len();
        // HV is only meaningful for multi-objective. Don't show it for single-objective
        // or when not yet loaded.
        if n_obj < 2 {
            return;
        }

        let is_auto = self.ref_point_override.is_none();

        // Starting point for editing. Under Manual, use the override; under Auto, use
        // the reference point used in the most recent computation (or 0.0 if none) as
        // the initial value.
        let seed: Vec<f64> = if let Some(r) = &self.ref_point_override {
            let mut v = r.clone();
            v.resize(n_obj, 0.0);
            v
        } else {
            match &self.history {
                Some(h) if h.ref_point.len() == n_obj => h.ref_point.clone(),
                _ => vec![0.0; n_obj],
            }
        };
        if self.ref_point_buf.len() != n_obj {
            self.ref_point_buf = seed.clone();
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Reference point:")
                    .small()
                    .color(crate::theme::TEXT_SECONDARY()),
            );

            // Auto toggle
            let mut auto = is_auto;
            if ui.checkbox(&mut auto, "Auto").changed() {
                if auto {
                    self.pending_ref_point = Some(RefPointChange::Auto);
                } else {
                    // Switching to Manual: commit the current seed value as the initial
                    // spec.
                    self.ref_point_buf = seed.clone();
                    self.pending_ref_point = Some(RefPointChange::Manual(seed.clone()));
                }
            }

            // Per-objective numeric fields (editable only under Manual)
            ui.add_enabled_ui(!is_auto, |ui| {
                let mut commit = false;
                for (j, name) in self.objective_names.iter().enumerate() {
                    ui.label(
                        egui::RichText::new(name)
                            .small()
                            .color(crate::theme::TEXT_SECONDARY()),
                    );
                    let resp = ui.add(
                        egui::DragValue::new(&mut self.ref_point_buf[j])
                            .speed(0.1)
                            .max_decimals(6),
                    );
                    if resp.lost_focus() || resp.drag_stopped() {
                        commit = true;
                    }
                }
                if commit && !is_auto {
                    self.pending_ref_point =
                        Some(RefPointChange::Manual(self.ref_point_buf.clone()));
                }
            });
        });
    }

    /// Renders the convergence indicator chart.
    ///
    /// `view` / `param_names` / `artifact_map` are for the trial detail modal opened by
    /// clicking a point of the base Study. Points of comparison Studies have no
    /// corresponding row in the base Study's `view`, so they aren't clickable.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        // Don't render convergence indicators for single-objective (or when the
        // objective count isn't yet determined).
        if self.objective_names.len() < 2 {
            ui.label("Convergence indicators are defined only for multi-objective studies (≥2 objectives).");
            return;
        }

        // Lay out the indicator selector and supplementary info (direction, sampling
        // interval) on one line. Uses the space to the right of the combo box to save
        // vertical space.
        let mut new_indicator = self.indicator;
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("convergence_indicator")
                .selected_text(self.indicator.label())
                .show_ui(ui, |ui| {
                    for ind in MoIndicator::all() {
                        ui.selectable_value(&mut new_indicator, ind, ind.label());
                    }
                });

            // Direction (whether higher or lower is better)
            let direction_text = if self.indicator.higher_is_better() {
                "Higher is better"
            } else {
                "Lower is better"
            };
            ui.label(
                egui::RichText::new(direction_text)
                    .small()
                    .color(crate::theme::TEXT_SECONDARY()),
            );

            // Sampling interval (only when data is available)
            if !self.computing {
                if let Some(history) = &self.history {
                    let step = history.sample_step;
                    let sampling_label = if step <= 1 {
                        "Sampling: Every trial".to_string()
                    } else {
                        format!("Sampling: Every {step} trials")
                    };
                    ui.separator();
                    ui.label(
                        egui::RichText::new(sampling_label)
                            .small()
                            .color(crate::theme::TEXT_SECONDARY()),
                    );
                }
            }
        });
        if new_indicator != self.indicator {
            self.pending_indicator = Some(new_indicator);
        }

        // Show the reference point controls only when HV is selected.
        if self.indicator == MoIndicator::Hypervolume {
            self.show_ref_point_controls(ui);
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Computing {}...", self.indicator.label()));
            });
            return;
        }

        let Some(history) = &self.history else {
            ui.label(format!("No {} data", self.indicator.label()));
            return;
        };

        let base_points = Self::to_points(history);
        let base_label = if self.base_name.is_empty() {
            self.indicator.label().to_string()
        } else {
            self.base_name.clone()
        };

        // Build the base Study's points as (trial_id, row index, [x, y]) for hit
        // testing. Reuse `base_points` as-is so drawn points and coordinates match, and
        // resolve the row on `view` from trial_id (points that can't be resolved aren't
        // clickable). Build the trial_id -> row index reverse lookup map once, turning
        // a per-point linear scan (O(m·n)) into O(m+n) (M-17). For duplicate trial_ids,
        // keep the first row (preserving the old `position()` behavior).
        let mut row_by_trial_id: HashMap<u32, usize> = HashMap::with_capacity(view.trial_ids.len());
        for (row, &tid) in view.trial_ids.iter().enumerate() {
            row_by_trial_id.entry(tid).or_insert(row);
        }
        let base_hit_points: Vec<(u32, usize, [f64; 2])> = base_points
            .iter()
            .enumerate()
            .filter_map(|(i, &pt)| {
                let tid = *history.trial_ids.get(i)?;
                let row = *row_by_trial_id.get(&tid)?;
                Some((tid, row, pt))
            })
            .collect();

        // Precompute comparison series point lists (skip empty histories).
        let comparison_series: Vec<(&str, egui::Color32, Vec<[f64; 2]>)> = self
            .comparisons
            .iter()
            .filter(|s| !s.history.values.is_empty())
            .map(|s| (s.name.as_str(), s.color, Self::to_points(&s.history)))
            .collect();

        // The clicked point of the base Study (trial_id, row index, indicator value).
        let mut clicked_detail: Option<(u32, usize, f64)> = None;

        egui_plot::Plot::new("convergence_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .x_axis_label("Trial")
            .y_axis_label(self.indicator.label())
            .include_x(0.0)
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                // Clicking a point opens the trial detail modal (base Study points
                // only).
                let resp = plot_ui.response();
                if resp.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        if let Some((tid, row)) =
                            hit_test_nearest(plot_ui, &base_hit_points, pos, HIT_THRESHOLD)
                        {
                            let value = base_hit_points
                                .iter()
                                .find(|(t, _, _)| *t == tid)
                                .map(|(_, _, [_, y])| *y)
                                .unwrap_or(f64::NAN);
                            clicked_detail = Some((tid, row, value));
                        }
                    }
                }

                // Base Study
                if !base_points.is_empty() {
                    let color = COLOR_CONVERGENCE_LINE();
                    let line_pts: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.line(egui_plot::Line::new(&base_label, line_pts).color(color));
                    let scatter: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(&base_label, scatter)
                            .color(color)
                            .radius(3.0),
                    );
                }

                // Overlay comparison Studies with distinct colors
                for (name, color, points) in &comparison_series {
                    let line_pts: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.line(egui_plot::Line::new(*name, line_pts).color(*color));
                    let scatter: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(*name, scatter)
                            .color(*color)
                            .radius(3.0),
                    );
                }
            });

        // If a point was clicked, open the modal with the indicator name and value as
        // extra context.
        if let Some((trial_id, row, value)) = clicked_detail {
            let context = vec![(self.indicator.label().to_string(), format!("{value:.6}"))];
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        // Render the detail modal (the same shared implementation as the scatter plot).
        if self.detail_modal.is_open() {
            self.detail_modal
                .show(ui, view, param_names, obj_names, artifact_map);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::ConvergenceHistory;

    #[test]
    fn convergence_chart_default() {
        let chart = ConvergenceChart::default();
        assert!(chart.history.is_none());
        assert!(!chart.computing);
        // Default is Auto (no override) with no pending change.
        assert!(chart.ref_point_override.is_none());
        assert!(chart.pending_ref_point.is_none());
        // Default indicator is Hypervolume.
        assert_eq!(chart.indicator, MoIndicator::Hypervolume);
        assert!(chart.pending_indicator.is_none());
    }

    #[test]
    fn pending_ref_point_encodes_auto_and_manual() {
        // A change request is represented as one of two values: Auto / Manual(value).
        let to_auto = Some(RefPointChange::Auto);
        let to_manual = Some(RefPointChange::Manual(vec![1.0, 2.0]));
        assert!(matches!(to_auto, Some(RefPointChange::Auto)));
        assert!(matches!(to_manual, Some(RefPointChange::Manual(ref v)) if v == &[1.0, 2.0]));
    }

    #[test]
    fn adopt_compute_state_clears_stuck_computing() {
        // Pulling in computing=false from the global side after compute finishes drops
        // the computing flag on the item side that had been stuck showing a spinner.
        let mut item = ConvergenceChart {
            computing: true,
            ..Default::default()
        };
        let global = ConvergenceChart::default(); // computing=false
        item.adopt_compute_state(&global);
        assert!(!item.computing);
    }

    #[test]
    fn convergence_show_uses_index_times_step() {
        let history = ConvergenceHistory {
            trial_ids: vec![10000, 10050, 10100],
            values: vec![0.1, 0.5, 0.8],
            sample_step: 50,
            ref_point: vec![],
        };
        // x values should be 0, 50, 100 — not 10000, 10050, 10100
        let step = history.sample_step;
        let points: Vec<[f64; 2]> = history
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| [(i * step) as f64, v])
            .collect();
        assert_eq!(points[0][0], 0.0);
        assert_eq!(points[1][0], 50.0);
        assert_eq!(points[2][0], 100.0);
    }

    #[test]
    fn indicator_variants_accessible() {
        // Confirm all 4 indicators can be enumerated.
        let all = MoIndicator::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&MoIndicator::Hypervolume));
        assert!(all.contains(&MoIndicator::IgdPlus));
        assert!(all.contains(&MoIndicator::Epsilon));
        assert!(all.contains(&MoIndicator::R2));
    }

    #[test]
    fn indicator_higher_is_better_only_for_hv() {
        assert!(MoIndicator::Hypervolume.higher_is_better());
        assert!(!MoIndicator::IgdPlus.higher_is_better());
        assert!(!MoIndicator::Epsilon.higher_is_better());
        assert!(!MoIndicator::R2.higher_is_better());
    }
}
