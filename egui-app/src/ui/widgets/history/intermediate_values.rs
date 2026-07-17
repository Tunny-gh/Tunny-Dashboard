//! Intermediate Values widget.
//!
//! Overlays each trial's intermediate values as a learning curve.
//! Optuna's pruning decides whether to stop a trial by watching the progression of these
//! intermediate values, so this lets users see at a glance, per state, "after how many trials,
//! and settling into what shape."

use tunny_core::extras::{StudyExtras, TrialExtra, TrialState};

use super::state_colors::{
    dim, distinct_states_in_order, empty_state, show_state_legend, state_color,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{hit_test_nearest, show_hover_tooltip, HIT_THRESHOLD};

/// Upper bound on the number of learning curves drawn. When the trial count exceeds this,
/// evenly subsamples them (drawing every trial would spike the per-frame draw cost).
const MAX_CURVES: usize = 2000;

/// Learning curve for a single trial. `points` are raw `(step, value)` values (unconverted).
#[derive(Debug, Clone, PartialEq)]
pub struct IntermediateCurve {
    pub trial_id: u32,
    pub trial_number: u32,
    pub state: TrialState,
    pub points: Vec<[f64; 2]>,
}

/// Original data for the tooltip corresponding to a hit-test point (indexed the same as the drawn point).
#[derive(Debug, Clone)]
struct HoverPoint {
    trial_number: u32,
    state: TrialState,
    step: f64,
    value: f64,
}

/// Cache of data determined solely by the identity of `extras` and the log scale — curves,
/// hit-test points, legend state, etc. (avoids per-frame rebuilding; M-17).
/// The key is the identity address of `extras` (`StudyExtras`) + log scale. On a live update,
/// `ArcSwap` swaps in a new Arc, so a change in the referenced address can be treated as a data
/// update (same idea as the Timeline widget).
#[derive(Debug, Clone)]
struct IntermediateCache {
    key: (usize, bool),
    curves: Vec<IntermediateCurve>,
    total_eligible: usize,
    present: Vec<TrialState>,
    /// Point set for hit testing (in drawing coordinates, i.e. after log conversion).
    hit_points: Vec<(u32, usize, [f64; 2])>,
    /// Original data for tooltips (indexed the same as `hit_points`).
    hover_lookup: Vec<HoverPoint>,
}

/// Intermediate Values chart widget.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct IntermediateValuesChart {
    /// Toggle for Y-axis log scale.
    pub log_scale: bool,
    #[serde(skip)]
    cache: Option<IntermediateCache>,
}

