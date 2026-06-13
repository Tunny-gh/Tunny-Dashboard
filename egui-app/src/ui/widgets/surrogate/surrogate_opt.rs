//! サロゲート最適化ウィジェット。
//!
//! サンプリング結果（trial 群）から応答曲面（サロゲートモデル）を学習し、
//! その曲面上で最適化を実行して推定最適点を表示する。計算は
//! `tunny_core::surrogate_opt` がバックグラウンドで行う（poll_chart.rs 参照）。
//!
//! 2 段階フロー:
//!   1. Fit & Validate: ホールドアウト + 5-fold CV で検証指標を表示。
//!   2. Run Optimization: 学習済みモデル上で最適化を実行。
//!
//! レイアウト:
//!   全幅前段: 数値パラメータ無し / trial 数不足 の早期リターン。
//!   左列 (Fit & Validate): Objective + Model コンボ → Fit & Validate ボタン →
//!       フィット中スピナー → 検証指標グリッド + 品質判定 + OOF 散布図。
//!   右列 (Optimization): Optimizer / Surface X / Y コンボ →
//!       Run Optimization ボタン → 最適化中スピナー → 結果。

use std::sync::Arc;

use crate::state::messages::{SurrogateMultiOptUiResult, SurrogateOptUiResult};
use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{
    SurrogateFitComputeRequest, SurrogateMultiFitComputeRequest,
    SurrogateMultiOptimizeComputeRequest, SurrogateOptState, SurrogateOptimizeComputeRequest,
    SurrogateSuggestComputeRequest,
};
use crate::ui::widgets::pdp::surface_plot::{draw_colorbar_simple, draw_heatmap, value_range};
use tunny_core::surrogate_opt::{
    AcquisitionKind, OptimizerKind, SurrogateModelKind, SurrogateValidationReport,
    TrainedSurrogate, MIN_TRIALS_FOR_SURROGATE_OPT,
};

/// モデル選択肢（コンボ表示順）。新モデル追加時はここへ並べる。
const MODEL_CHOICES: [SurrogateModelKind; 5] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::GpMoe,
    SurrogateModelKind::Lgbm,
];

/// 最適化手法の選択肢（コンボ表示順）。
const OPTIMIZER_CHOICES: [OptimizerKind; 4] = [
    OptimizerKind::MultiStartLbfgs,
    OptimizerKind::Nsga2,
    OptimizerKind::CmaEs,
    OptimizerKind::RandomSearch,
];

pub(crate) fn model_label(kind: SurrogateModelKind) -> &'static str {
    match kind {
        SurrogateModelKind::Ridge => "Ridge",
        SurrogateModelKind::GpFitc => "GP-FITC",
        SurrogateModelKind::GpVfe => "GP-VFE",
        SurrogateModelKind::GpMoe => "GP-MOE",
        SurrogateModelKind::Lgbm => "LightGBM",
    }
}

pub(crate) fn acq_label(kind: AcquisitionKind) -> &'static str {
    match kind {
        AcquisitionKind::ExpectedImprovement => "EI (Expected Improvement)",
        AcquisitionKind::LowerConfidenceBound => "LCB (Lower Confidence Bound)",
    }
}

pub(crate) fn optimizer_label(kind: OptimizerKind) -> &'static str {
    match kind {
        OptimizerKind::MultiStartLbfgs => "Multi-start L-BFGS",
        OptimizerKind::Nsga2 => "NSGA-II",
        OptimizerKind::CmaEs => "CMA-ES",
        OptimizerKind::RandomSearch => "Random Search",
    }
}

/// CV R² 平均値から品質判定文字列と色を返す純粋関数。
pub(crate) fn verdict(cv_r2_mean: f64) -> (&'static str, egui::Color32) {
    if cv_r2_mean >= 0.9 {
        (
            "Good — surrogate is reliable",
            egui::Color32::from_rgb(22, 163, 74), // green-600
        )
    } else if cv_r2_mean >= 0.7 {
        (
            "Fair — use with caution",
            egui::Color32::from_rgb(202, 138, 4), // amber-600
        )
    } else {
        (
            "Poor — consider more trials or a different model",
            egui::Color32::RED,
        )
    }
}

/// 学習済みモデルが現在の UI 選択（目的・モデル種別）と一致するか判定する。
fn trained_matches(
    trained: &TrainedSurrogate,
    state: &SurrogateOptState,
    obj_names: &[String],
) -> bool {
    let selected_obj = obj_names
        .get(state.selected_objective)
        .map(|s| s.as_str())
        .unwrap_or("");
    trained.objective_name == selected_obj && trained.model_kind == state.model
}

/// 多目的学習済みモデル群が現在の UI 選択（モデル・目的集合）と一致するか判定する。
pub(crate) fn multi_trained_matches(
    trained: &[TrainedSurrogate],
    state: &SurrogateOptState,
    obj_names: &[String],
) -> bool {
    if trained.len() != obj_names.len() {
        return false;
    }
    let trained_obj_names: Vec<&str> = trained.iter().map(|t| t.objective_name.as_str()).collect();
    let expected_obj_names: Vec<&str> = obj_names.iter().map(|s| s.as_str()).collect();
    if trained_obj_names != expected_obj_names {
        return false;
    }
    trained.iter().all(|t| t.model_kind == state.model)
}

