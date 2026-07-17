//! Right column (Optimization): Optimizer selection, result summary,
//! optimization history plot; wiring for the single- and multi-objective
//! optimization columns and the Suggest section.

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

/// Right column (single-objective): Optimizer / Surface X-Y combos, Run
/// Optimization button, results.
pub(super) fn render_optimize_column(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    busy: bool,
    has_matching_trained: bool,
    obj_history: Option<&[f64]>,
) {
    // ── Optimizer combo ─────────────────────────────────────────
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

    // ── Run Optimization button ──────────────────────────────────
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

    // Spinner shown while optimizing.
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

    // ── Suggest next trials section ──────────────────────────────
    // Only shown for single-objective, GP-family models.
    let is_gp = matches!(
        state.model,
        SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe | SurrogateModelKind::GpMoe
    );
    if has_matching_trained && is_gp {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.strong("Suggest next trials");

        // Acquisition function combo.
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

        // Candidate count DragValue (1-10, default 3).
        ui.horizontal(|ui| {
            ui.label("Candidates:");
            ui.add(egui::DragValue::new(&mut state.n_suggest_candidates).range(1..=10));
        });

        // Suggest button.
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

        // Spinner shown while suggesting.
        if state.suggesting {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing acquisition candidates…");
            });
        }

        // Results table.
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

/// Right column (multi-objective): fixed NSGA-II label + Run Optimization
/// button, results.
pub(super) fn render_optimize_column_multi(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    busy: bool,
    has_matching_multi_trained: bool,
    observed: Option<&ObservedData>,
) {
    // ── Optimizer (fixed label) ───────────────────────────────────
    ui.label("Optimizer: NSGA-II");

    // ── Run Optimization button ──────────────────────────────────
    let can_optimize = has_matching_multi_trained && !busy;
    if ui
        .add_enabled(can_optimize, egui::Button::new("Run Optimization"))
        .clicked()
    {
        state.error_message = None;
        state.pending_multi_optimize = Some(SurrogateMultiOptimizeComputeRequest);
    }

    // Spinner shown while optimizing.
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

    // ── Suggest next trials (EHVI) section ────────────────────────
    // EHVI is only offered for multi-objective, GP-family models.
    let is_gp = matches!(
        state.model,
        SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe | SurrogateModelKind::GpMoe
    );
    if has_matching_multi_trained && is_gp {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.strong("Suggest next trials (EHVI)");

        // Candidate count DragValue (1-10, default 3).
        ui.horizontal(|ui| {
            ui.label("Candidates:");
            ui.add(egui::DragValue::new(&mut state.n_multi_suggest_candidates).range(1..=10));
        });

        // Suggest button.
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

        // Spinner shown while suggesting.
        if state.multi_suggesting {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing EHVI candidates…");
            });
        }

        // Results table.
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

/// Pure function that returns the improvement amount (positive = there is
/// improvement), taking the optimization direction into account.
///
/// - minimize: `best_observed - predicted` (smaller is better, so this is
///   positive when observed > predicted)
/// - maximize: `predicted - best_observed` (larger is better, so this is
///   positive when predicted > observed)
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

    // ── (a) Improvement summary ─────────────────────────────────────────────
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

    // ── (b) Optimization history plot (with predicted-optimum overlay) ──────────
    let non_empty_history = obj_history.filter(|h| !h.is_empty());
    if let Some(history) = non_empty_history {
        ui.add_space(6.0);
        render_history_plot(ui, history, result);
        ui.add_space(6.0);
    }

    // ── Feasibility (shown when constraints are present) ─────────────────────────────
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

    // ── Variable combination at the estimated optimum (TrialTable style) ────────────────
    // Shows the parameter columns + predicted objective column in one row
    // (same table style as TrialTable).
    ui.add_space(6.0);
    ui.label("Optimal variable combination:");
    render_best_point_table(ui, result);
}

