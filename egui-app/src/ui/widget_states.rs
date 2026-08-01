use crate::ui::widgets::about_modal::AboutModalState;
use crate::ui::widgets::edf_plot::EdfComparison;
use crate::ui::widgets::license_modal::LicenseModalState;
use crate::ui::widgets::optimization_history::OptHistoryComparison;
use crate::ui::widgets::trial_detail_modal::TrialDetailModal;
use crate::ui::widgets::{
    artifact_gallery::ArtifactGallery, box_plot::BoxPlotChart, cluster_scatter::ClusterScatter,
    cluster_scatter_3d::ClusterScatter3D, compare::SurrogateCompareChart,
    comparison_table::ComparisonTableChart, convergence::ConvergenceChart,
    correlation_matrix::CorrelationMatrixChart, dendrogram::DendrogramChart,
    edf_plot::EdfPlotChart, histogram::HistogramChart, importance_chart::ImportanceChart,
    intermediate_values::IntermediateValuesChart, mcdm_chart::McdmRankChart,
    mcdm_scatter_chart::McdmScatterChart, mcdm_scatter_chart_3d::McdmScatterChart3D,
    optimization_history::OptimizationHistoryChart, parallel_coords::ParallelCoordsChart,
    pareto_2d::ParetoScatter2D, pareto_3d::Pareto3dChart, pca_biplot::PcaBiplotChart,
    pdp_2d::PdpChart2DState, pdp_chart::PdpChart, radar_comparison::RadarComparisonChart,
    rank_plot::RankPlotChart, response_surface::ResponseSurfaceChart, robustness::RobustnessChart,
    scatter_matrix::ScatterMatrix, sensitivity_heatmap::SensitivityHeatmap,
    slice_chart::SliceChart, som_map::SomMapChart, timeline::TimelineChart,
    trial_table::TrialTable,
};

// ── Observed Contour (contours from interpolating observed points) ────

/// The compute request for Observed Contour. Consumed by poll_chart.
pub struct ObservedContourComputeRequest {
    pub x: String,
    pub y: String,
    pub value: String,
    pub n_grid: usize,
    /// Sparsity guard (longest-edge threshold in normalized space). 0.0 disables it.
    pub max_edge_ratio: f64,
    pub feasible_only: bool,
}

/// UI state for the Observed Contour widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ObservedContourState {
    pub selected_x: String,
    pub selected_y: String,
    /// The column name used for value (color) (params ∪ objectives).
    pub selected_value: String,
    /// The Coverage slider (sparsity guard threshold).
    pub max_edge_ratio: f64,
    pub show_points: bool,
    pub feasible_only: bool,
    /// Log scale for color (Phase 2).
    pub log_scale: bool,
    /// Overlaying contour lines (Phase 2).
    pub show_contour_lines: bool,
    /// 3D display (Phase 3).
    pub view_3d: bool,
    /// Point-density shading: darkens sparsely-observed cells to curb overconfidence
    /// (3D, Phase 3).
    pub density_shade: bool,
    pub camera: crate::ui::widgets::scatter_3d::ArcballCamera,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub result: Option<crate::state::messages::ObservedContourResult>,
    #[serde(skip)]
    pub error_message: Option<String>,
    #[serde(skip)]
    pub pending_compute: Option<ObservedContourComputeRequest>,
    /// The signature (x, y, value, max_edge_ratio, feasible_only) at the time the
    /// last compute was issued.
    /// Used to detect selection changes and trigger automatic recomputation.
    #[serde(skip)]
    pub applied_sig: Option<(String, String, String, f64, bool)>,
    /// Trial detail modal opened by clicking a point (Phase 2).
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
}

impl Default for ObservedContourState {
    fn default() -> Self {
        Self {
            selected_x: String::new(),
            selected_y: String::new(),
            selected_value: String::new(),
            max_edge_ratio: 0.15,
            show_points: true,
            feasible_only: false,
            log_scale: false,
            show_contour_lines: false,
            view_3d: false,
            density_shade: true,
            camera: crate::ui::widgets::scatter_3d::ArcballCamera::isometric_default(),
            computing: false,
            result: None,
            error_message: None,
            pending_compute: None,
            applied_sig: None,
            detail_modal: TrialDetailModal::default(),
        }
    }
}

impl ObservedContourState {
    /// Adopts the global widget's compute execution state, result, and error (for
    /// propagation to each canvas item).
    /// UI selections such as axes, value, and sliders are kept on each item's side.
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.result = src.result.clone();
        self.error_message = src.error_message.clone();
    }
}

