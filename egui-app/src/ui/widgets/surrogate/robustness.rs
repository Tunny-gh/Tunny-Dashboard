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

// モデル選択肢（コンボ表示順）。3 ウィジェット共通の単一情報源（`super::MODEL_CHOICES`）を使う。
use super::MODEL_CHOICES;

/// サンプル数の選択肢。
const SAMPLE_CHOICES: [usize; 3] = [256, 1024, 4096];

/// フィット段階の計算リクエスト。poll_chart が消費する。
pub struct RobustnessFitRequest {
    pub objective_index: usize,
    pub model: SurrogateModelKind,
}

/// キャッシュキーのスカラー部分（center を除く）: (フィット世代 ID,
/// ノイズ%のビット表現, サンプル数, 認識論的不確かさ込みか, ノイズ分布種別,
/// Weibull 形状パラメータのビット表現, LSL のビット表現（未指定なら None）,
/// USL のビット表現（未指定なら None）)。シードは固定 42 のためキーに含めない。
///
/// 先頭要素は以前 `Arc::as_ptr` だったが、解放後に同一アドレスが再利用されると別モデルの
/// 結果を誤表示しうる（ABA）。フィット採用時に単調増加する世代 ID
/// （`RobustnessChart::fit_generation`）へ置き換えて回避する。
///
/// center（Vec<u64>）とは別フィールドに分離している。スカラー部分は Copy で
/// ヒープ確保が無いため、毎フレームの再計算・比較を安価に行える。center は
/// 実際にキャッシュを再構築する（ミス時）ときだけ Vec<u64> 化する
/// （`cache_matches` はキャッシュ済み center とのゼロコピー要素比較で済ませる）。
type RobustnessScalarKey = (u64, u64, usize, bool, u8, u64, Option<u64>, Option<u64>);

/// 中心点解決結果のキャッシュキー: (フィット世代 ID, 中心選択, DataFrame 恒等性)。
/// 中心点解決（`resolve_center`）は全 trial を走査する O(N) 処理のため、
/// 入力が変わらないフレームでは再走査を避ける。
type CenterCacheKey = (u64, CenterChoice, usize);

/// キャッシュした解析結果と、そこから一度だけ組み立てたヒストグラムのバー幾何
/// `(中心, 高さ, 幅)`。結果と一緒に保持することで、毎フレームの `compute_histogram`
/// と Bar Vec 生成を避ける（結果が不変な限り再計算しない）。
struct RobustnessRender {
    result: RobustnessResult,
    bars: Vec<(f64, f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RobustnessCacheKey {
    scalar: RobustnessScalarKey,
    center_bits: Vec<u64>,
}

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
    cache: Option<(RobustnessCacheKey, RobustnessRender)>,
    /// 中心点解決結果のキャッシュ（毎フレームの O(N) 走査回避）。
    #[serde(skip)]
    center_cache: Option<(CenterCacheKey, Vec<f64>)>,
    /// フィット採用時に単調増加する世代 ID。キャッシュキーの `Arc::as_ptr` 置換用。
    #[serde(skip)]
    fit_generation: u64,
    /// 直近フレームで観測した学習済みモデルの Arc ポインタ（世代 ID 更新の変化検出用）。
    #[serde(skip)]
    fit_ptr: usize,
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
            center_cache: None,
            fit_generation: 0,
            fit_ptr: 0,
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
        self.cache.as_ref().map(|(_, r)| &r.result)
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

    // スライダー / DragValue をドラッグ操作中かどうか。ドラッグ中は値が毎フレーム
    // 変わりキャッシュミスし続けるため、最大 4096 サンプルのロバスト解析を毎フレーム
    // 同期再実行してしまう。ドラッグ中は再計算を保留（前回結果を表示）し、指を離した
    // フレーム（`dragged()` が false になる）で一度だけ再計算する（デバウンス）。
    let mut dragging = false;

    // ── ノイズ・サンプル数・認識論的不確かさ ─────────────────────
    ui.horizontal(|ui| {
        ui.label("Noise % (1σ of range):");
        dragging |= ui
            .add(egui::Slider::new(&mut state.noise_pct, 0.1..=10.0))
            .dragged();
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
            dragging |= ui
                .add(
                    egui::DragValue::new(&mut state.weibull_shape)
                        .speed(0.1)
                        .range(0.2..=20.0),
                )
                .dragged();
        }
    });