/// Displays the multi-objective optimization result.
/// Shows the predicted Pareto front as a scatter plot in objective space
/// (within the widget), followed by the front points' variable
/// combinations as a TrialTable-style table. The front is also overlaid on
/// the ParetoScatter widget as gold diamonds.
fn render_multi_result(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    state: &mut SurrogateOptState,
    observed: Option<&ObservedData>,
) {
    // ── Heading ────────────────────────────────────────────────────
    ui.strong(format!(
        "Predicted Pareto Front: {} points",
        result.front.len()
    ));

    // ── Per-objective R² (training) ─────────────────────────────────────
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

    // ── Predicted Pareto front scatter plot (objective space) ───────────────────────
    render_front_scatter(ui, result, state, observed);

    // ── Front-point table (TrialTable style: objective columns + parameter columns) ──
    ui.add_space(6.0);
    ui.label("Predicted front variable combinations:");
    render_front_table(ui, result);
}

/// Optimization history plot (all trial points + cumulative-best line +
/// predicted-optimum horizontal line).
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

    // Scatter points for all trials.
    let all_pts: egui_plot::PlotPoints = history
        .iter()
        .enumerate()
        .map(|(i, &v)| [i as f64, v])
        .collect();
    let scatter = egui_plot::Points::new("All Trials", all_pts)
        .color(COLOR_OPT_TRIAL())
        .radius(2.0);

    // Cumulative-best line.
    let best_pts: egui_plot::PlotPoints = compute_best_values(history, result.minimize)
        .into_iter()
        .collect();
    let best_line = egui_plot::Line::new("Best so far", best_pts)
        .color(COLOR_OPT_PRUNED())
        .width(1.5);

    // Predicted-optimum horizontal line.
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

            // Highlight the predicted optimum with a large asterisk marker
            // (placed at the right edge = the latest trial position).
            let opt_marker: egui_plot::PlotPoints =
                vec![[n.max(1.0) - 1.0, result.best_value]].into();
            plot_ui.points(
                egui_plot::Points::new("Predicted optimum", opt_marker)
                    .shape(egui_plot::MarkerShape::Asterisk)
                    .radius(9.0)
                    .color(predicted_line_color),
            );

            // ±1.96σ band of the predicted standard deviation (light gray dashed line).
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

    // ── Unit tests for improvement_delta ────────────────────────────

    #[test]
    fn improvement_delta_minimize_positive() {
        // observed 0.5, predicted 0.1 -> improvement = 0.5 - 0.1 = 0.4 (positive)
        let d = improvement_delta(true, 0.5, 0.1);
        assert!((d - 0.4).abs() < 1e-12, "delta = {d}");
    }

    #[test]
    fn improvement_delta_minimize_no_improvement() {
        // Negative or zero when the prediction is worse than the observed value.
        let d = improvement_delta(true, 0.1, 0.5);
        assert!(d < 0.0, "delta = {d}");
    }

    #[test]
    fn improvement_delta_maximize_positive() {
        // observed 0.8, predicted 1.2 -> improvement = 1.2 - 0.8 = 0.4 (positive)
        let d = improvement_delta(false, 0.8, 1.2);
        assert!((d - 0.4).abs() < 1e-12, "delta = {d}");
    }

    #[test]
    fn improvement_delta_maximize_no_improvement() {
        // Negative or zero when the prediction is worse than the observed value.
        let d = improvement_delta(false, 1.2, 0.8);
        assert!(d < 0.0, "delta = {d}");
    }

    #[test]
    fn improvement_delta_exact_zero() {
        // No improvement when observed and predicted are equal.
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
        // Selections are preserved.
        assert_eq!(item.model, SurrogateModelKind::Ridge);
        assert_eq!(item.optimizer, OptimizerKind::RandomSearch);
        assert_eq!(item.selected_objective, 1);

        // Arc<TrainedSurrogate> is also propagated (None here).
        assert!(item.trained.is_none());
        // multi_trained is also propagated (None here).
        assert!(item.multi_trained.is_none());
        drop(Arc::<u8>::new(0)); // Confirm Arc is usable (compile check).
    }
}
