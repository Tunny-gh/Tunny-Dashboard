//! The setup modal for confirming an optimization problem opened via .ghx D&D and
//! starting a background optimization through Rhino.Compute.
//!
//! The variables and objectives extracted by `extract_problem` are shown read-only
//! (there is no way to edit the ghx-side slider ranges from the UI). The user can
//! only edit the objectives' optimization directions, the Rhino.Compute connection
//! settings, the sampler settings, and the output destination.

use egui::RichText;

use crate::state::app_state::GhOptDialogState;
use crate::ui::widgets::common::modal::ModalScaffold;

/// The dialog's action result.
pub enum GhxOptAction {
    /// Start optimization with the current settings.
    Run,
    /// Close the dialog (start nothing).
    Cancel,
}

/// Computes NSGA-II's total evaluation count (population size rounded up to an even number ×
/// (generation count + 1)). The rounding rule is the same as the implementation on the
/// `tunny_core::gh::runner` side (`(pop.max(4) + 1) & !1`).
fn nsga2_total_evaluations(population_size: usize, generations: usize) -> usize {
    let even_pop = (population_size.max(4) + 1) & !1;
    even_pop * (generations + 1)
}

/// Renders the .ghx optimization setup modal.
///
/// Keeps the dialog open until `Some(GhxOptAction::Run)` / `Some(GhxOptAction::Cancel)`
/// is returned (the caller must call this again every frame, passing `state`).
pub fn show(ctx: &egui::Context, state: &mut GhOptDialogState) -> Option<GhxOptAction> {
    let mut run_clicked = false;
    let mut cancel_clicked = false;

    let file_name = state
        .ghx_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("(unknown)")
        .to_string();

    let can_run = !state.study_name.trim().is_empty()
        && !state.journal_path.trim().is_empty()
        && !state.compute_target.trim().is_empty();

    let outcome = ModalScaffold::new("ghx_opt_modal", 520.0)
        .heading("Grasshopper Optimization")
        .show(ctx, |ui| {
            ui.label(format!("File: {file_name}"));
            ui.label(format!(
                "Tunny component: {}",
                state.problem.tunny_component
            ));

            if !state.problem.warnings.is_empty() {
                ui.add_space(4.0);
                for w in &state.problem.warnings {
                    ui.colored_label(crate::theme::WARNING_COLOR(), format!("⚠ {w}"));
                }
            }
            ui.separator();

            // ── Variables (read-only) ────────────────────────
            ui.label(RichText::new("Variables").strong());
            if state.problem.variables.is_empty() {
                ui.label(
                    RichText::new("No variables detected.").color(crate::theme::TEXT_SECONDARY()),
                );
            } else {
                egui::Grid::new("ghx_opt_variables")
                    .num_columns(3)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Name").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("Range").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("Type").color(crate::theme::TEXT_SECONDARY()));
                        ui.end_row();
                        for v in &state.problem.variables {
                            ui.label(&v.name);
                            ui.label(format!("{}..{}", v.low, v.high));
                            let ty = if v.is_integer {
                                "int".to_string()
                            } else {
                                format!("{} digits", v.digits)
                            };
                            ui.label(ty);
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(8.0);

            // ── Objectives (only the direction is editable) ──────────────────────
            ui.label(RichText::new("Objectives").strong());
            egui::Grid::new("ghx_opt_objectives")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    // objectives and maximize should have the same length, but to avoid
                    // panicking even if they don't match, iterate only over corresponding
                    // elements via zip (any leftover is ignored — the same policy as
                    // csv_import_modal).
                    for (i, (obj, is_max)) in state
                        .problem
                        .objectives
                        .iter()
                        .zip(state.maximize.iter_mut())
                        .enumerate()
                    {
                        ui.label(&obj.name);
                        egui::ComboBox::from_id_salt(("ghx_obj_dir", i))
                            .selected_text(if *is_max { "Maximize" } else { "Minimize" })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(is_max, false, "Minimize");
                                ui.selectable_value(is_max, true, "Maximize");
                            });
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);

            // ── Rhino.Compute connection settings ────────────────────────────────
            ui.label(RichText::new("Rhino.Compute").strong());
            ui.horizontal(|ui| {
                ui.label("Server:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.compute_target)
                        .hint_text("http://localhost:6500 or rhino.compute.exe path")
                        .desired_width(240.0),
                );
            });
            // Only show the port input when an EXE path is specified, and indicate that
            // the Dashboard manages the launch.
            if matches!(
                tunny_core::gh::classify_compute_input(&state.compute_target),
                tunny_core::gh::ComputeTarget::Exe(_)
            ) {
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut state.compute_port).range(1..=65535));
                    ui.label(
                        RichText::new("Launches the EXE and connects (stopped when the run ends)")
                            .color(crate::theme::TEXT_SECONDARY()),
                    );
                });
            }
            ui.horizontal(|ui| {
                ui.label("API key:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.api_key)
                        .password(true)
                        .desired_width(240.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Max parallel:");
                ui.add(egui::DragValue::new(&mut state.max_parallel).range(1..=16));
            });
            ui.add_space(8.0);

            // ── Sampler ────────────────────────────────────────
            ui.label(RichText::new("Sampler").strong());
            ui.horizontal(|ui| {
                ui.label("Method:");
                egui::ComboBox::from_id_salt("ghx_opt_sampler")
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

            // ── Output ────────────────────────────────────────
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
        Some(GhxOptAction::Run)
    } else if cancel_clicked || outcome.should_close {
        Some(GhxOptAction::Cancel)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_problem(objectives: usize) -> tunny_core::gh::GhProblem {
        tunny_core::gh::GhProblem {
            variables: vec![tunny_core::gh::GhVariable {
                instance_guid: "g".to_string(),
                name: "x".to_string(),
                low: 0.0,
                high: 10.0,
                value: 5.0,
                digits: 2,
                is_integer: false,
            }],
            objectives: (0..objectives)
                .map(|i| tunny_core::gh::GhObjective {
                    source_guid: format!("guid-{i}"),
                    name: format!("f{i}"),
                })
                .collect(),
            tunny_component: "Tunny".to_string(),
            warnings: vec![],
        }
    }

    fn make_state() -> GhOptDialogState {
        GhOptDialogState::new(
            std::path::PathBuf::from("/tmp/model.ghx"),
            "<xml/>".to_string(),
            make_problem(2),
        )
    }

    #[test]
    fn nsga2_total_evaluations_matches_runner_evenization() {
        // pop=16, gen=10 → even_pop=16 (already even) * (10+1) = 176
        assert_eq!(nsga2_total_evaluations(16, 10), 176);
        // pop=15 (odd) → (15+1)&!1 = 16 * (10+1) = 176
        assert_eq!(nsga2_total_evaluations(15, 10), 176);
        // pop below 4 is rounded up to 4: pop=1 → (4+1)&!1 = 4 * (0+1) = 4
        assert_eq!(nsga2_total_evaluations(1, 0), 4);
    }

    #[test]
    fn default_state_has_no_maximize_flags_set() {
        let state = make_state();
        assert_eq!(state.maximize, vec![false, false]);
    }
}
