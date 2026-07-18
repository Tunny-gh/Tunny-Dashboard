use crate::state::app_state::{
    ClusterResult, HeatmapMatrix, McdmResult, SensitivityResult, SobolResult, StudyContext,
    StudyMeta,
};
use crate::state::results::{ConvergenceHistory, EntropyResult};
use crate::ui::widgets::cluster_scatter::ClusterCacheKey;
use crate::ui::widgets::mcdm_chart::McdmCacheKey;

/// The chart that started clustering. Results are shared via the config key,
/// but execution state (spinner / error) must be reflected on the originating
/// widget, so completion/failure messages carry which chart triggered the
/// computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterChartSource {
    Scatter2D,
    Scatter3D,
    Table,
    ArtifactGallery,
}

/// The chart that started the MCDM computation. Like clustering, this is
/// carried so execution state can be reflected on the originating widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McdmChartSource {
    Rank,
    Scatter2D,
    Scatter3D,
    Table,
    ArtifactGallery,
}

// ============================================================
// PDP Result types (placeholder for TASK-2025)
// ============================================================

#[derive(Debug, Clone)]
pub struct PdpResult1d {
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
    pub y_upper: Option<Vec<f64>>,
    pub y_lower: Option<Vec<f64>>,
    pub ice_lines: Vec<Vec<f64>>,
    pub r2: Option<f64>,
    pub param_name: String,
}

#[derive(Debug, Clone)]
pub struct PdpResult2d {
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
    pub z_values: Vec<Vec<f64>>,
    pub param1_name: String,
    pub param2_name: String,
    pub objective_name: String,
    /// Posterior variance grid (GP methods only).
    pub uncertainties: Option<Vec<Vec<f64>>>,
}

#[derive(Debug, Clone)]
pub struct ClusterUiError {
    pub user_message: String,
    pub detail_for_dev: Option<String>,
    pub retryable: bool,
}

pub fn cluster_ui_error(
    user_message: impl Into<String>,
    detail: Option<String>,
    retryable: bool,
) -> ClusterUiError {
    ClusterUiError {
        user_message: user_message.into(),
        detail_for_dev: if cfg!(debug_assertions) { detail } else { None },
        retryable,
    }
}

// ============================================================
// Observed Contour related types
// ============================================================

/// Rendering result for Observed Contour. Holds the grid interpolated only
/// from observed trial points (`tunny_core::contour::ObservedSurface`), plus
/// the observed points used for overlay display.
#[derive(Debug, Clone)]
pub struct ObservedContourResult {
    pub x_name: String,
    pub y_name: String,
    pub value_name: String,
    pub surface: tunny_core::contour::ObservedSurface,
    /// Observed points for overlay display (feasible filter already applied). `[x, y, value]`.
    pub points: Vec<[f64; 3]>,
    /// trial_id in the same order as `points` (for point-click → detail view).
    pub point_trial_ids: Vec<u32>,
}

// ============================================================
// Surrogate Optimizer related types
// ============================================================

/// UI display result for acquisition function suggestions.
#[derive(Debug, Clone)]
pub struct SurrogateSuggestUiResult {
    /// Suggested candidates (acquisition function optimization results).
    pub candidates: Vec<tunny_core::surrogate_opt::SuggestedCandidate>,
    /// Parameter names (same order as `candidates[*].params`).
    pub param_names: Vec<String>,
    /// Objective name (for display).
    pub objective_name: String,
}

/// UI display result for EHVI-based multi-objective next-candidate suggestions.
#[derive(Debug, Clone)]
pub struct SurrogateMultiSuggestUiResult {
    /// Suggested candidates (EHVI optimization results).
    pub candidates: Vec<tunny_core::surrogate_opt::MultiSuggestedCandidate>,
    /// Parameter names (same order as `candidates[*].params`).
    pub param_names: Vec<String>,
    /// Objective names (same order as `candidates[*].predicted_values`).
    pub objective_names: Vec<String>,
}