impl IntermediateValuesChart {
    pub fn show(&mut self, ui: &mut egui::Ui, extras: Option<&StudyExtras>) {
        let Some(extras) = extras.filter(|e| e.has_intermediate()) else {
            self.cache = None;
            empty_state(ui, "No intermediate values in this study");
            return;
        };

        if ui.selectable_label(self.log_scale, "Log Scale").clicked() {
            self.log_scale = !self.log_scale;
        }

        // Cache keyed by the address of extras (StudyExtras) + log scale.
        let key = (extras as *const StudyExtras as usize, self.log_scale);
        let cache_valid = self.cache.as_ref().is_some_and(|c| c.key == key);
        if !cache_valid {
            let (curves, total_eligible) =
                build_intermediate_curves(&extras.trials, self.log_scale, MAX_CURVES);
            if curves.is_empty() {
                self.cache = None;
                empty_state(ui, "No intermediate values in this study");
                return;
            }
            let present = distinct_states_in_order(curves.iter().map(|c| c.state));

            // Keep the hit-test point set (in drawing coordinates, i.e. after log conversion)
            // and the tooltip's original data indexed the same.
            let log_scale = self.log_scale;
            let mut hit_points: Vec<(u32, usize, [f64; 2])> = Vec::new();
            let mut hover_lookup: Vec<HoverPoint> = Vec::new();
            for c in &curves {
                for &[step, value] in &c.points {
                    let plot_y = if log_scale && value > 0.0 {
                        value.log10()
                    } else {
                        value
                    };
                    let idx = hover_lookup.len();
                    hit_points.push((c.trial_id, idx, [step, plot_y]));
                    hover_lookup.push(HoverPoint {
                        trial_number: c.trial_number,
                        state: c.state,
                        step,
                        value,
                    });
                }
            }
            self.cache = Some(IntermediateCache {
                key,
                curves,
                total_eligible,
                present,
                hit_points,
                hover_lookup,
            });
        }
        let cache = self.cache.as_ref().expect("cache built above");

        ui.horizontal(|ui| {
            if cache.curves.len() < cache.total_eligible {
                ui.label(
                    egui::RichText::new(format!(
                        "showing {} of {} trials",
                        cache.curves.len(),
                        cache.total_eligible
                    ))
                    .small()
                    .color(crate::theme::TEXT_SECONDARY()),
                );
            }
            show_state_legend(ui, &cache.present);
        });

        let log_scale = self.log_scale;
        let mut plot = egui_plot::Plot::new("intermediate_values_plot")
            .unified_nav()
            .x_axis_label("step")
            .y_axis_label("value");
        if log_scale {
            plot = crate::ui::widgets::common::log_scale::apply_log_y_axis(plot);
        }

        let mut hovered_idx: Option<usize> = None;
        plot.show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            if let Some(pos) = plot_ui.response().hover_pos() {
                hovered_idx = hit_test_nearest(plot_ui, &cache.hit_points, pos, HIT_THRESHOLD)
                    .map(|(_, i)| i);
            }
            let hovered_trial_id = hovered_idx
                .and_then(|i| cache.hit_points.get(i))
                .map(|&(t, ..)| t);

            for c in &cache.curves {
                let is_hovered = hovered_trial_id == Some(c.trial_id);
                let base = state_color(c.state);
                let color = if hovered_trial_id.is_some() && !is_hovered {
                    dim(base)
                } else {
                    base
                };
                let width = if is_hovered { 2.5 } else { 1.2 };
                let pts: egui_plot::PlotPoints = c
                    .points
                    .iter()
                    .map(|&[step, value]| {
                        let y = if log_scale && value > 0.0 {
                            value.log10()
                        } else {
                            value
                        };
                        [step, y]
                    })
                    .collect();
                plot_ui.line(
                    egui_plot::Line::new(format!("Trial {}", c.trial_number), pts)
                        .color(color)
                        .width(width),
                );
            }
        });

        if let Some(hp) = hovered_idx.and_then(|i| cache.hover_lookup.get(i)) {
            let rows = vec![
                ("State".to_string(), hp.state.label().to_string()),
                ("Step".to_string(), format!("{}", hp.step)),
                ("Value".to_string(), format!("{:.6}", hp.value)),
            ];
            show_hover_tooltip(ui, "intermediate_values_hover", hp.trial_number, &rows);
        }
    }
}