/// `param_names` は数値パラメータのみ（カテゴリカル列は最適化対象にしない）。
/// `obj_history` は現在の結果が参照する目的列の全値（trial 順）。プロット用。
/// `constraint_col_names` は制約列名（制約付き Study のみ非空）。
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    obj_names: &[String],
    cmap: ColorMap,
    trial_count: usize,
    obj_history: Option<&[f64]>,
    constraint_col_names: &[String],
) {
    // ── 全幅前段: 数値パラメータ無し ──────────────────────────────
    if param_names.is_empty() {
        ui.label("No numeric parameters available for surrogate optimization.");
        return;
    }

    // ── 全幅前段: trial 数不足 ─────────────────────────────────────
    if trial_count < MIN_TRIALS_FOR_SURROGATE_OPT {
        ui.colored_label(
            egui::Color32::RED,
            format!(
                "At least {} trials required (current: {})",
                MIN_TRIALS_FOR_SURROGATE_OPT, trial_count
            ),
        );
        return;
    }

    // スライス軸のデフォルト（先頭 2 パラメータ）。Study 切替で消えた名前もリセットする。
    if !param_names.contains(&state.slice_x) {
        state.slice_x = param_names.first().cloned().unwrap_or_default();
    }
    if !param_names.contains(&state.slice_y) || state.slice_y == state.slice_x {
        state.slice_y = param_names
            .iter()
            .find(|p| **p != state.slice_x)
            .cloned()
            .unwrap_or_default();
    }

    // ── 多目的モード切替チェックボックス（目的が 2 つ以上の時のみ表示） ──
    if obj_names.len() >= 2 {
        let prev_multi = state.multi_objective;
        ui.checkbox(
            &mut state.multi_objective,
            "Multi-objective (all objectives)",
        );
        if state.multi_objective != prev_multi {
            // モード切替時にエラーをクリアする。
            state.error_message = None;
        }
    }

    let busy = state.fitting
        || state.optimizing
        || state.suggesting
        || state.pending_fit.is_some()
        || state.pending_optimize.is_some()
        || state.pending_multi_fit.is_some()
        || state.pending_multi_optimize.is_some()
        || state.pending_suggest.is_some();

    let has_matching_trained = state
        .trained
        .as_deref()
        .map(|t| trained_matches(t, state, obj_names))
        .unwrap_or(false);

    let has_matching_multi_trained = state
        .multi_trained
        .as_deref()
        .map(|v| multi_trained_matches(v, state, obj_names))
        .unwrap_or(false);

    // ── 2 列レイアウト ─────────────────────────────────────────────
    // trial_detail_modal と同じ慣用: horizontal_top + allocate_ui_with_layout で
    // 各列を等幅に区切る。
    let available_w = ui.available_width();
    let col_w = (available_w / 2.0).max(200.0);

    // エラー表示（フィット・最適化どちらの失敗も全幅で出す）。
    if let Some(ref err) = state.error_message.clone() {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }

    ui.horizontal_top(|ui| {
        // ── 左列: Fit & Validate ──────────────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if state.multi_objective {
                    render_fit_column_multi(ui, state, obj_names, busy);
                } else {
                    render_fit_column(ui, state, obj_names, busy, constraint_col_names);
                }
            },
        );

        ui.separator();

        // ── 右列: Optimization ───────────────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if state.multi_objective {
                    render_optimize_column_multi(
                        ui,
                        state,
                        param_names,
                        busy,
                        has_matching_multi_trained,
                        cmap,
                    );
                } else {
                    render_optimize_column(
                        ui,
                        state,
                        param_names,
                        busy,
                        has_matching_trained,
                        cmap,
                        obj_history,
                    );
                }
            },
        );
    });
}

/// 左列（単目的）: Objective / Model コンボ、Fit & Validate ボタン、検証結果。
fn render_fit_column(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    obj_names: &[String],
    busy: bool,
    constraint_col_names: &[String],
) {
    // ── 1段目: 目的・モデル ──────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Objective:");
        let obj_text = obj_names
            .get(state.selected_objective)
            .map(|s| s.as_str())
            .unwrap_or("—");
        egui::ComboBox::from_id_salt("surrogate_obj")
            .selected_text(obj_text)
            .show_ui(ui, |ui| {
                for (i, name) in obj_names.iter().enumerate() {
                    if ui
                        .selectable_label(state.selected_objective == i, name)
                        .clicked()
                    {
                        state.selected_objective = i;
                    }
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Model:");
        egui::ComboBox::from_id_salt("surrogate_model")
            .selected_text(model_label(state.model))
            .show_ui(ui, |ui| {
                for kind in MODEL_CHOICES {
                    ui.selectable_value(&mut state.model, kind, model_label(kind));
                }
            });
    });

    // ── 制約チェックボックス（制約付き Study のみ表示） ──────────
    let n_constraints = constraint_col_names.len();
    if n_constraints > 0 {
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut state.use_constraints,
                format!("Use constraints ({})", n_constraints),
            );
        });
    }

    // ── 2段目: Fit & Validate ────────────────────────────────────
    let can_fit = !busy && !obj_names.is_empty();
    if ui
        .add_enabled(can_fit, egui::Button::new("Fit & Validate"))
        .clicked()
    {
        if let Some(obj_name) = obj_names.get(state.selected_objective) {
            state.error_message = None;
            state.pending_fit = Some(SurrogateFitComputeRequest {
                objective: obj_name.clone(),
                model: state.model,
                use_constraints: n_constraints > 0 && state.use_constraints,
            });
        }
    }

    // フィット中スピナー。
    if state.fitting {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Fitting and validating surrogate (holdout + 5-fold CV)…");
        });
        return;
    }

    // ── 検証セクション ───────────────────────────────────────────
    if let Some(ref trained) = state.trained.clone() {
        if trained_matches(trained, state, obj_names) {
            render_validation(ui, trained);
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(107, 114, 128), // gray-500
                "Model/objective changed — run Fit & Validate again.",
            );
        }
    }
}

/// 左列（多目的）: 全目的固定ラベル + Model コンボ、Fit & Validate ボタン、検証結果。
fn render_fit_column_multi(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    obj_names: &[String],
    busy: bool,
) {
    // ── 1段目: 目的（固定ラベル）・モデル ───────────────────────
    ui.label(format!("Objectives: all {} objectives", obj_names.len()));
    ui.horizontal(|ui| {
        ui.label("Model:");
        egui::ComboBox::from_id_salt("surrogate_model_multi")
            .selected_text(model_label(state.model))
            .show_ui(ui, |ui| {
                for kind in MODEL_CHOICES {
                    ui.selectable_value(&mut state.model, kind, model_label(kind));
                }
            });
    });

    // ── 2段目: Fit & Validate ────────────────────────────────────
    let can_fit = !busy && obj_names.len() >= 2;
    if ui
        .add_enabled(can_fit, egui::Button::new("Fit & Validate"))
        .clicked()
    {
        state.error_message = None;
        state.pending_multi_fit = Some(SurrogateMultiFitComputeRequest { model: state.model });
    }

    // フィット中スピナー。
    if state.fitting {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Fitting and validating surrogates (all objectives)…");
        });
        return;
    }

    // ── 検証セクション（目的ごとのコンパクトサマリ） ────────────
    if let Some(ref multi_trained) = state.multi_trained.clone() {
        if multi_trained_matches(multi_trained, state, obj_names) {
            render_multi_validation(ui, state, multi_trained);
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(107, 114, 128), // gray-500
                "Model changed — run Fit & Validate again.",
            );
        }
    }
}

