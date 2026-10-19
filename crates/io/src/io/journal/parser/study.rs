//! Owned Journal-to-domain replay with fallible batch delivery.
//! Builders exist only during acquisition; only the caller's manager commits data.
use super::{get_u64, ParserState};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tdashboard_study::*;

pub(crate) fn distribution(value: &Value) -> Option<Distribution> {
    let parsed;
    let value = if let Some(text) = value.as_str() {
        parsed = serde_json::from_str::<Value>(text).ok()?;
        &parsed
    } else {
        value
    };
    let attrs = value.get("attributes").unwrap_or(value);
    let number = |name| attrs.get(name).and_then(Value::as_f64).map(Number);
    let log = attrs.get("log").and_then(Value::as_bool).unwrap_or(false);
    match value.get("name")?.as_str()? {
        "CategoricalDistribution" => Some(Distribution::Categorical {
            choices: Arc::new(
                attrs
                    .get("choices")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            ),
        }),
        name @ ("FloatDistribution" | "IntDistribution") => {
            let int = name == "IntDistribution";
            let step = number("step");
            let kind = if int {
                NumericKind::Int
            } else {
                NumericKind::Float
            };
            Some(Distribution::Numeric {
                kind,
                low: number("low"),
                high: number("high"),
                step,
                log,
                discrete: kind.is_discrete(step.map(|s| s.0)),
            })
        }
        _ => None,
    }
}
pub(crate) fn parameter(
    encoded: Option<&Value>,
    dist: Option<&Value>,
    external: bool,
) -> ParameterObservation {
    let distribution = dist.and_then(distribution);
    let value = match (&distribution, encoded) {
        (_, None) => ParameterValue::Unavailable(UnavailableValue::MissingEncodedValue),
        (Some(Distribution::Categorical { choices }), Some(value)) => {
            let category = if external {
                Some(value.clone())
            } else {
                value
                    .as_f64()
                    .filter(|n| n.is_finite() && *n >= 0.0 && n.fract() == 0.0)
                    .and_then(|n| choices.get(n as usize))
                    .cloned()
            };
            category
                .map(|v| ParameterValue::Category(Arc::new(v)))
                .unwrap_or(ParameterValue::Unavailable(
                    UnavailableValue::InvalidEncodedValue,
                ))
        }
        (_, Some(value)) => value
            .as_f64()
            .map(|n| ParameterValue::Numeric(Number(n)))
            .unwrap_or(ParameterValue::Unavailable(
                UnavailableValue::InvalidEncodedValue,
            )),
    };
    ParameterObservation {
        encoded_numeric: if matches!(distribution, Some(Distribution::Categorical { .. }))
            || matches!(value, ParameterValue::Unavailable(_))
        {
            encoded.and_then(Value::as_f64).map(Number)
        } else {
            None
        },
        value,
        distribution,
        external_encoding: external,
    }
}

/// Decode an observed number without losing nonfinite values in a JSON conversion.
pub(crate) fn parameter_number(
    value: f64,
    dist: Option<&Value>,
    external: bool,
) -> ParameterObservation {
    let mut observation = parameter(Some(&Value::from(value)), dist, external);
    if matches!(
        observation.distribution,
        Some(Distribution::Categorical { .. })
    ) {
        if !value.is_finite() {
            observation.value = ParameterValue::Unavailable(UnavailableValue::InvalidEncodedValue);
            observation.encoded_numeric = Some(Number(value));
        }
    } else {
        observation.value = ParameterValue::Numeric(Number(value));
    }
    observation
}

/// Ordered replay of acquired bytes, with fallible synchronous backpressure.
/// Recorded user/constraint attributes correct prior trials at batch boundaries
/// or EOF, without postponing the initial COMPLETE prefix (ADR-0022).
pub fn read_batches(
    bytes: &[u8],
    key: StudyKey,
    batch_size: usize,
    mut sink: impl FnMut(StudyBatch) -> Result<(), String>,
) -> Result<(), String> {
    read_batches_with_final(bytes, key, batch_size, |batch, _| sink(batch))
}

