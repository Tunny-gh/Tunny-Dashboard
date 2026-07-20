use std::path::PathBuf;

// ============================================================
// .ghx D&D -> optimization setup modal / background run state
// ============================================================

/// State of the optimization setup dialog opened via .ghx D&D.
///
/// While `AppState::gh_opt_dialog` is `Some`, `ghx_opt_modal` displays and edits this state.
/// The side that receives [`GhxOptAction::Run`](crate::ui::widgets::common::ghx_opt_modal::GhxOptAction)
/// (app.rs) assembles `GhRunConfig` / `ComputeConfig` from it.
#[derive(Debug, Clone)]
pub struct GhOptDialogState {
    /// Absolute path of the .ghx file selected via D&D / Open.
    pub ghx_path: PathBuf,
    /// The original full .ghx XML text (passed as-is to `build_compute_definition` on Run).
    pub ghx_text: String,
    /// Extraction result from `extract_problem` (variables, objectives, warnings).
    pub problem: tunny_core::gh::GhProblem,
    /// Per-objective maximize flag (same order and length as `problem.objectives`, default false = Minimize).
    pub maximize: Vec<bool>,
    /// Study name used to keep it unique within the journal (default: "<ghx stem>-<last 6 digits of unix seconds>").
    pub study_name: String,
    /// Output journal path (default: "<stem>_optuna.log" in the same directory as the ghx).
    pub journal_path: String,
    /// true = launch a local rhino.compute EXE (the Dashboard starts/stops the
    /// process; default), false = connect to an already-running server URL.
    /// EXE mode is the default because the app can manage the whole lifecycle.
    pub compute_use_exe: bool,
    /// Server URL used when `compute_use_exe` is false (default "http://localhost:6500").
    /// A pasted EXE path is still tolerated here (classified on run).
    pub compute_url: String,
    /// Path to the rhino.compute executable used when `compute_use_exe` is true.
    pub compute_exe_path: String,
    /// Port passed to rhino.compute in EXE mode (default 6500). Unused in URL mode.
    pub compute_port: u16,
    /// Rhino.Compute API key (treated as `ComputeConfig.api_key = None` if empty).
    pub api_key: String,
    /// Upper bound on concurrent requests (default 4, 1..=16).
    pub max_parallel: usize,
    /// Sampler selection (default NSGA-II).
    pub sampler: GhSamplerChoice,
    /// Number of trials for the Random sampler (default 50).
    pub n_trials: usize,
    /// NSGA-II population size (default 16).
    pub population_size: usize,
    /// Number of NSGA-II generations (default 10).
    pub generations: usize,
    /// Adaptive sampler: random bootstrap trials before the first fit (default 10,
    /// floored to the surrogate minimum on the core side).
    pub adaptive_initial: usize,
    /// Adaptive sampler: candidates evaluated per iteration (default 4).
    pub adaptive_batch: usize,
    /// Adaptive sampler: fit → suggest → evaluate iterations (default 10).
    pub adaptive_iterations: usize,
    /// Adaptive sampler: enable convergence-based early stopping (default false).
    pub adaptive_early_stop: bool,
    /// Adaptive sampler: consecutive low-improvement iterations before stopping
    /// (default 3). Only used when `adaptive_early_stop` is true.
    pub adaptive_patience: usize,
    /// Adaptive sampler: relative-improvement threshold in percent (default 1.0).
    /// Only used when `adaptive_early_stop` is true.
    pub adaptive_min_improvement_pct: f64,
    /// Random seed (default 42).
    pub seed: u64,
    /// Display text for when Run fails (errors from `build_compute_definition` / `prepare_gh_run`).
    pub error: Option<String>,
}

/// Sampler selection for a .ghx optimization run (maps onto
/// `tunny_core::gh::GhSampler`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GhSamplerChoice {
    Nsga2,
    Random,
    /// Adaptive surrogate loop (random bootstrap → fit → EI/EHVI suggest →
    /// evaluate → refit).
    Adaptive,
}

impl GhSamplerChoice {
    pub fn label(&self) -> &'static str {
        match self {
            GhSamplerChoice::Nsga2 => "NSGA-II",
            GhSamplerChoice::Random => "Random",
            GhSamplerChoice::Adaptive => "Adaptive (surrogate)",
        }
    }

    pub fn to_core(self) -> tunny_core::gh::GhSampler {
        match self {
            GhSamplerChoice::Nsga2 => tunny_core::gh::GhSampler::Nsga2,
            GhSamplerChoice::Random => tunny_core::gh::GhSampler::Random,
            GhSamplerChoice::Adaptive => tunny_core::gh::GhSampler::Adaptive,
        }
    }
}

/// User preferences for the .ghx optimization setup, persisted across app
/// sessions (eframe storage). Captured from the dialog when a run starts and
/// applied as defaults the next time the dialog opens, so the Compute
/// connection (EXE path, port, …) and sampler settings don't have to be
/// re-entered every session.
///
/// Per-file values (study name, journal path, directions) are intentionally
/// not persisted — they are derived from the dropped .ghx.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GhComputePrefs {
    pub compute_use_exe: bool,
    pub compute_exe_path: String,
    pub compute_url: String,
    pub compute_port: u16,
    pub api_key: String,
    pub max_parallel: usize,
    pub sampler: GhSamplerChoice,
    pub n_trials: usize,
    pub population_size: usize,
    pub generations: usize,
    pub adaptive_initial: usize,
    pub adaptive_batch: usize,
    pub adaptive_iterations: usize,
    pub adaptive_early_stop: bool,
    pub adaptive_patience: usize,
    pub adaptive_min_improvement_pct: f64,
    pub seed: u64,
}

