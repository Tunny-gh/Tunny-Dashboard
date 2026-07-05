use std::collections::HashMap;

/// Documentation.
/// Documentation.
#[derive(Clone)]
pub struct TrialRow {
    /// ストレージ横断のグローバル trial_id（op_code=4 出現順）。
    pub trial_id: u32,
    /// Study 内 0 始まりの trial.number（Optuna の `trial.number`、作成順）。
    pub trial_number: u32,
    /// Documentation.
    pub param_display: HashMap<String, f64>,
    /// Documentation.
    pub param_category_label: HashMap<String, String>,
    /// objectivevalue list（obj0, obj1, ...）
    pub objective_values: Vec<f64>,
    /// user_attr numeric type（REQ-012）
    pub user_attrs_numeric: HashMap<String, f64>,
    /// user_attr string type（REQ-012）
    pub user_attrs_string: HashMap<String, String>,
    /// constraints value list（REQ-013）
    pub constraint_values: Vec<f64>,
}
