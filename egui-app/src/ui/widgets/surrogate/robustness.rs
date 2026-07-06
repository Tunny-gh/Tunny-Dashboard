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
/// サンプル数, 認識論的不確かさ込みか, ノイズ分布種別, Weibull 形状パラメータのビット表現,
/// LSL のビット表現（未指定なら None）, USL のビット表現（未指定なら None）)。
/// シードは固定 42 のためキーに含めない。
type RobustnessCacheKey = (
    usize,
    Vec<u64>,
    u64,
    usize,
    bool,
    u8,
    u64,
    Option<u64>,
    Option<u64>,
);

/// ノイズ分布の選択肢（widget ローカル）。
///
/// コアの `NoiseDistribution` は `Weibull { shape }` を持つデータ運搬型で
/// シリアライズを実装していないため、UI 状態の永続化用にこの薄いミラーを持つ。
/// Weibull の形状パラメータ自体は別フィールド `weibull_shape` に持たせる。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoiseDistKind {
    #[default]
    Normal,
    Uniform,
    Weibull,
}

/// コンボ表示用ラベル。
fn noise_dist_label(kind: NoiseDistKind) -> &'static str {
    match kind {
        NoiseDistKind::Normal => "Normal",
        NoiseDistKind::Uniform => "Uniform",
        NoiseDistKind::Weibull => "Weibull",
    }
}

/// ロバスト性解析ウィジェットの UI 状態。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RobustnessChart {
    pub selected_objective: usize,
    pub model: SurrogateModelKind,
    pub center: CenterChoice,
    /// ノイズの 1σ（パラメータレンジに対する割合、%）。
    pub noise_pct: f64,
    /// ノイズの分布形。
    pub noise_dist: NoiseDistKind,
    /// Weibull 分布の形状パラメータ k（[0.2, 20] の範囲）。
    pub weibull_shape: f64,
    pub n_samples: usize,
    pub include_epistemic: bool,
    /// 仕様下限（LSL）を有効にするか、およびその値。
    pub use_lower_spec: bool,
    pub lower_spec_value: f64,
    /// 仕様上限（USL）を有効にするか、およびその値。
    pub use_upper_spec: bool,
    pub upper_spec_value: f64,

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
            noise_dist: NoiseDistKind::Normal,
            weibull_shape: 1.5,
            n_samples: 1024,
            include_epistemic: false,
            use_lower_spec: false,
            lower_spec_value: 0.0,
            use_upper_spec: false,
            upper_spec_value: 0.0,
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

    // ── ノイズ分布 ────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Noise dist:");
        egui::ComboBox::from_id_salt("robustness_noise_dist")
            .selected_text(noise_dist_label(state.noise_dist))
            .show_ui(ui, |ui| {
                for kind in [
                    NoiseDistKind::Normal,
                    NoiseDistKind::Uniform,
                    NoiseDistKind::Weibull,
                ] {
                    ui.selectable_value(&mut state.noise_dist, kind, noise_dist_label(kind));
                }
            });
        if state.noise_dist == NoiseDistKind::Weibull {
            ui.label("k:");
            ui.add(
                egui::DragValue::new(&mut state.weibull_shape)
                    .speed(0.1)
                    .range(0.2..=20.0),
            );
        }
    });

    // ── 仕様限界（LSL/USL） ───────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Spec limits:");
        // 既存結果の nominal を基準にスケール調整したステップ量。結果がまだ無ければ 0.1。
        let speed = state
            .cache
            .as_ref()
            .map(|(_, r)| (r.nominal.abs() * 0.01).max(0.01))
            .unwrap_or(0.1);
        ui.checkbox(&mut state.use_lower_spec, "LSL");
        ui.add_enabled(
            state.use_lower_spec,
            egui::DragValue::new(&mut state.lower_spec_value).speed(speed),
        );
        ui.checkbox(&mut state.use_upper_spec, "USL");
        ui.add_enabled(
            state.use_upper_spec,
            egui::DragValue::new(&mut state.upper_spec_value).speed(speed),
        );
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

    let lower_spec = state.use_lower_spec.then_some(state.lower_spec_value);
    let upper_spec = state.use_upper_spec.then_some(state.upper_spec_value);
    let key = cache_key(
        &trained,
        &center,
        state.noise_pct,
        state.n_samples,
        state.include_epistemic,
        state.noise_dist,
        state.weibull_shape,
        lower_spec,
        upper_spec,
    );
    if state.cache.as_ref().map(|(k, _)| k) != Some(&key) {
        let distribution = match state.noise_dist {
            NoiseDistKind::Normal => tunny_core::surrogate_opt::NoiseDistribution::Normal,
            NoiseDistKind::Uniform => tunny_core::surrogate_opt::NoiseDistribution::Uniform,
            NoiseDistKind::Weibull => tunny_core::surrogate_opt::NoiseDistribution::Weibull {
                shape: state.weibull_shape,
            },
        };
        let spec = RobustnessSpec {
            center,
            relative_sigma: state.noise_pct / 100.0,
            distribution,
            n_samples: state.n_samples,
            include_epistemic: state.include_epistemic,
            seed: 42,
            lower_spec,
            upper_spec,
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
        render_result(ui, result, lower_spec, upper_spec);
    }
}

