//! The setup modal for a generic process-integration optimization: run any
//! external command-line tool as the objective evaluator, with the Dashboard
//! driving the sampling (no Python / Optuna at runtime).
//!
//! The loaded `ProcessDefinition` (command + how its I/O maps to parameters and
//! objectives) is shown read-only — it is authored as JSON, not edited here.
//! The user fills in the search **range** for each parameter, the objective
//! **directions**, the sampler settings, and the journal output destination.

use egui::RichText;

use crate::state::types::ProcessOptDialogState;
use crate::ui::widgets::common::modal::ModalScaffold;

/// The dialog's action result.
pub enum ProcessOptAction {
    /// Start optimization with the current settings.
    Run,
    /// Close the dialog (start nothing).
    Cancel,
}

/// Computes NSGA-II's total evaluation count (population size rounded up to an
/// even number × (generation count + 1)). The rounding rule mirrors the runner
/// side (`(pop.max(4) + 1) & !1`).
fn nsga2_total_evaluations(population_size: usize, generations: usize) -> usize {
    let even_pop = (population_size.max(4) + 1) & !1;
    even_pop * (generations + 1)
}

/// Renders the process-integration optimization setup modal.
///
/// Keeps the dialog open until `Some(ProcessOptAction::Run)` /
/// `Some(ProcessOptAction::Cancel)` is returned (the caller must call this again
/// every frame, passing `state`).
pub fn show(ctx: &egui::Context, state: &mut ProcessOptDialogState) -> Option<ProcessOptAction> {
    let mut run_clicked = false;
    let mut cancel_clicked = false;

    // A run needs a study name, an output path, and every parameter range valid
    // (low strictly less than high; also rejects NaN).
    let ranges_ok = state
        .ranges
        .iter()
        .all(|r| r.low.partial_cmp(&r.high) == Some(std::cmp::Ordering::Less));
    let can_run =
        !state.study_name.trim().is_empty() && !state.journal_path.trim().is_empty() && ranges_ok;

    let program = state.def.command.program.clone();
    let args_display = state.def.command.args.join(" ");

    let outcome = ModalScaffold::new("process_opt_modal", 560.0)
        .heading("Tool Optimization")
        .show(ctx, |ui| {
            // ── Command (read-only) ──────────────────────────────
            ui.label(RichText::new("Command").strong());
            ui.label(format!("Program: {program}"));
            if !args_display.is_empty() {
                ui.label(
                    RichText::new(format!("Args: {args_display}"))
                        .color(crate::theme::TEXT_SECONDARY()),
                );
            }
            ui.label(
                RichText::new(format!(
                    "{} objective(s), {} constraint(s)",
                    state.def.objectives.len(),
                    state.def.constraints.len()
                ))
                .color(crate::theme::TEXT_SECONDARY()),
            );
            ui.separator();

            // ── Variables (editable search ranges) ───────────────
            ui.label(RichText::new("Variables").strong());
            if state.ranges.is_empty() {
                ui.label(
                    RichText::new("This definition declares no parameters.")
                        .color(crate::theme::WARNING_COLOR()),
                );
            } else {
                egui::Grid::new("process_opt_variables")
                    .num_columns(5)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Name").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("Low").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("High").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("Digits").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("Integer").color(crate::theme::TEXT_SECONDARY()));
                        ui.end_row();
                        for r in state.ranges.iter_mut() {
                            ui.label(&r.name);
                            ui.add(egui::DragValue::new(&mut r.low).speed(0.1));
                            ui.add(egui::DragValue::new(&mut r.high).speed(0.1));
                            ui.add_enabled(
                                !r.is_integer,
                                egui::DragValue::new(&mut r.digits).range(0..=15),
                            );
                            if ui.checkbox(&mut r.is_integer, "").changed() && r.is_integer {
                                // Integer parameters are rounded to whole numbers;
                                // decimal digits no longer apply.
                                r.digits = 0;
                            }
                            // Flag an empty/invalid range inline next to the row.
                            if r.low.partial_cmp(&r.high) != Some(std::cmp::Ordering::Less) {
                                ui.colored_label(crate::theme::ERROR_COLOR(), "low < high");
                            }
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(8.0);

            // ── Objectives (only the direction is editable) ──────
            ui.label(RichText::new("Objectives").strong());
            if state.def.objectives.is_empty() {
                ui.label(
                    RichText::new("This definition declares no objectives.")
                        .color(crate::theme::WARNING_COLOR()),
                );
            } else {
                egui::Grid::new("process_opt_objectives")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        // Iterate over corresponding elements via zip so a length
                        // mismatch can never panic (same policy as ghx_opt_modal).
                        for (i, (obj, is_max)) in state
                            .def
                            .objectives
                            .iter()
                            .zip(state.maximize.iter_mut())
                            .enumerate()
                        {
                            ui.label(&obj.name);
                            egui::ComboBox::from_id_salt(("process_obj_dir", i))
                                .selected_text(if *is_max { "Maximize" } else { "Minimize" })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(is_max, false, "Minimize");
                                    ui.selectable_value(is_max, true, "Maximize");
                                });
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(8.0);

            // ── Constraints (read-only) ──────────────────────────
            if !state.def.constraints.is_empty() {
                ui.label(RichText::new("Constraints").strong());
                for c in &state.def.constraints {
                    ui.label(&c.name);
                }
                ui.label(
                    RichText::new(
                        "Feasible when ≤ 0. Recorded per trial and used to steer NSGA-II.",
                    )
                    .color(crate::theme::TEXT_SECONDARY()),
                );
                ui.add_space(8.0);
            }

            // ── Sampler ──────────────────────────────────────────
            ui.label(RichText::new("Sampler").strong());
            ui.horizontal(|ui| {
                ui.label("Method:");
                egui::ComboBox::from_id_salt("process_opt_sampler")
                    .selected_text(if state.sampler_is_random {
                        "Random"
                    } else {
                        "NSGA-II"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.sampler_is_random, false, "NSGA-II");
                        ui.selectable_value(&mut state.sampler_is_random, true, "Random");
                    });
            });
            if state.sampler_is_random {
                ui.horizontal(|ui| {
                    ui.label("Trials:");
                    ui.add(egui::DragValue::new(&mut state.n_trials).range(1..=1_000_000));
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Population:");
                    ui.add(egui::DragValue::new(&mut state.population_size).range(4..=10_000));
                });
                ui.horizontal(|ui| {
                    ui.label("Generations:");
                    ui.add(egui::DragValue::new(&mut state.generations).range(0..=100_000));
                });
                let total = nsga2_total_evaluations(state.population_size, state.generations);
                ui.label(
                    RichText::new(format!("Total evaluations = {total}"))
                        .color(crate::theme::TEXT_SECONDARY()),
                );
            }
            ui.horizontal(|ui| {
                ui.label("Seed:");
                ui.add(egui::DragValue::new(&mut state.seed));
            });
            ui.add_space(8.0);

            // ── Output ───────────────────────────────────────────
            ui.label(RichText::new("Output").strong());
            ui.horizontal(|ui| {
                ui.label("Journal path:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.journal_path)
                        .desired_width(ui.available_width()),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Study name:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.study_name)
                        .desired_width(ui.available_width()),
                );
            });

            if let Some(err) = &state.error {
                ui.add_space(4.0);
                ui.colored_label(crate::theme::ERROR_COLOR(), err);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(can_run, egui::Button::new("Run")).clicked() {
                    run_clicked = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
            });
        });

    if run_clicked {
        Some(ProcessOptAction::Run)
    } else if cancel_clicked || outcome.should_close {
        Some(ProcessOptAction::Cancel)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nsga2_total_evaluations_matches_runner_evenization() {
        // pop=16, gen=10 → even_pop=16 * (10+1) = 176
        assert_eq!(nsga2_total_evaluations(16, 10), 176);
        // pop=15 (odd) → 16 * (10+1) = 176
        assert_eq!(nsga2_total_evaluations(15, 10), 176);
        // pop below 4 rounds up to 4: pop=1 → 4 * (0+1) = 4
        assert_eq!(nsga2_total_evaluations(1, 0), 4);
    }
}
