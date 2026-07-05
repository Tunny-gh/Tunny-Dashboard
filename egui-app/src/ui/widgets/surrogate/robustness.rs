//! ロバスト性解析ウィジェット。
//!
//! `SurrogateOpt` と同様にサロゲートを非同期で学習するが（フィット段階のみ、
//! poll_chart.rs 参照）、ロバスト性解析自体（`tunny_core::surrogate_opt::robustness_analysis`）
//! はミリ秒オーダーのためレンダーパスで同期実行し、結果をキャッシュする。
//!
//! 候補設計点（Best trial または pin 留めした trial）の周りにガウス入力ノイズを与え、
//! サロゲート予測を通した出力分布をヒストグラムで表示する。理論的背景は
//! theory/{en,ja}/optimization/robustness-analysis.md。

use std::sync::Arc;

use tunny_core::statistics::{compute_histogram, BinRule};
use tunny_core::surrogate_opt::{
    RobustnessResult, RobustnessSpec, SurrogateModelKind, TrainedSurrogate,
    MIN_TRIALS_FOR_SURROGATE_OPT,
};

use super::anchor::{center_label, resolve_center, CenterChoice};
use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::{COLOR_BAR_ACCENT, COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};

/// モデル選択肢（コンボ表示順）。`surrogate_opt` の一覧と揃える。
const MODEL_CHOICES: [SurrogateModelKind; 5] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::GpMoe,
    SurrogateModelKind::Lgbm,
];

/// サンプル数の選択肢。
const SAMPLE_CHOICES: [usize; 3] = [256, 1024, 4096];

/// フィット段階の計算リクエスト。poll_chart が消費する。
pub struct RobustnessFitRequest {
    pub objective_index: usize,
    pub model: SurrogateModelKind,
}

/// キャッシュキー: (学習済みモデルのポインタ恒等性, 中心点のビット表現, ノイズ%のビット表現,
/// サンプル数, 認識論的不確かさ込みか)。シードは固定 42 のためキーに含めない。
type RobustnessCacheKey = (usize, Vec<u64>, u64, usize, bool);

/// ロバスト性解析ウィジェットの UI 状態。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RobustnessChart {
    pub selected_objective: usize,
    pub model: SurrogateModelKind,
    pub center: CenterChoice,
    /// ノイズの 1σ（パラメータレンジに対する割合、%）。
    pub noise_pct: f64,
    pub n_samples: usize,
    pub include_epistemic: bool,

    #[serde(skip)]
    pub trained: Option<Arc<TrainedSurrogate>>,
    #[serde(skip)]
    pub fitting: bool,
    #[serde(skip)]
    pub fit_error: Option<String>,
    #[serde(skip)]
    pub pending_fit: Option<RobustnessFitRequest>,
    #[serde(skip)]
    cache: Option<(RobustnessCacheKey, RobustnessResult)>,
}

impl Default for RobustnessChart {
    fn default() -> Self {
        Self {
            selected_objective: 0,
            model: SurrogateModelKind::GpFitc,
            center: CenterChoice::default(),
            noise_pct: 2.0,
            n_samples: 1024,
            include_epistemic: false,
            trained: None,
            fitting: false,
            fit_error: None,
            pending_fit: None,
            cache: None,
        }
    }
}

impl RobustnessChart {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む（キャンバス各アイテム伝播用）。
    /// 目的・モデル・中心点・ノイズ設定などの UI 選択は各アイテム側を維持する。
    pub fn adopt_compute_state(&mut self, global: &Self) {
        self.trained = global.trained.clone();
        self.fitting = global.fitting;
        self.fit_error = global.fit_error.clone();
    }

    /// 直近のロバスト性解析結果（キャッシュ）。CSV エクスポート等が参照する。
    pub fn cached_result(&self) -> Option<&RobustnessResult> {
        self.cache.as_ref().map(|(_, r)| r)
    }
}

