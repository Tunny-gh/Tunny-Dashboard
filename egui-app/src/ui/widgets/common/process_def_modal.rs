//! GUI editor ("builder") for a process-integration definition.
//!
//! A `ProcessDefinition` (the external command plus how its I/O maps to
//! parameters, objectives, and constraints) is otherwise authored as JSON by
//! hand. This modal turns that JSON into an editable form: add/remove parameters,
//! pick the input-passing scheme, describe the command, and declare each
//! objective/constraint's source and extractor. The result is saved back to JSON
//! (or handed straight to the run setup modal).
//!
//! Like the other modals, `show` returns `None` while open and a
//! [`ProcessDefBuilderAction`] once the user acts; the caller re-invokes it every
//! frame. All file I/O (load / save dialogs) is performed by the caller in
//! response to the returned action, matching the toolbar/action convention.

use egui::RichText;

use crate::state::types::{
    CommandEdit, ExtractorKind, InputKind, OutputSpecEdit, ProcessDefBuilderState, SourceKind,
};
use crate::ui::widgets::common::modal::ModalScaffold;

/// What the user asked the builder to do.
pub enum ProcessDefBuilderAction {
    /// Validate and write the definition to a JSON file (caller shows the dialog).
    Save,
    /// Validate and open the run setup modal with this definition.
    Optimize,
    /// Import an existing definition JSON into the form (caller shows the dialog).
    Load,
    /// Close the builder without saving.
    Cancel,
}

/// Renders the process-definition builder modal.
///
/// Keeps the dialog open until an action is returned; the caller must re-invoke
/// this every frame with the same `state`.
pub fn show(
    ctx: &egui::Context,
    state: &mut ProcessDefBuilderState,
) -> Option<ProcessDefBuilderAction> {
    let mut save_clicked = false;
    let mut optimize_clicked = false;
    let mut load_clicked = false;
    let mut cancel_clicked = false;

    let outcome = ModalScaffold::new("process_def_modal", 600.0)
        .max_width(640.0)
        .heading("Tool Definition")
        .show(ctx, |ui| {
            ui.label(
                RichText::new(
                    "Author a process definition: the external command and how its \
                     I/O maps to parameters and objectives.",
                )
                .color(crate::theme::TEXT_SECONDARY()),
            );
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .max_height(480.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    parameters_section(ui, state);
                    ui.separator();
                    input_section(ui, state);
                    ui.separator();
                    ui.label(RichText::new("Command").strong());
                    command_editor(ui, egui::Id::new("proc_def_cmd"), &mut state.command);
                    ui.separator();
                    outputs_section(
                        ui,
                        "Objectives",
                        None,
                        "proc_def_obj",
                        &mut state.objectives,
                    );
                    ui.separator();
                    outputs_section(
                        ui,
                        "Constraints",
                        Some("Feasible when the extracted value is ≤ 0."),
                        "proc_def_con",
                        &mut state.constraints,
                    );
                    ui.separator();
                    hooks_section(ui, state);
                });

            if let Some(status) = &state.status {
                ui.add_space(4.0);
                ui.colored_label(crate::theme::TEXT_SECONDARY(), status);
            }
            if let Some(err) = &state.error {
                ui.add_space(4.0);
                ui.colored_label(crate::theme::ERROR_COLOR(), err);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Save to File…").clicked() {
                    save_clicked = true;
                }
                if ui.button("Optimize →").clicked() {
                    optimize_clicked = true;
                }
                if ui.button("Load…").clicked() {
                    load_clicked = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
            });
        });

    if save_clicked {
        Some(ProcessDefBuilderAction::Save)
    } else if optimize_clicked {
        Some(ProcessDefBuilderAction::Optimize)
    } else if load_clicked {
        Some(ProcessDefBuilderAction::Load)
    } else if cancel_clicked || outcome.should_close {
        Some(ProcessDefBuilderAction::Cancel)
    } else {
        None
    }
}