pub fn read_batches_with_final(
    bytes: &[u8],
    key: StudyKey,
    batch_size: usize,
    mut sink: impl FnMut(StudyBatch, bool) -> Result<(), String>,
) -> Result<(), String> {
    if bytes.is_empty() {
        return Err("Empty journal data".into());
    }
    let mut state = ParserState::new_with_target(key.stored_id);
    let mut params: BTreeMap<u32, BTreeMap<String, ParameterObservation>> = BTreeMap::new();
    let mut associations: BTreeMap<u32, Vec<ArtifactAssociation>> = BTreeMap::new();
    let mut dirty = BTreeSet::new();
    let mut terminal = BTreeSet::new();
    let mut orders = BTreeMap::new();
    let mut order = 0_u64;
    let mut completions = 0;
    let mut first = true;
    for line in String::from_utf8_lossy(bytes).lines() {
        let Ok(record) = crate::optuna_json::record(line.trim()) else {
            continue;
        };
        let json = &record.json;
        let Some(op) = get_u64(json, "op_code").and_then(|n| u8::try_from(n).ok()) else {
            continue;
        };
        let tid = if op == 4 {
            Some(state.next_trial_id)
        } else {
            get_u64(json, "trial_id").and_then(|n| u32::try_from(n).ok())
        };
        let was_terminal = tid.is_some_and(|t| terminal.contains(&t));
        let constraint_update = op == 9
            && json
                .get("system_attr")
                .and_then(Value::as_object)
                .is_some_and(|attrs| attrs.contains_key("constraints"));
        if was_terminal && op != 8 && !constraint_update {
            continue;
        }
        if op == 4 && state.next_trial_id == u32::MAX {
            return Err("Journal trial identity exhausted".into());
        }
        state.process_record(op, &record);
        if let Some(tid) = tid.filter(|t| state.trial_builders.contains_key(t)) {
            dirty.insert(tid);
            if op == 5 {
                if let Some(name) = json
                    .get("param_name")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    let observation = if let Some(value) = record
                        .root()
                        .get("param_value_internal")
                        .and_then(|v| record.number(v))
                    {
                        parameter_number(value, json.get("distribution"), false)
                    } else {
                        parameter(
                            json.get("param_value_internal"),
                            json.get("distribution"),
                            false,
                        )
                    };
                    params
                        .entry(tid)
                        .or_default()
                        .insert(name.to_string(), observation);
                }
            } else if op == 4 {
                if let Some(values) = json.get("params").and_then(Value::as_object) {
                    for (name, value) in values {
                        let dist = json.get("distributions").and_then(|d| d.get(name));
                        let observation = if let Some(number) = record
                            .root()
                            .get("params")
                            .and_then(|p| p.get(name))
                            .and_then(|v| record.number(v))
                            .filter(|number| !number.is_finite())
                        {
                            parameter_number(number, dist, true)
                        } else {
                            parameter(Some(value), dist, true)
                        };
                        params
                            .entry(tid)
                            .or_default()
                            .insert(name.clone(), observation);
                    }
                }
            }
            if let Some(attrs) = json
                .get(if op == 4 {
                    "system_attrs"
                } else {
                    "system_attr"
                })
                .and_then(Value::as_object)
                .filter(|_| !was_terminal)
            {
                for (name, value) in attrs {
                    let Some(id) = name.strip_prefix("artifacts:") else {
                        continue;
                    };
                    let Some(meta) = value
                        .as_str()
                        .and_then(|s| serde_json::from_str::<Value>(s).ok())
                    else {
                        continue;
                    };
                    let entries = associations.entry(tid).or_default();
                    let entry_order = u64::try_from(entries.len()).map_err(|e| e.to_string())?;
                    entries.push(ArtifactAssociation {
                        trial: TrialKey {
                            source: key.source,
                            stored_id: tid,
                        },
                        reference: meta
                            .get("artifact_id")
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(id)
                            .to_string(),
                        name: meta
                            .get("filename")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        mime: meta
                            .get("mimetype")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        order: entry_order,
                    });
                }
            }
            let trial = &state.trial_builders[&tid];
            if (op == 4 || op == 6) && matches!(trial.state, 1..=3) {
                terminal.insert(tid);
                if trial.state == 1 {
                    orders.insert(tid, order);
                    order = order.checked_add(1).ok_or("Completion order exhausted")?;
                    completions += 1;
                }
            }
        }
        if completions >= batch_size.max(1) {
            sink(
                batch(
                    &state,
                    key,
                    &mut dirty,
                    &params,
                    &associations,
                    &orders,
                    first,
                )?,
                false,
            )?;
            first = false;
            completions = 0;
        }
    }
    // Always deliver final metadata, even with no COMPLETE rows or no data change.
    sink(
        batch(
            &state,
            key,
            &mut dirty,
            &params,
            &associations,
            &orders,
            first,
        )?,
        true,
    )
}