/// UI display result for multi-objective surrogate optimization.
/// The computation runs in the background via `tunny_core::surrogate_opt`,
/// and parameter names, objective names, and directions are attached and
/// repacked here.
#[derive(Debug, Clone)]
pub struct SurrogateMultiOptUiResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    /// Predicted Pareto front (sorted ascending by the first objective).
    pub front: Vec<tunny_core::surrogate_opt::ParetoFrontPoint>,
    /// Coefficient of determination on training data, per objective.
    pub r_squared: Vec<f64>,
}

/// UI display result for surrogate optimization.
/// The computation runs in the background via `tunny_core::surrogate_opt`,
/// and the parameter name/value mapping and direction (minimize/maximize)
/// are attached and repacked here.
#[derive(Debug, Clone)]
pub struct SurrogateOptUiResult {
    /// Estimated optimum point (parameter name, value).
    pub best_params: Vec<(String, f64)>,
    /// Surrogate predicted value at the estimated optimum (original units).
    pub best_value: f64,
    /// Predicted standard deviation (Gaussian process methods only).
    pub predicted_std: Option<f64>,
    /// Coefficient of determination of the surrogate on training data.
    pub r_squared: f64,
    pub objective_name: String,
    /// true = optimized as a minimization problem.
    pub minimize: bool,
    /// Best value among observed data (original units). Minimum for minimization, maximum for maximization.
    pub best_observed_value: f64,
    /// Constraint surrogate predicted values at the estimated optimum (original units, same order as constraint names). Empty if there are no constraints.
    pub predicted_constraints: Vec<(String, f64)>,
    /// Feasibility probability (0.0 to 1.0). None if there are no constraints.
    pub feasibility_probability: Option<f64>,
}

// ============================================================
// Compare Surrogates related types
// ============================================================

/// Compare Surrogates: a comparison row of CV metrics for one model. If
/// fitting/validation fails, the reason is left in `error` and the other
/// numeric fields stay at the invalid value (0.0) and are not displayed on
/// the UI side.
#[derive(Debug, Clone)]
pub struct SurrogateCompareRow {
    pub kind: tunny_core::surrogate_opt::SurrogateModelKind,
    pub cv_r2_mean: f64,
    pub cv_r2_std: f64,
    pub holdout_r2: f64,
    pub holdout_rmse: f64,
    pub train_r2: f64,
    /// Error message when fitting/validation fails.
    pub error: Option<String>,
}

/// UI display result for the Compare Surrogates widget. Holds the CV metric
/// comparison from fitting all model kinds against the selected objective,
/// plus the overlay of 1D prediction slices anchored on the best observed
/// trial.
#[derive(Debug, Clone)]
pub struct SurrogateCompareUiResult {
    /// CV metric comparison rows per model (display order is sorted on the UI side).
    pub rows: Vec<SurrogateCompareRow>,
    /// 1D prediction slices for successfully fitted models (passing through the anchor point).
    pub slices: Vec<(
        tunny_core::surrogate_opt::SurrogateModelKind,
        tunny_core::surrogate_opt::LineSlice,
    )>,
    /// Observed data (x, y) for the parameter being sliced.
    pub observed: Vec<(f64, f64)>,
    /// Name of the parameter being sliced.
    pub param_name: String,
    pub objective_name: String,
    /// Anchor point (original units, in the parameter order used for training).
    pub anchor: Vec<f64>,
}

// ============================================================
// Live update poller startup preparation results (H-1 / H-2)
// ============================================================

/// The result of preparing, in the background without blocking the UI
/// thread, the initial state needed to start the live update poller. Holds a
/// fully prepared context per storage kind that can be passed directly to
/// `*LivePoller::start`.
///
/// - RDB fingerprint retrieval (DB connection + query)
/// - Full journal read + trial count
///
/// Both involve I/O and were previously run synchronously on the UI thread by
/// `restart_poller` (causing the window to freeze with slow DBs or large
/// journals — H-1 / H-2). This is now delivered asynchronously as
/// `AppMessage::PollerReady`.
pub enum PollerPrep {
    Journal(tunny_core::io::journal::live_update::LiveUpdateContext),
    Sqlite(crate::io::live_update_poller::SqliteLiveUpdateContext),
    Rdb(crate::io::live_update_poller::RdbLiveUpdateContext),
}