/// `obj_names` / `directions` は現在の Study の全目的（Best trial 解決用）。
/// `pinned_trials` は pin 留めした trial_id（Center コンボの候補）。
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut RobustnessChart,
    view: &StudyView,
    obj_names: &[String],
    directions: &[Direction],
    trial_count: usize,
    pinned_trials: &[u32],
) {
    if obj_names.is_empty() {
        ui.label("No objectives available.");
        return;
    }
    if state.selected_objective >= obj_names.len() {
        state.selected_objective = 0;
    }

    // ── 目的・モデル選択 ─────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Objective:");
        egui::ComboBox::from_id_salt("robustness_obj")
            .selected_text(obj_names[state.selected_objective].as_str())
            .show_ui(ui, |ui| {
                for (i, name) in obj_names.iter().enumerate() {
                    ui.selectable_value(&mut state.selected_objective, i, name);
                }
            });

        ui.label("Model:");
        egui::ComboBox::from_id_salt("robustness_model")
            .selected_text(super::surrogate_opt::model_label(state.model))
            .show_ui(ui, |ui| {
                for kind in MODEL_CHOICES {
                    ui.selectable_value(
                        &mut state.model,
                        kind,
                        super::surrogate_opt::model_label(kind),
                    );
                }
            });
    });

    // ── 中心点選択 ──────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Center:");
        let center_text = center_label(state.center, view);
        egui::ComboBox::from_id_salt("robustness_center")
            .selected_text(center_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.center, CenterChoice::BestTrial, "Best trial");
                for &trial_id in pinned_trials {
                    let Some(row) = view.trial_ids.iter().position(|&t| t == trial_id) else {
                        continue;
                    };
                    let number = view.df.get_trial_number(row).unwrap_or(trial_id);
                    ui.selectable_value(
                        &mut state.center,
                        CenterChoice::Pinned(trial_id),
                        format!("Trial #{number}"),
                    );
                }
            });
    });

    // ── ノイズ・サンプル数・認識論的不確かさ ─────────────────────
    ui.horizontal(|ui| {
        ui.label("Noise % (1σ of range):");
        ui.add(egui::Slider::new(&mut state.noise_pct, 0.1..=10.0));
    });
    ui.horizontal(|ui| {
        ui.label("Samples:");
        egui::ComboBox::from_id_salt("robustness_samples")
            .selected_text(state.n_samples.to_string())
            .show_ui(ui, |ui| {
                for n in SAMPLE_CHOICES {
                    ui.selectable_value(&mut state.n_samples, n, n.to_string());
                }
            });
        ui.checkbox(&mut state.include_epistemic, "Model uncertainty");
    });

    // ── trial 数不足 ─────────────────────────────────────────────
    if trial_count < MIN_TRIALS_FOR_SURROGATE_OPT {
        ui.label(
            egui::RichText::new(format!(
                "At least {} trials required (current: {})",
                MIN_TRIALS_FOR_SURROGATE_OPT, trial_count
            ))
            .weak(),
        );
        return;
    }

    // ── Fit Surrogate ボタン ─────────────────────────────────────
    let can_fit = !state.fitting && state.pending_fit.is_none();
    if ui
        .add_enabled(can_fit, egui::Button::new("Fit Surrogate"))
        .clicked()
    {
        state.fit_error = None;
        state.fitting = true;
        state.pending_fit = Some(RobustnessFitRequest {
            objective_index: state.selected_objective,
            model: state.model,
        });
    }

    if state.fitting {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Fitting surrogate...");
        });
    }

    if let Some(err) = state.fit_error.clone() {
        ui.colored_label(egui::Color32::RED, err);
    }

    let Some(trained) = state.trained.clone() else {
        return;
    };

    let Some(center) = resolve_center(&trained, state.center, view, obj_names, directions) else {
        ui.colored_label(
            egui::Color32::RED,
            "Could not resolve the center point for the trained parameters.",
        );
        return;
    };

    let key = cache_key(
        &trained,
        &center,
        state.noise_pct,
        state.n_samples,
        state.include_epistemic,
    );
    if state.cache.as_ref().map(|(k, _)| k) != Some(&key) {
        let spec = RobustnessSpec {
            center,
            relative_sigma: state.noise_pct / 100.0,
            n_samples: state.n_samples,
            include_epistemic: state.include_epistemic,
            seed: 42,
        };
        match tunny_core::surrogate_opt::robustness_analysis(&trained, &spec) {
            Ok(result) => state.cache = Some((key, result)),
            Err(e) => {
                state.cache = None;
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Robustness analysis failed: {e}"),
                );
                return;
            }
        }
    }

    if let Some((_, result)) = &state.cache {
        render_result(ui, result);
    }
}