/// Parameter-name list with add/remove. Names are referenced as `{name}` in the
/// input template / args, so the order and spelling matter.
fn parameters_section(ui: &mut egui::Ui, state: &mut ProcessDefBuilderState) {
    ui.label(RichText::new("Parameters").strong());
    ui.label(
        RichText::new("Referenced as {name} in the input template / args.")
            .color(crate::theme::TEXT_SECONDARY()),
    );
    let mut remove: Option<usize> = None;
    for (i, name) in state.param_names.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(name).desired_width(260.0));
            if ui.small_button("✕").on_hover_text("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        state.param_names.remove(i);
    }
    if ui.button("+ Add parameter").clicked() {
        state.param_names.push(String::new());
    }
}

/// Input-passing scheme selector plus its variant-specific fields.
fn input_section(ui: &mut egui::Ui, state: &mut ProcessDefBuilderState) {
    ui.label(RichText::new("Input").strong());
    ui.horizontal(|ui| {
        ui.label("How parameters reach the command:");
        egui::ComboBox::from_id_salt("proc_def_input_kind")
            .selected_text(input_kind_label(state.input_kind))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut state.input_kind,
                    InputKind::Args,
                    input_kind_label(InputKind::Args),
                );
                ui.selectable_value(
                    &mut state.input_kind,
                    InputKind::Env,
                    input_kind_label(InputKind::Env),
                );
                ui.selectable_value(
                    &mut state.input_kind,
                    InputKind::JsonStdin,
                    input_kind_label(InputKind::JsonStdin),
                );
                ui.selectable_value(
                    &mut state.input_kind,
                    InputKind::Template,
                    input_kind_label(InputKind::Template),
                );
            });
    });
    match state.input_kind {
        InputKind::Args => {
            ui.horizontal(|ui| {
                ui.label("Arg template:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.input_arg_template)
                        .desired_width(ui.available_width()),
                );
            });
            ui.label(
                RichText::new("{name} / {value} are expanded per parameter, then split on whitespace into argv entries.")
                    .color(crate::theme::TEXT_SECONDARY()),
            );
        }
        InputKind::Env => {
            ui.label(
                RichText::new(
                    "Each parameter is passed as an environment variable named after it.",
                )
                .color(crate::theme::TEXT_SECONDARY()),
            );
        }
        InputKind::JsonStdin => {
            ui.label(
                RichText::new(
                    "A {\"param\": value} JSON object is written to the command's stdin.",
                )
                .color(crate::theme::TEXT_SECONDARY()),
            );
        }
        InputKind::Template => {
            ui.horizontal(|ui| {
                ui.label("Template:");
                ui.add(
                    egui::TextEdit::multiline(&mut state.input_template)
                        .desired_rows(3)
                        .desired_width(ui.available_width()),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Write to path:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.input_path)
                        .desired_width(ui.available_width()),
                );
            });
            ui.label(
                RichText::new("{name} placeholders are substituted, then the result is written to the path (relative to the working dir).")
                    .color(crate::theme::TEXT_SECONDARY()),
            );
        }
    }
}

fn input_kind_label(kind: InputKind) -> &'static str {
    match kind {
        InputKind::Args => "Command-line args",
        InputKind::Env => "Environment variables",
        InputKind::JsonStdin => "JSON on stdin",
        InputKind::Template => "Input file template",
    }
}

/// A reusable objective/constraint list with add/remove. Each row is a boxed
/// group holding the name, output source, and extractor.
fn outputs_section(
    ui: &mut egui::Ui,
    heading: &str,
    hint: Option<&str>,
    id_prefix: &str,
    rows: &mut Vec<OutputSpecEdit>,
) {
    ui.label(RichText::new(heading).strong());
    if let Some(hint) = hint {
        ui.label(RichText::new(hint).color(crate::theme::TEXT_SECONDARY()));
    }
    let mut remove: Option<usize> = None;
    for (i, row) in rows.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut row.name).desired_width(200.0));
                if ui.small_button("✕").on_hover_text("Remove").clicked() {
                    remove = Some(i);
                }
            });
            output_editor(ui, egui::Id::new((id_prefix, i)), row);
        });
    }
    if let Some(i) = remove {
        rows.remove(i);
    }
    if ui
        .button(format!(
            "+ Add {}",
            heading.trim_end_matches('s').to_lowercase()
        ))
        .clicked()
    {
        rows.push(OutputSpecEdit::default());
    }
}