// ── Surrogate Optimizer compute request (fit stage) ──────────────
pub struct SurrogateFitComputeRequest {
    pub objective: String,
    /// The concrete model kind used when `auto_select = false`. It's a placeholder
    /// that's ignored when Auto (core re-selects it via CV).
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
    /// When true, core cross-validates `AUTO_CANDIDATES` and automatically selects the best model.
    pub auto_select: bool,
    /// Whether to use constraints (when true, packs constraint columns into
    /// ConstraintData and passes them along).
    pub use_constraints: bool,
}

// ── Surrogate Optimizer compute request (optimize stage) ────────
pub struct SurrogateOptimizeComputeRequest {
    pub optimizer: tunny_core::surrogate_opt::OptimizerKind,
}

// ── Surrogate Optimizer compute request (suggest stage) ──────────
pub struct SurrogateSuggestComputeRequest {
    /// The acquisition function to use.
    pub acquisition: tunny_core::surrogate_opt::AcquisitionKind,
    /// Number of candidates to suggest.
    pub n_candidates: usize,
    /// true = suggest as a minimization problem.
    pub minimize: bool,
}

/// The fit-stage request for multi-objective surrogate optimization.
pub struct SurrogateMultiFitComputeRequest {
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
}

/// The optimize-stage request for multi-objective surrogate optimization (a run
/// signal only).
pub struct SurrogateMultiOptimizeComputeRequest;

/// A multi-objective candidate suggestion request via EHVI.
pub struct SurrogateMultiSuggestComputeRequest {
    /// Number of candidates to suggest.
    pub n_candidates: usize,
}

// ── Surrogate Optimizer UI state ───────────────────────────────────
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SurrogateOptState {
    pub selected_objective: usize,
    pub model: tunny_core::surrogate_opt::SurrogateModelKind,
    /// True when "Auto (cross-validated)" is selected in the Model combo.
    /// In that case `model` is treated as a placeholder, and core picks the best
    /// model via CV.
    pub auto_select: bool,
    pub optimizer: tunny_core::surrogate_opt::OptimizerKind,
    /// The spinner flag for the fit stage.
    #[serde(skip)]
    pub fitting: bool,
    /// A shared handle for fit progress/cancellation (shared with the training
    /// thread).
    /// `Some` only while `fitting` is true. Referenced by the Cancel button and
    /// progress bar.
    #[serde(skip)]
    pub fit_progress: Option<tunny_core::surrogate_opt::FitProgress>,
    /// The spinner flag for the optimize stage.
    #[serde(skip)]
    pub optimizing: bool,
    /// The validated training result (kept after fit completes).
    #[serde(skip)]
    pub trained: Option<std::sync::Arc<tunny_core::surrogate_opt::TrainedSurrogate>>,
    #[serde(skip)]
    pub result: Option<crate::state::messages::SurrogateOptUiResult>,
    #[serde(skip)]
    pub error_message: Option<String>,
    #[serde(skip)]
    pub pending_fit: Option<SurrogateFitComputeRequest>,
    #[serde(skip)]
    pub pending_optimize: Option<SurrogateOptimizeComputeRequest>,
    /// When true, multi-objective mode (optimizes all objectives simultaneously with NSGA-II).
    pub multi_objective: bool,
    /// The pending compute request for the multi-objective fit stage.
    #[serde(skip)]
    pub pending_multi_fit: Option<SurrogateMultiFitComputeRequest>,
    /// The pending compute request for the multi-objective optimize stage.
    #[serde(skip)]
    pub pending_multi_optimize: Option<SurrogateMultiOptimizeComputeRequest>,
    /// The trained surrogates (in objective order) after multi-objective fit completes.
    #[serde(skip)]
    pub multi_trained: Option<std::sync::Arc<Vec<tunny_core::surrogate_opt::TrainedSurrogate>>>,
    /// The completed result of multi-objective optimization.
    #[serde(skip)]
    pub multi_result: Option<crate::state::messages::SurrogateMultiOptUiResult>,
    /// The X-axis objective index for the predicted Pareto front scatter plot.
    pub multi_front_x_obj: usize,
    /// The Y-axis objective index for the predicted Pareto front scatter plot.
    pub multi_front_y_obj: usize,
    /// The Z-axis objective index for the predicted Pareto front scatter plot (when
    /// shown in 3D).
    pub multi_front_z_obj: usize,
    /// Whether to show the predicted Pareto front as a 3D scatter plot (only
    /// enabled with 3+ objectives).
    pub multi_front_3d: bool,
    /// Camera state for the predicted Pareto front 3D scatter plot.
    pub multi_front_camera: crate::ui::widgets::scatter_3d::ArcballCamera,
    /// Whether to show the observed Pareto front (rank 0, feasible) in the front
    /// scatter plot.
    pub show_observed_front: bool,
    /// Whether to show observed dominated points (rank>0, feasible) in the front
    /// scatter plot.
    pub show_observed_dominated: bool,
    /// Whether to show observed infeasible solutions in the front scatter plot.
    pub show_observed_infeasible: bool,
    /// The selected objective index (target of the OOF plot) in the multi-objective
    /// validation display.
    pub multi_validation_objective: usize,
    /// Whether to use constraints (shown in the UI only for constrained Studies;
    /// true = pass constraints along).
    pub use_constraints: bool,
    // ── candidate suggestion via acquisition function ─────────────────
    /// The currently selected acquisition function.
    pub acq_kind: tunny_core::surrogate_opt::AcquisitionKind,
    /// Number of candidates to suggest (1-10).
    pub n_suggest_candidates: usize,
    /// The computing-in-progress flag for candidate suggestion.
    #[serde(skip)]
    pub suggesting: bool,
    /// The pending request for candidate suggestion.
    #[serde(skip)]
    pub pending_suggest: Option<SurrogateSuggestComputeRequest>,
    /// The result of candidate suggestion.
    #[serde(skip)]
    pub suggest_result: Option<crate::state::messages::SurrogateSuggestUiResult>,
    /// Whether to overlay predicted standard deviation (±σ) on the response surface
    /// slice (GP models only; off by default).
    pub show_slice_uncertainty: bool,
    // ── multi-objective candidate suggestion via EHVI ──────────────────
    /// Number of candidates for multi-objective suggestion (1-10).
    pub n_multi_suggest_candidates: usize,
    /// The computing-in-progress flag for multi-objective candidate suggestion.
    #[serde(skip)]
    pub multi_suggesting: bool,
    /// The pending request for multi-objective candidate suggestion.
    #[serde(skip)]
    pub pending_multi_suggest: Option<SurrogateMultiSuggestComputeRequest>,
    /// The result of multi-objective candidate suggestion.
    #[serde(skip)]
    pub multi_suggest_result: Option<crate::state::messages::SurrogateMultiSuggestUiResult>,
}

