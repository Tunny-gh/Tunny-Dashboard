//! Robustness analysis widget.
//!
//! Like `SurrogateOpt`, it trains a surrogate asynchronously (fit stage only,
//! see poll_chart.rs), but the robustness analysis itself
//! (`tunny_core::surrogate_opt::robustness_analysis`) runs on the order of
//! milliseconds, so it is executed synchronously during the render pass and
//! the result is cached.
//!
//! Gaussian input noise is applied around a candidate design point (the best
//! trial or a pinned trial), and the resulting output distribution through the
//! surrogate prediction is shown as a histogram. See
//! theory/{en,ja}/optimization/robustness-analysis.md for the theoretical
//! background.
//!
//! Module layout (split by responsibility; the externally visible public API
//! is kept in this mod):
//!   - `cache`: the fit-stage compute request and cache-key logic.
//!   - `render`: histogram bar geometry construction and result rendering.

mod cache;
mod render;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use tunny_core::surrogate_opt::{
    RobustnessResult, RobustnessSpec, SurrogateModelKind, TrainedSurrogate,
    MIN_TRIALS_FOR_SURROGATE_OPT,
};

use super::anchor::{center_label, resolve_center, CenterChoice};
use crate::state::types::{Direction, StudyView};
use cache::{cache_key, cache_matches, RobustnessCacheKey, RobustnessFitRequest};
use render::{build_histogram_bars, render_result, RobustnessRender};

// Model choices (combo display order). Shares the single source of truth
// (`super::MODEL_CHOICES`) across all three widgets.
use super::MODEL_CHOICES;

/// Sample count choices.
const SAMPLE_CHOICES: [usize; 3] = [256, 1024, 4096];

/// Cache key for the resolved center point: (fit generation ID, center
/// choice, DataFrame identity). Center resolution (`resolve_center`) is an
/// O(N) scan over all trials, so avoid re-scanning on frames where the inputs
/// have not changed.
type CenterCacheKey = (u64, CenterChoice, usize);

/// Noise distribution choices (widget-local).
///
/// The core `NoiseDistribution` is a data-carrying type with `Weibull { shape }`
/// that does not implement serialization, so this thin mirror exists to persist
/// UI state. The Weibull shape parameter itself is kept in a separate field,
/// `weibull_shape`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoiseDistKind {
    #[default]
    Normal,
    Uniform,
    Weibull,
}

/// Label for combo box display.
fn noise_dist_label(kind: NoiseDistKind) -> &'static str {
    match kind {
        NoiseDistKind::Normal => "Normal",
        NoiseDistKind::Uniform => "Uniform",
        NoiseDistKind::Weibull => "Weibull",
    }
}

/// UI state for the robustness analysis widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RobustnessChart {
    pub selected_objective: usize,
    pub model: SurrogateModelKind,
    pub center: CenterChoice,
    /// Noise 1σ (as a percentage of the parameter range).
    pub noise_pct: f64,
    /// Shape of the noise distribution.
    pub noise_dist: NoiseDistKind,
    /// Shape parameter k of the Weibull distribution (range [0.2, 20]).
    pub weibull_shape: f64,
    pub n_samples: usize,
    pub include_epistemic: bool,
    /// Whether the lower spec limit (LSL) is enabled, and its value.
    pub use_lower_spec: bool,
    pub lower_spec_value: f64,
    /// Whether the upper spec limit (USL) is enabled, and its value.
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
    /// Cache of the resolved center point (avoids an O(N) scan every frame).
    #[serde(skip)]
    center_cache: Option<(CenterCacheKey, Vec<f64>)>,
    /// Generation ID that increases monotonically when a fit is adopted. Used
    /// in place of `Arc::as_ptr` in the cache key.
    #[serde(skip)]
    fit_generation: u64,
    /// Arc pointer of the trained model observed in the most recent frame
    /// (used to detect changes for the generation ID update).
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
    /// Pulls in the global widget's compute state, result, and error (used to
    /// propagate state to each canvas item). UI selections such as objective,
    /// model, center point, and noise settings remain per-item.
    pub fn adopt_compute_state(&mut self, global: &Self) {
        self.trained = global.trained.clone();
        self.fitting = global.fitting;
        self.fit_error = global.fit_error.clone();
    }

    /// The most recent robustness analysis result (cached). Referenced by CSV
    /// export etc.
    pub fn cached_result(&self) -> Option<&RobustnessResult> {
        self.cache.as_ref().map(|(_, r)| &r.result)
    }
}

/// `obj_names` / `directions` are all objectives of the current study (used to
/// resolve the best trial). `pinned_trials` is the list of pinned trial IDs
/// (candidates for the Center combo box).
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

    // ── Objective / model selection ─────────────────────────────
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

    // ── Center point selection ───────────────────────────────────
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

    // Whether a slider / DragValue is currently being dragged. While dragging,
    // the value changes every frame and keeps missing the cache, which would
    // synchronously re-run the up-to-4096-sample robustness analysis every
    // frame. Recomputation is deferred while dragging (the previous result is
    // shown), and recomputed exactly once on the frame where the drag ends
    // (`dragged()` becomes false) — i.e. debounced.
    let mut dragging = false;

    // ── Noise / sample count / epistemic uncertainty ─────────────
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

    // ── Noise distribution ───────────────────────────────────────
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

    // ── Spec limits (LSL/USL) ─────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Spec limits:");
        // Step size scaled based on the nominal value of the existing result.
        // 0.1 if there is no result yet.
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

    // ── Insufficient trial count ──────────────────────────────────
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

    // ── Fit Surrogate button ───────────────────────────────────────
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

    // Detect that a fit was adopted (the trained Arc was swapped for a
    // different model) and advance the generation ID. The cache key uses this
    // generation ID to avoid address reuse (ABA problem) with `Arc::as_ptr`.
    let trained_ptr = Arc::as_ptr(&trained) as usize;
    if trained_ptr != state.fit_ptr {
        state.fit_ptr = trained_ptr;
        state.fit_generation = state.fit_generation.wrapping_add(1);
    }

    // Center point resolution is an O(N) scan over all trials. Reuse the
    // previous result on frames where the inputs (generation, selection,
    // DataFrame) have not changed.
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
    // Compare the scalar part first, and only compare center (without
    // allocating a Vec) if it matches. This means no Vec<u64> allocation
    // happens on frames where the cache hits.
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
    // Don't recompute on a cache miss while dragging (debounced). Show the
    // previous result and recompute exactly once on the frame where the drag
    // ends (`dragging` is false).
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
                // The histogram bar geometry can be reused as long as the
                // result is unchanged, so build it here exactly once.
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
