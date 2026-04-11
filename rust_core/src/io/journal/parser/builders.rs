use std::collections::{HashMap, HashSet};

use super::types::OptimizationDirection;

/// Documentation.
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
}

/// Documentation.
pub(super) struct TrialBuilder {
    pub(super) study_id: u32,
    pub(super) state: u8,
    pub(super) values: Option<Vec<f64>>,
    pub(super) param_display: HashMap<String, f64>,
    pub(super) param_category_label: HashMap<String, String>,
    pub(super) user_attrs_numeric: HashMap<String, f64>,
    pub(super) user_attrs_string: HashMap<String, String>,
    pub(super) constraint_values: Vec<f64>,
    pub(super) has_constraints: bool,
}
