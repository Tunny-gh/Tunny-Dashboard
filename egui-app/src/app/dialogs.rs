use super::*;

impl TunnyApp {
    /// Renders the "Report…" modal, and on Export confirmation collects a snapshot of
    /// the study and delegates report generation to a background thread (called every
    /// frame from here after `ToolbarAction::OpenReportDialog` starts
    /// `app_state.report_dialog`).
    pub(super) fn show_report_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::report_modal::{self, ReportModalAction};

        let Some(mut dialog) = self.app_state.report_dialog.take() else {
            return;
        };
        let study_name = self
            .app_state
            .current_study
            .as_ref()
            .map(|s| s.meta.name.clone());

        let action = report_modal::show(ctx, &mut dialog, study_name.as_deref());
        let can_start_export = !dialog.generating && dialog.success_paths.is_none();

        match action {
            Some(ReportModalAction::Close) => {
                // OK to close without waiting even while generating (the background job
                // continues fire-and-forget; without the dialog, completion/failure just
                // won't be reported).
            }
            Some(ReportModalAction::Export) if can_start_export => {
                match dialog.selected_formats() {
                    Err(e) => dialog.error = Some(e.to_string()),
                    Ok(formats) => {
                        dialog.error = None;
                        let default_name = report_modal::default_file_name_for(
                            study_name.as_deref().unwrap_or("study"),
                            &formats,
                        );
                        let chosen = rfd::FileDialog::new()
                            .set_file_name(&default_name)
                            .add_filter("Report", &["html", "md", "json"])
                            .save_file();
                        if let Some(base_path) = chosen {
                            if let Some(ctx_study) = &self.app_state.current_study {
                                let meta = ctx_study.meta.clone();
                                let df = ctx_study.view.df.clone();
                                let extras = tunny_core::dataframe::active_extras_snapshot();
                                let storage_display = crate::io::report_export::storage_display(
                                    self.app_state.journal_path.as_deref(),
                                );
                                dialog.generating = true;
                                crate::io::report_export::spawn_report_export(
                                    meta,
                                    df,
                                    extras,
                                    storage_display,
                                    dialog.lang,
                                    dialog.top_n,
                                    formats,
                                    base_path,
                                    self.sender(),
                                );
                            }
                        }
                    }
                }
                self.app_state.report_dialog = Some(dialog);
            }
            _ => {
                self.app_state.report_dialog = Some(dialog);
            }
        }
    }

    /// Renders the "Open URL…" dialog and, on Open confirmation, feeds the normalized
    /// URL string into `open_path` (the same path as `ToolbarAction::OpenJournal`).
    pub(super) fn show_db_url_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::rdb_url_modal::{self, RdbUrlDialogAction};

        let Some(mut input) = self.app_state.db_url_dialog.take() else {
            return;
        };
        match rdb_url_modal::show(ctx, &mut input) {
            Some(RdbUrlDialogAction::Open(normalized_url)) => {
                self.open_path(std::path::PathBuf::from(normalized_url));
            }
            Some(RdbUrlDialogAction::Cancel) => {
                // Drop input to close the dialog.
            }
            None => {
                // Not confirmed yet. Keep showing it on the next frame.
                self.app_state.db_url_dialog = Some(input);
            }
        }
    }

    /// Renders the CSV import confirmation dialog and, on confirmation, applies the
    /// edited values to the Study and activates it.
    pub(super) fn show_csv_import_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::csv_import_modal::{self, CsvImportAction};

        let Some(mut settings) = self.app_state.csv_import_settings.take() else {
            return;
        };
        match csv_import_modal::show(ctx, &mut settings) {
            Some(CsvImportAction::Apply) => {
                // Apply the edited values to the all_studies entry before dispatching
                // select_study.
                if let Some(slot) = self
                    .app_state
                    .all_studies
                    .iter_mut()
                    .find(|s| s.study_id == settings.study_id)
                {
                    settings.apply_to(slot);
                }
                if let Some(meta) = self
                    .app_state
                    .all_studies
                    .iter()
                    .find(|s| s.study_id == settings.study_id)
                    .cloned()
                {
                    self.is_loading = true;
                    crate::io::study_worker::dispatch_select_study(meta, self.tx.clone());
                }
                // Drop settings to close the dialog.
            }
            None => {
                // Not confirmed yet. Keep showing it on the next frame.
                self.app_state.csv_import_settings = Some(settings);
            }
        }
    }

    /// While files are being dragged over the window, dims the screen and shows
    /// what will happen on drop (Grasshopper optimization for .ghx, normal open
    /// for storage files, or an unsupported-type notice). This makes the
    /// always-available drop target visible.
    pub(super) fn show_drop_hover_overlay(&self, ctx: &egui::Context) {
        let hovered: Vec<_> = ctx.input(|i| i.raw.hovered_files.clone());
        if hovered.is_empty() {
            return;
        }
        let paths: Vec<std::path::PathBuf> = hovered.into_iter().filter_map(|f| f.path).collect();
        let text = if paths.iter().any(|p| crate::io::file::is_ghx_path(p)) {
            "Drop to set up Grasshopper optimization"
        } else if paths.iter().any(|p| crate::io::file::is_storage_path(p)) {
            "Drop to open"
        } else if paths.iter().any(|p| crate::io::file::is_gh_binary_path(p)) {
            ".gh is not supported — in Grasshopper, save as .ghx (Grasshopper XML) and drop that"
        } else if paths.is_empty() {
            // Some platforms don't expose the path while hovering.
            "Drop files to open"
        } else {
            "Unsupported file type (.log / .db / .sqlite / .csv / .ghx are supported)"
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("file_drop_overlay"),
        ));
        let rect = ctx.content_rect();
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(120));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(22.0),
            egui::Color32::WHITE,
        );
    }

    /// Renders the .ghx optimization setup modal. On Run confirmation, wires into
    /// `start_ghx_run`; setup errors (failures from `build_compute_definition` /
    /// `prepare_gh_run`) are sent back to the dialog, which stays open.
    pub(super) fn show_ghx_opt_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::ghx_opt_modal::{self, GhxOptAction};

        let Some(mut dialog) = self.app_state.gh_opt_dialog.take() else {
            return;
        };
        match ghx_opt_modal::show(ctx, &mut dialog) {
            Some(GhxOptAction::Run) => {
                self.start_ghx_run(dialog);
                // If start_ghx_run fails due to a setup error, it puts the dialog back
                // into gh_opt_dialog itself.
            }
            Some(GhxOptAction::Cancel) => {
                // Drop dialog to close it.
            }
            None => {
                // Not confirmed yet. Keep showing it on the next frame.
                self.app_state.gh_opt_dialog = Some(dialog);
            }
        }
    }

    /// Renders the process-definition builder (GUI editor). Save writes the form to
    /// a JSON file; Optimize hands the definition to the run setup modal; Load
    /// imports an existing JSON. Validation / I/O failures are shown in the builder,
    /// which stays open. All file dialogs are performed here (not in the widget).
    pub(super) fn show_process_def_builder(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::process_def_modal::{self, ProcessDefBuilderAction};

        let Some(mut builder) = self.app_state.process_def_builder.take() else {
            return;
        };
        match process_def_modal::show(ctx, &mut builder) {
            Some(ProcessDefBuilderAction::Save) => {
                self.save_process_definition(&mut builder);
                self.app_state.process_def_builder = Some(builder);
            }
            Some(ProcessDefBuilderAction::Optimize) => {
                // Validate, then hand the definition to the run setup modal.
                let def = builder.to_definition();
                if let Err(e) = def.validate() {
                    builder.error = Some(e);
                    builder.status = None;
                    self.app_state.process_def_builder = Some(builder);
                } else {
                    let path = builder
                        .source_path
                        .clone()
                        .unwrap_or_else(|| std::path::PathBuf::from("tool_definition.json"));
                    self.app_state.process_opt_dialog = Some(
                        crate::state::app_state::ProcessOptDialogState::new(def, &path),
                    );
                    // Drop the builder to close it.
                }
            }
            Some(ProcessDefBuilderAction::Load) => {
                self.load_into_process_builder(&mut builder);
                self.app_state.process_def_builder = Some(builder);
            }
            Some(ProcessDefBuilderAction::Cancel) => {
                // Drop the builder to close it.
            }
            None => {
                self.app_state.process_def_builder = Some(builder);
            }
        }
    }

    /// Renders the process-integration setup modal. On Run confirmation, wires into
    /// `start_process_run`; setup errors (invalid ranges / journal open / study
    /// creation) are sent back to the dialog, which stays open.
    pub(super) fn show_process_opt_dialog(&mut self, ctx: &egui::Context) {
        use crate::ui::widgets::process_opt_modal::{self, ProcessOptAction};

        let Some(mut dialog) = self.app_state.process_opt_dialog.take() else {
            return;
        };
        match process_opt_modal::show(ctx, &mut dialog) {
            Some(ProcessOptAction::Run) => {
                self.start_process_run(dialog);
                // On a setup error, start_process_run puts the dialog back itself.
            }
            Some(ProcessOptAction::Cancel) => {
                // Drop dialog to close it.
            }
            None => {
                // Not confirmed yet. Keep showing it on the next frame.
                self.app_state.process_opt_dialog = Some(dialog);
            }
        }
    }

    /// Displays a running (or just-finished) .ghx optimization in a non-modal progress
    /// overlay. Shows a progress bar + Cancel while running, and a result message +
    /// Close once finished.
    pub(super) fn show_ghx_opt_overlay(&mut self, ctx: &egui::Context) {
        let Some(run) = self.app_state.gh_opt_run.as_ref() else {
            return;
        };

        let mut cancel_clicked = false;
        let mut close_clicked = false;

        egui::Window::new("Grasshopper Optimization")
            .id(egui::Id::new("ghx_opt_progress_window"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .show(ctx, |ui| {
                ui.set_min_width(260.0);
                match &run.finished {
                    None => {
                        // Request a repaint at a fixed interval to smoothly update the
                        // progress.
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(250));
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new(&run.study_name).strong());
                        });
                        let snapshot = run.progress.snapshot();
                        if snapshot.total > 0 {
                            let frac =
                                (snapshot.done as f32 / snapshot.total as f32).clamp(0.0, 1.0);
                            ui.add(
                                egui::ProgressBar::new(frac)
                                    .show_percentage()
                                    .desired_width(240.0),
                            );
                        }
                        if !snapshot.stage.is_empty() {
                            ui.label(
                                egui::RichText::new(&snapshot.stage)
                                    .color(crate::theme::TEXT_SECONDARY()),
                            );
                        }
                        // The view is not updated while the run writes trials, so
                        // point at the control that does update it. The reload on
                        // completion is automatic, so this hint is only for the
                        // in-progress state.
                        ui.label(
                            egui::RichText::new(
                                "Click Reload in the toolbar to load the trials completed so far.",
                            )
                            .color(crate::theme::TEXT_SECONDARY())
                            .size(11.0),
                        );
                        let cancelling = run.progress.is_cancelled();
                        let label = if cancelling {
                            "Cancelling…"
                        } else {
                            "Cancel"
                        };
                        if ui
                            .add_enabled(!cancelling, egui::Button::new(label))
                            .clicked()
                        {
                            cancel_clicked = true;
                        }
                    }
                    Some(result) => {
                        match result {
                            Ok(msg) => {
                                ui.label(msg);
                            }
                            Err(err) => {
                                ui.colored_label(crate::theme::ERROR_COLOR(), err);
                            }
                        }
                        if ui.button("Close").clicked() {
                            close_clicked = true;
                        }
                    }
                }
            });

        if cancel_clicked {
            run.progress.request_cancel();
        }
        if close_clicked {
            self.app_state.gh_opt_run = None;
        }
    }
}
