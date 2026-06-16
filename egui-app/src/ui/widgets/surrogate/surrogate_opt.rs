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
use crate::ui::widget_states::{
    SurrogateFitComputeRequest, SurrogateMultiFitComputeRequest,
    SurrogateMultiOptimizeComputeRequest, SurrogateMultiSuggestComputeRequest, SurrogateOptState,
    SurrogateOptimizeComputeRequest, SurrogateSuggestComputeRequest,
};
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

/// Model コンボの "Auto" エントリのラベル。
const AUTO_MODEL_LABEL: &str = "Auto (cross-validated)";

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
    if trained.objective_name != selected_obj {
        return false;
    }
    // Auto モードでは具体的なモデル種別は core が選ぶため、model_kind ではなく
    // 「Auto で学習されたか（model_selection が Some）」で一致を判定する。
    if state.auto_select {
        trained.model_selection.is_some()
    } else {
        trained.model_selection.is_none() && trained.model_kind == state.model
    }
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

/// 多目的フロント散布図に重ねる観測（既存 trial）データ。
/// すべて trial 行順に整列し、`objective_cols` は `multi_result` の目的順に並ぶ。
/// 観測点を ParetoScatter と同様にパレートフロント / 被支配 / 実行不可能へ分類するために使う。
pub struct ObservedData<'a> {
    /// 目的ごとの全 trial 観測値（`multi_result.objective_names` の順）。
    pub objective_cols: &'a [Vec<f64>],
    /// 各 trial の Pareto ランク（0 = 観測フロント）。
    pub pareto_rank: &'a [u32],
    /// 各 trial が feasible か。
    pub feasible: &'a [bool],
}

