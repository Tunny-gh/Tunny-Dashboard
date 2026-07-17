//! Timeline widget.
//!
//! Lays out each trial's start-to-completion datetime as a horizontal bar
//! (Gantt chart), to give an overview of parallel worker count and
//! scheduling skew.

use tunny_core::extras::{StudyExtras, TrialExtra, TrialState};

use super::state_colors::{
    dim, distinct_states_in_order, empty_state, show_state_legend, state_color,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::common::range_math::value_range;
use crate::ui::widgets::trial_detail_modal::show_hover_tooltip;

/// Height of the horizontal bar (occupies the range trial_number ± half_width).
const BAR_WIDTH: f64 = 0.8;

/// A timeline bar for a single trial. `start` / `end` are elapsed seconds,
/// re-based so the earliest `datetime_start` in the study is 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineBar {
    pub trial_id: u32,
    pub trial_number: u32,
    pub state: TrialState,
    pub start: f64,
    pub end: f64,
}

/// X-axis display unit. Chosen automatically based on the overall span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
}

impl TimeUnit {
    /// Factor to convert elapsed seconds into a value in this unit.
    pub fn divisor(self) -> f64 {
        match self {
            TimeUnit::Seconds => 1.0,
            TimeUnit::Minutes => 60.0,
            TimeUnit::Hours => 3600.0,
        }
    }

    /// Unit notation used in the axis label.
    pub fn suffix(self) -> &'static str {
        match self {
            TimeUnit::Seconds => "s",
            TimeUnit::Minutes => "min",
            TimeUnit::Hours => "h",
        }
    }
}

/// Timeline chart widget.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TimelineChart {
    /// Cache that avoids rebuilding bar positions (the result of converting
    /// datetime -> elapsed seconds). Since hover state can change every
    /// frame, only the coloring is lightly re-applied on top of this each
    /// time inside `show` (`TimelineCache` holds positions only).
    #[serde(skip)]
    cache: Option<TimelineCache>,
}

/// A cache bundling the result of `build_timeline_bars` (bars with
/// positions already computed) together with the display unit and legend
/// state list determined from it.
///
/// The key is the identity (address) of `extras` (`StudyExtras`). Since the
/// caller reuses the same `Arc<StudyExtras>` allocation for the duration of
/// a given study via `ArcSwap::load_full()` (the same idea as the DataFrame
/// Arc identity in poll_chart.rs), a change in the referenced address can
/// be treated as a data update.
#[derive(Debug, Clone)]
struct TimelineCache {
    key: usize,
    bars: Vec<TimelineBar>,
    unit: TimeUnit,
    present: Vec<TrialState>,
}

impl TimelineChart {
    pub fn show(&mut self, ui: &mut egui::Ui, extras: Option<&StudyExtras>) {
        let Some(extras) = extras.filter(|e| e.has_datetimes()) else {
            self.cache = None;
            empty_state(ui, "No datetime information in this study");
            return;
        };

        // Use the address of extras (StudyExtras) as the data identity. On
        // a live update, ArcSwap swaps in a new Arc, so the referenced
        // address also changes.
        let key = extras as *const StudyExtras as usize;
        let cache_valid = self.cache.as_ref().is_some_and(|c| c.key == key);
        if !cache_valid {
            let bars = build_timeline_bars(&extras.trials);
            if bars.is_empty() {
                self.cache = None;
                empty_state(ui, "No datetime information in this study");
                return;
            }
            let span = bars.iter().map(|b| b.end).fold(0.0_f64, f64::max);
            let unit = select_time_unit(span);
            let present = distinct_states_in_order(bars.iter().map(|b| b.state));
            self.cache = Some(TimelineCache {
                key,
                bars,
                unit,
                present,
            });
        }
        let cache = self.cache.as_ref().expect("cache just populated above");
        let bars = &cache.bars;
        let unit = cache.unit;
        let divisor = unit.divisor();
        let x_label = format!("elapsed [{}]", unit.suffix());

        show_state_legend(ui, &cache.present);

        let half_width = BAR_WIDTH / 2.0;
        let mut hovered: Option<usize> = None;

        let plot = egui_plot::Plot::new("timeline_plot")
            .unified_nav()
            .x_axis_label(x_label)
            .y_axis_label("trial")
            .include_y(0.0);

        plot.show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            if plot_ui.response().hovered() {
                if let Some(p) = plot_ui.pointer_coordinate() {
                    hovered = bar_at_position(bars, p.x * divisor, p.y, half_width);
                }
            }

            // Positions (start/end) are already cached. Only the
            // hover-dependent coloring is lightly applied each frame here
            // (no datetime -> elapsed-seconds recomputation occurs).
            let plot_bars: Vec<egui_plot::Bar> = bars
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    let is_hovered = hovered == Some(i);
                    let base = state_color(b.state);
                    let color = if hovered.is_some() && !is_hovered {
                        dim(base)
                    } else {
                        base
                    };
                    egui_plot::Bar::new(b.trial_number as f64, (b.end - b.start) / divisor)
                        .base_offset(b.start / divisor)
                        .width(BAR_WIDTH)
                        .horizontal()
                        .fill(color)
                        .stroke(egui::Stroke::NONE)
                })
                .collect();
            plot_ui.bar_chart(egui_plot::BarChart::new("Trials", plot_bars));
        });

        if let Some(b) = hovered.and_then(|i| bars.get(i)) {
            let rows = vec![
                ("State".to_string(), b.state.label().to_string()),
                ("Start".to_string(), format_elapsed(b.start, unit)),
                ("End".to_string(), format_elapsed(b.end, unit)),
                (
                    "Duration".to_string(),
                    format_elapsed(b.end - b.start, unit),
                ),
            ];
            show_hover_tooltip(ui, "timeline_hover", b.trial_number, &rows);
        }
    }
}