impl Default for SurrogateOptState {
    fn default() -> Self {
        Self {
            selected_objective: 0,
            model: tunny_core::surrogate_opt::SurrogateModelKind::GpFitc,
            auto_select: false,
            optimizer: tunny_core::surrogate_opt::OptimizerKind::MultiStartLbfgs,
            fitting: false,
            fit_progress: None,
            optimizing: false,
            trained: None,
            result: None,
            error_message: None,
            pending_fit: None,
            pending_optimize: None,
            multi_objective: false,
            pending_multi_fit: None,
            pending_multi_optimize: None,
            multi_trained: None,
            multi_result: None,
            multi_front_x_obj: 0,
            multi_front_y_obj: 1,
            multi_front_z_obj: 2,
            multi_front_3d: true,
            // Isometric initial viewpoint of Y-axis 45deg + X-axis -30deg (same as Pareto 3D).
            multi_front_camera: crate::ui::widgets::scatter_3d::ArcballCamera::isometric_default(),
            show_observed_front: true,
            show_observed_dominated: true,
            show_observed_infeasible: true,
            multi_validation_objective: 0,
            use_constraints: true,
            acq_kind: tunny_core::surrogate_opt::AcquisitionKind::ExpectedImprovement,
            n_suggest_candidates: 3,
            suggesting: false,
            pending_suggest: None,
            suggest_result: None,
            show_slice_uncertainty: false,
            n_multi_suggest_candidates: 3,
            multi_suggesting: false,
            pending_multi_suggest: None,
            multi_suggest_result: None,
        }
    }
}