    // ── 仕様限界（LSL/USL） ───────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Spec limits:");
        // 既存結果の nominal を基準にスケール調整したステップ量。結果がまだ無ければ 0.1。
        let speed = state
            .cache
            .as_ref()
            .map(|(_, r)| (r.result.nominal.abs() * 0.01).max(0.01))
            .unwrap_or(0.1);
        ui.checkbox(&mut state.use_lower_spec, "LSL");
        dragging |= ui
            .add_enabled(
                state.use_lower_spec,
                egui::DragValue::new(&mut state.lower_spec_value).speed(speed),
            )
            .dragged();
        ui.checkbox(&mut state.use_upper_spec, "USL");
        dragging |= ui
            .add_enabled(
                state.use_upper_spec,
                egui::DragValue::new(&mut state.upper_spec_value).speed(speed),
            )
            .dragged();
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

    // フィット採用（trained の Arc が別モデルへ差し替わった）を検出して世代 ID を進める。
    // キャッシュキーはこの世代 ID を使い、`Arc::as_ptr` のアドレス再利用（ABA）を避ける。
    let trained_ptr = Arc::as_ptr(&trained) as usize;
    if trained_ptr != state.fit_ptr {
        state.fit_ptr = trained_ptr;
        state.fit_generation = state.fit_generation.wrapping_add(1);
    }

    // 中心点解決は全 trial を走査する O(N) 処理。入力（世代・選択・DataFrame）が
    // 変わらないフレームでは前回結果を再利用する。
    let center_key: CenterCacheKey = (
        state.fit_generation,
        state.center,
        Arc::as_ptr(&view.df) as usize,
    );
    if state.center_cache.as_ref().map(|(k, _)| k) != Some(&center_key) {
        state.center_cache = resolve_center(&trained, state.center, view, obj_names, directions)
            .map(|c| (center_key, c));
    }
    let Some((_, center)) = state.center_cache.as_ref() else {
        ui.colored_label(
            egui::Color32::RED,
            "Could not resolve the center point for the trained parameters.",
        );
        return;
    };
    let center = center.clone();