/// Draws the source + extractor controls for one output row.
fn output_editor(ui: &mut egui::Ui, id: egui::Id, spec: &mut OutputSpecEdit) {
    ui.horizontal(|ui| {
        ui.label("Read from:");
        egui::ComboBox::from_id_salt(id.with("src"))
            .selected_text(match spec.source_kind {
                SourceKind::Stdout => "stdout",
                SourceKind::File => "file",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut spec.source_kind, SourceKind::Stdout, "stdout");
                ui.selectable_value(&mut spec.source_kind, SourceKind::File, "file");
            });
        if spec.source_kind == SourceKind::File {
            ui.label("path:");
            ui.add(
                egui::TextEdit::singleline(&mut spec.source_path)
                    .desired_width(ui.available_width()),
            );
        }
    });
    ui.horizontal(|ui| {
        ui.label("Extractor:");
        egui::ComboBox::from_id_salt(id.with("ext"))
            .selected_text(match spec.extractor.kind {
                ExtractorKind::Regex => "regex",
                ExtractorKind::JsonPath => "json path",
                ExtractorKind::Csv => "csv",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut spec.extractor.kind, ExtractorKind::Regex, "regex");
                ui.selectable_value(
                    &mut spec.extractor.kind,
                    ExtractorKind::JsonPath,
                    "json path",
                );
                ui.selectable_value(&mut spec.extractor.kind, ExtractorKind::Csv, "csv");
            });
    });
    match spec.extractor.kind {
        ExtractorKind::Regex => {
            ui.horizontal(|ui| {
                ui.label("Pattern:");
                ui.add(
                    egui::TextEdit::singleline(&mut spec.extractor.regex_pattern)
                        .desired_width(ui.available_width()),
                );
            });
            ui.label(
                RichText::new(
                    "First capture group is parsed as a number (whole match if no group).",
                )
                .color(crate::theme::TEXT_SECONDARY()),
            );
        }
        ExtractorKind::JsonPath => {
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.add(
                    egui::TextEdit::singleline(&mut spec.extractor.json_path)
                        .desired_width(ui.available_width()),
                );
            });
            ui.label(
                RichText::new("Dotted path, e.g. results.weight or values.0.")
                    .color(crate::theme::TEXT_SECONDARY()),
            );
        }
        ExtractorKind::Csv => {
            ui.horizontal(|ui| {
                ui.label("Row:");
                egui::ComboBox::from_id_salt(id.with("row"))
                    .selected_text(if spec.extractor.csv_row_last {
                        "last"
                    } else {
                        "index"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut spec.extractor.csv_row_last, true, "last");
                        ui.selectable_value(&mut spec.extractor.csv_row_last, false, "index");
                    });
                if !spec.extractor.csv_row_last {
                    ui.add(egui::DragValue::new(&mut spec.extractor.csv_row_index));
                }
            });
            ui.horizontal(|ui| {
                ui.label("Column:");
                egui::ComboBox::from_id_salt(id.with("col"))
                    .selected_text(if spec.extractor.csv_col_by_header {
                        "header"
                    } else {
                        "index"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut spec.extractor.csv_col_by_header, false, "index");
                        ui.selectable_value(&mut spec.extractor.csv_col_by_header, true, "header");
                    });
                if spec.extractor.csv_col_by_header {
                    ui.add(
                        egui::TextEdit::singleline(&mut spec.extractor.csv_col_header)
                            .desired_width(160.0),
                    );
                } else {
                    ui.add(egui::DragValue::new(&mut spec.extractor.csv_col_index));
                }
            });
            // A header-named column implies a header row; keep the checkbox in sync
            // but let the user opt in for index columns too.
            let mut has_header = spec.extractor.csv_has_header || spec.extractor.csv_col_by_header;
            ui.add_enabled(
                !spec.extractor.csv_col_by_header,
                egui::Checkbox::new(&mut has_header, "First row is a header"),
            );
            spec.extractor.csv_has_header = has_header;
        }
    }
}

/// Optional pre-/post-command hooks.
fn hooks_section(ui: &mut egui::Ui, state: &mut ProcessDefBuilderState) {
    ui.label(RichText::new("Hooks (optional)").strong());
    ui.checkbox(
        &mut state.pre_command_enabled,
        "Run a command before each evaluation",
    );
    if state.pre_command_enabled {
        command_editor(ui, egui::Id::new("proc_def_pre"), &mut state.pre_command);
    }
    ui.checkbox(
        &mut state.post_command_enabled,
        "Run a command after each evaluation",
    );
    if state.post_command_enabled {
        command_editor(ui, egui::Id::new("proc_def_post"), &mut state.post_command);
    }
}

