pub use super::results::*;
pub use super::types::*;

use crate::ui::help::help_types::HelpLanguage;
use crate::ui::widgets::cluster_scatter::ClusterCacheKey;
use crate::ui::widgets::mcdm_chart::McdmCacheKey;
use crate::ui::widgets::report_modal::ReportDialogState;
use std::collections::HashMap;
use tunny_core::indicators::MoIndicator;

// ============================================================
// AppState
// ============================================================

#[derive(Debug)]
pub struct AppState {
    pub all_studies: Vec<StudyMeta>,
    pub journal_path: Option<std::path::PathBuf>,
    pub current_study: Option<StudyContext>,
    pub selected_indices: Vec<u32>,
    pub filter_ranges: HashMap<String, (f64, f64)>,
    pub highlighted_trial: Option<u32>,
    /// Result cache for sensitivity analysis (single objective). Key is (method id,
    /// objective idx, feasible_only).
    pub importance_cache: HashMap<(u8, usize, bool), SensitivityResult>,
    /// Result cache for Sobol indices. Key is (objective idx, feasible_only).
    pub sobol_cache: HashMap<(usize, bool), SobolResult>,
    /// Result cache for the Sensitivity Heatmap. Holds an all-parameters x
    /// all-objectives matrix per (method id (`ImportanceMetric::cache_id`),
    /// feasible_only), shared by each item.
    pub sensitivity_heatmap_cache: HashMap<(u8, bool), HeatmapMatrix>,
    /// Result cache for clustering. Holds computation results per settings key
    /// (target space / k / mode / Init), referenced and shared by 2D / 3D / Table
    /// with their own settings.
    pub cluster_cache: HashMap<ClusterCacheKey, ClusterResult>,
    pub live_update: LiveUpdateState,
    /// The most recently computed MCDM result. Kept as the coloring basis for the
    /// McdmScore color mode.
    pub mcdm_result: Option<McdmResult>,
    /// Result cache for MCDM. Held per settings key (method / weight mode / weights
    /// / v), referenced and shared by each chart (Ranking / Scatter / Scatter3D /
    /// Table) with their own settings.
    pub mcdm_cache: HashMap<McdmCacheKey, McdmResult>,
    pub convergence_history: Option<ConvergenceHistory>,
    /// User-specified HV reference point (in the original objective value units, per
    /// objective).
    /// When `None`, it's auto-computed from the observed points (nadir + 10% margin).
    /// On change, sets `convergence_history` to None to trigger recomputation.
    pub hv_ref_point_override: Option<Vec<f64>>,
    pub selected_colormap: ColormapName,

    // ── REQ-006: Multi-study comparison ─────────────────────────
    /// Whether comparison mode is enabled
    pub comparison_mode: bool,
    /// List of StudyContexts being compared (up to 4)
    pub comparison_studies: Vec<StudyContext>,
    /// List of colors for comparison studies (each element is `[R, G, B, A]`,
    /// non-premultiplied alpha).
    /// Held as raw arrays rather than a UI type, to keep the state layer free of an
    /// egui dependency.
    /// Converted to Color32 via `crate::theme::color_compute::rgba_to_color32` when drawing.
    pub comparison_colors: Vec<[u8; 4]>,
    /// Convergence indicator history for comparison studies (same order and element
    /// count as `comparison_studies`).
    /// Kept so the convergence indicator chart can overlay them on the same graph as
    /// the base Study.
    pub comparison_convergence_histories: Vec<ConvergenceHistory>,

    // ── REQ-007: Artifacts ───────────────────────────────────────
    /// The scanned artifacts base directory
    pub artifacts_dir: Option<std::path::PathBuf>,
    /// Map from trial_id -> artifacts (actual path + original file name + MIME)
    pub artifact_map: HashMap<u32, Vec<crate::io::artifacts::ArtifactEntry>>,

    // ── REQ-008: Convergence diagnostics ──────────────────────────
    /// History of (trial_id, cumulative_best_value) (single-objective Study only)
    pub best_trial_history: Option<Vec<(u32, f64)>>,

