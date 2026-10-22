use super::*;

impl TunnyApp {
    /// Upper bound in seconds to wait for rhino.compute to start locally. The first
    /// startup can take tens of seconds while Rhino loads, so give it some margin.
    const COMPUTE_STARTUP_TIMEOUT_SECS: u64 = 180;

    /// On Run confirmation, prepares the Compute run, journal, and progress
    /// handle, and starts the execution crate's Compute lifecycle on a background thread.
    /// If `prepare_compute_run` fails, the error is put in
    /// `dialog.error` and sent back to `gh_opt_dialog`, keeping the modal open.
    ///
    /// The Compute target is either a URL (an existing server) or a rhino.compute EXE
    /// path. For the EXE case, starting the process takes time, so process launch and
    /// waiting also happen on the background task side (the progress overlay shows
    /// "Starting…").
    pub(super) fn start_ghx_run(&mut self, mut dialog: crate::state::app_state::GhOptDialogState) {
        // Remember the confirmed settings as the defaults for the next dialog
        // (persisted across sessions via eframe storage).
        self.app_state.gh_compute_prefs = crate::state::app_state::GhComputePrefs::capture(&dialog);

        use tdashboard_analysis::surrogate_opt::FitProgress;
        use tdashboard_io::journal::parser::OptimizationDirection;
        use tdashboard_optimization_runner::gh::{
            classify_compute_input, prepare_compute_run, ComputeLaunchConfig, ComputeTarget,
            GhRunConfig,
        };

        let directions: Vec<OptimizationDirection> = dialog
            .maximize
            .iter()
            .map(|&is_max| {
                if is_max {
                    OptimizationDirection::Maximize
                } else {
                    OptimizationDirection::Minimize
                }
            })
            .collect();
        let run_cfg = GhRunConfig {
            study_name: dialog.study_name.clone(),
            load_if_exists: dialog.load_if_exists,
            directions,
            sampler: dialog.sampler.to_core(),
            n_trials: dialog.n_trials,
            population_size: dialog.population_size,
            generations: dialog.generations,
            cma_generations: dialog.cma_generations,
            adaptive_initial: dialog.adaptive_initial,
            adaptive_batch: dialog.adaptive_batch,
            adaptive_iterations: dialog.adaptive_iterations,
            // Patience 0 disables early stopping on the core side.
            adaptive_patience: if dialog.adaptive_early_stop {
                dialog.adaptive_patience.max(1)
            } else {
                0
            },
            adaptive_min_improvement: dialog.adaptive_min_improvement_pct / 100.0,
            seed: dialog.seed,
        };
        // In EXE mode the path comes from the dedicated field; in URL mode a
        // pasted EXE path is still tolerated via classification.
        let target = if dialog.compute_use_exe {
            ComputeTarget::Exe(std::path::PathBuf::from(dialog.compute_exe_path.trim()))
        } else {
            classify_compute_input(&dialog.compute_url)
        };
        let compute_port = dialog.compute_port;
        let api_key = if dialog.api_key.trim().is_empty() {
            None
        } else {
            Some(dialog.api_key.clone())
        };
        let max_parallel = dialog.max_parallel;

        let journal_path = std::path::PathBuf::from(&dialog.journal_path);
        let prep =
            match prepare_compute_run(&journal_path, &dialog.ghx_text, &dialog.problem, &run_cfg) {
                Ok(prep) => prep,
                Err(e) => {
                    dialog.error = Some(e);
                    self.app_state.gh_opt_dialog = Some(dialog);
                    return;
                }
            };

        let progress = FitProgress::new();
        self.app_state.gh_opt_run = Some(crate::state::app_state::GhOptRunState {
            progress: progress.clone(),
            journal_path: journal_path.clone(),
            study_name: dialog.study_name.clone(),
            finished: None,
        });
        let problem = dialog.problem.clone();
        spawn_task(self.sender(), move || {
            let result = prep.run(
                &problem,
                &run_cfg,
                ComputeLaunchConfig {
                    target,
                    port: compute_port,
                    startup_timeout_secs: Self::COMPUTE_STARTUP_TIMEOUT_SECS,
                    api_key,
                    max_parallel,
                },
                &progress,
            );
            AppMessage::GhOptFinished { result }
        });

        // The study is already written to the journal, so opening it shows it in the
        // study list (if there's only one, poll_messages auto-selects it). Trials
        // completed after this point reach the view when the user hits Reload, or
        // automatically once the run finishes (`refresh_after_gh_opt`).
        self.open_path(journal_path);
        // Drop dialog to close the modal (don't put it back as None).
    }

