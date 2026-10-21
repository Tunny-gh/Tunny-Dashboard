use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DistributionMetadata {
    pub categorical: HashSet<String>,
    pub numeric_discrete: HashMap<String, bool>,
    pub categories: HashMap<String, serde_json::Value>,
}

/// Transient derived row; never an independently updated trial authority.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct TrialRow {
    pub distribution_metadata: DistributionMetadata,
    pub trial_id: u32,
    pub trial_number: u32,
    pub param_display: HashMap<String, f64>,
    pub param_category_label: HashMap<String, String>,
    pub objective_values: Vec<f64>,
    pub user_attrs_numeric: HashMap<String, f64>,
    pub user_attrs_string: HashMap<String, String>,
    pub user_attrs_json: HashMap<String, serde_json::Value>,
    pub constraint_values: Vec<f64>,
}