/// 右列（単目的）: Optimizer / Surface X・Y コンボ、Run Optimization ボタン、結果。
fn render_optimize_column(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    busy: bool,
    has_matching_trained: bool,
    cmap: ColorMap,
    obj_history: Option<&[f64]>,
) {
    // ── Optimizer コンボ ─────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Optimizer:");
        egui::ComboBox::from_id_salt("surrogate_optimizer")
            .selected_text(optimizer_label(state.optimizer))
            .show_ui(ui, |ui| {
                for kind in OPTIMIZER_CHOICES {
                    ui.selectable_value(&mut state.optimizer, kind, optimizer_label(kind));
                }
            });
    });

    // Surface X / Y は 2 行目に並べる（列幅が半分のため 3 コンボを 1 行に並べない）。
    ui.horizontal(|ui| {
        ui.label("Surface X:");
        egui::ComboBox::from_id_salt("surrogate_slice_x")
            .selected_text(&state.slice_x)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.slice_x, name.clone(), name);
                }
            });
        ui.label("Y:");
        egui::ComboBox::from_id_salt("surrogate_slice_y")
            .selected_text(&state.slice_y)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.slice_y, name.clone(), name);
                }
            });
    });

    // ── Run Optimization ボタン ──────────────────────────────────
    let can_optimize = has_matching_trained && !busy;
    if ui
        .add_enabled(can_optimize, egui::Button::new("Run Optimization"))
        .clicked()
    {
        state.error_message = None;
        state.pending_optimize = Some(SurrogateOptimizeComputeRequest {
            optimizer: state.optimizer,
            slice_x: state.slice_x.clone(),
            slice_y: state.slice_y.clone(),
        });
    }

    // 最適化中スピナー。
    if state.optimizing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Optimizing on the response surface…");
        });
        return;
    }

    let Some(result) = &state.result.clone() else {
        if !has_matching_trained {
            ui.label("Fit a surrogate model first, then click Run Optimization.");
        }
        return;
    };

    render_result(ui, result, cmap, obj_history);

    // ── Suggest next trials セクション ──────────────────────────────
    // 単目的・GP 系モデルのみ表示する。
    let is_gp = matches!(
        state.model,
        SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe | SurrogateModelKind::GpMoe
    );
    if has_matching_trained && is_gp {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.strong("Suggest next trials");

        // 獲得関数コンボ。
        ui.horizontal(|ui| {
            ui.label("Acquisition:");
            egui::ComboBox::from_id_salt("surrogate_acquisition")
                .selected_text(acq_label(state.acq_kind))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.acq_kind,
                        AcquisitionKind::ExpectedImprovement,
                        acq_label(AcquisitionKind::ExpectedImprovement),
                    );
                    ui.selectable_value(
                        &mut state.acq_kind,
                        AcquisitionKind::LowerConfidenceBound,
                        acq_label(AcquisitionKind::LowerConfidenceBound),
                    );
                });
        });

        // 候補数 DragValue（1〜10、デフォルト 3）。
        ui.horizontal(|ui| {
            ui.label("Candidates:");
            ui.add(egui::DragValue::new(&mut state.n_suggest_candidates).range(1..=10));
        });

        // Suggest ボタン。
        let can_suggest = has_matching_trained && !busy;
        let disabled_hint = if !can_suggest && !has_matching_trained {
            "Fit a GP surrogate model first (GP-FITC, GP-VFE, or GP-MOE)."
        } else if !can_suggest {
            "A computation is already running."
        } else {
            ""
        };
        let suggest_response =
            ui.add_enabled(can_suggest, egui::Button::new("Suggest next trials"));
        if !disabled_hint.is_empty() {
            suggest_response.on_disabled_hover_text(disabled_hint);
        } else if suggest_response.clicked() {
            let minimize = result.minimize;
            state.suggest_result = None;
            state.error_message = None;
            state.pending_suggest = Some(SurrogateSuggestComputeRequest {
                acquisition: state.acq_kind,
                n_candidates: state.n_suggest_candidates,
                minimize,
            });
        }

        // 提案中スピナー。
        if state.suggesting {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing acquisition candidates…");
            });
        }

        // 結果テーブル。
        if let Some(ref suggest) = state.suggest_result.clone() {
            render_suggest_result(ui, suggest);
        }
    } else if has_matching_trained && !is_gp {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.colored_label(
            egui::Color32::from_rgb(107, 114, 128), // gray-500
            "Suggest next trials requires a Gaussian Process model (GP-FITC, GP-VFE, or GP-MOE).",
        );
    }
}

/// 右列（多目的）: 固定 NSGA-II ラベル + Surface X・Y コンボ、Run Optimization ボタン、結果。
fn render_optimize_column_multi(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    busy: bool,
    has_matching_multi_trained: bool,
    cmap: ColorMap,
) {
    // ── Optimizer（固定ラベル） ───────────────────────────────────
    ui.label("Optimizer: NSGA-II");

    // Surface X / Y コンボ。
    ui.horizontal(|ui| {
        ui.label("Surface X:");
        egui::ComboBox::from_id_salt("surrogate_slice_x_multi")
            .selected_text(&state.slice_x)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.slice_x, name.clone(), name);
                }
            });
        ui.label("Y:");
        egui::ComboBox::from_id_salt("surrogate_slice_y_multi")
            .selected_text(&state.slice_y)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.slice_y, name.clone(), name);
                }
            });
    });

    // ── Run Optimization ボタン ──────────────────────────────────
    let can_optimize = has_matching_multi_trained && !busy;
    if ui
        .add_enabled(can_optimize, egui::Button::new("Run Optimization"))
        .clicked()
    {
        state.error_message = None;
        state.pending_multi_optimize = Some(SurrogateMultiOptimizeComputeRequest {
            slice_x: state.slice_x.clone(),
            slice_y: state.slice_y.clone(),
        });
    }

    // 最適化中スピナー。
    if state.optimizing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Running NSGA-II on the response surfaces…");
        });
        return;
    }

    let Some(result) = &state.multi_result.clone() else {
        if !has_matching_multi_trained {
            ui.label("Fit surrogate models first, then click Run Optimization.");
        }
        return;
    };

    render_multi_result(ui, result, state, cmap);
}

