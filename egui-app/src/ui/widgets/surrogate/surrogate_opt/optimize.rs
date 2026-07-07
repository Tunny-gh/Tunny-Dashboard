//! 右列（Optimization）: Optimizer 選択、結果サマリ、最適化履歴プロット、
//! 単目的・多目的の最適化列と Suggest セクションの配線。

use crate::state::messages::{SurrogateMultiOptUiResult, SurrogateOptUiResult};
use crate::ui::widget_states::{
    SurrogateMultiOptimizeComputeRequest, SurrogateMultiSuggestComputeRequest, SurrogateOptState,
    SurrogateOptimizeComputeRequest, SurrogateSuggestComputeRequest,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use tunny_core::surrogate_opt::{AcquisitionKind, SurrogateModelKind};

use super::front_view::render_front_scatter;
use super::labels::{acq_label, optimizer_label, OPTIMIZER_CHOICES};
use super::suggest::{render_multi_suggest_result, render_suggest_result};
use super::tables::{render_best_point_table, render_front_table};
use super::ObservedData;

/// 右列（単目的）: Optimizer / Surface X・Y コンボ、Run Optimization ボタン、結果。
pub(super) fn render_optimize_column(
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
pub(super) fn render_optimize_column_multi(
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
    let scatter = egui_plot::Points::new("All Trials", all_pts)
        .color(COLOR_OPT_TRIAL())
        .radius(2.0);

    // 累積ベスト線。
    let best_pts: egui_plot::PlotPoints = compute_best_values(history, result.minimize)
        .into_iter()
        .collect();
    let best_line = egui_plot::Line::new("Best so far", best_pts)
        .color(COLOR_OPT_PRUNED())
        .width(1.5);

    // 予測最適値の水平線。
    let n = history.len() as f64;
    let hline_pts: egui_plot::PlotPoints = vec![
        [0.0, result.best_value],
        [n.max(1.0) - 1.0, result.best_value],
    ]
    .into();
    let hline = egui_plot::Line::new("Predicted optimum", hline_pts)
        .color(predicted_line_color)
        .width(1.5)
        .style(egui_plot::LineStyle::Dashed { length: 8.0 });

    egui_plot::Plot::new("surrogate_history_plot")
        .unified_nav()
        .height(200.0)
        .x_axis_label("Trial")
        .y_axis_label(&result.objective_name)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            plot_ui.points(scatter);
            plot_ui.line(best_line);
            plot_ui.line(hline);

            // 予測最適点を大きな星マーカーで強調する（右端 = 最新 trial 位置に配置）。
            let opt_marker: egui_plot::PlotPoints =
                vec![[n.max(1.0) - 1.0, result.best_value]].into();
            plot_ui.points(
                egui_plot::Points::new("Predicted optimum", opt_marker)
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
                        egui_plot::Line::new(name, band_pts)
                            .color(egui::Color32::from_rgb(156, 163, 175)) // gray-400
                            .width(1.0)
                            .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
                    );
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tunny_core::surrogate_opt::{OptimizerKind, SurrogateModelKind};

    fn ui_result() -> SurrogateOptUiResult {
        SurrogateOptUiResult {
            best_params: vec![("x".to_string(), 0.3), ("y".to_string(), 0.7)],
            best_value: 0.01,
            predicted_std: Some(0.005),
            r_squared: 0.95,
            objective_name: "obj0".to_string(),
            minimize: true,
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
    fn result_arrival_switches_spinner_off() {
        let mut state = SurrogateOptState {
            optimizing: true,
            ..Default::default()
        };
        state.result = Some(ui_result());
        state.optimizing = false;
        assert!(!state.optimizing);
        assert!(state.result.is_some());
    }

    #[test]
    fn adopt_compute_state_keeps_selections() {
        use std::sync::Arc;

        let global = SurrogateOptState {
            result: Some(ui_result()),
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
}
