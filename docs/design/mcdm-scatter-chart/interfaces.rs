// MCDM Scatter Chart - Rust Type Definitions
// Location: docs/design/mcdm-scatter-chart/interfaces.rs
// Purpose: Type specifications for McdmScatterChart widget implementation
//
// These types are defined as documentation. Actual implementation
// will be in: egui-app/src/ui/widgets/mcdm_scatter_chart.rs
//            egui-app/src/state/ (as needed)

// ============================================================================
// SECTION 1: Widget State Structures
// ============================================================================

/// McdmScatterChart widget state
/// Manages scatter plot visualization with independent state management
#[derive(Clone, Debug)]
pub struct McdmScatterChart {
    /// Selected X axis identifier (e.g., "Objective0", "VIKOR_Q")
    pub x_axis: String,

    /// Selected Y axis identifier (e.g., "Objective1", "VIKOR_S")
    pub y_axis: String,

    /// Color threshold for ranking visualization
    pub color_threshold: TopN,

    /// Enable/disable downsampling for large datasets
    pub use_downsample: bool,

    /// Cached scatter point data: (x_normalized, y_normalized, color)
    /// None = cache invalid, requires recomputation
    pub display_rows_cache: Option<Vec<(f64, f64, egui::Color32)>>,

    /// Cache validation key: (trial_count, axis_hash)
    /// Used to detect when cache needs invalidation
    pub cache_key: (usize, u64),

    /// Pending computation flag
    /// Some(()) = computation requested, waiting for background task
    pub pending_compute: Option<()>,

    /// Computation in progress flag
    pub computing: bool,

    /// Error message if computation failed
    pub error_message: Option<String>,

    /// Hover info: (point_index, tooltip_text)
    pub hover_info: Option<(usize, String)>,

    /// Selected point index for highlight
    pub selected_point: Option<usize>,
}

impl McdmScatterChart {
    pub fn new() -> Self {
        Self {
            x_axis: "Objective0".to_string(),
            y_axis: "Objective1".to_string(),
            color_threshold: TopN::Top5,
            use_downsample: true,
            display_rows_cache: None,
            cache_key: (0, 0),
            pending_compute: None,
            computing: false,
            error_message: None,
            hover_info: None,
            selected_point: None,
        }
    }

    /// Invalidate cache when axes or display settings change
    pub fn invalidate_cache(&mut self) {
        self.display_rows_cache = None;
        self.pending_compute = Some(());
    }

    /// Update cache key based on current state and trial count
    pub fn update_cache_key(&mut self, trial_count: usize) {
        let axis_hash = Self::hash_axes(&self.x_axis, &self.y_axis);
        self.cache_key = (trial_count, axis_hash);
    }

    /// Hash function for axis combination
    fn hash_axes(x: &str, y: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        x.hash(&mut hasher);
        y.hash(&mut hasher);
        hasher.finish()
    }
}

// ============================================================================
// SECTION 2: Color and Ranking Types
// ============================================================================

/// Top N threshold for color highlighting
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TopN {
    Top5,
    Top10,
    Top20,
}

impl TopN {
    pub fn threshold(&self) -> usize {
        match self {
            TopN::Top5 => 5,
            TopN::Top10 => 10,
            TopN::Top20 => 20,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            TopN::Top5 => "Top5",
            TopN::Top10 => "Top10",
            TopN::Top20 => "Top20",
        }
    }

    pub fn all_options() -> Vec<TopN> {
        vec![TopN::Top5, TopN::Top10, TopN::Top20]
    }
}

/// Color scheme for scatter plot points
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointColor {
    /// Rank 0-4: Red (top performers)
    Top5,

    /// Rank 5-9: Orange (strong performers)
    Top10,

    /// Rank 10-19: Yellow (acceptable performers)
    Top20,

    /// Rank 20+: Gray (other performers)
    Other,
}

impl PointColor {
    pub fn to_egui_color(&self) -> egui::Color32 {
        match self {
            PointColor::Top5 => egui::Color32::from_rgb(255, 0, 0),         // Red
            PointColor::Top10 => egui::Color32::from_rgb(255, 165, 0),      // Orange
            PointColor::Top20 => egui::Color32::from_rgb(255, 255, 0),      // Yellow
            PointColor::Other => egui::Color32::from_rgb(200, 200, 200),    // Gray
        }
    }

