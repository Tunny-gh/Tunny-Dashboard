//! Timeline ウィジェット。
//!
//! trial ごとの開始〜完了日時を横棒（ガントチャート）として並べ、
//! 並列実行のワーカー数・スケジューリングの偏りを俯瞰できるようにする。

use tunny_core::extras::{StudyExtras, TrialExtra, TrialState};

use super::state_colors::{show_state_legend, state_color};
use crate::theme::chart_colors::COLOR_EMPTY_STATE;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::show_hover_tooltip;

/// 横棒の高さ（trial_number ± half_width の範囲を占める）。
const BAR_WIDTH: f64 = 0.8;

/// 1 trial 分のタイムラインバー。`start` / `end` は study 内最早の
/// `datetime_start` を 0 とした経過秒（elapsed seconds）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineBar {
    pub trial_id: u32,
    pub trial_number: u32,
    pub state: TrialState,
    pub start: f64,
    pub end: f64,
}

/// X 軸の表示単位。全体スパンに応じて自動選択する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
}

impl TimeUnit {
    /// 経過秒をこの単位の値に変換する係数。
    pub fn divisor(self) -> f64 {
        match self {
            TimeUnit::Seconds => 1.0,
            TimeUnit::Minutes => 60.0,
            TimeUnit::Hours => 3600.0,
        }
    }

    /// 軸ラベルに使う単位表記。
    pub fn suffix(self) -> &'static str {
        match self {
            TimeUnit::Seconds => "s",
            TimeUnit::Minutes => "min",
            TimeUnit::Hours => "h",
        }
    }
}

/// Timeline チャートウィジェット。
#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TimelineChart {}

impl TimelineChart {
    pub fn show(&mut self, ui: &mut egui::Ui, extras: Option<&StudyExtras>) {
        let Some(extras) = extras.filter(|e| e.has_datetimes()) else {
            empty_state(ui, "No datetime information in this study");
            return;
        };

        let bars = build_timeline_bars(&extras.trials);
        if bars.is_empty() {
            empty_state(ui, "No datetime information in this study");
            return;
        }

        let span = bars.iter().map(|b| b.end).fold(0.0_f64, f64::max);
        let unit = select_time_unit(span);
        let divisor = unit.divisor();
        let x_label = format!("elapsed [{}]", unit.suffix());

        let mut present: Vec<TrialState> = Vec::new();
        for b in &bars {
            if !present.contains(&b.state) {
                present.push(b.state);
            }
        }
        show_state_legend(ui, &present);

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
                    hovered = bar_at_position(&bars, p.x * divisor, p.y, half_width);
                }
            }

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

/// hover 中でないバーを薄く見せる（アルファのみ落とす）。
fn dim(color: egui::Color32) -> egui::Color32 {
    let [r, g, b, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, 90)
}

fn empty_state(ui: &mut egui::Ui, message: &str) {
    ui.centered_and_justified(|ui| {
        ui.colored_label(COLOR_EMPTY_STATE, message);
    });
}

/// `trials` からタイムラインバーを構築する（純粋関数・テスト対象）。
///
/// - `datetime_start` を持たない trial は除外する。
/// - 経過秒は study 内最早の `datetime_start` を 0 として re-base する。
/// - `datetime_complete` が無い trial（RUNNING など）は、study 内で判明している
///   最大の日時（開始・完了いずれか）まで棒を伸ばす。
pub fn build_timeline_bars(trials: &[TrialExtra]) -> Vec<TimelineBar> {
    let t0 = trials
        .iter()
        .filter_map(|t| t.datetime_start)
        .fold(f64::INFINITY, f64::min);
    if !t0.is_finite() {
        return Vec::new();
    }

    let max_ts = trials
        .iter()
        .flat_map(|t| [t.datetime_start, t.datetime_complete])
        .flatten()
        .fold(f64::NEG_INFINITY, f64::max);

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

/// 全体スパン（経過秒の最大値）から表示単位を選ぶ。
/// 600 秒超で分、7200 秒（2 時間）超で時間に切り替える。
pub fn select_time_unit(total_span_seconds: f64) -> TimeUnit {
    if total_span_seconds > 7200.0 {
        TimeUnit::Hours
    } else if total_span_seconds > 600.0 {
        TimeUnit::Minutes
    } else {
        TimeUnit::Seconds
    }
}

/// プロット座標 `(x, y)`（x は経過秒、y は trial_number 相当）にヒットするバーの
/// index を返す。`half_width` はバーの縦方向の半幅（`BAR_WIDTH / 2`）。
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
