//! Intermediate Values ウィジェット。
//!
//! trial ごとの中間値（intermediate values）を学習曲線として重ね描きする。
//! Optuna の pruning はこの中間値の推移を見て打ち切りを判断するため、
//! 「どのくらいの trial 数で・どんな形状に落ち着くか」を state 別に俯瞰できるようにする。

use tunny_core::extras::{StudyExtras, TrialExtra, TrialState};

use super::state_colors::{show_state_legend, state_color};
use crate::theme::chart_colors::COLOR_EMPTY_STATE;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{hit_test_nearest, show_hover_tooltip, HIT_THRESHOLD};

/// 描画する学習曲線の上限。これを超える trial 数のときは均等間引きする
/// （全 trial を描くと 1 フレームの描画コストが跳ね上がるため）。
const MAX_CURVES: usize = 2000;

/// 1 trial 分の学習曲線。`points` は `(step, value)` の実値（未変換）。
#[derive(Debug, Clone, PartialEq)]
pub struct IntermediateCurve {
    pub trial_id: u32,
    pub trial_number: u32,
    pub state: TrialState,
    pub points: Vec<[f64; 2]>,
}

/// Intermediate Values チャートウィジェット。
#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct IntermediateValuesChart {
    /// Y 軸対数スケール切替。
    pub log_scale: bool,
}

impl IntermediateValuesChart {
    pub fn show(&mut self, ui: &mut egui::Ui, extras: Option<&StudyExtras>) {
        let Some(extras) = extras.filter(|e| e.has_intermediate()) else {
            empty_state(ui, "No intermediate values in this study");
            return;
        };

        if ui.selectable_label(self.log_scale, "Log Scale").clicked() {
            self.log_scale = !self.log_scale;
        }

        let (curves, total_eligible) =
            build_intermediate_curves(&extras.trials, self.log_scale, MAX_CURVES);

        if curves.is_empty() {
            empty_state(ui, "No intermediate values in this study");
            return;
        }

        ui.horizontal(|ui| {
            if curves.len() < total_eligible {
                ui.label(
                    egui::RichText::new(format!(
                        "showing {} of {} trials",
                        curves.len(),
                        total_eligible
                    ))
                    .small()
                    .color(crate::theme::TEXT_SECONDARY()),
                );
            }
            let mut present: Vec<TrialState> = Vec::new();
            for c in &curves {
                if !present.contains(&c.state) {
                    present.push(c.state);
                }
            }
            show_state_legend(ui, &present);
        });

        // ヒットテスト用の点群（描画座標系＝ log 変換後）と、ツールチップ用の元データを
        // 同じ index で対応づけて保持する。
        struct HoverPoint {
            trial_number: u32,
            state: TrialState,
            step: f64,
            value: f64,
        }
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
                hovered_idx =
                    hit_test_nearest(plot_ui, &hit_points, pos, HIT_THRESHOLD).map(|(_, i)| i);
            }
            let hovered_trial_id = hovered_idx
                .and_then(|i| hit_points.get(i))
                .map(|&(t, ..)| t);

            for c in &curves {
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

        if let Some(hp) = hovered_idx.and_then(|i| hover_lookup.get(i)) {
            let rows = vec![
                ("State".to_string(), hp.state.label().to_string()),
                ("Step".to_string(), format!("{}", hp.step)),
                ("Value".to_string(), format!("{:.6}", hp.value)),
            ];
            show_hover_tooltip(ui, "intermediate_values_hover", hp.trial_number, &rows);
        }
    }
}

/// hover 中でない曲線を薄く見せる（アルファのみ落とす）。
fn dim(color: egui::Color32) -> egui::Color32 {
    let [r, g, b, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, 90)
}

fn empty_state(ui: &mut egui::Ui, message: &str) {
    ui.centered_and_justified(|ui| {
        ui.colored_label(COLOR_EMPTY_STATE(), message);
    });
}

/// `trials` から学習曲線を構築する（純粋関数・テスト対象）。
///
/// - 中間値を持たない trial は除外する。
/// - `log_scale` が true のときは各曲線内の非正値の点を落とす（描画時の
///   log10 変換前に不正な点を除いておく）。
/// - 対象 trial 数が `max_curves` を超える場合は均等間引きして上限に収める。
///
/// 戻り値は `(曲線一覧, 中間値を持つ trial の総数)`。総数は間引き前の値であり、
/// 呼び出し側が「showing N of M trials」の注記を出すために使う。
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
        // total はログ適用前の「中間値を持つ trial 数」なので 2 のまま。
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
        // 間引きは先頭から一定間隔なので、先頭 trial は必ず残る。
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
