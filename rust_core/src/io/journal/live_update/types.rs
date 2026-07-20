//! Data types for live update diff parsing: diff results, trial rows, in-flight
//! parsing state, and the context passed to the polling thread.

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// Result types
// =============================================================================

#[derive(Debug, Clone)]
pub struct AppendDiffResult {
    pub consumed_bytes: usize,
    pub pending_running: usize,
    pub new_trial_rows: Vec<TrialRow>,
    pub updated_study_counts: Vec<(u32, usize)>,
    /// Diff events to apply to the extras (auxiliary info) of all trials (all states).
    pub extras_events: ExtrasDiff,
}

/// Extras (state / datetime / intermediate value) update events extracted from a live diff.
///
/// While `new_trial_rows` only handles COMPLETE trials, this collects events for all states.
/// The consumer (egui-app) merges these into the study's [`crate::extras::StudyExtras`].
#[derive(Debug, Clone, Default)]
pub struct ExtrasDiff {
    /// op_code=4 (CREATE_TRIAL): (trial_id, study_id, trial_number, datetime_start).
    pub new_trials: Vec<(u32, u32, u32, Option<f64>)>,
    /// op_code=7 (SET_TRIAL_INTERMEDIATE_VALUE): (trial_id, step, value).
    pub intermediate_values: Vec<(u32, u64, f64)>,
    /// op_code=6 (SET_TRIAL_STATE_VALUES): (trial_id, state, datetime_complete). Records all states.
    pub state_changes: Vec<(u32, u8, Option<f64>)>,
}

/// Trial row data built from incremental diff parsing.
#[derive(Debug, Clone)]
pub struct TrialRow {
    pub trial_id: u32,
    pub trial_number: u32,
    pub params: HashMap<String, f64>,
    pub param_categories: HashMap<String, String>,
    pub objectives: Vec<f64>,
    pub user_attrs_numeric: HashMap<String, f64>,
    pub user_attrs_string: HashMap<String, String>,
    pub constraint_values: Vec<f64>,
    pub study_id: u32,
}

// =============================================================================
// Internal state
// =============================================================================

#[derive(Debug, Default)]
pub(super) struct PendingTrial {
    pub(super) study_idx: u32,
    /// 0-based trial.number within the study (fixed at creation time).
    pub(super) trial_number: u32,
    pub(super) values: Option<Vec<f64>>,
    pub(super) param_display: HashMap<String, f64>,
    pub(super) param_category_label: HashMap<String, String>,
    pub(super) user_attrs_numeric: HashMap<String, f64>,
    pub(super) user_attrs_string: HashMap<String, String>,
    pub(super) constraint_values: Vec<f64>,
}

#[derive(Debug, Default)]
pub(super) struct LiveUpdateState {
    pub(super) next_trial_id: u32,
    /// study_id → number of trials created so far (i.e. the next trial.number).
    /// Seeded from the existing file's per-study creation count when going live.
    pub(super) next_trial_number: HashMap<u32, u32>,
    pub(super) pending: HashMap<u32, PendingTrial>,
}

// =============================================================================
// Context types
// =============================================================================

/// Context passed to the polling thread for incremental parsing.
#[derive(Debug, Clone)]
pub struct LiveUpdateContext {
    pub file_path: PathBuf,
    pub initial_byte_offset: u64,
    pub next_trial_id: u32,
    /// Per-study creation counts from the existing file (study_id → count). Seeds each study's next trial.number.
    pub study_trial_number_seeds: HashMap<u32, u32>,
    pub study_distributions: Vec<StudyDistributionInfo>,
    /// Milliseconds of no file change before sending completion hint (default: 60_000)
    pub no_change_timeout_ms: u64,
}

/// Per-study distribution info needed for incremental TrialRow construction.
#[derive(Debug, Clone)]
pub struct StudyDistributionInfo {
    pub study_id: u32,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub distributions: HashMap<String, Value>,
}
