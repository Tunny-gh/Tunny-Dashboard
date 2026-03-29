//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-101/journal-parser-requirements.md

use serde_json::Value;
use std::collections::{HashMap, HashSet};

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationDirection {
    Minimize, // directions[i] == 1 (Optuna StudyDirection.MINIMIZE)
    Maximize, // directions[i] == 2 (Optuna StudyDirection.MAXIMIZE)
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<OptimizationDirection>,
    pub completed_trials: u32,
    pub total_trials: u32,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
}

/// Documentation.
#[derive(Debug)]
pub struct ParseResult {
    pub studies: Vec<StudyMeta>,
    pub duration_ms: f64,
}

pub struct JournalParser;

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
/// Reference: optuna-journal-log-format.md §SET_TRIAL_PARAM distribution 🟢
#[derive(Debug)]
enum Distribution {
    /// Documentation.
    Float { log: bool },
    /// Documentation.
    Int { low: i64, step: i64, log: bool },
    /// Documentation.
    Categorical { choices: Vec<Value> },
    /// Documentation.
    Uniform,
}

impl Distribution {
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    fn from_json(json: &Value) -> Self {
        // Documentation.
        if let Some(s) = json.as_str() {
            if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                return Distribution::from_json(&parsed);
            }
            return Distribution::Uniform;
        }

        // Documentation.
        let attrs = json.get("attributes").unwrap_or(json);

        match get_str(json, "name").unwrap_or("") {
            "FloatDistribution" => Distribution::Float {
                log: attrs.get("log").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "IntDistribution" => Distribution::Int {
                low: attrs.get("low").and_then(|v| v.as_i64()).unwrap_or(0),
                // Documentation.
                step: attrs
                    .get("step")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1)
                    .max(1),
                log: attrs.get("log").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "CategoricalDistribution" => Distribution::Categorical {
                choices: attrs
                    .get("choices")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            },
            _ => Distribution::Uniform,
        }
    }

    /// Documentation.
    /// Documentation.
    fn to_display_f64(&self, internal: f64) -> f64 {
        match self {
            // REQ-010: FloatDistribution log=true → exp(v) 🟢
            Distribution::Float { log } => {
                if *log {
                    internal.exp()
                } else {
                    internal
                }
            }
            // REQ-010: IntDistribution — low + round(v) * step 🟡
            Distribution::Int { low, step, log } => {
                let rounded = if *log {
                    internal.exp().round() as i64
                } else {
                    internal.round() as i64
                };
                (*low + rounded * *step) as f64
            }
            // Documentation.
            Distribution::Categorical { .. } => internal.round(),
            Distribution::Uniform => internal,
        }
    }

    /// Documentation.
    fn categorical_label(&self, internal: f64) -> Option<String> {
        let Distribution::Categorical { choices } = self else {
            return None;
        };
        let idx = internal.round() as usize;
        choices.get(idx).map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            other => other.to_string(),
        })
    }
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
struct StudyBuilder {
    study_id: u32,
    name: String,
    directions: Vec<OptimizationDirection>,
    /// Documentation.
    total_trials: u32,
    /// Documentation.
    completed_trials: u32,
    param_names: HashSet<String>,
    /// Documentation.
    objective_names: Vec<String>,
    user_attr_names: HashSet<String>,
    has_constraints: bool,
}

/// Documentation.
struct TrialBuilder {
    study_id: u32,
    /// TrialState: 0=RUNNING 1=COMPLETE 2=PRUNED 3=FAIL 4=WAITING 🟢
    state: u8,
    values: Option<Vec<f64>>,
    /// Documentation.
    param_display: HashMap<String, f64>,
    /// Documentation.
    param_category_label: HashMap<String, String>,
    /// user_attr numeric type（REQ-012）🟢
    user_attrs_numeric: HashMap<String, f64>,
    /// user_attr string type（REQ-012）🟢
    user_attrs_string: HashMap<String, String>,
    /// constraint value list（REQ-013）🟢
    constraint_values: Vec<f64>,
    has_constraints: bool,
}

