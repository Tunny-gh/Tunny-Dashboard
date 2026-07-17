//! Left column (Fit & Validate): objective/model selection, fit progress, validation
//! metrics, and OOF scatter plot.
//!
//! Handles both single-objective and multi-objective fit columns. Training itself runs in
//! the background (see poll_chart.rs); this module is responsible for the selection UI and
//! the display of the fit's validation results.

use std::sync::Arc;

use crate::ui::widget_states::{
    SurrogateFitComputeRequest, SurrogateMultiFitComputeRequest, SurrogateOptState,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::surrogate::MODEL_CHOICES;
use tunny_core::surrogate_opt::{SurrogateValidationReport, TrainedSurrogate};

use super::labels::{model_label, verdict, AUTO_MODEL_LABEL};
use super::{multi_trained_matches, trained_matches};

/// Left column (single-objective): Objective / Model combo boxes, Fit & Validate button,
/// validation results.
pub(super) fn render_fit_column(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    obj_names: &[String],
    busy: bool,
    constraint_col_names: &[String],
) {
    // ── Row 1: objective / model ──────────────────────────────────────
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
                // Auto (automatically selected via cross-validation) comes first. Selecting
                // it sets auto_select = true.
                if ui
                    .selectable_label(state.auto_select, AUTO_MODEL_LABEL)
                    .clicked()
                {
                    state.auto_select = true;
                }
                // Selecting a specific model sets auto_select = false and that kind.
                for kind in MODEL_CHOICES {
                    let selected = !state.auto_select && state.model == kind;
                    if ui.selectable_label(selected, model_label(kind)).clicked() {
                        state.auto_select = false;
                        state.model = kind;
                    }
                }
            });
    });

    // ── Constraint checkbox (only shown for constrained Studies) ──────────
    let n_constraints = constraint_col_names.len();
    if n_constraints > 0 {
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut state.use_constraints,
                format!("Use constraints ({})", n_constraints),
            );
        });
    }

    // ── Row 2: Fit & Validate ────────────────────────────────────
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

    // While fitting: progress bar + cancel button.
    if state.fitting {
        render_fit_progress(ui, state);
        return;
    }

    // ── Validation section ───────────────────────────────────────────
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

/// Renders the in-progress fit (stage label, progress bar) and the cancel button.
///
/// The progress handle is shared with the training thread; the Cancel button sets an
/// internal cancellation flag (the training side detects it at stage boundaries and aborts).
/// While training, the UI is repainted every frame to keep the progress bar updated.
fn render_fit_progress(ui: &mut egui::Ui, state: &SurrogateOptState) {
    // Request a repaint so the progress updates smoothly.
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

    // Progress bar (when the total number of steps is known).
    if let Some(s) = snapshot.as_ref().filter(|s| s.total > 0) {
        let frac = (s.done as f32 / s.total as f32).clamp(0.0, 1.0);
        ui.add(
            egui::ProgressBar::new(frac)
                .show_percentage()
                .desired_width(240.0),
        );
    }

    // Cancel button.
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

/// Left column (multi-objective): fixed label for all objectives + Model combo, Fit &
/// Validate button, validation results.
pub(super) fn render_fit_column_multi(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    obj_names: &[String],
    busy: bool,
) {
    // ── Row 1: objectives (fixed label) / model ───────────────────────
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

    // ── Row 2: Fit & Validate ────────────────────────────────────
    let can_fit = !busy && obj_names.len() >= 2;
    if ui
        .add_enabled(can_fit, egui::Button::new("Fit & Validate"))
        .clicked()
    {
        state.error_message = None;
        state.pending_multi_fit = Some(SurrogateMultiFitComputeRequest { model: state.model });
    }

    // While fitting: progress bar + cancel button.
    if state.fitting {
        render_fit_progress(ui, state);
        return;
    }

    // ── Validation section (compact per-objective summary) ────────────
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

/// Renders the validation metrics section.
fn render_validation(ui: &mut egui::Ui, trained: &Arc<TrainedSurrogate>) {
    let v = &trained.validation;
    ui.add_space(4.0);

    // ── History of the Auto selection (only shown for an Auto fit) ────────────────
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

    // Quality verdict.
    let (verdict_text, verdict_color) = verdict(v.cv_r2_mean);
    ui.colored_label(verdict_color, verdict_text);

    // predicted-vs-actual scatter plot.
    render_oof_plot(ui, v, "single", false);
}

/// Shows how the Auto model selection was made. Displays an "Auto selected: <chosen>"
/// heading and a compact table of each candidate's CV R² sorted in descending order
/// (candidates that failed to fit/validate show "—").
fn render_model_selection(
    ui: &mut egui::Ui,
    selection: &tunny_core::surrogate_opt::ModelSelectionReport,
) {
    ui.strong(format!(
        "Auto selected: {}",
        model_label(selection.chosen)
    ))
    .on_hover_text(
        "Cross-validated the candidate models and selected the one with the highest CV R² (ties prefer the simpler model).",
    );

    // Sort candidates by descending CV R² (failed candidates = NEG_INFINITY go last).
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
                // Highlight the chosen candidate.
                if kind == selection.chosen {
                    ui.strong(model_label(kind));
                } else {
                    ui.label(model_label(kind));
                }
                if score.is_finite() {
                    ui.monospace(format!("{:.3}", score));
                } else {
                    // Candidate that failed to fit/validate.
                    ui.monospace("—");
                }
                ui.end_row();
            }
        });
    ui.add_space(4.0);
}