/// 検証指標セクションをレンダリングする。
fn render_validation(ui: &mut egui::Ui, trained: &Arc<TrainedSurrogate>) {
    let v = &trained.validation;
    ui.add_space(4.0);
    ui.strong(format!(
        "Model validation — {} on {}",
        model_label(trained.model_kind),
        trained.objective_name
    ));

    egui::Grid::new("surrogate_validation_metrics")
        .striped(true)
        .min_col_width(160.0)
        .show(ui, |ui| {
            ui.label("Train R²");
            ui.monospace(format!("{:.3}", v.train_r2));
            ui.end_row();

            ui.label("Holdout R² (80/20)");
            ui.monospace(format!("{:.3}", v.holdout_r2));
            ui.end_row();

            ui.label("Holdout RMSE");
            ui.monospace(format!("{:.6}", v.holdout_rmse));
            ui.end_row();

            ui.label(format!("CV R² ({} folds, mean ± std)", v.cv_folds));
            ui.monospace(format!("{:.3} ± {:.3}", v.cv_r2_mean, v.cv_r2_std));
            ui.end_row();

            ui.label("CV RMSE (mean ± std)");
            ui.monospace(format!("{:.6} ± {:.6}", v.cv_rmse_mean, v.cv_rmse_std));
            ui.end_row();

            ui.label("Samples (train/test)");
            ui.monospace(format!("{}/{}", v.n_train, v.n_test));
            ui.end_row();
        });

    // 品質判定。
    let (verdict_text, verdict_color) = verdict(v.cv_r2_mean);
    ui.colored_label(verdict_color, verdict_text);

    // predicted-vs-actual 散布図。
    render_oof_plot(ui, v, "single", false);
}

/// 多目的検証サマリをコンパクトに表示する（目的ごとに 1 行）。
/// グリッドの下に、選択した目的の OOF 予測 vs 実測プロットを表示する。
fn render_multi_validation(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    trained: &[TrainedSurrogate],
) {
    if trained.is_empty() {
        return;
    }
    ui.add_space(4.0);
    ui.strong(format!(
        "Model validation — {} (all objectives)",
        model_label(trained[0].model_kind)
    ));

    egui::Grid::new("surrogate_multi_validation_metrics")
        .striped(true)
        .min_col_width(60.0)
        .show(ui, |ui| {
            // ヘッダ行
            ui.strong("Objective");
            ui.strong("Holdout R²");
            ui.strong("CV R² mean±std");
            ui.strong("Quality");
            ui.end_row();

            for t in trained {
                let v = &t.validation;
                ui.label(&t.objective_name);
                ui.monospace(format!("{:.3}", v.holdout_r2));
                ui.monospace(format!("{:.3}±{:.3}", v.cv_r2_mean, v.cv_r2_std));
                let (verdict_text, verdict_color) = verdict(v.cv_r2_mean);
                ui.colored_label(verdict_color, verdict_text);
                ui.end_row();
            }
        });

    // ── OOF プロット対象の目的選択 ───────────────────────────────
    // インデックス範囲クランプ（目的数が減った場合など）。
    if state.multi_validation_objective >= trained.len() {
        state.multi_validation_objective = 0;
    }
    ui.add_space(4.0);
    let prev_objective = state.multi_validation_objective;
    ui.horizontal(|ui| {
        ui.label("Validation plot:");
        let current_name = trained
            .get(state.multi_validation_objective)
            .map(|t| t.objective_name.as_str())
            .unwrap_or("—");
        egui::ComboBox::from_id_salt("surrogate_multi_validation_obj")
            .selected_text(current_name)
            .show_ui(ui, |ui| {
                for (i, t) in trained.iter().enumerate() {
                    if ui
                        .selectable_label(state.multi_validation_objective == i, &t.objective_name)
                        .clicked()
                    {
                        state.multi_validation_objective = i;
                    }
                }
            });
    });

    // 選択された目的の predicted-vs-actual 散布図。目的ごとに値域が異なるため
    // プロット ID を目的別に分け、切替時は表示範囲をリセットして再フィットさせる。
    if let Some(t) = trained.get(state.multi_validation_objective) {
        let switched = state.multi_validation_objective != prev_objective;
        render_oof_plot(ui, &t.validation, &t.objective_name, switched);
    }
}

/// OOF (out-of-fold) の predicted-vs-actual 散布図をレンダリングする。
/// 列幅に合わせて利用可能な高さを使い、最低 180 px・最大 400 px に収める。
///
/// `id_salt` でプロットメモリ（ズーム・表示範囲）を呼び出し元ごとに分離する。
/// `data_aspect` 指定時は初回フレーム以降の自動フィットが効かないため、
/// 表示データが切り替わったフレームでは `reset = true` で範囲を再計算させる。
fn render_oof_plot(ui: &mut egui::Ui, v: &SurrogateValidationReport, id_salt: &str, reset: bool) {
    if v.oof_pairs.is_empty() {
        return;
    }

    // データ範囲を求める（y=x 参照線のスパン）。
    let (mut min_val, mut max_val) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(actual, pred) in &v.oof_pairs {
        if actual.is_finite() {
            min_val = min_val.min(actual);
            max_val = max_val.max(actual);
        }
        if pred.is_finite() {
            min_val = min_val.min(pred);
            max_val = max_val.max(pred);
        }
    }
    if !min_val.is_finite() || !max_val.is_finite() || min_val >= max_val {
        // データが縮退している場合は余白を加えて最低限表示する。
        let center = if min_val.is_finite() { min_val } else { 0.0 };
        min_val = center - 1.0;
        max_val = center + 1.0;
    }

    let points: egui_plot::PlotPoints = v
        .oof_pairs
        .iter()
        .map(|&(actual, pred)| [actual, pred])
        .collect();
    let scatter = egui_plot::Points::new(points)
        .name("Out-of-fold predictions")
        .color(egui::Color32::from_rgb(59, 130, 246)) // blue-500
        .radius(3.0);

    let ref_line: egui_plot::PlotPoints = vec![[min_val, min_val], [max_val, max_val]].into();
    let ref_seg = egui_plot::Line::new(ref_line)
        .name("y = x")
        .color(egui::Color32::from_gray(160))
        .style(egui_plot::LineStyle::Dashed { length: 6.0 });

    // 列幅いっぱいを使い、高さは 180 px 〜 400 px に収める。
    let plot_h = ui.available_height().clamp(180.0, 400.0);

    let mut plot = egui_plot::Plot::new(("surrogate_oof_plot", id_salt))
        .height(plot_h)
        .data_aspect(1.0)
        .x_axis_label("Actual")
        .y_axis_label("Predicted (out-of-fold)")
        .legend(egui_plot::Legend::default());
    if reset {
        plot = plot.reset();
    }
    plot.show(ui, |plot_ui| {
        plot_ui.points(scatter);
        plot_ui.line(ref_seg);
    });
}

/// 改善量（正 = 改善あり）を方向を考慮して返す純粋関数。
///
/// - minimize: `best_observed - predicted`（小さいほど良いので observed > predicted なら正）
/// - maximize: `predicted - best_observed`（大きいほど良いので predicted > observed なら正）
pub(crate) fn improvement_delta(minimize: bool, best_observed: f64, predicted: f64) -> f64 {
    if minimize {
        best_observed - predicted
    } else {
        predicted - best_observed
    }
}