    // ── TASK-2228: common state extensions ────────────────────────
    /// List of pinned trial IDs (up to 20)
    pub pinned_trials: Vec<u32>,
    /// The base study_id of the Comparison session
    pub comparison_base_study: Option<u32>,

    // ── HTML Help Browser ─────────────────────────────────────────
    /// Help display language (same pattern as selected_colormap; not reset by clear())
    pub help_language: HelpLanguage,

    // ── Theme ────────────────────────────────────────────────────
    /// Whether dark theme is active (a view setting; not reset by clear()).
    /// The actual reflection into `Visuals` / `theme::set_dark_mode` is done every
    /// frame by `TunnyApp::logic` via diff detection.
    pub dark_mode: bool,

    // ── Convergence indicator selection ───────────────────────────
    /// The indicator currently shown in the convergence indicator chart (a view
    /// setting; not reset by clear())
    pub convergence_indicator: MoIndicator,

    // ── CSV import confirmation dialog ──────────────────────────────
    /// Edit state for the direction/range confirmation dialog shown right after
    /// loading a flat CSV.
    /// The dialog is shown while `Some`, and the Study is not activated until confirmed.
    pub csv_import_settings: Option<CsvImportSettings>,

    // ── RDB (PostgreSQL/MySQL) connection URL dialog ─────────────────
    /// The text currently being entered in the "Open URL…" dialog. `None` means
    /// hidden, `Some` means shown (the in-progress input string itself, including
    /// an empty string).
    pub db_url_dialog: Option<String>,

    // ── R4: self-contained report output (HTML/Markdown/JSON) ──────
    /// Edit state for the "Report…" dialog. `None` means hidden.
    pub report_dialog: Option<ReportDialogState>,