// ============================================================
// AppMessage
// ============================================================

pub enum AppMessage {
    JournalParsed {
        studies: Vec<StudyMeta>,
        path: std::path::PathBuf,
    },
    StudySelected {
        meta: StudyMeta,
        /// Shared store reference key. The UI side obtains Arc<DataFrame> via snapshot(study_id).
        study_id: u32,
        /// Pareto rank (in row index order, computed at the app layer). Goes into StudyView's parallel array.
        pareto_rank: Vec<u32>,
        pareto_indices: Vec<u32>,
    },
    /// Incremental (streaming) load on Study selection. Completed Trials are
    /// sent every 1000 rows, and the UI side appends and rebuilds the
    /// DataFrame per batch to update rendering (avoids freezing during load).
    /// The Pareto rank is computed and finalized only once, on the
    /// `is_final` batch.
    StudyChunkLoaded {
        study_id: u32,
        /// Cumulative StudyMeta up to this point.
        meta: StudyMeta,
        /// Trial rows newly completed in this batch (core representation).
        new_rows: Vec<tunny_core::dataframe::TrialRow>,
        /// Cumulative parameter column names (sorted).
        param_names: Vec<String>,
        /// Objective column names.
        objective_names: Vec<String>,
        /// Cumulative user_attr numeric column names.
        user_attr_numeric_names: Vec<String>,
        /// Cumulative user_attr string column names.
        user_attr_string_names: Vec<String>,
        /// Maximum number of observed constraints.
        max_constraints: usize,
        /// Whether this is the first batch (a new StudyContext is created).
        is_first: bool,
        /// Whether this is the final batch (Pareto finalized, loading ends).
        is_final: bool,
    },
    SensitivityDone {
        /// (metric cache_id, objective idx, feasible_only)
        key: (u8, usize, bool),
        result: SensitivityResult,
    },
    /// For Sensitivity Heatmap: sensitivity matrix of all parameters × all objectives for the selected method.
    SensitivityHeatmapDone {
        metric: crate::ui::widgets::importance_chart::ImportanceMetric,
        feasible_only: bool,
        result: HeatmapMatrix,
    },
    SobolDone {
        /// (objective idx, feasible_only)
        key: (usize, bool),
        result: SobolResult,
    },
    ClusteringDone {
        source: ClusterChartSource,
        key: ClusterCacheKey,
        result: ClusterResult,
    },
    ClusterFailed {
        source: ClusterChartSource,
        err: ClusterUiError,
    },
    McdmDone {
        source: McdmChartSource,
        key: McdmCacheKey,
        result: McdmResult,
    },
    McdmFailed {
        source: McdmChartSource,
        message: String,
    },
    EntropyDone {
        source: McdmChartSource,
        result: EntropyResult,
    },
    PdpDone {
        param: String,
        objective: String,
        model_type: String,
        feasible_only: bool,
        result: PdpResult1d,
    },
    Pdp2dDone(PdpResult2d),
    LiveUpdateDone {
        new_trial_rows: Vec<tunny_core::io::journal::live_update::TrialRow>,
        updated_study_counts: Vec<(u32, usize)>,
        /// Extras diff event to apply to the incidental info of all trials (all states).
        extras_events: tunny_core::io::journal::live_update::ExtrasDiff,
    },
    /// The poller detected consecutive errors (e.g., file access failures)
    LiveUpdateError(String),
    /// Detected a possible optimization completion: no file changes for 60 seconds
    LiveUpdateMaybeComplete,
    /// SQLite live update: detected a fingerprint change.
    /// Since SQLite updates trial state in place (RUNNING→COMPLETE, etc.), a
    /// byte-offset diff like the journal's isn't possible. This is a signal
    /// message indicating that a full reload of the target study needs to be
    /// requested from the worker thread.
    ///
    /// RDB (PostgreSQL/MySQL) live updates use the same fingerprint approach,
    /// so this message is reused as-is instead of adding a new message kind
    /// (`RdbLivePoller` also sends this).
    SqliteLiveChanged {
        study_id: u32,
    },
    /// SQLite live update: the reload of the target study has completed.
    /// Since the worker thread has already finished up through
    /// `tunny_core::dataframe::swap_snapshot` / `store_extras_for`, only the
    /// StudyView rebuild (including Pareto recomputation) and cache
    /// invalidation are done here.
    ///
    /// RDB live update reload completion (`dispatch_reload_rdb_study` →
    /// `crate::io::rdb::reload_single_study_task`) also reuses this message
    /// as-is.
    SqliteLiveReloadDone {
        study_id: u32,
        meta: StudyMeta,
    },
    /// The convergence indicator (HV / IGD+ / epsilon / R2) history
    /// computation has completed. All series for the base Study and
    /// comparison Studies are computed in one batch and normalized against a
    /// common reference set.
    IndicatorHistoryDone {
        indicator: tunny_core::indicators::MoIndicator,
        /// Indicator history for the base Study.
        base: ConvergenceHistory,
        /// Indicator history for comparison Studies (same order as comparison_studies).
        comparisons: Vec<ConvergenceHistory>,
    },
    Error(String),
    SensitivityError(String),
    /// M-4: caught a panic from a worker thread launched via `spawn_task`.
    /// The panic message is reflected in `load_error` to make it visible to
    /// the user (without catching it, the corresponding widget's
    /// computing/fitting indicator would stay stuck on).
    TaskPanicked(String),
    /// H-1 / H-2: startup preparation for the live update poller has
    /// completed in the background. `generation` is used to detect staleness
    /// if a toggle/Study change happens during preparation; if it doesn't
    /// match `TunnyApp::poller_generation` on receipt, it's discarded.
    /// Poller startup happens in `app.rs` (`poll_messages`), which holds
    /// tx/poller, so this message is not handled in `MessageHandler::handle`.
    PollerReady {
        generation: u64,
        prep: PollerPrep,
    },

