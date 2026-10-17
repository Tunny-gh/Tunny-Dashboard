//! Temporary analysis projection, replaced by #252. Never an ingestion target.
use crate::data::extras::{StudyExtras, TrialExtra};
use crate::dataframe::{DataFrame, TrialRow};
use std::collections::BTreeSet;
use tdashboard_study::*;

pub struct Projection {
    pub stamp: AcquiredStamp,
    pub rows: Vec<TrialKey>,
    pub meta: super::StudyMeta,
    pub dataframe: DataFrame,
    pub extras: StudyExtras,
    pub history: Option<Vec<(u32, f64)>>,
}

/// Transitional caller-owned projection state (#252). Only the derived empty
/// schema is retained, never trial values or another writable domain authority.
#[derive(Debug, Default)]
pub struct TableProjection {
    input: Option<InputStamp>,
    schema: Option<DataFrame>,
}

impl TableProjection {
    pub fn project(&mut self, input: &StudyInput) -> Projection {
        let stamp = input.stamp().input;
        if !self.input.is_some_and(|previous| {
            previous.study == stamp.study
                && previous.generation == stamp.generation
                && previous.revision <= stamp.revision
        }) {
            self.schema = None;
        }
        let result = project_with_schema(input, self.schema.as_ref());
        self.schema = Some(result.dataframe.empty_with_schema());
        self.input = Some(stamp);
        result
    }
}

pub fn project(input: &StudyInput) -> Projection {
    project_with_schema(input, None)
}