#[allow(clippy::too_many_arguments)]
fn cache_key(
    trained: &Arc<TrainedSurrogate>,
    center: &[f64],
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
    noise_dist: NoiseDistKind,
    weibull_shape: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> RobustnessCacheKey {
    (
        Arc::as_ptr(trained) as usize,
        center.iter().map(|v| v.to_bits()).collect(),
        noise_pct.to_bits(),
        n_samples,
        include_epistemic,
        noise_dist as u8,
        weibull_shape.to_bits(),
        lower_spec.map(f64::to_bits),
        upper_spec.map(f64::to_bits),
    )
}

/// ヒストグラム + 統計サマリを描画する。
/// `lower_spec` / `upper_spec` は有効な場合のみヒストグラムに LSL/USL の縦線を描く。
fn render_result(
    ui: &mut egui::Ui,
    result: &RobustnessResult,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) {
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
    let chart = egui_plot::BarChart::new("Samples", bars).color(COLOR_BAR_PRIMARY());

    // 統計・成功確率のラベル行（プロット下）分の高さを確保する。
    // 確保しないとプロットが利用可能高さ全体に広がり、ラベルが
    // ウィジェット外へ押し出されて見えなくなる。
    let label_rows = 1
        + usize::from(result.feasibility_rate.is_some())
        + usize::from(result.success_rate.is_some())
        + usize::from(result.clipped_fraction > 0.0);
    let reserved = 8.0 + label_rows as f32 * 20.0;
    let plot_height = (ui.available_height() - reserved).max(120.0);

    egui_plot::Plot::new("robustness_histogram")
        .unified_nav()
        .legend(egui_plot::Legend::default())
        .x_axis_label("Output")
        .y_axis_label("Count")
        .height(plot_height)
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            plot_ui.bar_chart(chart);
            plot_ui.vline(
                egui_plot::VLine::new("Nominal", result.nominal)
                    .color(crate::theme::chart_colors::COLOR_GRID_STROKE())
                    .style(egui_plot::LineStyle::Dashed { length: 8.0 }),
            );
            plot_ui.vline(egui_plot::VLine::new("Mean", result.mean).color(COLOR_BAR_NEGATIVE()));
            plot_ui.vline(
                egui_plot::VLine::new("P5", result.p05)
                    .color(COLOR_BAR_ACCENT())
                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
            );
            plot_ui.vline(
                egui_plot::VLine::new("P95", result.p95)
                    .color(COLOR_BAR_ACCENT())
                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
            );
            if let Some(lsl) = lower_spec {
                plot_ui.vline(egui_plot::VLine::new("LSL", lsl).color(COLOR_BAR_NEGATIVE()));
            }
            if let Some(usl) = upper_spec {
                plot_ui.vline(egui_plot::VLine::new("USL", usl).color(COLOR_BAR_NEGATIVE()));
            }
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
    if let Some(rate) = result.success_rate {
        let sigma = result.sigma_level.unwrap_or(0.0);
        let color = if sigma >= 4.0 {
            crate::theme::chart_colors::COLOR_FIT_HIGH()
        } else if sigma >= 2.0 {
            crate::theme::chart_colors::COLOR_FIT_MID()
        } else {
            crate::theme::chart_colors::COLOR_FIT_LOW()
        };
        let mut line = format!("Success: {:.2}%  ・ σ level: {:.2}σ", rate * 100.0, sigma);
        if let Some(cpk) = result.cpk {
            line.push_str(&format!("  ・ Cpk: {cpk:.2}"));
        }
        ui.colored_label(color, line);
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
        assert_eq!(state.noise_dist, NoiseDistKind::Normal);
        assert_eq!(state.weibull_shape, 1.5);
        assert_eq!(state.n_samples, 1024);
        assert!(!state.include_epistemic);
        assert!(!state.use_lower_spec);
        assert_eq!(state.lower_spec_value, 0.0);
        assert!(!state.use_upper_spec);
        assert_eq!(state.upper_spec_value, 0.0);
        assert!(state.trained.is_none());
        assert!(!state.fitting);
        assert!(state.pending_fit.is_none());
        assert!(state.cached_result().is_none());
    }

    #[test]
    fn noise_dist_kind_default_is_normal() {
        assert_eq!(NoiseDistKind::default(), NoiseDistKind::Normal);
    }

    #[test]
    fn noise_dist_labels_cover_all_choices() {
        for kind in [
            NoiseDistKind::Normal,
            NoiseDistKind::Uniform,
            NoiseDistKind::Weibull,
        ] {
            assert!(!noise_dist_label(kind).is_empty());
        }
    }

    /// 最低限のフィールドだけ埋めた TrainedSurrogate をフィットして作る（cache_key 検証用）。
    fn make_dummy_trained() -> TrainedSurrogate {
        let xs: Vec<Vec<f64>> = (0..12)
            .map(|i| vec![i as f64 / 12.0, (i as f64 / 12.0) * 0.5])
            .collect();
        let ys: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix: xs,
            y: ys,
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_name: "obj".to_string(),
            model: SurrogateModelKind::Ridge,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
        };
        tunny_core::surrogate_opt::fit_surrogate_with_validation(&req).expect("dummy fit")
    }

    #[test]
    fn cache_key_changes_with_distribution_shape_and_specs() {
        let trained = Arc::new(make_dummy_trained());
        let center = vec![0.5, 0.25];

        let base = cache_key(
            &trained,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Normal,
            1.5,
            None,
            None,
        );
        let different_dist = cache_key(
            &trained,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Uniform,
            1.5,
            None,
            None,
        );
        let different_shape = cache_key(
            &trained,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Weibull,
            1.5,
            None,
            None,
        );
        let different_shape2 = cache_key(
            &trained,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Weibull,
            3.0,
            None,
            None,
        );
        let with_lower = cache_key(
            &trained,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Normal,
            1.5,
            Some(-1.0),
            None,
        );
        let with_upper = cache_key(
            &trained,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Normal,
            1.5,
            None,
            Some(1.0),
        );

        assert_ne!(base, different_dist);
        assert_ne!(base, different_shape);
        assert_ne!(different_shape, different_shape2);
        assert_ne!(base, with_lower);
        assert_ne!(base, with_upper);
        assert_ne!(with_lower, with_upper);

        // 同じ引数なら同じキーになる。
        let base_again = cache_key(
            &trained,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Normal,
            1.5,
            None,
            None,
        );
        assert_eq!(base, base_again);
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