    // ── TASK-2112: new variants ────────────────────────────────────
    /// REQ-006: comparison Study load completed
    ComparisonStudyLoaded {
        context: Box<StudyContext>,
    },
    /// REQ-007: Artifacts directory scan completed
    ArtifactsDirScanned {
        trial_artifacts: std::collections::HashMap<u32, Vec<crate::io::artifacts::ArtifactEntry>>,
        artifacts_dir: std::path::PathBuf,
    },
    ComparisonStudyLoadFailed(String),
    /// Observed Contour grid generation has completed (interpolation of observed points).
    ObservedContourDone(ObservedContourResult),
    ObservedContourFailed(String),
    /// Surrogate fitting + validation has completed (the optimization stage is a separate message).
    SurrogateFitDone(std::sync::Arc<tunny_core::surrogate_opt::TrainedSurrogate>),
    SurrogateFitFailed(String),
    /// Surrogate fitting was cancelled by user action.
    SurrogateFitCancelled,
    SurrogateOptDone(SurrogateOptUiResult),
    /// Multi-objective surrogate fitting + validation has completed (holds training results for all objectives).
    SurrogateMultiFitDone(std::sync::Arc<Vec<tunny_core::surrogate_opt::TrainedSurrogate>>),
    SurrogateMultiFitFailed(String),
    /// Multi-objective surrogate fitting was cancelled by user action.
    SurrogateMultiFitCancelled,
    SurrogateMultiOptDone(SurrogateMultiOptUiResult),
    SurrogateMultiOptFailed(String),
    /// Candidate suggestion by the acquisition function has completed.
    SurrogateSuggestDone(SurrogateSuggestUiResult),
    /// Candidate suggestion by the acquisition function failed.
    SurrogateSuggestFailed(String),
    /// EHVI-based multi-objective candidate suggestion has completed.
    SurrogateMultiSuggestDone(SurrogateMultiSuggestUiResult),
    /// EHVI-based multi-objective candidate suggestion failed.
    SurrogateMultiSuggestFailed(String),
    /// Surrogate fitting for robustness analysis has completed.
    RobustnessFitDone(std::sync::Arc<tunny_core::surrogate_opt::TrainedSurrogate>),
    /// Surrogate fitting for robustness analysis failed.
    RobustnessFitFailed(String),
    /// Surrogate fitting for the response surface 3D viewer has completed.
    ResponseSurfaceFitDone(std::sync::Arc<tunny_core::surrogate_opt::TrainedSurrogate>),
    /// Surrogate fitting for the response surface 3D viewer failed.
    ResponseSurfaceFitFailed(String),
    /// Compare Surrogates: fitting + comparison of all model kinds has
    /// completed (individual model fit failures are stored in
    /// `SurrogateCompareRow::error`; `SurrogateCompareFailed` is sent here
    /// only when all models fail).
    SurrogateCompareDone(std::sync::Arc<SurrogateCompareUiResult>),
    /// Compare Surrogates: fitting of all models failed.
    SurrogateCompareFailed(String),