fn format_elapsed(seconds: f64, unit: TimeUnit) -> String {
    format!("{:.2} {}", seconds / unit.divisor(), unit.suffix())
}

/// Builds timeline bars from `trials` (a pure function, covered by tests).
///
/// - Trials without a `datetime_start` are excluded.
/// - Elapsed seconds are re-based so the earliest `datetime_start` in the
///   study is 0.
/// - Trials with no `datetime_complete` (e.g. RUNNING) have their bar
///   extended to the maximum known datetime in the study (either start or
///   complete).
pub fn build_timeline_bars(trials: &[TrialExtra]) -> Vec<TimelineBar> {
    let t0 = value_range(trials.iter().filter_map(|t| t.datetime_start))
        .map(|(mn, _)| mn)
        .unwrap_or(f64::INFINITY);
    if !t0.is_finite() {
        return Vec::new();
    }

    let max_ts = value_range(
        trials
            .iter()
            .flat_map(|t| [t.datetime_start, t.datetime_complete])
            .flatten(),
    )
    .map(|(_, mx)| mx)
    .unwrap_or(f64::NEG_INFINITY);

    trials
        .iter()
        .filter_map(|t| {
            let start = t.datetime_start?;
            let end_abs = t.datetime_complete.unwrap_or(max_ts).max(start);
            Some(TimelineBar {
                trial_id: t.trial_id,
                trial_number: t.trial_number,
                state: t.state,
                start: start - t0,
                end: end_abs - t0,
            })
        })
        .collect()
}

/// Chooses the display unit from the overall span (the max elapsed
/// seconds). Switches to minutes above 600 seconds, and hours above 7200
/// seconds (2 hours).
pub fn select_time_unit(total_span_seconds: f64) -> TimeUnit {
    if total_span_seconds > 7200.0 {
        TimeUnit::Hours
    } else if total_span_seconds > 600.0 {
        TimeUnit::Minutes
    } else {
        TimeUnit::Seconds
    }
}

/// Returns the index of the bar hit by the plot coordinate `(x, y)` (x is
/// elapsed seconds, y is roughly trial_number). `half_width` is the bar's
/// vertical half-width (`BAR_WIDTH / 2`).
pub fn bar_at_position(bars: &[TimelineBar], x: f64, y: f64, half_width: f64) -> Option<usize> {
    bars.iter()
        .position(|b| (b.trial_number as f64 - y).abs() <= half_width && x >= b.start && x <= b.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(id: u32, state: TrialState, start: Option<f64>, complete: Option<f64>) -> TrialExtra {
        TrialExtra {
            trial_id: id,
            trial_number: id,
            state,
            datetime_start: start,
            datetime_complete: complete,
            intermediate_values: vec![],
        }
    }

    #[test]
    fn build_bars_skips_trials_without_start() {
        let trials = vec![
            trial(0, TrialState::Complete, Some(100.0), Some(105.0)),
            trial(1, TrialState::Waiting, None, None),
        ];
        let bars = build_timeline_bars(&trials);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].trial_id, 0);
    }

    #[test]
    fn build_bars_rebases_to_earliest_start() {
        let trials = vec![
            trial(0, TrialState::Complete, Some(100.0), Some(110.0)),
            trial(1, TrialState::Complete, Some(105.0), Some(120.0)),
        ];
        let bars = build_timeline_bars(&trials);
        assert_eq!(bars[0].start, 0.0);
        assert_eq!(bars[0].end, 10.0);
        assert_eq!(bars[1].start, 5.0);
        assert_eq!(bars[1].end, 20.0);
    }

    #[test]
    fn build_bars_extends_running_trial_to_max_known_timestamp() {
        let trials = vec![
            trial(0, TrialState::Running, Some(100.0), None),
            trial(1, TrialState::Complete, Some(110.0), Some(130.0)),
        ];
        let bars = build_timeline_bars(&trials);
        // running trial (id=0) has no complete; should extend to max known ts (130).
        let running = bars.iter().find(|b| b.trial_id == 0).unwrap();
        assert_eq!(running.start, 0.0);
        assert_eq!(running.end, 30.0);
    }

    #[test]
    fn build_bars_empty_when_no_trial_has_start() {
        let trials = vec![trial(0, TrialState::Waiting, None, None)];
        assert!(build_timeline_bars(&trials).is_empty());
    }

    #[test]
    fn select_time_unit_thresholds() {
        assert_eq!(select_time_unit(0.0), TimeUnit::Seconds);
        assert_eq!(select_time_unit(600.0), TimeUnit::Seconds);
        assert_eq!(select_time_unit(600.1), TimeUnit::Minutes);
        assert_eq!(select_time_unit(7200.0), TimeUnit::Minutes);
        assert_eq!(select_time_unit(7200.1), TimeUnit::Hours);
    }

    #[test]
    fn bar_at_position_hits_within_range() {
        let bars = vec![TimelineBar {
            trial_id: 0,
            trial_number: 3,
            state: TrialState::Complete,
            start: 10.0,
            end: 20.0,
        }];
        assert_eq!(bar_at_position(&bars, 15.0, 3.0, 0.4), Some(0));
        assert_eq!(bar_at_position(&bars, 15.0, 3.6, 0.4), None); // outside half_width
        assert_eq!(bar_at_position(&bars, 25.0, 3.0, 0.4), None); // outside x range
    }
}