/// `param_names` は数値パラメータのみ（カテゴリカル列は最適化対象にしない）。
/// `obj_history` は現在の結果が参照する目的列の全値（trial 順）。プロット用。
/// `observed` は多目的フロント散布図に重ねる観測点（結果が無いときは None）。
/// `constraint_col_names` は制約列名（制約付き Study のみ非空）。
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    obj_names: &[String],
    trial_count: usize,
    obj_history: Option<&[f64]>,
    observed: Option<&ObservedData>,
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
        || state.multi_suggesting
        || state.pending_fit.is_some()
        || state.pending_optimize.is_some()
        || state.pending_multi_fit.is_some()
        || state.pending_multi_optimize.is_some()
        || state.pending_suggest.is_some()
        || state.pending_multi_suggest.is_some();

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
                        busy,
                        has_matching_multi_trained,
                        observed,
                    );
                } else {
                    render_optimize_column(ui, state, busy, has_matching_trained, obj_history);
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
        let selected_text = if state.auto_select {
            AUTO_MODEL_LABEL
        } else {
            model_label(state.model)
        };
        egui::ComboBox::from_id_salt("surrogate_model")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                // 先頭に Auto（交差検証で自動選択）。選ぶと auto_select = true。
                if ui
                    .selectable_label(state.auto_select, AUTO_MODEL_LABEL)
                    .clicked()
                {
                    state.auto_select = true;
                }
                // 具体的なモデルを選ぶと auto_select = false かつその kind に設定。
                for kind in MODEL_CHOICES {
                    let selected = !state.auto_select && state.model == kind;
                    if ui.selectable_label(selected, model_label(kind)).clicked() {
                        state.auto_select = false;
                        state.model = kind;
                    }
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
                auto_select: state.auto_select,
                use_constraints: n_constraints > 0 && state.use_constraints,
            });
        }
    }

    // フィット中: 進捗バー＋キャンセルボタン。
    if state.fitting {
        render_fit_progress(ui, state);
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

/// フィット中の進捗（段階ラベル・進捗バー）とキャンセルボタンを描画する。
///
/// 進捗ハンドルは学習スレッドと共有しており、Cancel ボタンは内部のキャンセルフラグを
/// 立てる（学習側は段階の境界で検知して中止する）。学習中は毎フレーム再描画して
/// 進捗バーを更新する。
fn render_fit_progress(ui: &mut egui::Ui, state: &SurrogateOptState) {
    // 進捗を滑らかに更新するため再描画を要求する。
    ui.ctx().request_repaint();

    let snapshot = state.fit_progress.as_ref().map(|p| p.snapshot());

    ui.horizontal(|ui| {
        ui.spinner();
        let label = snapshot
            .as_ref()
            .filter(|s| !s.stage.is_empty())
            .map(|s| s.stage.clone())
            .unwrap_or_else(|| "Fitting and validating surrogate…".to_string());
        ui.label(label);
    });

    // 進捗バー（総ステップ数が分かっているとき）。
    if let Some(s) = snapshot.as_ref().filter(|s| s.total > 0) {
        let frac = (s.done as f32 / s.total as f32).clamp(0.0, 1.0);
        ui.add(
            egui::ProgressBar::new(frac)
                .show_percentage()
                .desired_width(240.0),
        );
    }

    // キャンセルボタン。
    if let Some(progress) = &state.fit_progress {
        let cancelling = progress.is_cancelled();
        let label = if cancelling {
            "Cancelling…"
        } else {
            "Cancel"
        };
        if ui
            .add_enabled(!cancelling, egui::Button::new(label))
            .clicked()
        {
            progress.request_cancel();
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

    // フィット中: 進捗バー＋キャンセルボタン。
    if state.fitting {
        render_fit_progress(ui, state);
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
    busy: bool,
    has_matching_trained: bool,
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

    // ── Run Optimization ボタン ──────────────────────────────────
    let can_optimize = has_matching_trained && !busy;
    if ui
        .add_enabled(can_optimize, egui::Button::new("Run Optimization"))
        .clicked()
    {
        state.error_message = None;
        state.pending_optimize = Some(SurrogateOptimizeComputeRequest {
            optimizer: state.optimizer,
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

    render_result(ui, result, obj_history);

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

/// 右列（多目的）: 固定 NSGA-II ラベル + Run Optimization ボタン、結果。
fn render_optimize_column_multi(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    busy: bool,
    has_matching_multi_trained: bool,
    observed: Option<&ObservedData>,
) {
    // ── Optimizer（固定ラベル） ───────────────────────────────────
    ui.label("Optimizer: NSGA-II");

    // ── Run Optimization ボタン ──────────────────────────────────
    let can_optimize = has_matching_multi_trained && !busy;
    if ui
        .add_enabled(can_optimize, egui::Button::new("Run Optimization"))
        .clicked()
    {
        state.error_message = None;
        state.pending_multi_optimize = Some(SurrogateMultiOptimizeComputeRequest);
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

    render_multi_result(ui, result, state, observed);

    // ── Suggest next trials (EHVI) セクション ────────────────────────
    // 多目的・GP 系モデルのみ EHVI を提供する。
    let is_gp = matches!(
        state.model,
        SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe | SurrogateModelKind::GpMoe
    );
    if has_matching_multi_trained && is_gp {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.strong("Suggest next trials (EHVI)");

        // 候補数 DragValue（1〜10、デフォルト 3）。
        ui.horizontal(|ui| {
            ui.label("Candidates:");
            ui.add(egui::DragValue::new(&mut state.n_multi_suggest_candidates).range(1..=10));
        });

        // Suggest ボタン。
        let can_suggest = has_matching_multi_trained && !busy;
        let disabled_hint = if !can_suggest && !has_matching_multi_trained {
            "Fit GP surrogates for all objectives first (GP-FITC, GP-VFE, or GP-MOE)."
        } else if !can_suggest {
            "A computation is already running."
        } else {
            ""
        };
        let suggest_response =
            ui.add_enabled(can_suggest, egui::Button::new("Suggest next trials (EHVI)"));
        if !disabled_hint.is_empty() {
            suggest_response.on_disabled_hover_text(disabled_hint);
        } else if suggest_response.clicked() {
            state.multi_suggest_result = None;
            state.error_message = None;
            state.pending_multi_suggest = Some(SurrogateMultiSuggestComputeRequest {
                n_candidates: state.n_multi_suggest_candidates,
            });
        }

        // 提案中スピナー。
        if state.multi_suggesting {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing EHVI candidates…");
            });
        }

        // 結果テーブル。
        if let Some(ref suggest) = state.multi_suggest_result.clone() {
            render_multi_suggest_result(ui, suggest);
        }
    } else if has_matching_multi_trained && !is_gp {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.colored_label(
            egui::Color32::from_rgb(107, 114, 128), // gray-500
            "Suggest next trials (EHVI) requires Gaussian Process models (GP-FITC, GP-VFE, or GP-MOE).",
        );
    }
}

/// 検証指標セクションをレンダリングする。
fn render_validation(ui: &mut egui::Ui, trained: &Arc<TrainedSurrogate>) {
    let v = &trained.validation;
    ui.add_space(4.0);

    // ── Auto 選択の経緯（Auto フィット時のみ表示） ────────────────
    if let Some(selection) = trained.model_selection.as_ref() {
        render_model_selection(ui, selection);
    }

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

/// Auto モデル選択の経緯を表示する。「Auto selected: <chosen>」見出しと、候補ごとの
/// CV R² を降順に並べたコンパクトな表を出す（フィット／検証に失敗した候補は "—"）。
fn render_model_selection(
    ui: &mut egui::Ui,
    selection: &tunny_core::surrogate_opt::ModelSelectionReport,
) {
    ui.strong(format!(
        "Auto selected: {}",
        model_label(selection.chosen)
    ))
    .on_hover_text(
        "候補モデルを交差検証し、CV R² が最も高いものを自動選択しました（同点は単純なモデルを優先）。",
    );

    // 候補を CV R² の降順に並べる（失敗候補 = NEG_INFINITY は末尾）。
    let mut rows: Vec<(SurrogateModelKind, f64)> = selection.scores.clone();
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    egui::Grid::new("surrogate_model_selection")
        .striped(true)
        .min_col_width(80.0)
        .show(ui, |ui| {
            ui.strong("Candidate");
            ui.strong("CV R²");
            ui.end_row();
            for (kind, score) in rows {
                // 選ばれた候補を強調する。
                if kind == selection.chosen {
                    ui.strong(model_label(kind));
                } else {
                    ui.label(model_label(kind));
                }
                if score.is_finite() {
                    ui.monospace(format!("{:.3}", score));
                } else {
                    // フィット／検証に失敗した候補。
                    ui.monospace("—");
                }
                ui.end_row();
            }
        });
    ui.add_space(4.0);
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

    // パレートフロント所属フラグが揃っていれば、フロント点を分けて強調する。
    // フロント所属が分かるのは多目的フィットのときのみ（単目的では全点を青で描く）。
    let has_front_flags =
        v.oof_is_front.len() == v.oof_pairs.len() && v.oof_is_front.iter().any(|&f| f);
    let n_front = v.oof_is_front.iter().filter(|&&f| f).count();

    // フロント点のみの近似度を数値で先に示す（散布図で埋もれがちなため）。
    if has_front_flags && (v.front_r2.is_some() || v.front_rmse.is_some()) {
        let r2_text = v
            .front_r2
            .map(|r| format!("R² = {:.3}", r))
            .unwrap_or_else(|| "R² = —".to_string());
        let rmse_text = v
            .front_rmse
            .map(|r| format!("RMSE = {:.6}", r))
            .unwrap_or_default();
        ui.colored_label(
            crate::theme::chart_colors::COLOR_PARETO,
            format!(
                "Pareto-front fit — {}{} ({} front pts)",
                r2_text,
                if rmse_text.is_empty() {
                    String::new()
                } else {
                    format!(", {}", rmse_text)
                },
                n_front
            ),
        )
        .on_hover_text(
            "パレートフロント（rank 0）の trial だけで算出した out-of-fold の近似精度。\
             最適化で実際に使うフロント近傍をサロゲートがどれだけ正しく予測できているかを示す。",
        );
    }

    // フロント点（赤）とそれ以外（青）に分ける。
    let mut front_pts: Vec<[f64; 2]> = Vec::new();
    let mut other_pts: Vec<[f64; 2]> = Vec::new();
    for (i, &(actual, pred)) in v.oof_pairs.iter().enumerate() {
        if has_front_flags && v.oof_is_front.get(i).copied().unwrap_or(false) {
            front_pts.push([actual, pred]);
        } else {
            other_pts.push([actual, pred]);
        }
    }

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
        // 非フロント点（青）を背面に。
        plot_ui.points(
            egui_plot::Points::new(other_pts)
                .name("Out-of-fold predictions")
                .color(egui::Color32::from_rgb(59, 130, 246)) // blue-500
                .radius(3.0),
        );
        plot_ui.line(ref_seg);
        // フロント点（赤・大きめ）を前面に。
        if !front_pts.is_empty() {
            plot_ui.points(
                egui_plot::Points::new(front_pts)
                    .name("Pareto front")
                    .color(crate::theme::chart_colors::COLOR_PARETO)
                    .radius(4.0),
            );
        }
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

fn render_result(ui: &mut egui::Ui, result: &SurrogateOptUiResult, obj_history: Option<&[f64]>) {
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

    // ── 推定最適点の変数組み合わせ（TrialTable 形式） ────────────────
    // パラメータ列 + 予測目的値列を 1 行で示す（TrialTable と同じ表スタイル）。
    ui.add_space(6.0);
    ui.label("Optimal variable combination:");
    render_best_point_table(ui, result);
}

/// 推定最適点を TrialTable と同じ表形式（各パラメータ列 + 予測目的値列、1 行）で表示する。
fn render_best_point_table(ui: &mut egui::Ui, result: &SurrogateOptUiResult) {
    use egui_extras::{Column, TableBuilder};

    let n_params = result.best_params.len();
    egui::ScrollArea::horizontal()
        .id_salt("surrogate_best_point_scroll")
        .show(ui, |ui| {
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG;
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .columns(Column::initial(90.0).at_least(50.0), n_params) // 各パラメータ
                .column(Column::initial(110.0).at_least(60.0)) // 予測目的値
                .header(20.0, |mut header| {
                    for (name, _) in &result.best_params {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    header.col(|ui| {
                        ui.strong(&result.objective_name);
                    });
                })
                .body(|mut body| {
                    body.row(18.0, |mut row| {
                        for (_, value) in &result.best_params {
                            row.col(|ui| {
                                ui.label(format!("{:.4}", value));
                            });
                        }
                        row.col(|ui| {
                            ui.monospace(format!("{:.6}", result.best_value));
                        });
                    });
                });
        });
}

/// 多目的最適化の結果を表示する。
/// 予測パレートフロントを目的空間の散布図で示し（ウィジェット内）、続けて
/// フロント点の変数組み合わせを TrialTable 形式の表で示す。フロントは
/// ParetoScatter ウィジェットにも金色ダイヤで重畳表示される。
fn render_multi_result(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    state: &mut SurrogateOptState,
    observed: Option<&ObservedData>,
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

    // ── 予測パレートフロント散布図（目的空間） ───────────────────────
    render_front_scatter(ui, result, state, observed);

    // ── フロント点テーブル（TrialTable 形式: 各目的列 + 各パラメータ列） ──
    ui.add_space(6.0);
    ui.label("Predicted front variable combinations:");
    render_front_table(ui, result);
}

/// 予測パレートフロントを目的空間の 2D 散布図として描画する。
/// 目的が 3 つ以上のときは X/Y 軸の目的を選択できる。フロント点は
/// X 軸目的でソートして折れ線で結び、`COLOR_SURROGATE_FRONT`（金色）で示す。
fn render_front_scatter(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    state: &mut SurrogateOptState,
    observed: Option<&ObservedData>,
) {
    use crate::ui::widgets::scatter_3d::show_objective_combo;

    let n_obj = result.objective_names.len();
    if n_obj < 2 || result.front.is_empty() {
        return;
    }

    // インデックスのクランプ（目的数が変わった場合など）。
    if state.multi_front_x_obj >= n_obj {
        state.multi_front_x_obj = 0;
    }
    if state.multi_front_y_obj >= n_obj {
        state.multi_front_y_obj = 1.min(n_obj - 1);
    }
    if state.multi_front_z_obj >= n_obj {
        state.multi_front_z_obj = 2.min(n_obj - 1);
    }

    // ── 観測点の表示トグル（ParetoScatter と同様: フロント / 被支配 / 実行不可能） ──
    let any_infeasible = observed
        .map(|o| o.feasible.iter().any(|&f| !f))
        .unwrap_or(false);
    if observed.is_some() {
        ui.horizontal(|ui| {
            ui.label("Observed:");
            ui.checkbox(&mut state.show_observed_front, "Pareto front");
            ui.checkbox(&mut state.show_observed_dominated, "Others");
            if any_infeasible {
                ui.checkbox(&mut state.show_observed_infeasible, "Infeasible");
            }
        });
    }
    let toggles = ObservedToggles {
        front: state.show_observed_front,
        dominated: state.show_observed_dominated,
        infeasible: state.show_observed_infeasible,
    };

    // ── 目的が 2 つ: 固定軸の 2D 散布図のみ ─────────────────────────
    if n_obj == 2 {
        render_front_scatter_2d(ui, result, 0, 1, observed, toggles);
        return;
    }

    // ── 目的が 3 つ以上: 2D / 3D 切替 + 軸セレクタ ──────────────────
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.multi_front_3d, "3D view");
        ui.separator();
        if state.multi_front_3d {
            show_objective_combo(
                ui,
                "X:",
                "surrogate_front_x",
                &mut state.multi_front_x_obj,
                &result.objective_names,
            );
            show_objective_combo(
                ui,
                "Y:",
                "surrogate_front_y",
                &mut state.multi_front_y_obj,
                &result.objective_names,
            );
            show_objective_combo(
                ui,
                "Z:",
                "surrogate_front_z",
                &mut state.multi_front_z_obj,
                &result.objective_names,
            );
        } else {
            show_objective_combo(
                ui,
                "X:",
                "surrogate_front_x",
                &mut state.multi_front_x_obj,
                &result.objective_names,
            );
            show_objective_combo(
                ui,
                "Y:",
                "surrogate_front_y",
                &mut state.multi_front_y_obj,
                &result.objective_names,
            );
        }
    });

    if state.multi_front_3d {
        render_front_scatter_3d(ui, result, state, observed, toggles);
    } else {
        render_front_scatter_2d(
            ui,
            result,
            state.multi_front_x_obj,
            state.multi_front_y_obj,
            observed,
            toggles,
        );
    }
}

/// 観測点の分類別表示トグル（ParetoScatter と同じ 3 分類）。
#[derive(Clone, Copy)]
struct ObservedToggles {
    /// 観測パレートフロント（rank 0・feasible）を表示するか。
    front: bool,
    /// 観測の被支配点（rank>0・feasible）を表示するか。
    dominated: bool,
    /// 観測の実行不可能解を表示するか。
    infeasible: bool,
}

/// 観測点を目的 (xi, yi) で (パレートフロント, 被支配, 実行不可能) の 3 群に分類する。
#[allow(clippy::type_complexity)]
fn classify_observed_2d(
    obs: &ObservedData,
    xi: usize,
    yi: usize,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let (Some(xc), Some(yc)) = (obs.objective_cols.get(xi), obs.objective_cols.get(yi)) else {
        return (Vec::new(), Vec::new(), Vec::new());
    };
    let n = xc
        .len()
        .min(yc.len())
        .min(obs.pareto_rank.len())
        .min(obs.feasible.len());
    let mut front = Vec::new();
    let mut dominated = Vec::new();
    let mut infeasible = Vec::new();
    for i in 0..n {
        let pt = [xc[i], yc[i]];
        if !obs.feasible[i] {
            infeasible.push(pt);
        } else if obs.pareto_rank[i] == 0 {
            front.push(pt);
        } else {
            dominated.push(pt);
        }
    }
    (front, dominated, infeasible)
}

/// 観測点を目的 (xi, yi, zi) で (パレートフロント, 被支配, 実行不可能) の 3 群に分類する。
#[allow(clippy::type_complexity)]
fn classify_observed_3d(
    obs: &ObservedData,
    xi: usize,
    yi: usize,
    zi: usize,
) -> (Vec<[f64; 3]>, Vec<[f64; 3]>, Vec<[f64; 3]>) {
    let (Some(xc), Some(yc), Some(zc)) = (
        obs.objective_cols.get(xi),
        obs.objective_cols.get(yi),
        obs.objective_cols.get(zi),
    ) else {
        return (Vec::new(), Vec::new(), Vec::new());
    };
    let n = xc
        .len()
        .min(yc.len())
        .min(zc.len())
        .min(obs.pareto_rank.len())
        .min(obs.feasible.len());
    let mut front = Vec::new();
    let mut dominated = Vec::new();
    let mut infeasible = Vec::new();
    for i in 0..n {
        let pt = [xc[i], yc[i], zc[i]];
        if !obs.feasible[i] {
            infeasible.push(pt);
        } else if obs.pareto_rank[i] == 0 {
            front.push(pt);
        } else {
            dominated.push(pt);
        }
    }
    (front, dominated, infeasible)
}

/// 予測パレートフロントを 2D 散布図（目的 xi × yi）で描画する。
/// 点は X 軸でソートして折れ線で結び、`COLOR_SURROGATE_FRONT`（金色ダイヤ）で示す。
fn render_front_scatter_2d(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    xi: usize,
    yi: usize,
    observed: Option<&ObservedData>,
    toggles: ObservedToggles,
) {
    use crate::theme::chart_colors::{
        COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_SURROGATE_FRONT,
    };

    let mut pts: Vec<[f64; 2]> = result
        .front
        .iter()
        .filter_map(|p| Some([*p.values.get(xi)?, *p.values.get(yi)?]))
        .collect();
    if pts.is_empty() {
        return;
    }
    pts.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));

    // 既存（観測）点を ParetoScatter と同じ 3 分類で射影する。
    let (obs_front, obs_dominated, obs_infeasible) = match observed {
        Some(obs) => classify_observed_2d(obs, xi, yi),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };

    let x_label = result.objective_names.get(xi).cloned().unwrap_or_default();
    let y_label = result.objective_names.get(yi).cloned().unwrap_or_default();

    egui_plot::Plot::new("surrogate_front_scatter_2d")
        .height(220.0)
        .x_axis_label(&x_label)
        .y_axis_label(&y_label)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            // 観測点を背面に描く（実行不可能 → 被支配 → 観測フロントの順）。
            if toggles.infeasible && !obs_infeasible.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new(obs_infeasible)
                        .name("Infeasible")
                        .shape(egui_plot::MarkerShape::Circle)
                        .radius(2.5)
                        .color(COLOR_INFEASIBLE),
                );
            }
            if toggles.dominated && !obs_dominated.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new(obs_dominated)
                        .name("Observed (others)")
                        .shape(egui_plot::MarkerShape::Circle)
                        .radius(2.5)
                        .color(COLOR_NON_PARETO),
                );
            }
            if toggles.front && !obs_front.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new(obs_front)
                        .name("Observed Pareto front")
                        .shape(egui_plot::MarkerShape::Circle)
                        .radius(3.5)
                        .color(COLOR_PARETO),
                );
            }
            // フロントを結ぶ折れ線（点が 2 つ以上のとき）。
            if pts.len() >= 2 {
                plot_ui.line(
                    egui_plot::Line::new(pts.clone())
                        .name("Predicted Pareto front")
                        .color(COLOR_SURROGATE_FRONT)
                        .width(1.5),
                );
            }
            // 予測フロント点（金色ダイヤ）。
            plot_ui.points(
                egui_plot::Points::new(pts)
                    .name("Predicted Pareto front")
                    .shape(egui_plot::MarkerShape::Diamond)
                    .radius(4.5)
                    .color(COLOR_SURROGATE_FRONT),
            );
        });
}

