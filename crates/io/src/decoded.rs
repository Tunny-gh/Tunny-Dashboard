//! Acquisition-only decoded observations, consumed when building domain updates.
//! No analysis implementation, installed state, table cache or GUI resource.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tdashboard_study::*;

/// Declared numeric grouping eligibility and original categorical values.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DistributionMetadata {
    /// Declared categorical parameters, even when a value cannot be resolved.
    pub categorical: HashSet<String>,
    pub numeric_discrete: HashMap<String, bool>,
    pub categories: HashMap<String, serde_json::Value>,
}

/// Format-decoded COMPLETE observations used during CSV/RDB acquisition.
#[derive(Clone, Default, serde::Serialize)]
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

/// Acquisition-only record, before the caller supplies its source namespace.
#[derive(Default)]
pub(crate) struct DecodedTrial {
    pub id: u32,
    pub number: u32,
    pub parameters: std::collections::BTreeMap<String, ParameterObservation>,
    pub objectives: std::collections::BTreeMap<ObjectiveIndex, Number>,
    pub attributes: std::collections::BTreeMap<String, Arc<AttributeValue>>,
    pub constraints: ConstraintObservation,
}
pub(crate) fn metadata(meta: &crate::journal::parser::StudyMeta) -> StudyMetadata {
    StudyMetadata {
        name: meta.name.clone(),
        objectives: meta
            .directions
            .iter()
            .enumerate()
            .map(|(i, d)| ObjectiveDescriptor {
                index: ObjectiveIndex(i as u32),
                name: meta.objective_names.get(i).cloned(),
                direction: match d {
                    crate::journal::parser::OptimizationDirection::Minimize => Direction::Minimize,
                    crate::journal::parser::OptimizationDirection::Maximize => Direction::Maximize,
                },
            })
            .collect(),
        declared_bounds: meta
            .param_bounds
            .iter()
            .map(|(k, (l, h))| (k.clone(), (Number(*l), Number(*h))))
            .collect(),
        constrained: meta.has_constraints,
        ..Default::default()
    }
}