    let lower_spec = state.use_lower_spec.then_some(state.lower_spec_value);
    let upper_spec = state.use_upper_spec.then_some(state.upper_spec_value);
    // スカラー部分を先に比較し、一致した場合のみ center を（Vec 確保なしで）比較する。
    // これにより、キャッシュがヒットするフレームでは Vec<u64> の確保が発生しない。
    let cache_valid = state.cache.as_ref().is_some_and(|(k, _)| {
        cache_matches(
            k,
            state.fit_generation,
            &center,
            state.noise_pct,
            state.n_samples,
            state.include_epistemic,
            state.noise_dist,
            state.weibull_shape,
            lower_spec,
            upper_spec,
        )
    });
    // ドラッグ操作中はキャッシュミスしても再計算しない（デバウンス）。前回結果を表示し、
    // 指を離したフレーム（`dragging` が false）で一度だけ再計算する。
    if !cache_valid && !dragging {
        let key = cache_key(
            state.fit_generation,
            &center,
            state.noise_pct,
            state.n_samples,
            state.include_epistemic,
            state.noise_dist,
            state.weibull_shape,
            lower_spec,
            upper_spec,
        );
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
            Ok(result) => {
                // ヒストグラムのバー幾何は結果が不変な限り再利用できるため、ここで一度だけ組む。
                let bars = build_histogram_bars(&result.samples);
                state.cache = Some((key, RobustnessRender { result, bars }));
            }
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

    if let Some((_, render)) = &state.cache {
        render_result(ui, render, lower_spec, upper_spec);
    }
}

/// 出力サンプルからヒストグラムのバー幾何 `(中心, 高さ, 幅)` を組み立てる。
/// サンプル数が不足して `compute_histogram` が `None` を返す場合は空 Vec を返す
/// （呼び出し側はこれを「プロット不能」の合図として扱う）。
fn build_histogram_bars(samples: &[f64]) -> Vec<(f64, f64, f64)> {
    let Some(hist) = compute_histogram(samples, BinRule::Sturges) else {
        return Vec::new();
    };
    hist.bin_edges
        .windows(2)
        .zip(&hist.counts)
        .map(|(edge, &count)| {
            let raw_width = edge[1] - edge[0];
            let (center, width) = if raw_width > 0.0 {
                ((edge[0] + edge[1]) / 2.0, raw_width)
            } else {
                (edge[0], 1.0)
            };
            (center, count as f64, width)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn scalar_key(
    fit_generation: u64,
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
    noise_dist: NoiseDistKind,
    weibull_shape: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> RobustnessScalarKey {
    (
        fit_generation,
        noise_pct.to_bits(),
        n_samples,
        include_epistemic,
        noise_dist as u8,
        weibull_shape.to_bits(),
        lower_spec.map(f64::to_bits),
        upper_spec.map(f64::to_bits),
    )
}

/// キャッシュキーを構築する。center を Vec<u64> 化するため、キャッシュミス時
/// （実際に再計算・格納するとき）だけ呼ぶこと。毎フレームの比較には
/// `cache_matches` を使う。
#[allow(clippy::too_many_arguments)]
fn cache_key(
    fit_generation: u64,
    center: &[f64],
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
    noise_dist: NoiseDistKind,
    weibull_shape: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> RobustnessCacheKey {
    RobustnessCacheKey {
        scalar: scalar_key(
            fit_generation,
            noise_pct,
            n_samples,
            include_epistemic,
            noise_dist,
            weibull_shape,
            lower_spec,
            upper_spec,
        ),
        center_bits: center.iter().map(|v| v.to_bits()).collect(),
    }
}

/// 現在の入力がキャッシュ済みキーと一致するかを Vec 確保なしで判定する。
/// まずヒープ確保の無いスカラー部分を比較し、一致した場合のみ center を
/// 要素ごとにゼロコピーで比較する（新たな Vec<u64> は作らない）。
#[allow(clippy::too_many_arguments)]
fn cache_matches(
    cached: &RobustnessCacheKey,
    fit_generation: u64,
    center: &[f64],
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
    noise_dist: NoiseDistKind,
    weibull_shape: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> bool {
    let scalar = scalar_key(
        fit_generation,
        noise_pct,
        n_samples,
        include_epistemic,
        noise_dist,
        weibull_shape,
        lower_spec,
        upper_spec,
    );
    if cached.scalar != scalar {
        return false;
    }
    cached.center_bits.len() == center.len()
        && cached
            .center_bits
            .iter()
            .zip(center.iter())
            .all(|(&bits, &v)| bits == v.to_bits())
}

/// ヒストグラム + 統計サマリを描画する。
/// `lower_spec` / `upper_spec` は有効な場合のみヒストグラムに LSL/USL の縦線を描く。
/// バー幾何はキャッシュ済み（`render.bars`）のため、毎フレームの再ビン化は行わない。
fn render_result(
    ui: &mut egui::Ui,
    render: &RobustnessRender,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) {
    let result = &render.result;
    if render.bars.is_empty() {
        ui.label(egui::RichText::new("Not enough samples to plot.").weak());
        return;
    }

    // キャッシュ済みバー幾何（中心, 高さ, 幅）から egui_plot::Bar を再構築する（安価）。
    let bars: Vec<egui_plot::Bar> = render
        .bars
        .iter()
        .map(|&(center, height, width)| egui_plot::Bar::new(center, height).width(width))
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

    #[test]
    fn cache_key_changes_with_distribution_shape_and_specs() {
        // フィット世代 ID は固定値で検証する（`Arc::as_ptr` から置き換えた恒等性キー）。
        let gen = 1u64;
        let center = vec![0.5, 0.25];

        let base = cache_key(
            gen,
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
            gen,
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
            gen,
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
            gen,
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
            gen,
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
            gen,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Normal,
            1.5,
            None,
            Some(1.0),
        );
        // 世代 ID が変わると（＝別フィットを採用すると）キーも変わる。
        let different_generation = cache_key(
            gen + 1,
            &center,
            2.0,
            1024,
            false,
            NoiseDistKind::Normal,
            1.5,
            None,
            None,
        );

        assert_ne!(base, different_dist);
        assert_ne!(base, different_shape);
        assert_ne!(different_shape, different_shape2);
        assert_ne!(base, with_lower);
        assert_ne!(base, with_upper);
        assert_ne!(with_lower, with_upper);
        assert_ne!(base, different_generation);

        // 同じ引数なら同じキーになる。
        let base_again = cache_key(
            gen,
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