/// Displays a compact multi-objective validation summary (one row per objective).
/// Below the grid, shows the OOF predicted-vs-actual plot for the selected objective.
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
            // Header row
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

    // ── Objective selection for the OOF plot ───────────────────────────────
    // Clamp the index range (e.g. if the number of objectives decreased).
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

    // predicted-vs-actual scatter plot for the selected objective. Since value ranges
    // differ per objective, split the plot ID per objective, and reset the display range
    // when switching so it refits.
    if let Some(t) = trained.get(state.multi_validation_objective) {
        let switched = state.multi_validation_objective != prev_objective;
        render_oof_plot(ui, &t.validation, &t.objective_name, switched);
    }
}

/// Renders the OOF (out-of-fold) predicted-vs-actual scatter plot.
/// Uses the available height matched to the column width, clamped between 180 px and 400 px.
///
/// `id_salt` separates the plot memory (zoom, display range) per caller.
/// When `data_aspect` is set, auto-fit no longer takes effect after the first frame, so
/// `reset = true` recomputes the range whenever the displayed data changes.
fn render_oof_plot(ui: &mut egui::Ui, v: &SurrogateValidationReport, id_salt: &str, reset: bool) {
    if v.oof_pairs.is_empty() {
        return;
    }

    // Compute the data range (span of the y=x reference line).
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
        // If the data is degenerate, add margin to display something minimal.
        let center = if min_val.is_finite() { min_val } else { 0.0 };
        min_val = center - 1.0;
        max_val = center + 1.0;
    }

    // If Pareto-front membership flags are available, highlight front points separately.
    // Front membership is only known for a multi-objective fit (single-objective draws
    // every point in blue).
    let has_front_flags =
        v.oof_is_front.len() == v.oof_pairs.len() && v.oof_is_front.iter().any(|&f| f);
    let n_front = v.oof_is_front.iter().filter(|&&f| f).count();

    // Show the front-only fit quality as a number first (it tends to get buried in the
    // scatter plot).
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
            "Out-of-fold accuracy computed only on Pareto-front (rank 0) trials. \
             Shows how well the surrogate predicts the region near the front that \
             the optimization actually uses.",
        );
    }

    // Split into front points (red) and the rest (blue).
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

    // Use the full column width, and clamp the height to 180 px – 400 px.
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
        // Non-front points (blue) in the back.
        plot_ui.points(
            egui_plot::Points::new("Out-of-fold predictions", other_pts)
                .color(egui::Color32::from_rgb(59, 130, 246)) // blue-500
                .radius(3.0),
        );
        plot_ui.line(ref_seg);
        // Front points (red, larger) in front.
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

        // Same logic as clicking the Fit & Validate button.
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

        // Same logic as clicking the multi-objective Fit & Validate button.
        if obj_names.len() >= 2 {
            state.error_message = None;
            state.pending_multi_fit = Some(SurrogateMultiFitComputeRequest { model: state.model });
        }

        let req = state.pending_multi_fit.as_ref().unwrap();
        assert_eq!(req.model, SurrogateModelKind::GpFitc);
    }
}
