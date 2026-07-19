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

/// Upper bound on the adaptive sampler's evaluation count (bootstrap floored to
/// the surrogate minimum of 10 on the core side; deduped candidates can lower
/// the actual count).
fn adaptive_total_evaluations(initial: usize, batch: usize, iterations: usize) -> usize {
    initial.max(10) + batch.max(1) * iterations
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

    let compute_ok = if state.compute_use_exe {
        !state.compute_exe_path.trim().is_empty()
    } else {
        !state.compute_url.trim().is_empty()
    };
    let can_run =
        !state.study_name.trim().is_empty() && !state.journal_path.trim().is_empty() && compute_ok;

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

            // ── Constraints (read-only; wired via the attribute component) ───────
            if !state.problem.constraints.is_empty() {
                ui.label(RichText::new("Constraints").strong());
                for c in &state.problem.constraints {
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

            // ── Attributes (read-only; recorded as trial user attributes) ────────
            if !state.problem.attributes.is_empty() {
                ui.label(RichText::new("Attributes").strong());
                for a in &state.problem.attributes {
                    ui.label(&a.name);
                }
                ui.label(
                    RichText::new("Recorded per trial as user attributes.")
                        .color(crate::theme::TEXT_SECONDARY()),
                );
                ui.add_space(8.0);
            }

            // ── Rhino.Compute connection settings ────────────────────────────────
            ui.label(RichText::new("Rhino.Compute").strong());
            ui.horizontal(|ui| {
                ui.radio_value(&mut state.compute_use_exe, true, "Launch EXE");
                ui.radio_value(&mut state.compute_use_exe, false, "Server URL");
            });
            if state.compute_use_exe {
                ui.horizontal(|ui| {
                    ui.label("EXE path:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.compute_exe_path)
                            .hint_text(r"...\rhino.compute\rhino.compute.exe")
                            .desired_width(200.0),
                    );
                    if ui.button("Browse…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("rhino.compute (*.exe)", &["exe"])
                            .pick_file()
                        {
                            state.compute_exe_path = path.to_string_lossy().into_owned();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut state.compute_port).range(1..=65535));
                    ui.label(
                        RichText::new("Launches the EXE and connects (stopped when the run ends)")
                            .color(crate::theme::TEXT_SECONDARY()),
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Server URL:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.compute_url)
                            .hint_text("http://localhost:6500")
                            .desired_width(240.0),
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
            use crate::state::app_state::GhSamplerChoice;
            ui.label(RichText::new("Sampler").strong());
            ui.horizontal(|ui| {
                ui.label("Method:");
                egui::ComboBox::from_id_salt("ghx_opt_sampler")
                    .selected_text(state.sampler.label())
                    .show_ui(ui, |ui| {
                        for choice in [
                            GhSamplerChoice::Nsga2,
                            GhSamplerChoice::Random,
                            GhSamplerChoice::Adaptive,
                        ] {
                            ui.selectable_value(&mut state.sampler, choice, choice.label());
                        }
                    });
            });
            match state.sampler {
                GhSamplerChoice::Random => {
                    ui.horizontal(|ui| {
                        ui.label("Trials:");
                        ui.add(egui::DragValue::new(&mut state.n_trials).range(1..=1_000_000));
                    });
                }
                GhSamplerChoice::Nsga2 => {
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
                GhSamplerChoice::Adaptive => {
                    ui.horizontal(|ui| {
                        ui.label("Initial random trials:");
                        ui.add(
                            egui::DragValue::new(&mut state.adaptive_initial).range(10..=10_000),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Batch size:");
                        ui.add(egui::DragValue::new(&mut state.adaptive_batch).range(1..=100));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Iterations:");
                        ui.add(
                            egui::DragValue::new(&mut state.adaptive_iterations).range(1..=10_000),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut state.adaptive_early_stop, "Stop early on convergence");
                    });
                    if state.adaptive_early_stop {
                        ui.horizontal(|ui| {
                            ui.label("Patience (iterations):");
                            ui.add(
                                egui::DragValue::new(&mut state.adaptive_patience).range(1..=1_000),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Min improvement (%):");
                            ui.add(
                                egui::DragValue::new(&mut state.adaptive_min_improvement_pct)
                                    .range(0.0..=100.0)
                                    .speed(0.1),
                            );
                        });
                    }
                    let total = adaptive_total_evaluations(
                        state.adaptive_initial,
                        state.adaptive_batch,
                        state.adaptive_iterations,
                    );
                    let hint = if state.adaptive_early_stop {
                        format!(
                            "Up to {total} evaluations (stops early once the hypervolume / best \
                             value improves by less than {:.1}% for {} iterations). Each iteration \
                             fits a surrogate (Auto model) and evaluates the best candidates \
                             (EI / EHVI).",
                            state.adaptive_min_improvement_pct, state.adaptive_patience
                        )
                    } else {
                        format!(
                            "Up to {total} evaluations. Each iteration fits a surrogate (Auto \
                             model) and evaluates the most promising candidates (EI / EHVI)."
                        )
                    };
                    ui.label(RichText::new(hint).color(crate::theme::TEXT_SECONDARY()));
                }
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
                gene_pool: None,
            }],
            objectives: (0..objectives)
                .map(|i| tunny_core::gh::GhObjective {
                    source_guid: format!("guid-{i}"),
                    name: format!("f{i}"),
                })
                .collect(),
            constraints: vec![],
            attributes: vec![],
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
