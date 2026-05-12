// Dashboard Missing Features - Rust Type Design
// Location: docs/design/dashboard-missing-features/interfaces.rs
// Purpose: Documentation-only interface sketch for the missing dashboard features.
// Actual implementation targets egui-app/src and rust_core/src.

use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// SECTION 1: Toolbar and App State Extensions
// ============================================================================

#[derive(Debug, Clone)]
pub enum ToolbarAction {
    OpenJournal(PathBuf),
    SetLayoutMode(LayoutMode),
    SelectStudy(StudyMeta),
    ToggleLiveUpdate,
    SetPollInterval(u64),
    GenerateHtmlReport,
    ScanArtifacts(PathBuf),
    LoadSession,
    SaveSession,
    ClearLoadError,

    // New for dashboard-missing-features
    ExportCsv(ExportTarget),
    AddComparisonStudy,
    RemoveComparisonStudy(usize),
}

#[derive(Debug, Default)]
pub struct AppState {
    pub all_studies: Vec<StudyMeta>,
    pub journal_path: Option<PathBuf>,
    pub current_study: Option<StudyContext>,
    pub selected_indices: Vec<u32>,
    pub comparison_studies: Vec<StudyContext>,
    pub comparison_colors: Vec<[u8; 4]>,

    // New
    pub pinned_trials: Vec<u32>,
    pub comparison_base_study: Option<u32>,
}

impl AppState {
    pub fn toggle_pinned_trial(&mut self, trial_id: u32) -> Result<(), PinError> {
        unimplemented!()
    }

    pub fn effective_visible_trial_ids(&self, trial_rows: &[TrialRow]) -> Vec<u32> {
        unimplemented!()
    }

    pub fn reset_comparison_session(&mut self) {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinError {
    MaxPinnedReached { limit: usize },
    TrialNotFound(u32),
}

// ============================================================================
// SECTION 2: Selection and Display Helpers
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionSource {
    ParetoBrush,
    ParallelBrush,
    Clear,
    RestoreSession,
}

#[derive(Debug, Clone)]
pub struct SelectionPatch {
    pub source: SelectionSource,
    pub trial_ids: Vec<u32>,
    pub additive: bool,
}

pub fn merge_selected_with_pinned(
    selected_ids: &[u32],
    pinned_ids: &[u32],
    trial_rows: &[TrialRow],
) -> Vec<u32> {
    unimplemented!()
}

pub fn filter_rows_for_display<'a>(
    trial_rows: &'a [TrialRow],
    visible_ids: &[u32],
) -> Vec<&'a TrialRow> {
    unimplemented!()
}

// ============================================================================
// SECTION 3: Comparison Study and Diff Types
// ============================================================================

#[derive(Default, PartialEq, Clone, Copy)]
pub enum ComparisonView {
    #[default]
    Stats,
    HvHistory,
    ParetoFront,
    KdeDistribution,
    Diff,
}

#[derive(Debug, Clone)]
pub struct ComparisonDiffRow {
    pub study_name: String,
    pub trial_delta: isize,
    pub best_delta: Option<f64>,
    pub hv_delta: Option<f64>,
    pub domination_ratio: Option<f64>,
    pub compatible: bool,
    pub incompatibility_reason: Option<String>,
}

pub fn build_comparison_diff_rows(
    base: &StudyContext,
    comparisons: &[StudyContext],
) -> Vec<ComparisonDiffRow> {
    unimplemented!()
}

// ============================================================================
// SECTION 4: Surface Plot Widget Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfacePlotRenderMode {
    Heatmap,
    Contour,
}

#[derive(Debug, Clone)]
pub struct SurfacePlotComputeRequest {
    pub param_x: String,
    pub param_y: String,
    pub objective: String,
    pub n_grid: usize,
    pub model_type: String,
    pub render_mode: SurfacePlotRenderMode,
}

#[derive(Debug, Clone)]
pub struct SurfacePlotResult {
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
    pub z_values: Vec<Vec<f64>>,
    pub param_x_name: String,
    pub param_y_name: String,
    pub objective_name: String,
    pub r2: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SurfacePlotState {
    pub selected_x: String,
    pub selected_y: String,
    pub selected_objective: usize,
    pub model_type: ModelType,
    pub render_mode: SurfacePlotRenderMode,
    pub pending_compute: Option<SurfacePlotComputeRequest>,
    pub computing: bool,
    pub result: Option<SurfacePlotResult>,
    pub error_message: Option<String>,
}

impl Default for SurfacePlotState {
    fn default() -> Self {
        Self {
            selected_x: String::new(),
            selected_y: String::new(),
            selected_objective: 0,
            model_type: ModelType::Ridge,
            render_mode: SurfacePlotRenderMode::Heatmap,
            pending_compute: None,
            computing: false,
            result: None,
            error_message: None,
        }
    }
}

// ============================================================================
// SECTION 5: Cell Toolbar and PNG Capture Types
// ============================================================================

#[derive(Debug, Clone)]
pub enum CellToolbarAction {
    None,
    Close,
    Help(PanelItem),
    SavePng { item: PanelItem, row: usize, col: usize },
}

#[derive(Debug, Clone)]
pub struct ChartCaptureRequest {
    pub row: usize,
    pub col: usize,
    pub item: PanelItem,
    pub crop_rect: egui::Rect,
    pub save_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct ChartCaptureState {
    pub pending_capture: Option<ChartCaptureRequest>,
    pub last_cell_rects: HashMap<(usize, usize), egui::Rect>,
    pub last_error: Option<String>,
}

// ============================================================================
// SECTION 6: AppMessage Additions
// ============================================================================

pub enum AppMessage {
    ComparisonStudyLoaded {
        study_idx: usize,
        context: Box<StudyContext>,
    },
    ComparisonStudyLoadFailed(String),
    SurfacePlotDone(SurfacePlotResult),
    SurfacePlotFailed(String),
    ChartCaptureFailed(String),
}

// ============================================================================
// SECTION 7: Session Snapshot Extension
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionSnapshot {
    pub study_name: String,
    pub filter_ranges: HashMap<String, (f64, f64)>,
    pub selected_indices: Vec<u32>,

    // Existing in current repo; included here to document required wiring.
    pub pinned_trials: Vec<u32>,
}

// ============================================================================
// SECTION 8: WidgetStates Extension
// ============================================================================

#[derive(Default)]
pub struct WidgetStates {
    pub pdp_chart: PdpChart,
    pub parallel_coords: ParallelCoordsChart,
    pub pareto_2d: ParetoScatter2D,

    // New
    pub surface_plot: SurfacePlotState,
    pub capture: ChartCaptureState,
}

// ============================================================================
// SECTION 9: Notes
// ============================================================================

// - F-004 PDP overlay already has `show_observed` in the current workspace.
//   The main missing piece is feeding it filtered rows rather than all rows.
// - F-006 Parallel Coordinates already has brush_ranges and drag_start.
//   The missing piece is committing brush results into AppState.selected_indices.
// - F-008 capture uses cell-level export to keep plot/table/custom widgets on one path.