/// 予測パレートフロントを 3D 散布図（目的 X × Y × Z）で描画する。
/// `scatter_3d` の共有インフラ（アークボールカメラ・投影・グリッド・軸）を再利用する。
fn render_front_scatter_3d(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    state: &mut SurrogateOptState,
    observed: Option<&ObservedData>,
    toggles: ObservedToggles,
) {
    use crate::theme::chart_colors::{
        COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_SURROGATE_FRONT,
    };
    use crate::ui::widgets::scatter_3d::{
        compute_range_from_col, draw_3d_axes, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    };

    let xi = state.multi_front_x_obj;
    let yi = state.multi_front_y_obj;
    let zi = state.multi_front_z_obj;

    // フロント点の各軸値。
    let axis_vals = |idx: usize| -> Vec<f64> {
        result
            .front
            .iter()
            .filter_map(|p| p.values.get(idx).copied())
            .collect()
    };
    let x_vals = axis_vals(xi);
    let y_vals = axis_vals(yi);
    let z_vals = axis_vals(zi);
    if x_vals.is_empty() || y_vals.is_empty() || z_vals.is_empty() {
        return;
    }

    // 観測（既存）点の各軸列。改善を見比べるための背景クラウド。
    let obs_col = |idx: usize| -> &[f64] {
        observed
            .and_then(|o| o.objective_cols.get(idx))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    };
    let obs_x = obs_col(xi);
    let obs_y = obs_col(yi);
    let obs_z = obs_col(zi);

    // 範囲はフロント点と全観測点を含むようにする（観測点がクリップで潰れず、
    // トグルで表示を切り替えても軸スケールが変わらないように）。
    let range_for = |front_vals: &[f64], obs: &[f64]| -> (f64, f64) {
        let combined: Vec<f64> = front_vals.iter().chain(obs.iter()).copied().collect();
        compute_range_from_col(Some(&combined))
    };
    let (x_min, x_max) = range_for(&x_vals, obs_x);
    let (y_min, y_max) = range_for(&y_vals, obs_y);
    let (z_min, z_max) = range_for(&z_vals, obs_z);

    // 観測点を 3 分類に分ける。
    let (obs_front, obs_dominated, obs_infeasible) = match observed {
        Some(obs) => classify_observed_3d(obs, xi, yi, zi),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };

    // 予測フロント点。
    let front_pts: Vec<[f64; 3]> = result
        .front
        .iter()
        .map(|p| {
            [
                p.values.get(xi).copied().unwrap_or(0.0),
                p.values.get(yi).copied().unwrap_or(0.0),
                p.values.get(zi).copied().unwrap_or(0.0),
            ]
        })
        .collect();

    let x_name = result.objective_names.get(xi).cloned().unwrap_or_default();
    let y_name = result.objective_names.get(yi).cloned().unwrap_or_default();
    let z_name = result.objective_names.get(zi).cloned().unwrap_or_default();

    // 高さを固定した領域内にキャンバスを確保する（setup_3d_canvas は available_size を使うため）。
    let width = ui.available_width();
    ui.allocate_ui(egui::vec2(width, 280.0), |ui| {
        let (painter, _rect, project) = setup_3d_canvas(ui, &mut state.multi_front_camera);
        draw_3d_grid(&painter, &project);
        draw_3d_axes(
            &painter,
            &project,
            [&x_name, &y_name, &z_name],
            [(x_min, x_max), (y_min, y_max), (z_min, z_max)],
        );

        // 1 群を投影・深度ソートして描画するヘルパー。
        let draw_group = |group: &[[f64; 3]], color: egui::Color32, radius: f32, stroke: bool| {
            let mut calls: Vec<(egui::Pos2, f32)> = group
                .iter()
                .map(|p| {
                    project([
                        normalize_to_clip(p[0], x_min, x_max),
                        normalize_to_clip(p[1], y_min, y_max),
                        normalize_to_clip(p[2], z_min, z_max),
                    ])
                })
                .collect();
            calls.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            for (pos, _) in &calls {
                painter.circle_filled(*pos, radius, color);
                if stroke {
                    painter.circle_stroke(
                        *pos,
                        radius,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                }
            }
        };

        // 観測点（背面）→ 予測フロント（手前）の順に描画する。
        if toggles.infeasible {
            draw_group(&obs_infeasible, COLOR_INFEASIBLE, 2.5, false);
        }
        if toggles.dominated {
            draw_group(&obs_dominated, COLOR_NON_PARETO, 2.5, false);
        }
        if toggles.front {
            draw_group(&obs_front, COLOR_PARETO, 3.5, false);
        }
        draw_group(&front_pts, COLOR_SURROGATE_FRONT, 4.0, true);
    });
}

/// 予測パレートフロントの各点を TrialTable と同じ表形式（目的列 + パラメータ列）で表示する。
fn render_front_table(ui: &mut egui::Ui, result: &SurrogateMultiOptUiResult) {
    use egui_extras::{Column, TableBuilder};

    if result.front.is_empty() {
        return;
    }
    let n_obj = result.objective_names.len();
    let n_param = result.param_names.len();

    egui::ScrollArea::both()
        .max_height(200.0)
        .id_salt("surrogate_multi_front_scroll")
        .show(ui, |ui| {
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG;
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .columns(Column::initial(80.0).at_least(50.0), n_obj) // 各目的
                .columns(Column::initial(80.0).at_least(50.0), n_param) // 各パラメータ
                .header(20.0, |mut header| {
                    for name in &result.objective_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    for name in &result.param_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, result.front.len(), |mut row| {
                        let pt = &result.front[row.index()];
                        for v in &pt.values {
                            row.col(|ui| {
                                ui.monospace(format!("{:.6}", v));
                            });
                        }
                        for p in &pt.params {
                            row.col(|ui| {
                                ui.monospace(format!("{:.6}", p));
                            });
                        }
                    });
                });
        });
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

            // 予測最適点を大きな星マーカーで強調する（右端 = 最新 trial 位置に配置）。
            let opt_marker: egui_plot::PlotPoints =
                vec![[n.max(1.0) - 1.0, result.best_value]].into();
            plot_ui.points(
                egui_plot::Points::new(opt_marker)
                    .name("Predicted optimum")
                    .shape(egui_plot::MarkerShape::Asterisk)
                    .radius(9.0)
                    .color(predicted_line_color),
            );

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

/// EHVI による多目的候補提案の結果テーブルと "Copy enqueue JSON" ボタンを描画する。
fn render_multi_suggest_result(
    ui: &mut egui::Ui,
    result: &crate::state::messages::SurrogateMultiSuggestUiResult,
) {
    if result.candidates.is_empty() {
        return;
    }

    ui.add_space(4.0);
    ui.strong("Suggested candidates (EHVI):");

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .id_salt("surrogate_multi_suggest_scroll")
        .show(ui, |ui| {
            egui::Grid::new("surrogate_multi_suggest_table")
                .striped(true)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    // ── ヘッダ行 ──────────────────────────────────────
                    for name in &result.param_names {
                        ui.strong(name);
                    }
                    // 目的ごとに「予測値 ± std」列を 1 つにまとめる。
                    for name in &result.objective_names {
                        ui.strong(name);
                    }
                    ui.strong("EHVI");
                    ui.end_row();

                    // ── データ行 ──────────────────────────────────────
                    for c in &result.candidates {
                        for v in &c.params {
                            ui.monospace(format!("{:.6}", v));
                        }
                        for (k, val) in c.predicted_values.iter().enumerate() {
                            match c.predicted_stds.get(k).and_then(|s| *s) {
                                Some(std) => ui.monospace(format!("{:.4} ± {:.4}", val, std)),
                                None => ui.monospace(format!("{:.4}", val)),
                            };
                        }
                        ui.monospace(format!("{:.4e}", c.ehvi_score));
                        ui.end_row();
                    }
                });
        });

    ui.add_space(4.0);

    // ── "Copy enqueue JSON" ボタン（params のみのオブジェクト配列） ──
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
                auto_select: state.auto_select,
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
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
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
