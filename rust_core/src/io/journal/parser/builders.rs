use std::collections::{HashMap, HashSet};

use super::types::OptimizationDirection;

/// Intermediate state for assembling a `StudyMeta` from the journal's event stream.
pub(super) struct StudyBuilder {
    pub(super) study_id: u32,
    pub(super) name: String,
    pub(super) directions: Vec<OptimizationDirection>,
    pub(super) total_trials: u32,
    pub(super) completed_trials: u32,
    pub(super) param_names: HashSet<String>,
    pub(super) objective_names: Vec<String>,
    pub(super) user_attr_names: HashSet<String>,
    pub(super) has_constraints: bool,
    /// Declared range (low, high) per parameter (display units, recorded from the first-seen distribution).
    /// Numeric parameters (Float / Int) only. Used as the search range for surrogate optimization.
    pub(super) param_bounds: HashMap<String, (f64, f64)>,
}

/// Intermediate state for assembling a single trial's data from the journal's event stream.
pub(super) struct TrialBuilder {
    pub(super) study_id: u32,
    /// The 0-based trial.number within the study (creation order = order of op_code=4 occurrences within the study).
    pub(super) trial_number: u32,
    pub(super) state: u8,
    pub(super) values: Option<Vec<f64>>,
    pub(super) param_display: HashMap<String, f64>,
    pub(super) param_category_label: HashMap<String, String>,
    pub(super) user_attrs_numeric: HashMap<String, f64>,
    pub(super) user_attrs_string: HashMap<String, String>,
    pub(super) constraint_values: Vec<f64>,
    pub(super) has_constraints: bool,
    /// Start datetime. Naive unix seconds. Derived from `datetime_start` on op_code=4.
    pub(super) datetime_start: Option<f64>,
    /// Completion datetime. Naive unix seconds. Derived from `datetime_complete` on op_code=6.
    pub(super) datetime_complete: Option<f64>,
    /// Intermediate values `(step, value)`. Derived from op_code=7. Insertion order (sorted by ascending step later).
    pub(super) intermediate_values: Vec<(u64, f64)>,
}