impl SurrogateOptState {
    /// Adopts the global widget's compute execution state, results, and error.
    /// Used to propagate completion state to each canvas item's WidgetStates
    /// (selections such as objective, model, optimizer, and slice axes are kept as-is).
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.fitting = src.fitting;
        self.fit_progress = src.fit_progress.clone();
        self.optimizing = src.optimizing;
        self.trained = src.trained.clone();
        self.result = src.result.clone();
        self.multi_trained = src.multi_trained.clone();
        self.multi_result = src.multi_result.clone();
        self.error_message = src.error_message.clone();
        self.suggesting = src.suggesting;
        self.suggest_result = src.suggest_result.clone();
        self.multi_suggesting = src.multi_suggesting;
        self.multi_suggest_result = src.multi_suggest_result.clone();
    }
}

// ── TASK-2228/2245: chart capture state ───────────────────────────
/// The output destination for a captured PNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureDest {
    /// Save by opening a file dialog.
    #[default]
    File,
    /// Copy to the clipboard.
    Clipboard,
}

#[derive(Default)]
pub struct ChartCaptureState {
    pub last_error: Option<String>,
    /// The cell targeted for PNG save (reset to `None` once consumed)
    pub pending_capture: Option<crate::state::layout_state::PanelItem>,
    /// The draw rect of the target cell (used to crop after `ViewportCommand::Screenshot`)
    pub pending_capture_rect: Option<egui::Rect>,
    /// Flag that the Screenshot command has been issued (waits for
    /// `Event::Screenshot` next frame)
    pub screenshot_requested: bool,
    /// The output destination for the capture result (file save or clipboard)
    pub pending_capture_dest: CaptureDest,
}

// ── Cache to suppress per-frame rebuilding in render_chart ────────────
// In immediate mode, update runs every frame, so this avoids re-cloning objective
// columns to_vec or rebuilding comparison series unless the selection or data
// changes. All fields are runtime state, so they are not serialized (`#[serde(skip)]`
// on the `WidgetStates` side).

/// Cache key for the comparison series (OptimizationHistory / EdfPlot).
/// Includes the base Study's df identity, the selected objective name, and each
/// comparison Study's (df identity, color), so it's reliably invalidated on any of:
/// Study switch, comparison set change, color change, or objective switch.
#[derive(Clone, PartialEq)]
pub struct ComparisonSeriesKey {
    pub base_df: usize,
    pub sel_name: Option<String>,
    pub comps: Vec<(usize, [u8; 4])>,
}

/// Cache key for SurrogateOpt's observed data (objective column clone, feasibility).
#[derive(Clone, PartialEq)]
pub struct SurrogateObservedKey {
    pub df: usize,
    pub obj_history_name: Option<String>,
    pub multi_obj_names: Option<Vec<String>>,
}

/// The cache body for SurrogateOpt's observed data (the key plus owned buffers for reuse).
pub struct SurrogateObservedEntry {
    pub key: SurrogateObservedKey,
    pub obj_history: Option<Vec<f64>>,
    pub observed_cols: Option<Vec<Vec<f64>>>,
    pub observed_feasible: Vec<bool>,
}

/// The sync signature for a single comparison Study: (df identity, convergence
/// history's data identity (ptr, len), color).
type ConvergenceCompSig = (usize, Option<(usize, usize)>, [u8; 4]);

/// The key used to suppress ConvergenceIndicators' per-frame sync work (cloning
/// history/objective_names, rebuilding comparison series). Data identity is
/// detected via the Vec's data pointer + length.
#[derive(Clone, PartialEq)]
pub struct ConvergenceSyncKey {
    pub base_df: usize,
    pub history: Option<(usize, usize)>,
    pub indicator: tunny_core::indicators::MoIndicator,
    pub ref_override: Option<(usize, usize)>,
    pub comparisons: Vec<ConvergenceCompSig>,
}

/// The cache for the comparison series and observed data that render_chart used to
/// rebuild every frame.
#[derive(Default)]
pub struct RenderChartCache {
    opt_history: Option<(ComparisonSeriesKey, Vec<OptHistoryComparison>)>,
    edf: Option<(ComparisonSeriesKey, Vec<EdfComparison>)>,
    surrogate_observed: Option<SurrogateObservedEntry>,
    /// The key at the time of the last sync to ConvergenceChart (the value itself
    /// is kept on the widget side).
    pub convergence_sync: Option<ConvergenceSyncKey>,
}

impl RenderChartCache {
    /// Gets OptimizationHistory's comparison series. Only calls `build` when the key
    /// doesn't match.
    pub fn opt_history_comparisons(
        &mut self,
        key: ComparisonSeriesKey,
        build: impl FnOnce() -> Vec<OptHistoryComparison>,
    ) -> &[OptHistoryComparison] {
        if self.opt_history.as_ref().map(|(k, _)| k) != Some(&key) {
            self.opt_history = Some((key, build()));
        }
        &self.opt_history.as_ref().unwrap().1
    }