impl Default for GhComputePrefs {
    /// Must stay in sync with the dialog defaults in [`GhOptDialogState::new`].
    fn default() -> Self {
        Self {
            compute_use_exe: true,
            compute_exe_path: String::new(),
            compute_url: "http://localhost:6500".to_string(),
            compute_port: 6500,
            api_key: String::new(),
            max_parallel: 4,
            sampler: GhSamplerChoice::Nsga2,
            n_trials: 50,
            population_size: 16,
            generations: 10,
            adaptive_initial: 10,
            adaptive_batch: 4,
            adaptive_iterations: 10,
            adaptive_early_stop: false,
            adaptive_patience: 3,
            adaptive_min_improvement_pct: 1.0,
            seed: 42,
        }
    }
}

impl GhComputePrefs {
    /// Applies the stored preferences onto a freshly created dialog state
    /// (called right after [`GhOptDialogState::new`]).
    pub fn apply_to(&self, dialog: &mut GhOptDialogState) {
        dialog.compute_use_exe = self.compute_use_exe;
        dialog.compute_exe_path = self.compute_exe_path.clone();
        dialog.compute_url = self.compute_url.clone();
        dialog.compute_port = self.compute_port;
        dialog.api_key = self.api_key.clone();
        dialog.max_parallel = self.max_parallel.clamp(1, 16);
        dialog.sampler = self.sampler;
        dialog.n_trials = self.n_trials.max(1);
        dialog.population_size = self.population_size.max(1);
        dialog.generations = self.generations.max(1);
        dialog.adaptive_initial = self.adaptive_initial.max(1);
        dialog.adaptive_batch = self.adaptive_batch.max(1);
        dialog.adaptive_iterations = self.adaptive_iterations.max(1);
        dialog.adaptive_early_stop = self.adaptive_early_stop;
        dialog.adaptive_patience = self.adaptive_patience.max(1);
        dialog.adaptive_min_improvement_pct = self.adaptive_min_improvement_pct.clamp(0.0, 100.0);
        dialog.seed = self.seed;
    }

    /// Captures the settings the user just confirmed with Run.
    pub fn capture(dialog: &GhOptDialogState) -> Self {
        Self {
            compute_use_exe: dialog.compute_use_exe,
            compute_exe_path: dialog.compute_exe_path.clone(),
            compute_url: dialog.compute_url.clone(),
            compute_port: dialog.compute_port,
            api_key: dialog.api_key.clone(),
            max_parallel: dialog.max_parallel,
            sampler: dialog.sampler,
            n_trials: dialog.n_trials,
            population_size: dialog.population_size,
            generations: dialog.generations,
            adaptive_initial: dialog.adaptive_initial,
            adaptive_batch: dialog.adaptive_batch,
            adaptive_iterations: dialog.adaptive_iterations,
            adaptive_early_stop: dialog.adaptive_early_stop,
            adaptive_patience: dialog.adaptive_patience,
            adaptive_min_improvement_pct: dialog.adaptive_min_improvement_pct,
            seed: dialog.seed,
        }
    }
}

impl GhOptDialogState {
    /// Builds the dialog state with defaults right after .ghx extraction.
    pub fn new(ghx_path: PathBuf, ghx_text: String, problem: tunny_core::gh::GhProblem) -> Self {
        let stem = ghx_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("gh_opt")
            .to_string();
        let secs_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 1_000_000)
            .unwrap_or(0);
        let study_name = format!("{stem}-{secs_suffix:06}");
        let journal_path = ghx_path
            .parent()
            .map(|dir| dir.join(format!("{stem}_optuna.log")))
            .unwrap_or_else(|| PathBuf::from(format!("{stem}_optuna.log")))
            .to_string_lossy()
            .into_owned();
        let maximize = vec![false; problem.objectives.len()];
        Self {
            ghx_path,
            ghx_text,
            problem,
            maximize,
            study_name,
            journal_path,
            compute_use_exe: true,
            compute_url: "http://localhost:6500".to_string(),
            compute_exe_path: String::new(),
            compute_port: 6500,
            api_key: String::new(),
            max_parallel: 4,
            sampler: GhSamplerChoice::Nsga2,
            adaptive_initial: 10,
            adaptive_batch: 4,
            adaptive_iterations: 10,
            adaptive_early_stop: false,
            adaptive_patience: 3,
            adaptive_min_improvement_pct: 1.0,
            n_trials: 50,
            population_size: 16,
            generations: 10,
            seed: 42,
            error: None,
        }
    }
}

/// State of a running .ghx optimization (used by the non-modal progress overlay).
///
/// While `AppState::gh_opt_run` is `Some`, the progress overlay (app.rs) is displayed.
/// `progress` is a handle shared with the background thread running `run_prepared`;
/// progress is read via `snapshot()` and cancellation requested via `request_cancel()`.
/// `#[derive(Debug)]` isn't possible because `FitProgress` doesn't implement `Debug`
/// (the same constraint as other existing progress-holding fields like `SurrogateOptState::fit_progress`).
/// Since `AppState` derives `Debug`, a manual implementation is provided here that
/// substitutes a placeholder for `progress`.
#[derive(Clone)]
pub struct GhOptRunState {
    pub progress: tunny_core::surrogate_opt::FitProgress,
    pub journal_path: PathBuf,
    pub study_name: String,
    /// `None` = running. `Some` = finished (success message or error string).
    pub finished: Option<Result<String, String>>,
}

impl std::fmt::Debug for GhOptRunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GhOptRunState")
            .field("progress", &"FitProgress { .. }")
            .field("journal_path", &self.journal_path)
            .field("study_name", &self.study_name)
            .field("finished", &self.finished)
            .finish()
    }
}