    /// On Run confirmation, assembles the runner problem + `ProcessEvaluator` and
    /// starts the optimization loop (`run_prepared`) on a background thread. The
    /// Dashboard drives the sampling; the external command evaluates each trial.
    /// Reuses `gh_opt_run` for the progress overlay / live update (the run overlay
    /// is not Grasshopper-specific). Setup failures are put in `dialog.error` and
    /// the modal is reopened.
    pub(super) fn start_process_run(
        &mut self,
        mut dialog: crate::state::app_state::ProcessOptDialogState,
    ) {
        use tdashboard_analysis::surrogate_opt::FitProgress;
        use tdashboard_io::journal::parser::OptimizationDirection;
        use tdashboard_optimization_runner::process::{ProcessEvaluator, VarRange};
        use tdashboard_optimization_runner::runner::{
            prepare_run, run_prepared, RunConfig, Sampler,
        };

        // Build the search-range map from the edited rows, aligned by name.
        let ranges: std::collections::HashMap<String, VarRange> = dialog
            .ranges
            .iter()
            .map(|r| {
                (
                    r.name.clone(),
                    VarRange {
                        low: r.low,
                        high: r.high,
                        digits: r.digits,
                        is_integer: r.is_integer,
                    },
                )
            })
            .collect();

        // build_problem keeps the variable order / objective / constraint counts
        // aligned with the definition, so the evaluator never sees a mismatch.
        let problem = match dialog.def.build_problem(&ranges) {
            Ok(p) => p,
            Err(e) => {
                dialog.error = Some(e);
                self.app_state.process_opt_dialog = Some(dialog);
                return;
            }
        };

        let directions: Vec<OptimizationDirection> = dialog
            .maximize
            .iter()
            .map(|&is_max| {
                if is_max {
                    OptimizationDirection::Maximize
                } else {
                    OptimizationDirection::Minimize
                }
            })
            .collect();
        let cfg = RunConfig {
            study_name: dialog.study_name.clone(),
            load_if_exists: dialog.load_if_exists,
            directions,
            sampler: if dialog.sampler_is_random {
                Sampler::Random
            } else {
                Sampler::Nsga2
            },
            n_trials: dialog.n_trials,
            population_size: dialog.population_size,
            generations: dialog.generations,
            seed: dialog.seed,
        };

        let evaluator = match ProcessEvaluator::new(dialog.def.clone()) {
            Ok(e) => e,
            Err(e) => {
                dialog.error = Some(e);
                self.app_state.process_opt_dialog = Some(dialog);
                return;
            }
        };

        let journal_path = std::path::PathBuf::from(&dialog.journal_path);
        let prep = match prepare_run(&journal_path, &problem, &cfg) {
            Ok(prep) => prep,
            Err(e) => {
                dialog.error = Some(e);
                self.app_state.process_opt_dialog = Some(dialog);
                return;
            }
        };

        let progress = FitProgress::new();
        self.app_state.gh_opt_run = Some(crate::state::app_state::GhOptRunState {
            progress: progress.clone(),
            journal_path: journal_path.clone(),
            study_name: dialog.study_name.clone(),
            finished: None,
        });
        spawn_task(self.sender(), move || {
            let result = run_prepared(&prep, &problem, &evaluator, &cfg, &progress);
            AppMessage::ProcessOptFinished { result }
        });

        self.open_path(journal_path);
        // Drop dialog to close the modal.
    }

    /// Reloads the displayed study when an optimization run finishes, so the
    /// trials it produced land in the view without the user having to press
    /// Reload themselves. This runs the same re-read as the toolbar Reload — a
    /// full re-parse of the journal is authoritative, so the final state shown
    /// always matches the file. Skipped if the user has meanwhile opened a
    /// different file.
    pub(super) fn refresh_after_gh_opt(&mut self) {
        let Some(run) = self.app_state.gh_opt_run.as_ref() else {
            return;
        };
        if self.app_state.journal_path.as_deref() != Some(run.journal_path.as_path()) {
            return;
        }
        // A short run can finish before its journal has even finished loading.
        // Reloading on top of that load would interleave two scans, so defer
        // until the app is idle instead of dropping the refresh — the user was
        // told the view updates by itself once the run ends.
        if self.is_loading {
            self.reload_when_idle = true;
            return;
        }
        self.reload_current();
    }
}
