use super::*;

impl TunnyApp {
    /// Opens the given path (journal / CSV / SQLite / RDB URL — any of them).
    /// Shared handling called both from `ToolbarAction::OpenJournal` and the Open button
    /// of the "Open URL…" dialog (a URL is passed as
    /// `PathBuf::from(normalized url string)`).
    pub(super) fn open_path(&mut self, path: std::path::PathBuf) {
        // .ghx is separate from the existing journal/CSV/SQLite/RDB scan path (it's an
        // optimization problem definition, not a result store). Route it through the
        // same handling as D&D (`handle_dropped_files`), and open the optimization setup
        // modal once extraction succeeds.
        if crate::io::file::is_ghx_path(&path) {
            self.open_ghx_path(path);
            return;
        }
        self.is_loading = true;
        self.load_error = None;
        self.app_state.all_studies.clear();
        self.app_state.current_study = None;
        // Opening a different file (a different URL) changes the study_id space, so
        // discard any comparison session that assumed the same file.
        self.app_state.reset_comparison_session();
        // Abandon any in-flight or deferred reload: their target is the file
        // being replaced, and the incoming scan result is not theirs.
        self.pending_reload = None;
        self.reload_when_idle = false;
        dispatch_scan(path, self.sender());
    }

    // ── File D&D (.ghx -> optimization setup modal, storage -> open) ────

    /// Accepts drag & drop of files. Works on every screen, including the
    /// startup guidance screen (drops are read from the raw input every frame).
    ///
    /// - A `.ghx` file opens the Grasshopper optimization setup modal
    ///   (if several files are dropped, the first `.ghx` wins).
    /// - Otherwise, the first recognized result storage file
    ///   (journal / SQLite / CSV) is routed to the normal open flow.
    /// - Anything else surfaces an error explaining the supported types
    ///   (in particular, binary `.gh` must be re-saved as `.ghx`) — a silent
    ///   no-op here would look like the drop simply didn't work.
    pub(super) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let paths: Vec<std::path::PathBuf> = dropped.into_iter().filter_map(|f| f.path).collect();
        if let Some(path) = paths.iter().find(|p| crate::io::file::is_ghx_path(p)) {
            self.open_ghx_path(path.clone());
            return;
        }
        if let Some(path) = paths.iter().find(|p| crate::io::file::is_storage_path(p)) {
            self.open_path(path.clone());
            return;
        }
        self.load_error = Some(unsupported_drop_message(&paths));
    }

    /// Loads a .ghx, and if problem extraction (synchronous, fast) succeeds, opens the
    /// optimization setup modal. Shared handling called both from D&D
    /// (`handle_dropped_files`) and the .ghx path in `open_path`.
    fn open_ghx_path(&mut self, path: std::path::PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => match tunny_core::gh::extract_problem(&text) {
                Ok(problem) => {
                    let mut dialog =
                        crate::state::app_state::GhOptDialogState::new(path, text, problem);
                    // Pre-fill the Compute connection / sampler settings with the
                    // persisted preferences from the last run.
                    self.app_state.gh_compute_prefs.apply_to(&mut dialog);
                    self.app_state.gh_opt_dialog = Some(dialog);
                }
                Err(e) => self.load_error = Some(e),
            },
            Err(e) => self.load_error = Some(format!("{}: {e}", path.display())),
        }
    }

    /// Loads a process-integration definition (JSON) and, on success, opens the
    /// tool optimization setup modal. Parse / read errors surface via `load_error`.
    pub(super) fn open_process_definition(&mut self, path: std::path::PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => match tunny_core::process::ProcessDefinition::from_json(&text) {
                Ok(def) => {
                    let dialog = crate::state::app_state::ProcessOptDialogState::new(def, &path);
                    self.app_state.process_opt_dialog = Some(dialog);
                }
                Err(e) => self.load_error = Some(format!("{}: {e}", path.display())),
            },
            Err(e) => self.load_error = Some(format!("{}: {e}", path.display())),
        }
    }

    /// Validates the builder's form and, if valid, writes it to a user-chosen JSON
    /// file. Success/failure is reported back into the builder's status/error.
    pub(super) fn save_process_definition(
        &mut self,
        builder: &mut crate::state::app_state::ProcessDefBuilderState,
    ) {
        let def = builder.to_definition();
        if let Err(e) = def.validate() {
            builder.error = Some(e);
            builder.status = None;
            return;
        }
        let json = match def.to_json() {
            Ok(json) => json,
            Err(e) => {
                builder.error = Some(e);
                builder.status = None;
                return;
            }
        };
        let mut dialog =
            rfd::FileDialog::new().add_filter("Process definition (*.json)", &["json"]);
        if let Some(dir) = builder.source_path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(name) = builder.source_path.as_ref().and_then(|p| p.file_name()) {
            dialog = dialog.set_file_name(name.to_string_lossy());
        } else {
            dialog = dialog.set_file_name("tool_definition.json");
        }
        let Some(path) = dialog.save_file() else {
            return; // Cancelled; leave the form as-is.
        };
        match std::fs::write(&path, json) {
            Ok(()) => {
                builder.status = Some(format!("Saved to {}", path.display()));
                builder.error = None;
                builder.source_path = Some(path);
            }
            Err(e) => {
                builder.error = Some(format!("{}: {e}", path.display()));
                builder.status = None;
            }
        }
    }

    /// Prompts for a definition JSON and, on success, replaces the builder's form
    /// with the loaded definition. Read/parse errors are shown in the builder.
    pub(super) fn load_into_process_builder(
        &mut self,
        builder: &mut crate::state::app_state::ProcessDefBuilderState,
    ) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Process definition (*.json)", &["json"])
            .pick_file()
        else {
            return; // Cancelled.
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => match tunny_core::process::ProcessDefinition::from_json(&text) {
                Ok(def) => {
                    *builder = crate::state::app_state::ProcessDefBuilderState::from_definition(
                        &def,
                        Some(path.clone()),
                    );
                    builder.status = Some(format!("Loaded {}", path.display()));
                }
                Err(e) => {
                    builder.error = Some(format!("{}: {e}", path.display()));
                    builder.status = None;
                }
            },
            Err(e) => {
                builder.error = Some(format!("{}: {e}", path.display()));
                builder.status = None;
            }
        }
    }
}