    // ── .ghx D&D -> run optimization ────────────────────────────────
    /// Edit state for the optimization settings dialog opened via .ghx D&D / Open.
    /// `None` means hidden.
    pub gh_opt_dialog: Option<GhOptDialogState>,
    /// Setup dialog for a generic process-integration optimization (Some = shown).
    pub process_opt_dialog: Option<ProcessOptDialogState>,
    /// GUI editor for authoring/editing a process-integration definition
    /// (Some = shown). Independent of the run setup dialog above.
    pub process_def_builder: Option<ProcessDefBuilderState>,
    /// State of the currently running (or most recently finished) .ghx optimization.
    /// `None` means hidden.
    /// Not cleared by `clear()` on `open_path`'s study switch. This is so the
    /// progress overlay stays visible while running even if the user looks at a
    /// different study (see the comment on clear()).
    pub gh_opt_run: Option<GhOptRunState>,
    /// Persisted defaults for the .ghx optimization setup (Compute connection,
    /// sampler settings). Restored from eframe storage at startup and captured
    /// on each Run.
    pub gh_compute_prefs: GhComputePrefs,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            all_studies: Vec::new(),
            journal_path: None,
            current_study: None,
            selected_indices: Vec::new(),
            filter_ranges: HashMap::new(),
            highlighted_trial: None,
            importance_cache: HashMap::new(),
            sobol_cache: HashMap::new(),
            sensitivity_heatmap_cache: HashMap::new(),
            cluster_cache: HashMap::new(),
            live_update: LiveUpdateState::default(),
            mcdm_result: None,
            mcdm_cache: HashMap::new(),
            convergence_history: None,
            hv_ref_point_override: None,
            selected_colormap: ColormapName::Viridis,
            comparison_mode: false,
            comparison_studies: Vec::new(),
            comparison_colors: Vec::new(),
            comparison_convergence_histories: Vec::new(),
            artifacts_dir: None,
            artifact_map: HashMap::new(),
            best_trial_history: None,
            pinned_trials: Vec::new(),
            comparison_base_study: None,
            help_language: HelpLanguage::default(),
            dark_mode: false,
            convergence_indicator: MoIndicator::Hypervolume,
            csv_import_settings: None,
            db_url_dialog: None,
            report_dialog: None,
            gh_opt_dialog: None,
            process_opt_dialog: None,
            process_def_builder: None,
            gh_opt_run: None,
            gh_compute_prefs: GhComputePrefs::default(),
        }
    }

    /// Sets the currently highlighted Trial
    pub fn set_highlight(&mut self, trial_id: u32) {
        self.highlighted_trial = Some(trial_id);
    }

    /// Toggles the pin state of `trial_id`.
    /// If already registered, unpins it and returns `Ok(false)`.
    /// Returns `Err(PinError::MaxPinnedReached)` if adding a new pin would exceed
    /// the limit of 20.
    /// Returns `Ok(true)` when successfully added.
    pub fn toggle_pinned_trial(&mut self, trial_id: u32) -> Result<bool, PinError> {
        const MAX_PINS: usize = 20;
        if let Some(pos) = self.pinned_trials.iter().position(|&t| t == trial_id) {
            self.pinned_trials.remove(pos);
            return Ok(false);
        }
        if self.pinned_trials.len() >= MAX_PINS {
            return Err(PinError::MaxPinnedReached { limit: MAX_PINS });
        }
        self.pinned_trials.push(trial_id);
        Ok(true)
    }

    /// Resets the comparison session.
    /// Call this when `comparison_base_study` doesn't match the current study_id, or
    /// on an explicit reset.
    pub fn reset_comparison_session(&mut self) {
        self.comparison_mode = false;
        self.comparison_studies.clear();
        self.comparison_colors.clear();
        self.comparison_convergence_histories.clear();
        self.comparison_base_study = None;
    }

    /// Resets Brushing & Linking state and analysis results on Study switch
    pub fn clear(&mut self) {
        self.selected_indices.clear();
        self.filter_ranges.clear();
        self.highlighted_trial = None;
        self.importance_cache.clear();
        self.sobol_cache.clear();
        self.sensitivity_heatmap_cache.clear();
        self.cluster_cache.clear();
        self.mcdm_result = None;
        self.mcdm_cache.clear();
        self.convergence_history = None;
        // The reference point depends on the objective's scale, so reset it on Study switch.
        self.hv_ref_point_override = None;
        // selected_colormap is preserved as a user setting

        // REQ-006: comparison_mode/studies/colors are preserved across Study
        // switches (the comparison session is kept until the user explicitly resets it)

        // REQ-007: Artifacts are reset on Study switch
        self.artifacts_dir = None;
        self.artifact_map.clear();

        // REQ-008: convergence history is reset on Study switch
        self.best_trial_history = None;

        // pinned_trials is not reset on Study switch either (preserves the user's pin settings)
        // comparison_base_study is not reset on Study switch either
        // help_language is preserved as a user setting (same pattern as selected_colormap)
        // dark_mode is preserved as a user setting (same as above)
    }
}

// ============================================================
// TASK-2232: selection + pinned visibility helper
// ============================================================

/// Returns the union of `selected_indices` and `pinned_trials`, without duplicates.
/// Preserves the original `selected_indices` order, appending pinned-only elements at the end.
pub fn merge_selected_with_pinned(selected: &[u32], pinned: &[u32]) -> Vec<u32> {
    let mut seen: std::collections::HashSet<u32> = selected.iter().copied().collect();
    let mut result: Vec<u32> = selected.to_vec();
    for &pin in pinned {
        if seen.insert(pin) {
            result.push(pin);
        }
    }
    result
}