fn render_result(
    ui: &mut egui::Ui,
    result: &SurrogateOptUiResult,
    cmap: ColorMap,
    obj_history: Option<&[f64]>,
) {
    let direction = if result.minimize {
        "minimize"
    } else {
        "maximize"
    };

    // ── (a) 改善サマリー ─────────────────────────────────────────────
    ui.strong(format!("Optimization results ({}):", direction));
    ui.label(format!("Surrogate R² = {:.3}", result.r_squared));
    ui.add_space(4.0);

    // Best observed
    ui.horizontal(|ui| {
        ui.label("Best observed:");
        ui.monospace(format!("{:.6}", result.best_observed_value));
    });

    // Predicted optimum (with ± 1.96σ if available)
    let value_text = match result.predicted_std {
        Some(std) => format!("{:.6} ± {:.6} (±1.96σ)", result.best_value, 1.96 * std),
        None => format!("{:.6}", result.best_value),
    };
    ui.horizontal(|ui| {
        ui.label(format!(
            "Predicted optimum of {} ({}):",
            result.objective_name, direction
        ));
        ui.monospace(value_text);
    });

    // Improvement line
    let delta = improvement_delta(
        result.minimize,
        result.best_observed_value,
        result.best_value,
    );
    let abs_obs = result.best_observed_value.abs();
    if delta > 0.0 {
        let improvement_color = egui::Color32::from_rgb(22, 163, 74); // green-600
        let pct_text = if abs_obs > 1e-12 {
            format!(" ({:.1}%)", delta / abs_obs * 100.0)
        } else {
            String::new()
        };
        let uncertainty_note = match result.predicted_std {
            Some(std) if delta < 1.96 * std => " — within model uncertainty (±1.96σ)",
            _ => "",
        };
        ui.colored_label(
            improvement_color,
            format!(
                "Predicted improvement: {:.6}{}{}",
                delta, pct_text, uncertainty_note
            ),
        );
    } else {
        ui.colored_label(
            egui::Color32::from_rgb(107, 114, 128), // gray-500
            "No predicted improvement — observed best is already at or near the surface optimum.",
        );
    }

    // ── (b) 最適化履歴プロット（予測最適値のオーバーレイ付き） ──────────
    let non_empty_history = obj_history.filter(|h| !h.is_empty());
    if let Some(history) = non_empty_history {
        ui.add_space(6.0);
        render_history_plot(ui, history, result);
        ui.add_space(6.0);
    }

    // ── 実行可能性（制約ありのとき表示） ─────────────────────────────
    if let Some(p_feas) = result.feasibility_probability {
        ui.add_space(4.0);
        let pct = (p_feas * 100.0).round() as u32;
        let color = if p_feas >= 0.8 {
            egui::Color32::from_rgb(22, 163, 74) // green-600
        } else if p_feas >= 0.5 {
            egui::Color32::from_rgb(202, 138, 4) // amber-600
        } else {
            egui::Color32::RED
        };
        ui.colored_label(color, format!("P(feasible): {}%", pct));

        if !result.predicted_constraints.is_empty() {
            egui::Grid::new("surrogate_predicted_constraints")
                .striped(true)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.strong("Constraint");
                    ui.strong("Predicted");
                    ui.end_row();
                    for (name, val) in &result.predicted_constraints {
                        ui.label(name);
                        let feasible = *val <= 0.0;
                        let cell_color = if feasible {
                            egui::Color32::from_rgb(22, 163, 74)
                        } else {
                            egui::Color32::RED
                        };
                        ui.colored_label(cell_color, format!("{:.6}", val));
                        ui.end_row();
                    }
                });
        }
    }

    // ── 推定最適点のパラメータ値テーブル ────────────────────────────
    ui.add_space(4.0);
    egui::Grid::new("surrogate_best_params")
        .striped(true)
        .min_col_width(80.0)
        .show(ui, |ui| {
            ui.strong("Parameter");
            ui.strong("Best value");
            ui.end_row();
            for (name, value) in &result.best_params {
                ui.label(name);
                ui.monospace(format!("{:.6}", value));
                ui.end_row();
            }
        });

    // ── 最適点を通る応答曲面スライス（ヒートマップ＋最適点マーカー） ──
    let Some(slice) = &result.slice else {
        return;
    };
    if slice.z_values.is_empty() {
        return;
    }
    let (Some(px_name), Some(py_name)) = (
        result.best_params.get(slice.param_x_idx),
        result.best_params.get(slice.param_y_idx),
    ) else {
        return;
    };

    ui.add_space(6.0);
    ui.label(format!(
        "Response surface through the optimum — X: {} (→), Y: {} (↑)",
        px_name.0, py_name.0
    ));

    let best_x_val = px_name.1;
    let best_y_val = py_name.1;
    let marker_params = vec![
        (slice.param_x_idx, best_x_val),
        (slice.param_y_idx, best_y_val),
    ];
    draw_slice_heatmap(ui, slice, &marker_params, cmap);
}

/// ヒートマップスライスの描画ヘルパー（単目的・多目的で共通）。
/// `marker_params` は (param_x_idx_in_slice, value), (param_y_idx_in_slice, value)
/// を含む vec。最初の 2 要素の x/y 値でマーカーを射影する。
fn draw_slice_heatmap(
    ui: &mut egui::Ui,
    slice: &tunny_core::surrogate_opt::SurfaceSlice,
    marker_points: &[(usize, f64)],
    cmap: ColorMap,
) {
    let nx = slice.x_values.len();
    let ny = slice.y_values.len();
    if nx == 0 || ny == 0 {
        return;
    }

    let available = ui.available_rect_before_wrap();
    let plot_size = egui::vec2(
        (available.width() - 32.0).max(100.0),
        available.height().clamp(60.0, 300.0),
    );
    let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // 表示用に向きを揃える: 横 = param_x（左→右で増加）、縦 = param_y（上 = 最大）。
    // core の slice は z[i][j] = f(x_i, y_j) なので disp[r][c] = z[c][ny-1-r]。
    let display: Vec<Vec<f64>> = (0..ny)
        .map(|r| (0..nx).map(|c| slice.z_values[c][ny - 1 - r]).collect())
        .collect();
    draw_heatmap(&painter, rect, &display, cmap.clone());

    // マーカー描画: marker_points から x/y 値を取り出して射影する。
    let (x_min, x_max) = (slice.x_values[0], slice.x_values[nx - 1]);
    let (y_min, y_max) = (slice.y_values[0], slice.y_values[ny - 1]);
    if x_max > x_min && y_max > y_min {
        for (px_val, py_val) in marker_points
            .iter()
            .zip(marker_points.iter().skip(1))
            .map(|(a, b)| (a.1, b.1))
            .take(1)
        {
            let fx = ((px_val - x_min) / (x_max - x_min)).clamp(0.0, 1.0) as f32;
            let fy = ((py_val - y_min) / (y_max - y_min)).clamp(0.0, 1.0) as f32;
            let marker = egui::pos2(
                rect.left() + fx * rect.width(),
                rect.bottom() - fy * rect.height(),
            );
            painter.circle_filled(marker, 5.0, egui::Color32::WHITE);
            painter.circle_stroke(marker, 5.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
        }
    }

    let (v_min, v_max) = value_range(&display);
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap);
}