    /// Gets EdfPlot's comparison series. Only calls `build` when the key doesn't match.
    pub fn edf_comparisons(
        &mut self,
        key: ComparisonSeriesKey,
        build: impl FnOnce() -> Vec<EdfComparison>,
    ) -> &[EdfComparison] {
        if self.edf.as_ref().map(|(k, _)| k) != Some(&key) {
            self.edf = Some((key, build()));
        }
        &self.edf.as_ref().unwrap().1
    }

    /// Gets SurrogateOpt's observed data. Only calls `build` when the key doesn't match.
    /// `build` returns (obj_history, observed_cols, observed_feasible).
    pub fn surrogate_observed(
        &mut self,
        key: SurrogateObservedKey,
        build: impl FnOnce() -> (Option<Vec<f64>>, Option<Vec<Vec<f64>>>, Vec<bool>),
    ) -> &SurrogateObservedEntry {
        if self.surrogate_observed.as_ref().map(|e| &e.key) != Some(&key) {
            let (obj_history, observed_cols, observed_feasible) = build();
            self.surrogate_observed = Some(SurrogateObservedEntry {
                key,
                obj_history,
                observed_cols,
                observed_feasible,
            });
        }
        self.surrogate_observed.as_ref().unwrap()
    }
}

/// Holds the UI state for each chart widget, bundled together.
/// Pure UI state, separate from AppState (data).
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WidgetStates {
    pub pareto_2d: ParetoScatter2D,
    pub pareto_3d: Pareto3dChart,
    pub opt_history: OptimizationHistoryChart,
    pub convergence: ConvergenceChart,
    pub importance: ImportanceChart,
    pub pdp_chart: PdpChart,
    pub pdp_2d: PdpChart2DState,
    pub parallel_coords: ParallelCoordsChart,
    pub scatter_matrix: ScatterMatrix,
    pub sensitivity_heatmap: SensitivityHeatmap,
    pub cluster_scatter: ClusterScatter,
    pub cluster_scatter_3d: ClusterScatter3D,
    pub mcdm_chart: McdmRankChart,
    /// The table widget that unifies the trial list / cluster assignment / MCDM ranking.
    pub trial_table: TrialTable,
    pub artifact_gallery: ArtifactGallery,
    pub slice_chart: SliceChart,
    // TASK-1504: MCDM scatter plot widget
    pub scatter_chart: McdmScatterChart,
    pub mcdm_scatter_3d: McdmScatterChart3D,
    /// UI state for Observed Contour (contours from interpolating observed trial points)
    pub observed_contour: ObservedContourState,
    /// UI state for surrogate optimization (building a response surface + optimizing
    /// on that surface)
    pub surrogate_opt: SurrogateOptState,
    /// UI state for robustness analysis (MC propagation of input noise on the
    /// trained surrogate)
    pub robustness: RobustnessChart,
    pub histogram: HistogramChart,
    pub box_plot: BoxPlotChart,
    pub correlation_matrix: CorrelationMatrixChart,
    /// UI state for the radar comparison of pinned trials (decision-making phase)
    pub radar_comparison: RadarComparisonChart,
    /// UI state for the comparison table of pinned trials (decision-making phase)
    pub comparison_table: ComparisonTableChart,
    /// UI state for the PCA biplot (scores + loadings)
    pub pca_biplot: PcaBiplotChart,
    /// UI state for SOM (self-organizing map)
    pub som_map: SomMapChart,
    /// UI state for hierarchical clustering (dendrogram)
    pub dendrogram: DendrogramChart,
    /// UI state for the response surface 3D viewer
    pub response_surface: ResponseSurfaceChart,
    /// UI state for Compare Surrogates (CV metric comparison across all model kinds
    /// + predicted slice overlay)
    pub surrogate_compare: SurrogateCompareChart,
    /// UI state for Intermediate Values (learning curve per trial)
    pub intermediate_values: IntermediateValuesChart,
    /// UI state for Timeline (trial execution timeline)
    pub timeline: TimelineChart,
    /// UI state for the EDF (empirical distribution function) chart
    pub edf_plot: EdfPlotChart,
    /// UI state for Rank Plot (parameter pairs x objective function rank)
    pub rank_plot: RankPlotChart,
    #[serde(skip)]
    pub capture: ChartCaptureState,
    /// Cache to suppress render_chart's per-frame rebuild (comparison series /
    /// observed column clones).
    #[serde(skip)]
    pub render_cache: RenderChartCache,
    /// The widget currently maximized via double-click (None = normal display)
    #[serde(skip)]
    pub maximized_item: Option<crate::state::layout_state::PanelItem>,
    /// State of the open-source license display modal
    #[serde(skip)]
    pub license_modal: LicenseModalState,
    /// State of the About modal (version, beta notice, entry point to the licenses)
    #[serde(skip)]
    pub about_modal: AboutModalState,
}

