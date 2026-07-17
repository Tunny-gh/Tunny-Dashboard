use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::builders::{StudyBuilder, TrialBuilder};
use super::distribution::Distribution;
use super::types::OptimizationDirection;
use crate::io::datetime::parse_naive_datetime;

/// Intermediate state of journal parsing (study builders, trial builders, trial_id counter).
pub(super) struct ParserState {
    pub(super) studies: Vec<StudyBuilder>,
    pub(super) trial_builders: HashMap<u32, TrialBuilder>,
    pub(super) next_trial_id: u32,
    /// When Some(id), only create a TrialBuilder for trials with that study_id (used for Phase 2 on-demand parsing).
    pub(super) target_study_id: Option<u32>,
    /// Set of already-registered study names (used to skip duplicate create_study lines in O(1)).
    study_names: HashSet<String>,
}

/// Extracts the u64 value of `key` from a JSON object (returns `None` if missing or the wrong type).
#[inline]
pub(super) fn get_u64(json: &Value, key: &str) -> Option<u64> {
    json.get(key).and_then(|value| value.as_u64())
}

/// Extracts the string value of `key` from a JSON object (returns `None` if missing or the wrong type).
#[inline]
pub(super) fn get_str<'a>(json: &'a Value, key: &str) -> Option<&'a str> {
    json.get(key).and_then(|value| value.as_str())
}

impl ParserState {
    pub(super) fn new() -> Self {
        ParserState {
            studies: Vec::new(),
            trial_builders: HashMap::with_capacity(1024),
            next_trial_id: 0,
            target_study_id: None,
            study_names: HashSet::new(),
        }
    }

    pub(super) fn new_with_target(target_study_id: u32) -> Self {
        ParserState {
            studies: Vec::new(),
            trial_builders: HashMap::with_capacity(1024),
            next_trial_id: 0,
            target_study_id: Some(target_study_id),
            study_names: HashSet::new(),
        }
    }

    pub(super) fn process_op(&mut self, op: u8, json: &Value) {
        match op {
            0 => self.process_create_study(json),
            3 => self.process_set_study_system_attr(json),
            4 => self.process_create_trial(json),
            5 => self.process_set_trial_param(json),
            6 => self.process_set_trial_state_values(json),
            7 => self.process_set_trial_intermediate_value(json),
            8 => self.process_set_trial_user_attr(json),
            9 => self.process_set_trial_system_attr(json),
            _ => {}
        }
    }