    pub fn from_rank(rank: usize, threshold: TopN) -> Self {
        let threshold_val = threshold.threshold();
        match rank {
            0..=4 if threshold >= TopN::Top5 => PointColor::Top5,
            5..=9 if threshold >= TopN::Top10 => PointColor::Top10,
            10..=19 if threshold >= TopN::Top20 => PointColor::Top20,
            _ => PointColor::Other,
        }
    }
}

// ============================================================================
// SECTION 3: Data Transform Types
// ============================================================================

/// Scatter point with metadata
#[derive(Clone, Debug)]
pub struct ScatterPoint {
    /// Trial index in original dataset
    pub trial_index: usize,

    /// Normalized X coordinate [0.0, 1.0]
    pub x: f64,

    /// Normalized Y coordinate [0.0, 1.0]
    pub y: f64,

    /// Egui color for rendering
    pub color: egui::Color32,

    /// Original values before normalization
    pub original_x: f64,
    pub original_y: f64,

    /// Ranking position if available
    pub rank: Option<usize>,
}

/// Axis metadata and values
#[derive(Clone, Debug)]
pub struct AxisData {
    /// Axis identifier (e.g., "Objective0", "VIKOR_Q")
    pub axis_id: String,

    /// Human-readable axis label
    pub label: String,

    /// Raw values (one per trial)
    pub values: Vec<f64>,

    /// Min value in dataset
    pub min: f64,

    /// Max value in dataset
    pub max: f64,

    /// Unit/scaling note for display
    pub unit: String,
}

impl AxisData {
    pub fn normalize(&self) -> Vec<f64> {
        let range = self.max - self.min;

        if range == 0.0 {
            // All values equal → return mid-point
            vec![0.5; self.values.len()]
        } else {
            self.values
                .iter()
                .map(|&v| (v - self.min) / range)
                .collect()
        }
    }
}

/// Axis selection option for ComboBox
#[derive(Clone, Debug)]
pub struct AxisOption {
    /// Internal identifier
    pub id: String,

    /// Display label in UI
    pub label: String,

    /// Optional unit (for display in tooltip)
    pub unit: Option<String>,
}

// ============================================================================
// SECTION 4: Message Types
// ============================================================================

/// Extended AppMessage variants for scatter chart
/// (These would be added to egui-app/src/state/messages.rs)
pub enum McdmScatterMessage {
    /// Request scatter plot computation
    ComputeScatter {
        x_axis: String,
        y_axis: String,
    },

    /// Scatter plot computation completed
    ScatterComputed {
        x_axis: String,
        y_axis: String,
        points: Vec<ScatterPoint>,
        metadata: ScatterMetadata,
    },

    /// Scatter plot computation failed
    ScatterComputeFailed {
        reason: String,
    },

    /// Update UI controls
    SetColorThreshold(TopN),
    SetDownsample(bool),
}

/// Metadata about scatter plot computation
#[derive(Clone, Debug)]
pub struct ScatterMetadata {
    /// Total trials in dataset
    pub total_trials: usize,

    /// Points rendered after downsampling
    pub rendered_points: usize,

    /// Downsampling factor (e.g., 2 = every 2nd point)
    pub downsample_factor: usize,

    /// Computation time in milliseconds
    pub compute_time_ms: u128,

    /// X axis details
    pub x_axis_label: String,

    /// Y axis details
    pub y_axis_label: String,
}

// ============================================================================
// SECTION 5: Computation Types
// ============================================================================

/// Request to compute scatter plot
#[derive(Clone, Debug)]
pub struct ScatterComputeRequest {
    /// X axis identifier
    pub x_axis: String,

    /// Y axis identifier
    pub y_axis: String,

    /// Color threshold
    pub color_threshold: TopN,

    /// Downsample enabled
    pub use_downsample: bool,

    /// Max points to render (for downsampling)
    pub max_points: usize,
}

impl Default for ScatterComputeRequest {
    fn default() -> Self {
        Self {
            x_axis: "Objective0".to_string(),
            y_axis: "Objective1".to_string(),
            color_threshold: TopN::Top5,
            use_downsample: true,
            max_points: 300,
        }
    }
}

/// Result of scatter plot computation
#[derive(Clone, Debug)]
pub struct ScatterComputeResult {
    /// Computed points ready for rendering
    pub points: Vec<ScatterPoint>,

    /// Metadata about the computation
    pub metadata: ScatterMetadata,
}

// ============================================================================
// SECTION 6: Error Types
// ============================================================================