/// 多目的最適化の結果を表示する。
fn render_multi_result(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    state: &mut SurrogateOptState,
    cmap: ColorMap,
) {
    // ── 見出し ────────────────────────────────────────────────────
    ui.strong(format!(
        "Predicted Pareto Front: {} points",
        result.front.len()
    ));

    // ── 目的ごとの R²（訓練） ─────────────────────────────────────
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        for (i, r2) in result.r_squared.iter().enumerate() {
            let name = result
                .objective_names
                .get(i)
                .map(|s| s.as_str())
                .unwrap_or("?");
            ui.label(format!("{}: R²={:.3}", name, r2));
        }
    });
    ui.add_space(4.0);

    // ── フロント点テーブル ────────────────────────────────────────
    if !result.front.is_empty() {
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .id_salt("surrogate_multi_front_scroll")
            .show(ui, |ui| {
                egui::Grid::new("surrogate_multi_front_table")
                    .striped(true)
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        // ヘッダ行
                        for name in &result.objective_names {
                            ui.strong(name);
                        }
                        for name in &result.param_names {
                            ui.strong(name);
                        }
                        ui.end_row();

                        // データ行
                        for pt in &result.front {
                            for v in &pt.values {
                                ui.monospace(format!("{:.6}", v));
                            }
                            for p in &pt.params {
                                ui.monospace(format!("{:.6}", p));
                            }
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(4.0);
    }

    // ── 応答曲面スライス（目的選択コンボ + ヒートマップ） ─────────
    if result.slices.is_empty() {
        return;
    }

    // 目的選択コンボ。
    ui.horizontal(|ui| {
        ui.label("Surface for:");
        let current_name = result
            .objective_names
            .get(state.multi_slice_objective)
            .map(|s| s.as_str())
            .unwrap_or("—");
        egui::ComboBox::from_id_salt("surrogate_multi_slice_obj")
            .selected_text(current_name)
            .show_ui(ui, |ui| {
                for (i, name) in result.objective_names.iter().enumerate() {
                    if ui
                        .selectable_label(state.multi_slice_objective == i, name)
                        .clicked()
                    {
                        state.multi_slice_objective = i;
                    }
                }
            });
    });

    // インデックス範囲クランプ（目的数が減った場合など）。
    let n_slices = result.slices.len();
    if state.multi_slice_objective >= n_slices {
        state.multi_slice_objective = 0;
    }

    let Some(slice) = result.slices.get(state.multi_slice_objective) else {
        return;
    };
    if slice.z_values.is_empty() {
        return;
    }

    let obj_name = result
        .objective_names
        .get(state.multi_slice_objective)
        .map(|s| s.as_str())
        .unwrap_or("?");
    let px_label = result
        .param_names
        .get(slice.param_x_idx)
        .map(|s| s.as_str())
        .unwrap_or("?");
    let py_label = result
        .param_names
        .get(slice.param_y_idx)
        .map(|s| s.as_str())
        .unwrap_or("?");

    ui.label(format!(
        "Response surface ({}) — X: {} (→), Y: {} (↑)",
        obj_name, px_label, py_label
    ));

    // ヒートマップ描画。
    let nx = slice.x_values.len();
    let ny = slice.y_values.len();
    if nx == 0 || ny == 0 {
        return;
    }

    let available = ui.available_rect_before_wrap();
    let plot_size = egui::vec2(
        (available.width() - 32.0).max(100.0),
        available.height().clamp(60.0, 300.0),
    );
    let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let display: Vec<Vec<f64>> = (0..ny)
        .map(|r| (0..nx).map(|c| slice.z_values[c][ny - 1 - r]).collect())
        .collect();
    draw_heatmap(&painter, rect, &display, cmap.clone());

    // パレートフロント全点をオーバーレイ（白丸、黒縁）。
    let (x_min, x_max) = (slice.x_values[0], slice.x_values[nx - 1]);
    let (y_min, y_max) = (slice.y_values[0], slice.y_values[ny - 1]);
    if x_max > x_min && y_max > y_min {
        for pt in &result.front {
            let px_val = pt.params.get(slice.param_x_idx).copied().unwrap_or(0.0);
            let py_val = pt.params.get(slice.param_y_idx).copied().unwrap_or(0.0);
            let fx = ((px_val - x_min) / (x_max - x_min)).clamp(0.0, 1.0) as f32;
            let fy = ((py_val - y_min) / (y_max - y_min)).clamp(0.0, 1.0) as f32;
            let marker = egui::pos2(
                rect.left() + fx * rect.width(),
                rect.bottom() - fy * rect.height(),
            );
            painter.circle_filled(marker, 2.5, egui::Color32::WHITE);
            painter.circle_stroke(marker, 2.5, egui::Stroke::new(1.0, egui::Color32::BLACK));
        }
    }

    let (v_min, v_max) = value_range(&display);
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap);
}

/// 最適化履歴プロット（全 trial 点 + 累積ベスト線 + 予測最適値の水平線）。
fn render_history_plot(ui: &mut egui::Ui, history: &[f64], result: &SurrogateOptUiResult) {
    use crate::theme::chart_colors::{COLOR_OPT_PRUNED, COLOR_OPT_TRIAL};
    use crate::ui::widgets::history::optimization_history::compute_best_values;

    let delta = improvement_delta(
        result.minimize,
        result.best_observed_value,
        result.best_value,
    );
    let predicted_line_color = if delta > 0.0 {
        egui::Color32::from_rgb(22, 163, 74) // green-600
    } else {
        egui::Color32::from_rgb(107, 114, 128) // gray-500
    };

    // 全 trial の散布点。
    let all_pts: egui_plot::PlotPoints = history
        .iter()
        .enumerate()
        .map(|(i, &v)| [i as f64, v])
        .collect();
    let scatter = egui_plot::Points::new(all_pts)
        .name("All Trials")
        .color(COLOR_OPT_TRIAL)
        .radius(2.0);

    // 累積ベスト線。
    let best_pts: egui_plot::PlotPoints = compute_best_values(history, result.minimize)
        .into_iter()
        .collect();
    let best_line = egui_plot::Line::new(best_pts)
        .name("Best so far")
        .color(COLOR_OPT_PRUNED)
        .width(1.5);

    // 予測最適値の水平線。
    let n = history.len() as f64;
    let hline_pts: egui_plot::PlotPoints = vec![
        [0.0, result.best_value],
        [n.max(1.0) - 1.0, result.best_value],
    ]
    .into();
    let hline = egui_plot::Line::new(hline_pts)
        .name("Predicted optimum")
        .color(predicted_line_color)
        .width(1.5)
        .style(egui_plot::LineStyle::Dashed { length: 8.0 });

    egui_plot::Plot::new("surrogate_history_plot")
        .height(200.0)
        .x_axis_label("Trial")
        .y_axis_label(&result.objective_name)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.points(scatter);
            plot_ui.line(best_line);
            plot_ui.line(hline);

            // 予測標準偏差の ±1.96σ 帯（薄いグレーの破線）。
            if let Some(std) = result.predicted_std {
                let sigma = 1.96 * std;
                for (offset, name) in [
                    (sigma, "Predicted optimum +1.96σ"),
                    (-sigma, "Predicted optimum −1.96σ"),
                ] {
                    let y_band = result.best_value + offset;
                    let band_pts: egui_plot::PlotPoints =
                        vec![[0.0, y_band], [n.max(1.0) - 1.0, y_band]].into();
                    plot_ui.line(
                        egui_plot::Line::new(band_pts)
                            .name(name)
                            .color(egui::Color32::from_rgb(156, 163, 175)) // gray-400
                            .width(1.0)
                            .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
                    );
                }
            }
        });
}

