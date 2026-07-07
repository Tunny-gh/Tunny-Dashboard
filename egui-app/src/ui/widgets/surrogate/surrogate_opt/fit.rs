//! 左列（Fit & Validate）: 目的・モデル選択、フィット進捗、検証指標・OOF 散布図。
//!
//! 単目的・多目的いずれのフィット列も扱う。学習自体はバックグラウンドで
//! 行われ（poll_chart.rs 参照）、ここでは選択 UI とフィット結果の検証表示を担う。

use std::sync::Arc;

use crate::ui::widget_states::{
    SurrogateFitComputeRequest, SurrogateMultiFitComputeRequest, SurrogateOptState,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::surrogate::MODEL_CHOICES;
use tunny_core::surrogate_opt::{SurrogateValidationReport, TrainedSurrogate};

use super::labels::{model_label, verdict, AUTO_MODEL_LABEL};
use super::{multi_trained_matches, trained_matches};

/// 左列（単目的）: Objective / Model コンボ、Fit & Validate ボタン、検証結果。
pub(super) fn render_fit_column(
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
pub(super) fn render_fit_column_multi(
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
    let mut rows: Vec<(tunny_core::surrogate_opt::SurrogateModelKind, f64)> =
        selection.scores.clone();
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
            crate::theme::chart_colors::COLOR_PARETO(),
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
    let ref_seg = egui_plot::Line::new("y = x", ref_line)
        .color(crate::theme::chart_colors::COLOR_GRID_STROKE())
        .style(egui_plot::LineStyle::Dashed { length: 6.0 });

    // 列幅いっぱいを使い、高さは 180 px 〜 400 px に収める。
    let plot_h = ui.available_height().clamp(180.0, 400.0);

    let mut plot = egui_plot::Plot::new(("surrogate_oof_plot", id_salt))
        .unified_nav()
        .height(plot_h)
        .data_aspect(1.0)
        .x_axis_label("Actual")
        .y_axis_label("Predicted (out-of-fold)")
        .legend(egui_plot::Legend::default());
    if reset {
        plot = plot.reset();
    }
    plot.show(ui, |plot_ui| {
        apply_wheel_zoom(plot_ui);
        // 非フロント点（青）を背面に。
        plot_ui.points(
            egui_plot::Points::new("Out-of-fold predictions", other_pts)
                .color(egui::Color32::from_rgb(59, 130, 246)) // blue-500
                .radius(3.0),
        );
        plot_ui.line(ref_seg);
        // フロント点（赤・大きめ）を前面に。
        if !front_pts.is_empty() {
            plot_ui.points(
                egui_plot::Points::new("Pareto front", front_pts)
                    .color(crate::theme::chart_colors::COLOR_PARETO())
                    .radius(4.0),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tunny_core::surrogate_opt::SurrogateModelKind;

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
}