/// Documentation.
struct ParserState {
    studies: Vec<StudyBuilder>,
    /// Documentation.
    /// Documentation.
    trial_builders: HashMap<u32, TrialBuilder>,
    /// Documentation.
    next_trial_id: u32,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
#[inline]
fn get_u64(json: &Value, key: &str) -> Option<u64> {
    json.get(key).and_then(|v| v.as_u64())
}

/// Documentation.
#[inline]
fn get_str<'a>(json: &'a Value, key: &str) -> Option<&'a str> {
    json.get(key).and_then(|v| v.as_str())
}

// =============================================================================
// Documentation.
// =============================================================================

impl ParserState {
    fn new() -> Self {
        ParserState {
            studies: Vec::new(),
            // Documentation.
            trial_builders: HashMap::with_capacity(1024),
            next_trial_id: 0,
        }
    }

    /// Documentation.
    fn process_op(&mut self, op: u8, json: &Value) {
        match op {
            0 => self.process_create_study(json),
            // Documentation.
            4 => self.process_create_trial(json),
            5 => self.process_set_trial_param(json),
            6 => self.process_set_trial_state_values(json),
            8 => self.process_set_trial_user_attr(json),
            9 => self.process_set_trial_system_attr(json),
            // Documentation.
            _ => {}
        }
    }

    /// Documentation.
    fn process_create_study(&mut self, json: &Value) {
        let name = get_str(json, "study_name").unwrap_or("").to_string();
        let directions = json
            .get("directions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|d| {
                        match d.as_u64() {
                            Some(1) => OptimizationDirection::Minimize,
                            Some(2) => OptimizationDirection::Maximize,
                            // 0 = NOT_SET in Optuna; default to Minimize
                            _ => OptimizationDirection::Minimize,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Documentation.
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
        });
    }

    /// Documentation.
    fn process_create_trial(&mut self, json: &Value) {
        let study_id = get_u64(json, "study_id").unwrap_or(0) as u32;

        // Documentation.
        if (study_id as usize) >= self.studies.len() {
            return;
        }

        // Documentation.
        let trial_id = self.next_trial_id;
        self.next_trial_id += 1;

        // Documentation.
        self.studies[study_id as usize].total_trials += 1;

        self.trial_builders.insert(
            trial_id,
            TrialBuilder {
                study_id,
                state: 0, // Documentation.
                values: None,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: Vec::new(),
                has_constraints: false,
            },
        );
    }

    /// Documentation.
    fn process_set_trial_param(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let param_name = match get_str(json, "param_name") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return,
        };
        let internal = json
            .get("param_value_internal")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let dist = json
            .get("distribution")
            .map(Distribution::from_json)
            .unwrap_or(Distribution::Uniform);