/// Builds learning curves from `trials` (a pure function, covered by tests).
///
/// - Trials without intermediate values are excluded.
/// - When `log_scale` is true, non-positive points within each curve are dropped (removes
///   invalid points before the log10 conversion at draw time).
/// - If the eligible trial count exceeds `max_curves`, subsamples evenly to stay within the cap.
///
/// Returns `(curve list, total number of trials with intermediate values)`. The total is the
/// pre-subsampling value, used by the caller to display a "showing N of M trials" note.
pub fn build_intermediate_curves(
    trials: &[TrialExtra],
    log_scale: bool,
    max_curves: usize,
) -> (Vec<IntermediateCurve>, usize) {
    let eligible: Vec<&TrialExtra> = trials
        .iter()
        .filter(|t| !t.intermediate_values.is_empty())
        .collect();
    let total = eligible.len();

    let step = if max_curves == 0 {
        return (Vec::new(), total);
    } else if total > max_curves {
        total.div_ceil(max_curves)
    } else {
        1
    };

    let curves: Vec<IntermediateCurve> = eligible
        .into_iter()
        .step_by(step)
        .take(max_curves)
        .map(|t| {
            let points: Vec<[f64; 2]> = t
                .intermediate_values
                .iter()
                .filter(|&&(_, v)| !log_scale || v > 0.0)
                .map(|&(s, v)| [s as f64, v])
                .collect();
            IntermediateCurve {
                trial_id: t.trial_id,
                trial_number: t.trial_number,
                state: t.state,
                points,
            }
        })
        .filter(|c| !c.points.is_empty())
        .collect();

    (curves, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(id: u32, state: TrialState, values: &[(u64, f64)]) -> TrialExtra {
        TrialExtra {
            trial_id: id,
            trial_number: id,
            state,
            datetime_start: None,
            datetime_complete: None,
            intermediate_values: values.to_vec(),
        }
    }

    #[test]
    fn chart_default_has_log_scale_off() {
        let chart = IntermediateValuesChart::default();
        assert!(!chart.log_scale);
    }

    #[test]
    fn build_curves_skips_trials_without_intermediate_values() {
        let trials = vec![
            trial(0, TrialState::Complete, &[(0, 1.0), (1, 0.5)]),
            trial(1, TrialState::Running, &[]),
        ];
        let (curves, total) = build_intermediate_curves(&trials, false, 2000);
        assert_eq!(total, 1);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].trial_id, 0);
        assert_eq!(curves[0].points, vec![[0.0, 1.0], [1.0, 0.5]]);
    }

    #[test]
    fn build_curves_keeps_raw_points_when_linear() {
        let trials = vec![trial(
            0,
            TrialState::Complete,
            &[(0, -1.0), (1, 0.0), (2, 2.0)],
        )];
        let (curves, _) = build_intermediate_curves(&trials, false, 2000);
        assert_eq!(curves[0].points, vec![[0.0, -1.0], [1.0, 0.0], [2.0, 2.0]]);
    }

    #[test]
    fn build_curves_filters_non_positive_when_log_scale() {
        let trials = vec![trial(
            0,
            TrialState::Complete,
            &[(0, -1.0), (1, 0.0), (2, 2.0)],
        )];
        let (curves, _) = build_intermediate_curves(&trials, true, 2000);
        assert_eq!(curves[0].points, vec![[2.0, 2.0]]);
    }

    #[test]
    fn build_curves_drops_curve_that_becomes_empty_under_log_scale() {
        let trials = vec![
            trial(0, TrialState::Complete, &[(0, -1.0), (1, 0.0)]),
            trial(1, TrialState::Complete, &[(0, 1.0)]),
        ];
        let (curves, total) = build_intermediate_curves(&trials, true, 2000);
        // total stays at 2, since it's the "number of trials with intermediate values" before applying log.
        assert_eq!(total, 2);
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].trial_id, 1);
    }

    #[test]
    fn build_curves_subsamples_evenly_when_over_cap() {
        let trials: Vec<TrialExtra> = (0..5000)
            .map(|i| trial(i, TrialState::Complete, &[(0, 1.0)]))
            .collect();
        let (curves, total) = build_intermediate_curves(&trials, false, 2000);
        assert_eq!(total, 5000);
        assert!(curves.len() <= 2000);
        assert!(!curves.is_empty());
        // Subsampling steps at a fixed interval from the start, so the first trial always remains.
        assert_eq!(curves[0].trial_id, 0);
    }

    #[test]
    fn build_curves_no_subsample_when_under_cap() {
        let trials: Vec<TrialExtra> = (0..10)
            .map(|i| trial(i, TrialState::Complete, &[(0, 1.0)]))
            .collect();
        let (curves, total) = build_intermediate_curves(&trials, false, 2000);
        assert_eq!(total, 10);
        assert_eq!(curves.len(), 10);
    }
}
