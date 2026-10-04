use std::collections::{HashMap, HashSet};

/// Declared numeric grouping eligibility and original categorical values.
#[derive(Clone, Debug, Default)]
pub struct DistributionMetadata {
    /// Declared categorical parameters, even when a value cannot be resolved.
    pub categorical: HashSet<String>,
    pub numeric_discrete: HashMap<String, bool>,
    pub categories: HashMap<String, serde_json::Value>,
}

/// A row holding data for one trial. The intermediate representation used as input for
/// building a DataFrame.
#[derive(Clone, Default)]
pub struct TrialRow {
    pub distribution_metadata: DistributionMetadata,
    /// Global trial_id across storages (in order of op_code=4 occurrence).
    pub trial_id: u32,
    /// 0-based trial.number within the study (Optuna's `trial.number`, creation order).
    pub trial_number: u32,
    /// Map from param name to display value (numeric).
    pub param_display: HashMap<String, f64>,
    /// Map from categorical param name to choice label string.
    pub param_category_label: HashMap<String, String>,
    /// objective value list (obj0, obj1, ...)
    pub objective_values: Vec<f64>,
    /// user_attr numeric type (REQ-012)
    pub user_attrs_numeric: HashMap<String, f64>,
    /// user_attr string type (REQ-012)
    pub user_attrs_string: HashMap<String, String>,
    /// Original JSON values for present trial user attributes.
    pub user_attrs_json: HashMap<String, serde_json::Value>,
    /// Constraint values in original array order, with invalid positions as NaN.
    pub constraint_values: Vec<f64>,
}