        if let Some(trial) = self.trial_builders.get_mut(&trial_id) {
            // Documentation.
            trial
                .param_display
                .insert(param_name.clone(), dist.to_display_f64(internal));
            // Documentation.
            if let Some(label) = dist.categorical_label(internal) {
                trial.param_category_label.insert(param_name, label);
            }
        }
    }

    /// Documentation.
    /// Documentation.
    fn process_set_trial_state_values(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let state = get_u64(json, "state").unwrap_or(0) as u8;
        let values = json
            .get("values")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<_>>());

        if let Some(trial) = self.trial_builders.get_mut(&trial_id) {
            trial.state = state;
            if let Some(v) = values {
                trial.values = Some(v);
            }
        }
    }

    /// Documentation.
    fn process_set_trial_user_attr(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let Some(attrs) = json.get("user_attr").and_then(|v| v.as_object()) else {
            return;
        };
        let Some(trial) = self.trial_builders.get_mut(&trial_id) else {
            return;
        };

        for (key, val) in attrs {
            // Documentation.
            if let Some(n) = val.as_f64() {
                trial.user_attrs_numeric.insert(key.clone(), n);
            } else if let Some(s) = val.as_str() {
                trial.user_attrs_string.insert(key.clone(), s.to_string());
            }
            // Documentation.
        }
    }

    /// Documentation.
    fn process_set_trial_system_attr(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let Some(attrs) = json.get("system_attr").and_then(|v| v.as_object()) else {
            return;
        };
        let Some(trial) = self.trial_builders.get_mut(&trial_id) else {
            return;
        };

        // Documentation.
        // Documentation.
        if let Some(constraints) = attrs.get("constraints").and_then(|v| v.as_array()) {
            trial.constraint_values = constraints.iter().filter_map(|v| v.as_f64()).collect();
            trial.has_constraints = true;
        }
    }

    /// Documentation.
    ///
    /// Documentation.
    /// Documentation.
    /// Documentation.
    fn finalize(self) -> (Vec<StudyMeta>, Vec<crate::dataframe::DataFrame>) {
        use crate::dataframe::{DataFrame, TrialRow};

        let ParserState {
            mut studies,
            trial_builders,
            ..
        } = self;
        let n_studies = studies.len();

        // Documentation.
        // Documentation.
        let mut sorted_trials: Vec<(u32, TrialBuilder)> = trial_builders.into_iter().collect();
        sorted_trials.sort_by_key(|(id, _)| *id);

        // Documentation.
        // Documentation.
        let mut per_study_rows: Vec<Vec<TrialRow>> = (0..n_studies).map(|_| Vec::new()).collect();
        let mut per_study_unn: Vec<HashSet<String>> = // Documentation.
            (0..n_studies).map(|_| HashSet::new()).collect();
        let mut per_study_usn: Vec<HashSet<String>> = // Documentation.
            (0..n_studies).map(|_| HashSet::new()).collect();
        let mut per_study_max_c: Vec<usize> = vec![0; n_studies]; // Documentation.

        // Documentation.
        // Documentation.
        for (trial_id, trial) in sorted_trials {
            if trial.state != 1 {
                continue;
            }
            let study_idx = trial.study_id as usize;
            if study_idx >= n_studies {
                continue;
            }

            // Documentation.
            {
                let study = &mut studies[study_idx];
                study.completed_trials += 1;
                for name in trial.param_display.keys() {
                    study.param_names.insert(name.clone());
                }
                // Documentation.
                for name in trial.user_attrs_numeric.keys() {
                    study.user_attr_names.insert(name.clone());
                    per_study_unn[study_idx].insert(name.clone());
                }
                for name in trial.user_attrs_string.keys() {
                    study.user_attr_names.insert(name.clone());
                    per_study_usn[study_idx].insert(name.clone());
                }
                if trial.has_constraints {
                    study.has_constraints = true;
                }
                // Documentation.
                if study.objective_names.is_empty() {
                    if let Some(values) = &trial.values {
                        study.objective_names =
                            (0..values.len()).map(|i| format!("obj{i}")).collect();
                    }
                }
            } // Documentation.

            per_study_max_c[study_idx] =
                per_study_max_c[study_idx].max(trial.constraint_values.len());

            // Documentation.
            per_study_rows[study_idx].push(TrialRow {
                trial_id, // Documentation.
                param_display: trial.param_display,
                param_category_label: trial.param_category_label,
                objective_values: trial.values.unwrap_or_default(),
                user_attrs_numeric: trial.user_attrs_numeric,
                user_attrs_string: trial.user_attrs_string,
                constraint_values: trial.constraint_values,
            });
        }

        // Documentation.
        let mut study_metas = Vec::with_capacity(n_studies);
        let mut dataframes = Vec::with_capacity(n_studies);

        for (i, b) in studies.into_iter().enumerate() {
            let mut param_names: Vec<String> = b.param_names.into_iter().collect();
            param_names.sort();
            let mut user_attr_names: Vec<String> = b.user_attr_names.into_iter().collect();
            user_attr_names.sort();
            let objective_names = b.objective_names; // Documentation.

            study_metas.push(StudyMeta {
                study_id: b.study_id,
                name: b.name,
                directions: b.directions,
                completed_trials: b.completed_trials,
                total_trials: b.total_trials,
                param_names: param_names.clone(),
                objective_names: objective_names.clone(),
                user_attr_names,
                has_constraints: b.has_constraints,
            });

            // Documentation.
            // Documentation.
            let mut unn: Vec<String> = std::mem::take(&mut per_study_unn[i]).into_iter().collect();
            unn.sort();
            let mut usn: Vec<String> = std::mem::take(&mut per_study_usn[i]).into_iter().collect();
            usn.sort();

            dataframes.push(DataFrame::from_trials(
                &per_study_rows[i],
                &param_names,
                &objective_names,
                &unn,
                &usn,
                per_study_max_c[i],
            ));
        }

        (study_metas, dataframes)
    }
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    // Documentation.
    if data.is_empty() {
        crate::dataframe::store_dataframes(vec![]);
        return Ok(ParseResult {
            studies: vec![],
            duration_ms: 0.0,
        });
    }

    // Documentation.
    let text = String::from_utf8_lossy(data);

    // Documentation.
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        crate::dataframe::store_dataframes(vec![]);
        return Ok(ParseResult {
            studies: vec![],
            duration_ms: 0.0,
        });
    }

    let mut state = ParserState::new();
    let mut valid_lines: u32 = 0;

    for line in &lines {
        match serde_json::from_str::<Value>(line.trim()) {
            Ok(json) => {
                valid_lines += 1;
                if let Some(op) = get_u64(&json, "op_code") {
                    // Documentation.
                    #[allow(clippy::cast_possible_truncation)]
                    state.process_op(op as u8, &json);
                }
            }
            // Documentation.
            Err(_) => {}
        }
    }

    // Documentation.
    if valid_lines == 0 {
        return Err("No valid JSON lines found in journal".to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;
    // Documentation.
    let (studies, dataframes) = state.finalize();
    // Documentation.
    crate::dataframe::store_dataframes(dataframes);
    Ok(ParseResult {
        studies,
        duration_ms,
    })
}

// =============================================================================
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn to_bytes(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_101_01_create_study_basic() {
        // Documentation.
        let data = to_bytes(
            r#"{"op_code":0,"worker_id":"w1","study_name":"my_study","directions":[1,2]}"#,
        );
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies.len(), 1); // Documentation.
        assert_eq!(result.studies[0].name, "my_study"); // Documentation.
        assert_eq!(
            result.studies[0].directions,
            vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize
            ]
        ); // Documentation.
    }

    #[test]
    fn tc_101_02_create_trial_complete() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies[0].completed_trials, 1); // Documentation.
        assert_eq!(result.studies[0].total_trials, 1); // Documentation.
    }

    #[test]
    fn tc_101_03_float_distribution_no_log() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",\"param_value_internal\":0.5,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0,\"log\":false}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0].param_names.contains(&"x".to_string())); // Documentation.
    }

    #[test]
    fn tc_101_04_float_distribution_log_true() {
        // Documentation.
        let ln2: f64 = std::f64::consts::LN_2;
        let line = format!(
            "{{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"lr\",\"param_value_internal\":{ln2},\"distribution\":{{\"name\":\"FloatDistribution\",\"low\":1e-5,\"high\":1.0,\"log\":true}}}}"
        );
        let data = to_bytes(&format!(
            "{}\n{}\n{}\n{}\n",
            r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#,
            r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#,
            line,
            r#"{"op_code":6,"worker_id":"w","trial_id":0,"state":1,"values":[0.5],"datetime_complete":"2024-01-01T00:00:01.000000"}"#,
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0].param_names.contains(&"lr".to_string())); // Documentation.
    }

    #[test]
    fn tc_101_05_int_distribution_basic() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"n\",\"param_value_internal\":3.0,\"distribution\":{\"name\":\"IntDistribution\",\"low\":0,\"high\":10,\"step\":1,\"log\":false}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0].param_names.contains(&"n".to_string())); // Documentation.
    }

    #[test]
    fn tc_101_07_categorical_distribution_string() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":1.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0].param_names.contains(&"cat".to_string())); // Documentation.
    }

    #[test]
    fn tc_101_10_multiple_studies() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[0.2],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n",
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:03.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies.len(), 2); // Documentation.
        let sa = result.studies.iter().find(|s| s.name == "A").unwrap();
        let sb = result.studies.iter().find(|s| s.name == "B").unwrap();
        assert_eq!(sa.completed_trials, 2); // Documentation.
        assert_eq!(sb.completed_trials, 1); // Documentation.
    }

    #[test]
    fn tc_101_11_trial_id_sequential() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies[0].total_trials, 3); // Documentation.
    }

    #[test]
    fn tc_101_12_user_attr_numeric() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"loss\":0.123}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0]
            .user_attr_names
            .contains(&"loss".to_string())); // Documentation.
    }

    #[test]
    fn tc_101_13_user_attr_string() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"tag\":\"run_a\"}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0]
            .user_attr_names
            .contains(&"tag".to_string())); // Documentation.
    }

    #[test]
    fn tc_101_14_constraints_expansion() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-0.5,0.3]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0].has_constraints); // Documentation.
    }

    #[test]
    fn tc_101_15_constraints_all_feasible() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-1.0,-0.5,0.0]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0].has_constraints); // Documentation.
        assert_eq!(result.studies[0].completed_trials, 1); // Documentation.
    }

    #[test]
    fn tc_101_16_multi_objective_values() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1,2]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1,0.9],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies[0].objective_names.len(), 2); // Documentation.
    }

    #[test]
    fn tc_101_17_duration_ms_returned() {
        // Documentation.
        let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
        let result = parse_journal(&data).expect("translated");
        assert!(result.duration_ms >= 0.0); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_101_e01_incomplete_json_line_skipped() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data);
        assert!(result.is_ok()); // Documentation.
        assert_eq!(result.unwrap().studies[0].completed_trials, 1); // Documentation.
    }

    #[test]
    fn tc_101_e02_non_json_line_skipped() {
        // Documentation.
        let mut data = Vec::new();
        data.extend_from_slice(
            b"{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        );
        data.extend_from_slice(b"not-json-at-all\n");
        data.extend_from_slice(b"\xff\xfe\x00\n");
        data.extend_from_slice(b"{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n");
        data.extend_from_slice(b"{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n");
        let result = parse_journal(&data);
        assert!(result.is_ok()); // Documentation.
    }

    #[test]
    fn tc_101_e03_unknown_opcode_ignored() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":99,\"worker_id\":\"w\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data);
        assert!(result.is_ok()); // Documentation.
    }

    #[test]
    fn tc_101_e04_all_lines_invalid_returns_error() {
        // Documentation.
        let data: Vec<u8> = vec![0xff, 0xfe, 0x00, 0x01, 0x02];
        let result = parse_journal(&data);
        assert!(result.is_err()); // Documentation.
    }

    #[test]
    fn tc_101_e06_all_trials_not_complete() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies[0].completed_trials, 0); // Documentation.
        assert_eq!(result.studies[0].total_trials, 2); // Documentation.
    }

    #[test]
    fn tc_101_e07_distributed_optimization_overwrite() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w1\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w2\",\"trial_id\":0,\"state\":1,\"values\":[0.3],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies[0].completed_trials, 1); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_101_b01_empty_file() {
        // Documentation.
        let data: Vec<u8> = Vec::new();
        let result = parse_journal(&data);
        assert!(result.is_ok()); // Documentation.
        assert_eq!(result.unwrap().studies.len(), 0); // Documentation.
    }

    #[test]
    fn tc_101_b02_study_only_no_trials() {
        // Documentation.
        let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies.len(), 1); // Documentation.
        assert_eq!(result.studies[0].completed_trials, 0); // Documentation.
    }

    #[test]
    fn tc_101_b03_categorical_boundary_indices() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":0.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert!(result.studies[0].param_names.contains(&"cat".to_string())); // Documentation.
    }

    #[test]
    fn tc_101_b07_minimal_journal() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies[0].completed_trials, 1); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_101_p01_performance_50000_lines() {
        // Documentation.
        let mut lines = Vec::with_capacity(150_002);
        lines.push(
            r#"{"op_code":0,"worker_id":"w","study_name":"perf","directions":[0]}"#.to_string(),
        );
        for i in 0u32..50_000 {
            let val = f64::from(i) / 50_000.0;
            lines.push(r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#.to_string());
            lines.push(format!(r#"{{"op_code":5,"worker_id":"w","trial_id":{i},"param_name":"x","param_value_internal":{val},"distribution":{{"name":"FloatDistribution","low":0.0,"high":1.0,"log":false}}}}"#));
            lines.push(format!(r#"{{"op_code":6,"worker_id":"w","trial_id":{i},"state":1,"values":[{val}],"datetime_complete":"2024-01-01T00:00:01.000000"}}"#));
        }
        let data = lines.join("\n").into_bytes();

        let start = std::time::Instant::now();
        let result = parse_journal(&data).expect("50,000 translated");
        let elapsed_ms = start.elapsed().as_millis() as f64;

        assert_eq!(result.studies[0].completed_trials, 50_000); // Documentation.
        assert!(
            elapsed_ms < 5_000.0,
            "50,000 translated 5,000ms translated（translated: {elapsed_ms}ms）"
        ); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn distribution_float_log_false_identity() {
        // Documentation.
        let dist = Distribution::Float { log: false };
        assert!((dist.to_display_f64(0.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn distribution_float_log_true_exp() {
        // Documentation.
        let dist = Distribution::Float { log: true };
        let expected = std::f64::consts::LN_2.exp(); // ≈ 2.0
        assert!((dist.to_display_f64(std::f64::consts::LN_2) - expected).abs() < 1e-10);
    }

    #[test]
    fn distribution_int_step1() {
        // Documentation.
        let dist = Distribution::Int {
            low: 0,
            step: 1,
            log: false,
        };
        assert!((dist.to_display_f64(3.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn distribution_int_step2() {
        // Documentation.
        let dist = Distribution::Int {
            low: 0,
            step: 2,
            log: false,
        };
        // internal=2.0 → 0 + 2*2 = 4
        assert!((dist.to_display_f64(2.0) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn distribution_categorical_label() {
        // Documentation.
        let dist = Distribution::Categorical {
            choices: vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ],
        };
        assert_eq!(dist.categorical_label(1.0), Some("b".to_string()));
        assert_eq!(dist.categorical_label(0.0), Some("a".to_string()));
        assert_eq!(dist.categorical_label(2.0), Some("c".to_string()));
    }

    #[test]
    fn trial_builder_constraint_values_stored() {
        // Documentation.
        let trial = TrialBuilder {
            study_id: 0,
            state: 1,
            values: None,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![-1.0, -0.5, 0.0],
            has_constraints: true,
        };
        assert_eq!(trial.constraint_values.len(), 3);
        assert!(trial.constraint_values.iter().all(|&c| c <= 0.0)); // Documentation.
        let sum: f64 = trial.constraint_values.iter().sum();
        assert!((sum - (-1.5)).abs() < 1e-10); // constraint_sum 🟢
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn distribution_from_json_string_with_attributes() {
        // Documentation.
        let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.01, \"low\": -32.77, \"high\": 32.77, \"log\": false}}""#;
        let val: Value = serde_json::from_str(json_str).unwrap();
        let dist = Distribution::from_json(&val);
        // Documentation.
        assert!(matches!(dist, Distribution::Float { log: false }));
        // Documentation.
        assert!((dist.to_display_f64(7.4) - 7.4).abs() < 1e-10);
    }

    #[test]
    fn distribution_from_json_string_log_true() {
        // Documentation.
        let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.0, \"low\": 1e-5, \"high\": 1.0, \"log\": true}}""#;
        let val: Value = serde_json::from_str(json_str).unwrap();
        let dist = Distribution::from_json(&val);
        assert!(matches!(dist, Distribution::Float { log: true }));
        // Documentation.
        let ln2 = std::f64::consts::LN_2;
        assert!((dist.to_display_f64(ln2) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn distribution_from_json_object_with_attributes() {
        // Documentation.
        let val: Value = serde_json::from_str(
            r#"{"name": "IntDistribution", "attributes": {"low": 0, "high": 10, "step": 2, "log": false}}"#,
        ).unwrap();
        let dist = Distribution::from_json(&val);
        assert!(matches!(
            dist,
            Distribution::Int {
                low: 0,
                step: 2,
                log: false
            }
        ));
        // low=0, step=2: to_display = 0 + round(3.0)*2 = 6.0
        assert!((dist.to_display_f64(3.0) - 6.0).abs() < 1e-10);
    }

    #[test]
    fn parse_real_log_format_param_values() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-03-28T11:58:48.485367\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x0\",\"param_value_internal\":7.4,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x1\",\"param_value_internal\":17.43,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[21.64],\"datetime_complete\":\"2026-03-28T11:58:48.612043\"}\n"
        ));
        let result = parse_journal(&data).expect("translated");
        assert_eq!(result.studies[0].completed_trials, 1);
        assert!(result.studies[0].param_names.contains(&"x0".to_string()));
        assert!(result.studies[0].param_names.contains(&"x1".to_string()));
    }

    #[test]
    fn parse_real_log_file() {
        // Documentation.
        let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test.log");
        if !log_path.exists() {
            // Documentation.
            eprintln!("test.log not found at {:?}, skipping", log_path);
            return;
        }
        let data = std::fs::read(&log_path).expect("test.log translated");
        let result = parse_journal(&data).expect("test.log translated");

        // Documentation.
        assert!(result.studies.len() >= 2, "translated 2 Study translated");

        // Ackley Study: 10 parameter (Ackley_Variable0..9)
        let ackley = &result.studies[0];
        assert!(ackley.completed_trials > 0, "Ackley translated");
        assert_eq!(
            ackley.param_names.len(),
            10,
            "Ackley translated 10 parameter"
        );
        for i in 0..10 {
            let name = format!("Ackley_Variable{i}");
            assert!(
                ackley.param_names.contains(&name),
                "Ackley translated {name} translated"
            );
        }

        // DTLZ1 Study: 10 parameter (DTLZ1_Variable0..9)
        let dtlz = &result.studies[1];
        assert!(dtlz.completed_trials > 0, "DTLZ1 translated");
        assert_eq!(dtlz.param_names.len(), 10, "DTLZ1 translated 10 parameter");
        for i in 0..10 {
            let name = format!("DTLZ1_Variable{i}");
            assert!(
                dtlz.param_names.contains(&name),
                "DTLZ1 translated {name} translated"
            );
        }

        // Documentation.
        // Documentation.
        // Documentation.
        use crate::dataframe::with_df;
        let df_check = with_df(0, |df| {
            let param_cols = df.param_col_names();
            assert_eq!(
                param_cols.len(),
                10,
                "Ackley DataFrame translated 10 parametertranslated"
            );
            // Documentation.
            let col = df
                .get_numeric_column("Ackley_Variable0")
                .expect("translated");
            assert!(
                col[0].abs() > 1e-10,
                "Ackley_Variable0 translated: {}",
                col[0]
            );
        });
        assert!(df_check.is_some(), "DataFrame translated");
    }
}