fn project_with_schema(input: &StudyInput, previous: Option<&DataFrame>) -> Projection {
    let data = input.data();
    let metadata = data.metadata();
    let mut trials: Vec<_> = data
        .trials()
        .iter()
        .filter(|t| t.state == TrialState::Complete)
        .collect();
    trials.sort_by_key(|t| t.completed_order);
    // Declared names are source metadata; fallback column discovery remains
    // COMPLETE-only, including the valid empty-Study case.
    let fallback_width = trials
        .iter()
        .find(|t| !t.objectives.is_empty())
        .and_then(|t| t.objectives.keys().next_back())
        .map(|i| u64::from(i.0) + 1)
        .unwrap_or(0);
    let declared_names = metadata.objectives.iter().any(|o| o.name.is_some());
    let objectives: Vec<_> = metadata
        .objectives
        .iter()
        .filter(|o| {
            if declared_names {
                o.name.is_some()
            } else {
                u64::from(o.index.0) < fallback_width
            }
        })
        .collect();
    let objective_names: Vec<_> = objectives
        .iter()
        .map(|o| {
            o.name
                .clone()
                .unwrap_or_else(|| format!("obj{}", o.index.0))
        })
        .collect();
    let mut params = BTreeSet::new();
    let mut numeric_attrs = BTreeSet::new();
    let mut string_attrs = BTreeSet::new();
    let mut attrs = BTreeSet::new();
    let mut rows = Vec::new();
    for trial in &trials {
        let mut row = TrialRow {
            trial_id: trial.key.stored_id,
            trial_number: trial.number.0,
            ..Default::default()
        };
        for (name, observation) in trial.parameters.iter() {
            params.insert(name.clone());
            if let Some(Distribution::Numeric { discrete, .. }) = &observation.distribution {
                row.distribution_metadata
                    .numeric_discrete
                    .insert(name.clone(), *discrete);
            } else if matches!(
                observation.distribution,
                Some(Distribution::Categorical { .. })
            ) {
                // Presence distinguishes a declared but unavailable category from
                // the legacy label-only fallback in DataFrame construction.
                row.distribution_metadata
                    .numeric_discrete
                    .insert(name.clone(), false);
            }
            match &observation.value {
                ParameterValue::Numeric(n) => {
                    let n = if matches!(
                        observation.distribution,
                        Some(Distribution::Numeric {
                            kind: NumericKind::Int,
                            ..
                        })
                    ) && !observation.external_encoding
                    {
                        n.0.round()
                    } else {
                        n.0
                    };
                    row.param_display.insert(name.clone(), n);
                }
                ParameterValue::Category(value) => {
                    row.distribution_metadata.categorical.insert(name.clone());
                    row.distribution_metadata
                        .categories
                        .insert(name.clone(), (**value).clone());
                    let index = observation
                        .encoded_numeric
                        .filter(|_| !observation.external_encoding)
                        .map(|n| n.0.round())
                        .unwrap_or_else(|| match &observation.distribution {
                            Some(Distribution::Categorical { choices }) => choices
                                .iter()
                                .position(|v| v == value.as_ref())
                                .unwrap_or(0)
                                as f64,
                            _ => 0.0,
                        });
                    if !observation.external_encoding
                        || value.is_string()
                        || value.is_boolean()
                        || value.is_number()
                    {
                        row.param_display.insert(
                            name.clone(),
                            if observation.external_encoding {
                                value.as_f64().unwrap_or(index)
                            } else {
                                index
                            },
                        );
                    }
                    if !observation.external_encoding || value.is_string() || value.is_boolean() {
                        row.param_category_label.insert(
                            name.clone(),
                            value
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| value.to_string()),
                        );
                    }
                }
                ParameterValue::Unavailable(_) => {
                    if matches!(
                        observation.distribution,
                        Some(Distribution::Categorical { .. })
                    ) {
                        row.distribution_metadata.categorical.insert(name.clone());
                    }
                    if !observation.external_encoding {
                        let encoded = observation.encoded_numeric.map(|n| n.0).unwrap_or(f64::NAN);
                        row.param_display.insert(name.clone(), encoded.round());
                        if let Some(Distribution::Categorical { choices }) =
                            &observation.distribution
                        {
                            if let Some(value) = choices.get(encoded.round() as usize) {
                                row.param_category_label.insert(
                                    name.clone(),
                                    value
                                        .as_str()
                                        .map(str::to_string)
                                        .unwrap_or_else(|| value.to_string()),
                                );
                            }
                        }
                    }
                }
            }
        }
        row.objective_values = objectives
            .iter()
            .map(|o| {
                trial
                    .objectives
                    .get(&o.index)
                    .map(|n| n.0)
                    .unwrap_or(f64::NAN)
            })
            .collect();
        for (name, value) in trial.user_attributes.iter() {
            attrs.insert(name.clone());
            crate::io::user_attrs::insert_user_attr(
                name,
                (**value).clone(),
                &mut row.user_attrs_numeric,
                &mut row.user_attrs_string,
                &mut row.user_attrs_json,
            );
        }
        numeric_attrs.extend(row.user_attrs_numeric.keys().cloned());
        string_attrs.extend(row.user_attrs_string.keys().cloned());
        row.constraint_values = trial.constraints.values.iter().map(|n| n.0).collect();
        rows.push(row);
    }
    let param_names: Vec<_> = params.into_iter().collect();
    let schema = data.constraint_schema();
    let mut dataframe = previous
        .map(DataFrame::empty_with_schema)
        .unwrap_or_else(DataFrame::empty);
    dataframe.append_trials(
        &rows,
        &param_names,
        &objective_names,
        &numeric_attrs.into_iter().collect::<Vec<_>>(),
        &string_attrs.into_iter().collect::<Vec<_>>(),
        schema.width,
    );
    if schema.present && !dataframe.feasibility().has_constraints() {
        dataframe.mark_constrained();
    }
    let directions: Vec<_> = metadata
        .objectives
        .iter()
        .map(|o| match o.direction {
            Direction::Minimize => super::OptimizationDirection::Minimize,
            Direction::Maximize => super::OptimizationDirection::Maximize,
        })
        .collect();
    let history = if objective_names.len() == 1 && directions.len() == 1 {
        Some(crate::convergence::build_best_trial_history(
            &rows.iter().map(|r| r.trial_number).collect::<Vec<_>>(),
            &rows
                .iter()
                .map(|r| r.objective_values[0])
                .collect::<Vec<_>>(),
            matches!(directions[0], super::OptimizationDirection::Minimize),
        ))
    } else {
        None
    };
    let extras = StudyExtras {
        trials: data
            .trials()
            .iter()
            .map(|t| TrialExtra {
                trial_id: t.key.stored_id,
                trial_number: t.number.0,
                state: match t.state {
                    TrialState::Running => crate::data::extras::TrialState::Running,
                    TrialState::Complete => crate::data::extras::TrialState::Complete,
                    TrialState::Pruned => crate::data::extras::TrialState::Pruned,
                    TrialState::Fail => crate::data::extras::TrialState::Fail,
                    TrialState::Waiting => crate::data::extras::TrialState::Waiting,
                },
                datetime_start: t.datetime_start.map(|n| n.0),
                datetime_complete: t.datetime_complete.map(|n| n.0),
                intermediate_values: t.intermediate.iter().map(|(s, n)| (*s, n.0)).collect(),
            })
            .collect(),
    };
    Projection {
        stamp: input.stamp(),
        rows: trials.iter().map(|t| t.key).collect(),
        meta: super::StudyMeta {
            study_id: input.stamp().input.study.stored_id,
            name: metadata.name.clone(),
            directions,
            completed_trials: rows.len() as u32,
            total_trials: data.trials().len() as u32,
            param_names,
            objective_names,
            user_attr_names: attrs.into_iter().collect(),
            has_constraints: schema.present,
            param_bounds: metadata
                .declared_bounds
                .iter()
                .map(|(k, (l, h))| (k.clone(), (l.0, h.0)))
                .collect(),
        },
        dataframe,
        extras,
        history,
    }
}