fn cache_key(
    trained: &Arc<TrainedSurrogate>,
    center: &[f64],
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
) -> RobustnessCacheKey {
    (
        Arc::as_ptr(trained) as usize,
        center.iter().map(|v| v.to_bits()).collect(),
        noise_pct.to_bits(),
        n_samples,
        include_epistemic,
    )
}

/// ヒストグラム + 統計サマリを描画する。
fn render_result(ui: &mut egui::Ui, result: &RobustnessResult) {
    let Some(hist) = compute_histogram(&result.samples, BinRule::Sturges) else {
        ui.label(egui::RichText::new("Not enough samples to plot.").weak());
        return;
    };

    let bars: Vec<egui_plot::Bar> = hist
        .bin_edges
        .windows(2)
        .zip(&hist.counts)
        .map(|(edge, &count)| {
            let raw_width = edge[1] - edge[0];
            let (center, width) = if raw_width > 0.0 {
                ((edge[0] + edge[1]) / 2.0, raw_width)
            } else {
                (edge[0], 1.0)
            };
            egui_plot::Bar::new(center, count as f64).width(width)
        })
        .collect();
    let chart = egui_plot::BarChart::new("Samples", bars).color(COLOR_BAR_PRIMARY);

    egui_plot::Plot::new("robustness_histogram")
        .unified_nav()
        .legend(egui_plot::Legend::default())
        .x_axis_label("Output")
        .y_axis_label("Count")
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            plot_ui.bar_chart(chart);
            plot_ui.vline(
                egui_plot::VLine::new("Nominal", result.nominal)
                    .color(egui::Color32::from_gray(160))
                    .style(egui_plot::LineStyle::Dashed { length: 8.0 }),
            );
            plot_ui.vline(egui_plot::VLine::new("Mean", result.mean).color(COLOR_BAR_NEGATIVE));
            plot_ui.vline(
                egui_plot::VLine::new("P5", result.p05)
                    .color(COLOR_BAR_ACCENT)
                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
            );
            plot_ui.vline(
                egui_plot::VLine::new("P95", result.p95)
                    .color(COLOR_BAR_ACCENT)
                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
            );
        });

    ui.add_space(4.0);
    let shift = result.mean - result.nominal;
    ui.label(format!(
        "Mean {:.4} ± {:.4}   Shift {:+.4}   P5 {:.4} / P95 {:.4}",
        result.mean, result.std, shift, result.p05, result.p95
    ));
    if let Some(rate) = result.feasibility_rate {
        ui.label(format!("P(feasible) {:.1}%", rate * 100.0));
    }
    if result.clipped_fraction > 0.0 {
        ui.colored_label(
            egui::Color32::from_rgb(202, 138, 4), // amber-600
            format!("Clipped {:.1}%", result.clipped_fraction * 100.0),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robustness_chart_default_values() {
        let state = RobustnessChart::default();
        assert_eq!(state.selected_objective, 0);
        assert_eq!(state.center, CenterChoice::BestTrial);
        assert_eq!(state.noise_pct, 2.0);
        assert_eq!(state.n_samples, 1024);
        assert!(!state.include_epistemic);
        assert!(state.trained.is_none());
        assert!(!state.fitting);
        assert!(state.pending_fit.is_none());
        assert!(state.cached_result().is_none());
    }

    #[test]
    fn adopt_compute_state_propagates_and_keeps_selection() {
        let src = RobustnessChart {
            fitting: false,
            fit_error: Some("err".into()),
            ..Default::default()
        };
        let mut dst = RobustnessChart {
            fitting: true,
            selected_objective: 2,
            noise_pct: 5.0,
            ..Default::default()
        };
        dst.adopt_compute_state(&src);
        assert!(!dst.fitting);
        assert_eq!(dst.fit_error.as_deref(), Some("err"));
        // UI 選択は維持される
        assert_eq!(dst.selected_objective, 2);
        assert_eq!(dst.noise_pct, 5.0);
    }
}