/// Editor for a `CommandEdit` (program, fixed args, working dir, timeout, retries).
fn command_editor(ui: &mut egui::Ui, id: egui::Id, cmd: &mut CommandEdit) {
    ui.horizontal(|ui| {
        ui.label("Program:");
        ui.add(egui::TextEdit::singleline(&mut cmd.program).desired_width(ui.available_width()));
    });
    ui.label(RichText::new("Fixed args (passed verbatim):").color(crate::theme::TEXT_SECONDARY()));
    let mut remove: Option<usize> = None;
    for (i, arg) in cmd.args.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(arg)
                    .desired_width(260.0)
                    .id(id.with(("arg", i))),
            );
            if ui.small_button("✕").on_hover_text("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        cmd.args.remove(i);
    }
    if ui.button("+ Add arg").clicked() {
        cmd.args.push(String::new());
    }
    ui.horizontal(|ui| {
        ui.label("Working dir:");
        ui.add(
            egui::TextEdit::singleline(&mut cmd.working_dir).desired_width(ui.available_width()),
        );
    });
    ui.label(RichText::new("Blank = current directory.").color(crate::theme::TEXT_SECONDARY()));
    ui.horizontal(|ui| {
        ui.label("Timeout (s):");
        ui.add(egui::DragValue::new(&mut cmd.timeout_secs).range(0..=86_400));
        ui.label("0 = none");
        ui.add_space(12.0);
        ui.label("Retries:");
        ui.add(egui::DragValue::new(&mut cmd.retries).range(0..=100));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tunny_core::process::{
        CommandSpec, CsvColumn, CsvRow, Extractor, InputSpec, OutputSource, OutputSpec,
        ProcessDefinition,
    };

    fn sample_definition() -> ProcessDefinition {
        ProcessDefinition {
            param_names: vec!["x".to_string(), "y".to_string()],
            input: InputSpec::Args {
                arg_template: "--{name}={value}".to_string(),
            },
            command: CommandSpec {
                program: "solver".to_string(),
                args: vec!["--quiet".to_string()],
                working_dir: Some("/tmp/run".to_string()),
                timeout_secs: 30,
                retries: 2,
            },
            objectives: vec![OutputSpec {
                name: "f".to_string(),
                source: OutputSource::Stdout,
                extractor: Extractor::Regex {
                    pattern: "f=([-0-9.]+)".to_string(),
                },
            }],
            constraints: vec![OutputSpec {
                name: "g".to_string(),
                source: OutputSource::File {
                    path: "out.csv".to_string(),
                },
                extractor: Extractor::Csv {
                    row: CsvRow::Last,
                    column: CsvColumn::Header {
                        name: "loss".to_string(),
                    },
                    has_header: true,
                },
            }],
            pre_command: None,
            post_command: Some(CommandSpec::new("cleanup")),
        }
    }

    #[test]
    fn from_then_to_definition_round_trips() {
        let def = sample_definition();
        let builder = ProcessDefBuilderState::from_definition(&def, None);
        assert_eq!(builder.to_definition(), def);
    }

    #[test]
    fn new_starter_needs_a_program() {
        // The starter is intentionally incomplete: a blank program must fail
        // validate, mirroring the run flow's "author the command first" gate.
        let def = ProcessDefBuilderState::new().to_definition();
        assert!(def.validate().is_err());
    }

    #[test]
    fn to_definition_drops_blank_names() {
        let mut builder = ProcessDefBuilderState::new();
        builder.command.program = "run".to_string();
        builder.param_names = vec!["a".to_string(), "  ".to_string(), "b".to_string()];
        builder.objectives.push(OutputSpecEdit::default()); // blank-named objective
        let def = builder.to_definition();
        assert_eq!(def.param_names, vec!["a".to_string(), "b".to_string()]);
        // Only the one named objective ("f") from the starter survives.
        assert_eq!(def.objectives.len(), 1);
        assert_eq!(def.objectives[0].name, "f");
        assert!(def.validate().is_ok());
    }
}