    /// low perf: background write of a CSV export (chart / trial table / all
    /// trials) succeeded. There's no toast or other notification on success,
    /// so the UI side does nothing (unlike `ReportExportDone`, there's no
    /// information to display).
    CsvExportDone,
    /// low perf: background construction / writing of the CSV export failed.
    /// The cause is reflected in `load_error`, following the same policy as
    /// the existing save failure (`ToolbarAction::ExportCsv`).
    CsvExportFailed(String),

    /// R4: self-contained report export (HTML/Markdown/JSON) has completed
    /// in the background. The list of file paths actually written (multiple
    /// entries when multiple formats are selected).
    /// The existing `Error` is reused on failure.
    ReportExportDone {
        paths: Vec<std::path::PathBuf>,
        /// Non-primary sibling paths that overwrote existing files
        /// (the primary is not included since it's already confirmed via the
        /// OS save dialog).
        overwrote: Vec<std::path::PathBuf>,
    },

    /// .ghx background optimization (`tunny_core::gh::run_prepared`) has
    /// completed. `Ok` is a normal exit including cancellation, `Err` is a
    /// fatal error such as journal write failure (individual evaluation
    /// failures are already aggregated in `GhRunSummary.failed` and don't
    /// result in `Err`).
    GhOptFinished {
        result: Result<tunny_core::gh::GhRunSummary, String>,
    },
    /// A generic process-integration optimization (`runner::run_prepared` with a
    /// `ProcessEvaluator`) finished. Same contract as `GhOptFinished`: `Ok` is a
    /// normal exit (including cancellation), `Err` is a fatal error such as a
    /// journal write failure.
    ProcessOptFinished {
        result: Result<tunny_core::runner::RunSummary, String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_message_error_variant() {
        let msg = AppMessage::Error("test error".to_string());
        match msg {
            AppMessage::Error(e) => assert_eq!(e, "test error"),
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn pdp_result_one_dim() {
        let result = PdpResult1d {
            x_values: vec![0.0, 0.5, 1.0],
            y_values: vec![1.0, 0.5, 0.0],
            y_upper: None,
            y_lower: None,
            ice_lines: vec![],
            r2: None,
            param_name: "x".to_string(),
        };
        assert_eq!(result.x_values.len(), 3);
    }

    #[test]
    fn message_handler_accepts_new_message_family() {
        let msgs: Vec<AppMessage> = vec![AppMessage::ComparisonStudyLoadFailed("err".to_string())];
        // all variants should be matchable without panic
        for msg in msgs {
            if let AppMessage::ComparisonStudyLoadFailed(e) = msg {
                assert!(!e.is_empty())
            }
        }
    }
}