/// The comparison caller's existing full-read boundary. Reuse the same supported
/// replay without publishing a streaming prefix, and retain full-reader row order
/// in acquisition evidence (ADR-0021).
pub fn read_full(bytes: &[u8], key: StudyKey) -> Result<StudyBatch, String> {
    let mut result = None;
    read_batches(bytes, key, usize::MAX, |batch| {
        result = Some(batch);
        Ok(())
    })?;
    let Some(StudyBatch::Replace {
        mut trials,
        metadata,
    }) = result
    else {
        return Err("Journal full read did not return an authoritative batch".into());
    };
    trials.sort_by_key(|t| t.key.stored_id);
    let mut order = 0_u64;
    for trial in &mut trials {
        if trial.state == TrialState::Complete {
            Arc::make_mut(trial).completed_order = Some(order);
            order = order.checked_add(1).ok_or("Completion order exhausted")?;
        }
    }
    Ok(StudyBatch::Replace { trials, metadata })
}

fn batch(
    state: &ParserState,
    key: StudyKey,
    dirty: &mut BTreeSet<u32>,
    params: &BTreeMap<u32, BTreeMap<String, ParameterObservation>>,
    associations: &BTreeMap<u32, Vec<ArtifactAssociation>>,
    orders: &BTreeMap<u32, u64>,
    first: bool,
) -> Result<StudyBatch, String> {
    let study = state
        .studies
        .get(key.stored_id as usize)
        .ok_or_else(|| format!("study_id {} not found", key.stored_id))?;
    let mut metadata = StudyMetadata {
        name: study.name.clone(),
        declared_bounds: study
            .param_bounds
            .iter()
            .map(|(k, (l, h))| (k.clone(), (Number(*l), Number(*h))))
            .collect(),
        ..Default::default()
    };
    let names = &study.objective_names;
    for (i, direction) in study.directions.iter().enumerate() {
        metadata.objectives.push(ObjectiveDescriptor {
            index: ObjectiveIndex(u32::try_from(i).map_err(|e| e.to_string())?),
            name: names.get(i).cloned(),
            direction: match direction {
                super::OptimizationDirection::Minimize => Direction::Minimize,
                super::OptimizationDirection::Maximize => Direction::Maximize,
            },
        });
    }
    for (id, trial) in &state.trial_builders {
        if let Some(parameters) = params.get(id) {
            metadata
                .observed_parameters
                .extend(parameters.keys().cloned());
        }
        metadata
            .user_attribute_names
            .extend(trial.attributes.keys().cloned());
    }
    let mut upserts = Vec::new();
    for id in std::mem::take(dirty) {
        let b = &state.trial_builders[&id];
        let trial_state = match b.state {
            0 => TrialState::Running,
            1 => TrialState::Complete,
            2 => TrialState::Pruned,
            3 => TrialState::Fail,
            4 => TrialState::Waiting,
            _ => return Err("Unknown trial state".into()),
        };
        let mut trial = TrialRecord::new(key, id, StudyTrialNumber(b.trial_number), trial_state);
        trial.parameters = Arc::new(params.get(&id).cloned().unwrap_or_default());
        trial.objectives = Arc::new(
            b.values
                .as_deref()
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    Ok((
                        ObjectiveIndex(u32::try_from(i).map_err(|e| e.to_string())?),
                        Number(*n),
                    ))
                })
                .collect::<Result<_, String>>()?,
        );
        trial.user_attributes = Arc::new(b.attributes.clone());
        trial.constraints = ConstraintObservation {
            present: b.has_constraints,
            values: Arc::new(b.constraint_values.iter().copied().map(Number).collect()),
        };
        trial.datetime_start = b.datetime_start.map(Number);
        trial.datetime_complete = b.datetime_complete.map(Number);
        let mut intermediate: Vec<_> = b
            .intermediate_values
            .iter()
            .map(|(s, n)| (*s, Number(*n)))
            .collect();
        intermediate.sort_by_key(|(s, _)| *s);
        trial.intermediate = Arc::new(intermediate);
        trial.artifacts = Arc::new(associations.get(&id).cloned().unwrap_or_default());
        trial.completed_order = orders.get(&id).copied();
        upserts.push(Arc::new(trial));
    }
    if first {
        return Ok(StudyBatch::Replace {
            trials: upserts,
            metadata,
        });
    }
    Ok(StudyBatch::Delta {
        upserts,
        removals: Vec::new(),
        metadata: MetadataEdit::Set(metadata),
    })
}