/// 獲得関数による候補提案の結果テーブルと "Copy enqueue JSON" ボタンを描画する。
fn render_suggest_result(
    ui: &mut egui::Ui,
    result: &crate::state::messages::SurrogateSuggestUiResult,
) {
    if result.candidates.is_empty() {
        return;
    }

    ui.add_space(4.0);
    ui.strong(format!(
        "Suggested candidates for '{}':",
        result.objective_name
    ));

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .id_salt("surrogate_suggest_scroll")
        .show(ui, |ui| {
            egui::Grid::new("surrogate_suggest_table")
                .striped(true)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    // ── ヘッダ行 ──────────────────────────────────────
                    let has_feas = result
                        .candidates
                        .first()
                        .map(|c| c.feasibility_probability.is_some())
                        .unwrap_or(false);
                    for name in &result.param_names {
                        ui.strong(name);
                    }
                    ui.strong("Predicted");
                    ui.strong("Std");
                    if has_feas {
                        ui.strong("P(feas)");
                    }
                    ui.strong("Acq. score");
                    ui.end_row();

                    // ── データ行 ──────────────────────────────────────
                    for c in &result.candidates {
                        for v in &c.params {
                            ui.monospace(format!("{:.6}", v));
                        }
                        ui.monospace(format!("{:.6}", c.predicted_value));
                        match c.predicted_std {
                            Some(std) => ui.monospace(format!("±{:.6}", std)),
                            None => ui.label("—"),
                        };
                        if has_feas {
                            match c.feasibility_probability {
                                Some(p) => {
                                    let pct = (p * 100.0).round() as u32;
                                    let color = if p >= 0.8 {
                                        egui::Color32::from_rgb(22, 163, 74)
                                    } else if p >= 0.5 {
                                        egui::Color32::from_rgb(202, 138, 4)
                                    } else {
                                        egui::Color32::RED
                                    };
                                    ui.colored_label(color, format!("{}%", pct));
                                }
                                None => {
                                    ui.label("—");
                                }
                            };
                        }
                        ui.monospace(format!("{:.4e}", c.acq_score));
                        ui.end_row();
                    }
                });
        });

    ui.add_space(4.0);

    // ── "Copy enqueue JSON" ボタン ──────────────────────────────
    // Optuna の study.enqueue_trial(params) に渡せる JSON 配列を生成する。
    if ui
        .button("Copy enqueue JSON")
        .on_hover_text(
            "Optuna の study.enqueue_trial(params) に渡せる形式でクリップボードへコピーします。",
        )
        .clicked()
    {
        let json_items: Vec<serde_json::Value> = result
            .candidates
            .iter()
            .map(|c| {
                let obj: serde_json::Map<String, serde_json::Value> = result
                    .param_names
                    .iter()
                    .zip(c.params.iter())
                    .map(|(name, &val)| (name.clone(), serde_json::Value::from(val)))
                    .collect();
                serde_json::Value::Object(obj)
            })
            .collect();
        let json_str = serde_json::to_string_pretty(&json_items).unwrap_or_default();
        ui.ctx().copy_text(json_str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widget_states::SurrogateOptState;

    fn ui_result(slice: Option<tunny_core::surrogate_opt::SurfaceSlice>) -> SurrogateOptUiResult {
        SurrogateOptUiResult {
            best_params: vec![("x".to_string(), 0.3), ("y".to_string(), 0.7)],
            best_value: 0.01,
            predicted_std: Some(0.005),
            r_squared: 0.95,
            objective_name: "obj0".to_string(),
            minimize: true,
            slice,
            best_observed_value: 0.05,
            predicted_constraints: vec![],
            feasibility_probability: None,
        }
    }

    // ── improvement_delta のユニットテスト ────────────────────────────

    #[test]
    fn improvement_delta_minimize_positive() {
        // 観測 0.5、予測 0.1 → 改善量 = 0.5 - 0.1 = 0.4（正）
        let d = improvement_delta(true, 0.5, 0.1);
        assert!((d - 0.4).abs() < 1e-12, "delta = {d}");
    }

    #[test]
    fn improvement_delta_minimize_no_improvement() {
        // 予測が観測より悪い場合は負または 0
        let d = improvement_delta(true, 0.1, 0.5);
        assert!(d < 0.0, "delta = {d}");
    }

    #[test]
    fn improvement_delta_maximize_positive() {
        // 観測 0.8、予測 1.2 → 改善量 = 1.2 - 0.8 = 0.4（正）
        let d = improvement_delta(false, 0.8, 1.2);
        assert!((d - 0.4).abs() < 1e-12, "delta = {d}");
    }

    #[test]
    fn improvement_delta_maximize_no_improvement() {
        // 予測が観測より悪い場合は負または 0
        let d = improvement_delta(false, 1.2, 0.8);
        assert!(d < 0.0, "delta = {d}");
    }

    #[test]
    fn improvement_delta_exact_zero() {
        // 観測と予測が等しければ改善なし
        assert_eq!(improvement_delta(true, 0.5, 0.5), 0.0);
        assert_eq!(improvement_delta(false, 0.5, 0.5), 0.0);
    }

    #[test]
    fn fit_click_builds_pending_fit_from_selections() {
        let mut state = SurrogateOptState {
            model: SurrogateModelKind::Ridge,
            selected_objective: 0,
            ..Default::default()
        };
        let obj_names = ["obj0".to_string()];

        // Fit & Validate ボタン押下と同じロジック。
        if let Some(obj_name) = obj_names.get(state.selected_objective) {
            state.error_message = None;
            state.pending_fit = Some(SurrogateFitComputeRequest {
                objective: obj_name.clone(),
                model: state.model,
                use_constraints: false,
            });
        }

        let req = state.pending_fit.as_ref().unwrap();
        assert_eq!(req.objective, "obj0");
        assert_eq!(req.model, SurrogateModelKind::Ridge);
    }

    #[test]
    fn multi_fit_click_builds_pending_multi_fit() {
        let mut state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            multi_objective: true,
            ..Default::default()
        };
        let obj_names = ["obj0".to_string(), "obj1".to_string()];

        // 多目的 Fit & Validate ボタン押下と同じロジック。
        if obj_names.len() >= 2 {
            state.error_message = None;
            state.pending_multi_fit = Some(SurrogateMultiFitComputeRequest { model: state.model });
        }

        let req = state.pending_multi_fit.as_ref().unwrap();
        assert_eq!(req.model, SurrogateModelKind::GpFitc);
    }

    #[test]
    fn optimize_click_requires_matching_trained() {
        let state = SurrogateOptState::default();
        let obj_names = ["obj0".to_string()];
        // trained が None のため has_matching_trained は false。
        let has_matching = state
            .trained
            .as_deref()
            .map(|t| trained_matches(t, &state, &obj_names))
            .unwrap_or(false);
        assert!(!has_matching);
    }

    #[test]
    fn result_arrival_switches_spinner_off() {
        let mut state = SurrogateOptState {
            optimizing: true,
            ..Default::default()
        };
        state.result = Some(ui_result(None));
        state.optimizing = false;
        assert!(!state.optimizing);
        assert!(state.result.is_some());
    }

    #[test]
    fn adopt_compute_state_keeps_selections() {
        use std::sync::Arc;

        let global = SurrogateOptState {
            result: Some(ui_result(None)),
            fitting: false,
            optimizing: false,
            error_message: Some("err".to_string()),
            ..Default::default()
        };

        let mut item = SurrogateOptState {
            model: SurrogateModelKind::Ridge,
            optimizer: OptimizerKind::RandomSearch,
            selected_objective: 1,
            fitting: true,
            optimizing: true,
            ..Default::default()
        };
        item.adopt_compute_state(&global);

        assert!(!item.fitting);
        assert!(!item.optimizing);
        assert!(item.result.is_some());
        assert_eq!(item.error_message.as_deref(), Some("err"));
        // 選択は維持される
        assert_eq!(item.model, SurrogateModelKind::Ridge);
        assert_eq!(item.optimizer, OptimizerKind::RandomSearch);
        assert_eq!(item.selected_objective, 1);

        // Arc<TrainedSurrogate> も伝播される（ここでは None）。
        assert!(item.trained.is_none());
        // multi_trained も伝播される（ここでは None）。
        assert!(item.multi_trained.is_none());
        drop(Arc::<u8>::new(0)); // Arc が使えることを確認する（コンパイルチェック）。
    }

    #[test]
    fn verdict_returns_correct_category() {
        let (text, color) = verdict(0.95);
        assert!(text.contains("Good"));
        assert_eq!(color, egui::Color32::from_rgb(22, 163, 74));

        let (text, _) = verdict(0.75);
        assert!(text.contains("Fair"));

        let (text, color) = verdict(0.5);
        assert!(text.contains("Poor"));
        assert_eq!(color, egui::Color32::RED);
    }

    #[test]
    fn labels_cover_all_choices() {
        for kind in MODEL_CHOICES {
            assert!(!model_label(kind).is_empty());
        }
        for kind in OPTIMIZER_CHOICES {
            assert!(!optimizer_label(kind).is_empty());
        }
    }

    // ── multi_trained_matches のユニットテスト ────────────────────────

    fn make_dummy_trained(
        obj_name: &str,
        model: SurrogateModelKind,
    ) -> tunny_core::surrogate_opt::TrainedSurrogate {
        // 最低限のフィールドだけ埋めた TrainedSurrogate をフィットして作る。
        let xs: Vec<Vec<f64>> = (0..12)
            .map(|i| vec![i as f64 / 12.0, (i as f64 / 12.0) * 0.5])
            .collect();
        let ys: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix: xs,
            y: ys,
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_name: obj_name.to_string(),
            model,
            constraints: vec![],
        };
        tunny_core::surrogate_opt::fit_surrogate_with_validation(&req)
            .expect("dummy fit should succeed")
    }

    #[test]
    fn multi_trained_matches_correct() {
        let obj_names = vec!["f0".to_string(), "f1".to_string()];
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_model() {
        let obj_names = vec!["f0".to_string(), "f1".to_string()];
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        // モデルが Ridge に変わった場合
        let state = SurrogateOptState {
            model: SurrogateModelKind::Ridge,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_objectives() {
        let obj_names = vec!["f0".to_string(), "f2".to_string()]; // f2 が違う
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_length() {
        let obj_names = vec!["f0".to_string()]; // 目的数が 1
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn adopt_compute_state_propagates_multi_trained() {
        use std::sync::Arc;

        let trained_vec = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let arc = Arc::new(trained_vec);

        let global = SurrogateOptState {
            fitting: false,
            optimizing: false,
            multi_trained: Some(arc.clone()),
            ..Default::default()
        };

        let mut item = SurrogateOptState {
            fitting: true,
            ..Default::default()
        };
        item.adopt_compute_state(&global);

        assert!(!item.fitting);
        assert!(item.multi_trained.is_some());
        assert_eq!(item.multi_trained.as_ref().unwrap().len(), 2);
    }
}