impl WidgetStates {
    /// Resets the show_infeasible flag to true for all charts on Study switch.
    pub fn reset_infeasible_flags(&mut self) {
        self.pareto_3d.show_infeasible = true;
        self.cluster_scatter_3d.show_infeasible = true;
        self.mcdm_scatter_3d.show_infeasible = true;
        self.parallel_coords.show_infeasible = true;
        self.scatter_matrix.show_infeasible = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_states_default_has_capture_slot() {
        let ws = WidgetStates::default();
        assert!(ws.capture.last_error.is_none());
    }

    // ── Regression tests for SurrogateOptState's new 2-stage fields ──

    #[test]
    fn surrogate_opt_state_default_has_expected_flags() {
        let state = SurrogateOptState::default();
        assert!(!state.fitting);
        assert!(!state.optimizing);
        assert!(state.trained.is_none());
        assert!(state.pending_fit.is_none());
        assert!(state.pending_optimize.is_none());
        assert!(state.result.is_none());
        // Confirm initial values of multi-objective fields
        assert!(!state.multi_objective);
        assert!(state.pending_multi_fit.is_none());
        assert!(state.pending_multi_optimize.is_none());
        assert!(state.multi_trained.is_none());
        assert!(state.multi_result.is_none());
        assert_eq!(state.multi_front_x_obj, 0);
        assert_eq!(state.multi_front_y_obj, 1);
        assert_eq!(state.multi_validation_objective, 0);
    }

    #[test]
    fn surrogate_opt_adopt_compute_state_propagates_new_fields() {
        let src = SurrogateOptState {
            fitting: false,
            optimizing: false,
            error_message: Some("err".into()),
            ..Default::default()
        };

        let mut dst = SurrogateOptState {
            fitting: true,
            optimizing: true,
            model: tunny_core::surrogate_opt::SurrogateModelKind::Ridge,
            selected_objective: 2,
            multi_validation_objective: 1,
            ..Default::default()
        };
        dst.adopt_compute_state(&src);

        // Fields that are propagated
        assert!(!dst.fitting);
        assert!(!dst.optimizing);
        assert_eq!(dst.error_message.as_deref(), Some("err"));
        // Selections are preserved
        assert_eq!(
            dst.model,
            tunny_core::surrogate_opt::SurrogateModelKind::Ridge
        );
        assert_eq!(dst.selected_objective, 2);
        // The UI selection (OOF plot target) is not propagated and is preserved
        assert_eq!(dst.multi_validation_objective, 1);
        // multi_trained / multi_result are also propagated
        assert!(dst.multi_trained.is_none());
        assert!(dst.multi_result.is_none());
    }

    // F-008: PNG capture state transitions
    #[test]
    fn png_capture_state_transitions_are_covered() {
        use crate::state::layout_state::{ChartId, PanelItem};

        let mut capture = ChartCaptureState::default();
        assert!(capture.pending_capture.is_none());
        assert!(!capture.screenshot_requested);
        assert!(capture.pending_capture_rect.is_none());

        // "Save as PNG" pressed → pending set
        capture.pending_capture = Some(PanelItem::Chart(ChartId::ParallelCoordinates));
        capture.pending_capture_rect = Some(egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 80.0),
        ));
        assert!(capture.pending_capture.is_some());

        // Screenshot command issued
        capture.screenshot_requested = true;
        assert!(capture.screenshot_requested);

        // Screenshot received → consumed and reset
        capture.screenshot_requested = false;
        capture.pending_capture = None;
        capture.pending_capture_rect = None;
        assert!(!capture.screenshot_requested);
        assert!(capture.pending_capture.is_none());

        // Failure path: error stored
        capture.last_error = Some("crop rect outside image".into());
        assert!(capture.last_error.is_some());
    }
}