/// Errors that can occur during scatter plot operations
#[derive(Clone, Debug)]
pub enum ScatterError {
    /// Axis not found in MCDM result
    AxisNotFound { axis_id: String },

    /// Invalid axis data (NaN/Inf)
    InvalidAxisData { axis_id: String, reason: String },

    /// Computation failed (panicked in background task)
    ComputationFailed { reason: String },

    /// MCDM result not available
    NoMcdmResult,

    /// Not enough trials
    InsufficientTrials { required: usize, available: usize },
}

impl std::fmt::Display for ScatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScatterError::AxisNotFound { axis_id } => {
                write!(f, "Axis '{}' not found in MCDM result", axis_id)
            }
            ScatterError::InvalidAxisData { axis_id, reason } => {
                write!(f, "Invalid data for axis '{}': {}", axis_id, reason)
            }
            ScatterError::ComputationFailed { reason } => {
                write!(f, "Scatter plot computation failed: {}", reason)
            }
            ScatterError::NoMcdmResult => {
                write!(f, "No MCDM result available")
            }
            ScatterError::InsufficientTrials {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient trials: required {}, available {}",
                    required, available
                )
            }
        }
    }
}

// ============================================================================
// SECTION 7: Helper Functions (Specifications)
// ============================================================================

/// Extract axis options from MCDM result and trial metadata
pub fn get_axis_options(
    // mcdm_result: &McdmResult,
    // trial_metadata: &TrialMetadata,
) -> Vec<AxisOption> {
    // Returns list of selectable axes
    // Including: Objective1, Objective2, ... + MCDM scores
    vec![]
}

/// Normalize values to [0.0, 1.0] range
pub fn normalize_values(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }

    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    values
        .iter()
        .map(|&v| {
            if range == 0.0 {
                0.5 // All equal → mid-point
            } else {
                (v - min) / range
            }
        })
        .collect()
}

/// Downsample points array to maximum count
pub fn downsample_points<T: Clone>(points: &[T], max_count: usize) -> Vec<T> {
    if points.len() <= max_count {
        return points.to_vec();
    }

    let step = (points.len() as f64 / max_count as f64).ceil() as usize;
    points.iter().step_by(step).cloned().collect()
}

/// Compute downsampling factor
pub fn compute_downsample_factor(total_points: usize, max_points: usize) -> usize {
    if total_points <= max_points {
        1
    } else {
        (total_points as f64 / max_points as f64).ceil() as usize
    }
}

// ============================================================================
// SECTION 8: Widget Display Configuration
// ============================================================================

/// Configuration for McdmScatterChart UI rendering
#[derive(Clone, Debug)]
pub struct ScatterChartConfig {
    /// Point radius in pixels
    pub point_radius: f32,

    /// Point opacity (0.0 = transparent, 1.0 = opaque)
    pub point_opacity: f32,

    /// Enable grid lines
    pub show_grid: bool,

    /// Enable legend
    pub show_legend: bool,

    /// Max points to render before forcing downsample
    pub max_render_points: usize,

    /// Enable hover tooltips
    pub show_tooltips: bool,
}

impl Default for ScatterChartConfig {
    fn default() -> Self {
        Self {
            point_radius: 4.0,
            point_opacity: 1.0,
            show_grid: true,
            show_legend: true,
            max_render_points: 300,
            show_tooltips: true,
        }
    }
}

// ============================================================================
// SECTION 9: Integration with Existing Types
// ============================================================================

// Note: The following types are defined in egui-app/src/state/results.rs
// and egui-app/src/state/app_state.rs
// These are referenced here for completeness:

/*
pub enum McdmResult {
    Topsis(TopsisResult),
    Vikor(VikorResult),
    PrometheeI(PrometheeResult),
    PrometheeII(PrometheeResult),
}

pub struct VikorResult {
    pub s_values: Vec<f64>,
    pub r_values: Vec<f64>,
    pub q_values: Vec<f64>,
    pub display_scores: Vec<f64>,
    pub ranked_indices: Vec<usize>,
    pub best_values: Vec<f64>,
    pub worst_values: Vec<f64>,
    pub duration_ms: u128,
}

pub struct AppState {
    pub mcdm_result: Option<McdmResult>,
    pub trials: Vec<TrialRow>,
    pub widget_states: WidgetStates,
    // ... other fields
}

pub struct WidgetStates {
    pub mcdm_chart: McdmScatterChart,
    // ... other widget states
}
*/
