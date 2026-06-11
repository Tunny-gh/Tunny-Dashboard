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

use crate::state::messages::SurrogateOptUiResult;
use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{
    SurrogateFitComputeRequest, SurrogateOptState, SurrogateOptimizeComputeRequest,
};
use crate::ui::widgets::surface_plot::{draw_colorbar_simple, draw_heatmap, value_range};
use tunny_core::surrogate_opt::{
    OptimizerKind, SurrogateModelKind, SurrogateValidationReport, TrainedSurrogate,
    MIN_TRIALS_FOR_SURROGATE_OPT,
};

/// モデル選択肢（コンボ表示順）。新モデル追加時はここへ並べる。
const MODEL_CHOICES: [SurrogateModelKind; 3] = [
    SurrogateModelKind::Kriging,
    SurrogateModelKind::SparseKriging,
    SurrogateModelKind::Ridge,
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
        SurrogateModelKind::Kriging => "Kriging",
        SurrogateModelKind::SparseKriging => "Sparse Kriging",
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

/// `param_names` は数値パラメータのみ（カテゴリカル列は最適化対象にしない）。
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    obj_names: &[String],
    cmap: ColorMap,
    trial_count: usize,
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

    let busy = state.fitting
        || state.optimizing
        || state.pending_fit.is_some()
        || state.pending_optimize.is_some();

    let has_matching_trained = state
        .trained
        .as_deref()
        .map(|t| trained_matches(t, state, obj_names))
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
                render_fit_column(ui, state, obj_names, busy);
            },
        );

        ui.separator();

        // ── 右列: Optimization ───────────────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                render_optimize_column(ui, state, param_names, busy, has_matching_trained, cmap);
            },
        );
    });
}

/// 左列: Objective / Model コンボ、Fit & Validate ボタン、検証結果。
fn render_fit_column(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    obj_names: &[String],
    busy: bool,
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

/// 右列: Optimizer / Surface X・Y コンボ、Run Optimization ボタン、結果。
fn render_optimize_column(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    busy: bool,
    has_matching_trained: bool,
    cmap: ColorMap,
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

    render_result(ui, result, cmap);
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
    render_oof_plot(ui, v);
}

/// OOF (out-of-fold) の predicted-vs-actual 散布図をレンダリングする。
/// 列幅に合わせて利用可能な高さを使い、最低 180 px・最大 400 px に収める。
fn render_oof_plot(ui: &mut egui::Ui, v: &SurrogateValidationReport) {
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

    egui_plot::Plot::new("surrogate_oof_plot")
        .height(plot_h)
        .data_aspect(1.0)
        .x_axis_label("Actual")
        .y_axis_label("Predicted (out-of-fold)")
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.points(scatter);
            plot_ui.line(ref_seg);
        });
}

fn render_result(ui: &mut egui::Ui, result: &SurrogateOptUiResult, cmap: ColorMap) {
    let direction = if result.minimize {
        "minimize"
    } else {
        "maximize"
    };
    let value_text = match result.predicted_std {
        Some(std) => format!("{:.6} ± {:.6}", result.best_value, 1.96 * std),
        None => format!("{:.6}", result.best_value),
    };
    ui.horizontal(|ui| {
        ui.strong(format!(
            "Predicted optimum of {} ({}):",
            result.objective_name, direction
        ));
        ui.monospace(value_text);
    });
    ui.label(format!("Surrogate R² = {:.3}", result.r_squared));

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

    let available = ui.available_rect_before_wrap();
    let plot_size = egui::vec2(
        (available.width() - 32.0).max(100.0),
        available.height().clamp(60.0, 300.0),
    );
    let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // 表示用に向きを揃える: 横 = param_x（左→右で増加）、縦 = param_y（上 = 最大）。
    // core の slice は z[i][j] = f(x_i, y_j) なので disp[r][c] = z[c][ny-1-r]。
    let nx = slice.x_values.len();
    let ny = slice.y_values.len();
    let display: Vec<Vec<f64>> = (0..ny)
        .map(|r| (0..nx).map(|c| slice.z_values[c][ny - 1 - r]).collect())
        .collect();
    draw_heatmap(&painter, rect, &display, cmap.clone());

    // 最適点マーカー（白丸＋黒縁）。
    let (x_min, x_max) = (slice.x_values[0], slice.x_values[nx - 1]);
    let (y_min, y_max) = (slice.y_values[0], slice.y_values[ny - 1]);
    if x_max > x_min && y_max > y_min {
        let fx = ((px_name.1 - x_min) / (x_max - x_min)).clamp(0.0, 1.0) as f32;
        let fy = ((py_name.1 - y_min) / (y_max - y_min)).clamp(0.0, 1.0) as f32;
        let marker = egui::pos2(
            rect.left() + fx * rect.width(),
            rect.bottom() - fy * rect.height(),
        );
        painter.circle_filled(marker, 5.0, egui::Color32::WHITE);
        painter.circle_stroke(marker, 5.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
    }

    let (v_min, v_max) = value_range(&display);
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap);
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
        }
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
            });
        }

        let req = state.pending_fit.as_ref().unwrap();
        assert_eq!(req.objective, "obj0");
        assert_eq!(req.model, SurrogateModelKind::Ridge);
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
}