    fn process_create_study(&mut self, json: &Value) {
        let name = get_str(json, "study_name").unwrap_or("").to_string();
        // A journal file may contain multiple create_study entries for the same study
        // (e.g. from parallel workers). Skip if the name is already registered
        // (HashSet lookup; the Vec linear scan was O(n) per create_study line).
        if !self.study_names.insert(name.clone()) {
            return;
        }
        let directions = json
            .get("directions")
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .map(|direction| match direction.as_u64() {
                        Some(1) => OptimizationDirection::Minimize,
                        Some(2) => OptimizationDirection::Maximize,
                        _ => OptimizationDirection::Minimize,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let study_id = self.studies.len() as u32;
        self.studies.push(StudyBuilder {
            study_id,
            name,
            directions,
            total_trials: 0,
            completed_trials: 0,
            param_names: HashSet::new(),
            objective_names: Vec::new(),
            user_attr_names: HashSet::new(),
            has_constraints: false,
            param_bounds: HashMap::new(),
        });
    }

    fn process_create_trial(&mut self, json: &Value) {
        let study_id = get_u64(json, "study_id").unwrap_or(0) as u32;
        if (study_id as usize) >= self.studies.len() {
            return;
        }

        let trial_id = self.next_trial_id;
        self.next_trial_id += 1;
        // Optuna's trial.number is the creation order within the study (0-based). The value of
        // total_trials before it is incremented becomes that trial's number.
        let trial_number = self.studies[study_id as usize].total_trials;
        self.studies[study_id as usize].total_trials += 1;

        // Phase 2 on-demand parsing: skip TrialBuilder creation for studies other than the target.
        // Since ops 5/6/8/9 reference trial_id explicitly, updates to a trial whose builder
        // doesn't exist naturally become no-ops, so no inconsistency results.
        if let Some(target) = self.target_study_id {
            if study_id != target {
                return;
            }
        }

        // Start datetime (naive unix seconds). None if not a string or invalid.
        let datetime_start = get_str(json, "datetime_start").and_then(parse_naive_datetime);

        if json.get("distributions").is_some() {
            let state = get_u64(json, "state").unwrap_or(0) as u8;
            let values = json
                .get("values")
                .and_then(|value| value.as_array())
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|item| item.as_f64())
                        .collect::<Vec<_>>()
                })
                .or_else(|| {
                    json.get("value")
                        .and_then(|value| value.as_f64())
                        .map(|value| vec![value])
                });

            let mut param_display: HashMap<String, f64> = HashMap::new();
            let mut param_category_label: HashMap<String, String> = HashMap::new();

            let dist_obj = json
                .get("distributions")
                .and_then(|value| value.as_object());
            if let Some(params_obj) = json.get("params").and_then(|value| value.as_object()) {
                for (name, value) in params_obj {
                    if let Some(number) = value.as_f64() {
                        param_display.insert(name.clone(), number);
                    } else if let Some(text) = value.as_str() {
                        let idx = dist_obj
                            .and_then(|distributions| distributions.get(name))
                            .map(|distribution_value| {
                                let distribution = Distribution::from_json(distribution_value);
                                if let Distribution::Categorical { choices } = &distribution {
                                    choices
                                        .iter()
                                        .position(|choice| choice.as_str() == Some(text))
                                        .unwrap_or(0)
                                } else {
                                    0
                                }
                            })
                            .unwrap_or(0);
                        param_display.insert(name.clone(), idx as f64);
                        param_category_label.insert(name.clone(), text.to_string());
                    } else if let Some(flag) = value.as_bool() {
                        let idx = dist_obj
                            .and_then(|distributions| distributions.get(name))
                            .map(|distribution_value| {
                                let distribution = Distribution::from_json(distribution_value);
                                if let Distribution::Categorical { choices } = &distribution {
                                    choices
                                        .iter()
                                        .position(|choice| choice.as_bool() == Some(flag))
                                        .unwrap_or(0)
                                } else {
                                    0
                                }
                            })
                            .unwrap_or(0);
                        param_display.insert(name.clone(), idx as f64);
                        param_category_label.insert(name.clone(), flag.to_string());
                    }
                }
            }

            // Record the declared range (low, high) for numeric parameters on the study (first occurrence only).
            if let Some(distributions) = dist_obj {
                for (name, distribution_value) in distributions {
                    if let Some(bounds) = Distribution::from_json(distribution_value).bounds() {
                        self.studies[study_id as usize]
                            .param_bounds
                            .entry(name.clone())
                            .or_insert(bounds);
                    }
                }
            }

            let mut user_attrs_numeric: HashMap<String, f64> = HashMap::new();
            let mut user_attrs_string: HashMap<String, String> = HashMap::new();
            if let Some(attrs) = json.get("user_attrs").and_then(|value| value.as_object()) {
                for (key, value) in attrs {
                    if let Some(number) = value.as_f64() {
                        user_attrs_numeric.insert(key.clone(), number);
                    } else if let Some(text) = value.as_str() {
                        user_attrs_string.insert(key.clone(), text.to_string());
                    }
                }
            }

            let mut constraint_values: Vec<f64> = Vec::new();
            let mut has_constraints = false;
            if let Some(sys_attrs) = json.get("system_attrs").and_then(|value| value.as_object()) {
                if let Some(constraints) = sys_attrs
                    .get("constraints")
                    .and_then(|value| value.as_array())
                {
                    constraint_values = constraints
                        .iter()
                        .filter_map(|value| value.as_f64())
                        .collect();
                    has_constraints = true;
                }
            }

            self.trial_builders.insert(
                trial_id,
                TrialBuilder {
                    study_id,
                    trial_number,
                    state,
                    values,
                    param_display,
                    param_category_label,
                    user_attrs_numeric,
                    user_attrs_string,
                    constraint_values,
                    has_constraints,
                    datetime_start,
                    datetime_complete: None,
                    intermediate_values: Vec::new(),
                },
            );
        } else {
            self.trial_builders.insert(
                trial_id,
                TrialBuilder {
                    study_id,
                    trial_number,
                    state: 0,
                    values: None,
                    param_display: HashMap::new(),
                    param_category_label: HashMap::new(),
                    user_attrs_numeric: HashMap::new(),
                    user_attrs_string: HashMap::new(),
                    constraint_values: Vec::new(),
                    has_constraints: false,
                    datetime_start,
                    datetime_complete: None,
                    intermediate_values: Vec::new(),
                },
            );
        }
    }

    fn process_set_trial_param(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let param_name = match get_str(json, "param_name") {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => return,
        };
        let internal = json
            .get("param_value_internal")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let distribution = json
            .get("distribution")
            .map(Distribution::from_json)
            .unwrap_or(Distribution::Uniform);

        // Record the declared range of numeric parameters on the study (first occurrence only).
        // Read the trial's study_id first, then update studies (to avoid overlapping borrows).
        if let Some(bounds) = distribution.bounds() {
            if let Some(study_id) = self.trial_builders.get(&trial_id).map(|t| t.study_id) {
                if let Some(study) = self.studies.get_mut(study_id as usize) {
                    study
                        .param_bounds
                        .entry(param_name.clone())
                        .or_insert(bounds);
                }
            }
        }

        if let Some(trial) = self.trial_builders.get_mut(&trial_id) {
            trial
                .param_display
                .insert(param_name.clone(), distribution.to_display_f64(internal));
            if let Some(label) = distribution.categorical_label(internal) {
                trial.param_category_label.insert(param_name, label);
            }
        }
    }

    fn process_set_trial_state_values(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let state = get_u64(json, "state").unwrap_or(0) as u8;
        let values = json
            .get("values")
            .and_then(|value| value.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|item| item.as_f64())
                    .collect::<Vec<_>>()
            });

        // Completion datetime (naive unix seconds). None if not a string or invalid.
        let datetime_complete = get_str(json, "datetime_complete").and_then(parse_naive_datetime);

        if let Some(trial) = self.trial_builders.get_mut(&trial_id) {
            trial.state = state;
            if let Some(updated_values) = values {
                trial.values = Some(updated_values);
            }
            if datetime_complete.is_some() {
                trial.datetime_complete = datetime_complete;
            }
        }
    }

    /// op_code=7 (SET_TRIAL_INTERMEDIATE_VALUE): appends an intermediate value to a trial.
    /// Fields: `trial_id`(u64), `step`(u64), `intermediate_value`(f64).
    /// Skipped if value is missing or not numeric.
    fn process_set_trial_intermediate_value(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let step = get_u64(json, "step").unwrap_or(0);
        let Some(value) = json.get("intermediate_value").and_then(|v| v.as_f64()) else {
            return;
        };
        if let Some(trial) = self.trial_builders.get_mut(&trial_id) {
            trial.intermediate_values.push((step, value));
        }
    }

    fn process_set_trial_user_attr(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let Some(attrs) = json.get("user_attr").and_then(|value| value.as_object()) else {
            return;
        };
        let Some(trial) = self.trial_builders.get_mut(&trial_id) else {
            return;
        };

        for (key, value) in attrs {
            if let Some(number) = value.as_f64() {
                trial.user_attrs_numeric.insert(key.clone(), number);
            } else if let Some(text) = value.as_str() {
                trial
                    .user_attrs_string
                    .insert(key.clone(), text.to_string());
            }
        }
    }

    fn process_set_trial_system_attr(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let Some(attrs) = json.get("system_attr").and_then(|value| value.as_object()) else {
            return;
        };
        let Some(trial) = self.trial_builders.get_mut(&trial_id) else {
            return;
        };

        if let Some(constraints) = attrs.get("constraints").and_then(|value| value.as_array()) {
            trial.constraint_values = constraints
                .iter()
                .filter_map(|value| value.as_f64())
                .collect();
            trial.has_constraints = true;
        }
    }

    fn process_set_study_system_attr(&mut self, json: &Value) {
        let study_id = get_u64(json, "study_id").unwrap_or(0) as u32;
        if (study_id as usize) >= self.studies.len() {
            return;
        }
        let Some(attrs) = json.get("system_attr").and_then(|value| value.as_object()) else {
            return;
        };
        if let Some(names_arr) = attrs
            .get("study:metric_names")
            .and_then(|value| value.as_array())
        {
            let names: Vec<String> = names_arr
                .iter()
                .filter_map(|value| value.as_str())
                .map(|name| name.to_string())
                .collect();
            if !names.is_empty() {
                self.studies[study_id as usize].objective_names = names;
            }
        }
    }
}