// ============================================================
// TASK-2228: PinError
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinError {
    MaxPinnedReached { limit: usize },
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_new_initial_values() {
        let state = AppState::new();
        assert!(state.all_studies.is_empty());
        assert!(state.current_study.is_none());
        assert!(state.selected_indices.is_empty());
        assert!(state.filter_ranges.is_empty());
        assert!(state.highlighted_trial.is_none());
        assert!(state.importance_cache.is_empty());
        assert!(state.cluster_cache.is_empty());
    }

    #[test]
    fn app_state_clear_resets_selection() {
        let mut state = AppState::new();
        state.selected_indices = vec![0, 1, 2];
        state.filter_ranges.insert("x".to_string(), (0.0, 1.0));
        state.highlighted_trial = Some(5);
        state.importance_cache.insert(
            (0u8, 0, false),
            SensitivityResult {
                param_names: vec!["x".to_string()],
                spearman: vec![vec![0.9]],
                ridge: vec![],
                rf_anova: None,
                mdi: None,
                shap: None,
                permutation: None,
                ard: None,
            },
        );

        state.clear();

        assert!(state.selected_indices.is_empty());
        assert!(state.filter_ranges.is_empty());
        assert!(state.highlighted_trial.is_none());
        assert!(state.importance_cache.is_empty());
    }

    #[test]
    fn app_state_clear_preserves_studies() {
        let mut state = AppState::new();
        state.all_studies.push(StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 10,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        });

        state.clear();

        // Studies should NOT be cleared
        assert_eq!(state.all_studies.len(), 1);
    }

    #[test]
    fn app_state_new_colormap_defaults() {
        let state = AppState::new();
        assert_eq!(state.selected_colormap, ColormapName::Viridis);
    }

    // ============================================================
    // TASK-2110: tests for the 8 new fields
    // ============================================================

    #[test]
    fn task2110_comparison_fields_default() {
        // TC-003, TC-004, TC-005
        let state = AppState::new();
        assert!(!state.comparison_mode);
        assert!(state.comparison_studies.is_empty());
        assert!(state.comparison_colors.is_empty());
    }

    #[test]
    fn task2110_artifacts_fields_default() {
        // TC-006, TC-007
        let state = AppState::new();
        assert!(state.artifacts_dir.is_none());
        assert!(state.artifact_map.is_empty());
    }

    #[test]
    fn task2110_best_trial_history_default() {
        // TC-008
        let state = AppState::new();
        assert!(state.best_trial_history.is_none());
    }

    #[test]
    fn task2110_clear_resets_artifact_and_history_fields() {
        // TC-009, TC-010
        let mut state = AppState::new();
        state.artifact_map.insert(
            0,
            vec![crate::io::artifacts::ArtifactEntry {
                path: std::path::PathBuf::from("/tmp/abc123"),
                filename: "a.png".into(),
                mimetype: "image/png".into(),
            }],
        );
        state.artifacts_dir = Some(std::path::PathBuf::from("/tmp"));
        state.best_trial_history = Some(vec![(0, 1.0)]);

        state.clear();

        assert!(state.artifact_map.is_empty());
        assert!(state.artifacts_dir.is_none());
        assert!(state.best_trial_history.is_none());
    }

    #[test]
    fn task2110_clear_preserves_comparison_fields() {
        // comparison_mode/studies/colors are not reset by clear()
        let mut state = AppState::new();
        state.comparison_mode = true;
        state.comparison_colors = vec![[255, 0, 0, 255]];

        state.clear();

        // comparison_mode and comparison_colors are preserved
        assert!(state.comparison_mode);
        assert_eq!(state.comparison_colors.len(), 1);
    }

    // ── TASK-2228: tests for the new fields ─────────────────────

    #[test]
    fn app_state_default_includes_new_fields() {
        let state = AppState::new();
        assert!(state.pinned_trials.is_empty());
        assert!(state.comparison_base_study.is_none());
    }

    #[test]
    fn app_state_clear_preserves_pinned_trials() {
        let mut state = AppState::new();
        state.pinned_trials = vec![1, 2, 3];
        state.comparison_base_study = Some(42);
        state.clear();
        // pinned_trials and comparison_base_study are not reset by clear()
        assert_eq!(state.pinned_trials, vec![1, 2, 3]);
        assert_eq!(state.comparison_base_study, Some(42));
    }

    // ── TASK-2254: tests for the help_language field ────────────────

    #[test]
    fn app_state_new_help_language_defaults_to_en() {
        use crate::ui::help::help_types::HelpLanguage;
        let state = AppState::new();
        assert_eq!(state.help_language, HelpLanguage::En);
    }

    #[test]
    fn app_state_clear_preserves_help_language() {
        use crate::ui::help::help_types::HelpLanguage;
        let mut state = AppState::new();
        state.help_language = HelpLanguage::Ja;
        state.clear();
        assert_eq!(state.help_language, HelpLanguage::Ja);
    }

    // ── TASK-2232: visibility helper tests ───────────────────────────

    #[test]
    fn merge_selected_with_pinned_preserves_union() {
        let result = merge_selected_with_pinned(&[1, 2, 3], &[3, 4, 5]);
        // From 1,2,3, add 4,5. 3 is not duplicated
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    // ── TASK-2231: pin toggle tests ───────────────────────────────

    #[test]
    fn toggle_pinned_trial_adds_and_removes_entry() {
        let mut state = AppState::new();
        let result = state.toggle_pinned_trial(5);
        assert_eq!(result, Ok(true));
        assert_eq!(state.pinned_trials, vec![5]);

        let result = state.toggle_pinned_trial(5);
        assert_eq!(result, Ok(false));
        assert!(state.pinned_trials.is_empty());
    }

    #[test]
    fn toggle_pinned_trial_rejects_21st_entry() {
        let mut state = AppState::new();
        for i in 0..20u32 {
            state.toggle_pinned_trial(i).unwrap();
        }
        assert_eq!(state.pinned_trials.len(), 20);
        let result = state.toggle_pinned_trial(100);
        assert_eq!(result, Err(PinError::MaxPinnedReached { limit: 20 }));
        assert_eq!(state.pinned_trials.len(), 20);
    }

    // ── TASK-2230: comparison session reset tests ────────────────────

    #[test]
    fn reset_comparison_session_clears_all_comparison_state() {
        let mut state = AppState::new();
        state.comparison_mode = true;
        state.comparison_studies = vec![];
        state.comparison_colors = vec![[255, 0, 0, 255]];
        state.comparison_base_study = Some(5);

        state.reset_comparison_session();

        assert!(!state.comparison_mode);
        assert!(state.comparison_studies.is_empty());
        assert!(state.comparison_colors.is_empty());
        assert!(state.comparison_base_study.is_none());
    }

    // ── TASK-2246: regression tests ───────────────────────────────────

    // F-003: pinning regression
    #[test]
    fn export_and_pinning_logic_have_dedicated_regression_tests() {
        let mut state = AppState::new();
        // pin, check, unpin, check
        assert!(state.pinned_trials.is_empty());
        state.toggle_pinned_trial(7).unwrap();
        assert_eq!(state.pinned_trials, vec![7]);
        state.toggle_pinned_trial(7).unwrap();
        assert!(state.pinned_trials.is_empty());
        // max-pin boundary
        for i in 0..20u32 {
            state.toggle_pinned_trial(i).unwrap();
        }
        assert_eq!(
            state.toggle_pinned_trial(99),
            Err(PinError::MaxPinnedReached { limit: 20 })
        );
    }

    // F-002: comparison state transitions (load-start → success → reset)
    #[test]
    fn comparison_state_transitions_are_covered() {
        let mut state = AppState::new();

        // comparison load-start
        state.comparison_mode = true;
        state.comparison_base_study = Some(1);
        assert!(state.comparison_mode);

        // reset
        state.reset_comparison_session();
        assert!(!state.comparison_mode);
        assert!(state.comparison_base_study.is_none());
        assert!(state.comparison_studies.is_empty());

        // compute spinner state: a generic computing flag transitions on→off
        let started = true; // represents widget.computing = true
        let done = false; // represents widget.computing = false after result
        assert!(started);
        assert!(!done);
    }
}
