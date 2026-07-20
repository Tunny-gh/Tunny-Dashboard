//! Single-study row parsing: the Phase 2 query-building logic for `RDBStorage`.

use std::collections::HashMap;

use serde_json::Value;

use crate::data::dataframe::{DataFrame, TrialRow};
use crate::data::extras::{StudyExtras, TrialExtra, TrialState};
use crate::io::datetime::parse_naive_datetime;
use crate::io::journal::parser::distribution::Distribution;
use crate::io::journal::parser::StudyMeta;
use crate::io::rdb::backend::{OptunaBackend, SqlParam, SqlValue};

use super::study_list::{fetch_directions, fetch_metric_names, objective_names_for};
use super::{ensure_optuna_schema, query_scalar_i64};

/// Sorts numbers/strings using the same semantics as the journal parser (`state.rs`).
/// Number → numeric, String → string. Anything else (bool, array, object, null) is discarded
/// (journal's `process_set_trial_user_attr` discards them the same way, with no fallback to to_string).
fn classify_user_attr(
    value: &Value,
    key: &str,
    numeric: &mut HashMap<String, f64>,
    string: &mut HashMap<String, String>,
) {
    if let Some(number) = value.as_f64() {
        numeric.insert(key.to_string(), number);
    } else if let Some(text) = value.as_str() {
        string.insert(key.to_string(), text.to_string());
    }
}

struct TrialAccum {
    trial_number: u32,
    objective_values: Vec<(i64, f64)>,
    param_display: HashMap<String, f64>,
    param_category_label: HashMap<String, String>,
    user_attrs_numeric: HashMap<String, f64>,
    user_attrs_string: HashMap<String, String>,
    constraint_values: Vec<f64>,
}

/// Return value of `parse_single_study_rows`. Holds row-oriented data before `DataFrame` assembly.
///
/// Exposes the intermediate representation before calling `DataFrame::from_trials`
/// so the egui-app side can treat it as a single chunk in the same shape as
/// journal's `StudyStreamBatch` (meta + `Vec<TrialRow>` + column name sets).
pub struct RdbStudyRows {
    /// The finalized `StudyMeta` (includes param_bounds / param_names / user_attr_names).
    pub meta: StudyMeta,
    /// Row data for COMPLETE trials (ascending trial_id order).
    pub rows: Vec<TrialRow>,
    /// Parameter column names (sorted).
    pub param_names: Vec<String>,
    /// Objective column names.
    pub objective_names: Vec<String>,
    /// user_attr numeric column names (sorted).
    pub user_attr_numeric_names: Vec<String>,
    /// user_attr string column names (sorted).
    pub user_attr_string_names: Vec<String>,
    /// Maximum number of observed constraints.
    pub max_constraints: usize,
    /// Supplementary info for every trial (all states): state / datetime /
    /// intermediate values. Ascending trial_id order.
    pub extras: StudyExtras,
}

/// Phase 2: reads all COMPLETE trials for the given study, and returns the
/// finalized metadata and row data.
/// Since this is the intermediate representation before `DataFrame` assembly,
/// `parse_single_study` is just a wrapper that passes it straight to
/// `DataFrame::from_trials`.
pub fn parse_single_study_rows(
    backend: &mut dyn OptunaBackend,
    study_id: u32,
) -> Result<RdbStudyRows, String> {
    ensure_optuna_schema(backend)?;

    let sid = i64::from(study_id);

    // As in the original rusqlite implementation, both a missing study and a
    // query failure are folded into "study_id {id} not found in database".
    let name = backend
        .query(
            "SELECT study_name FROM studies WHERE study_id = ?",
            &[SqlParam::I64(sid)],
        )
        .ok()
        .and_then(|rows| rows.into_iter().next())
        .and_then(|row| row.into_iter().next())
        .and_then(SqlValue::into_text)
        .ok_or_else(|| format!("study_id {study_id} not found in database"))?;

    let directions = fetch_directions(backend, sid)?;
    let metric_names = fetch_metric_names(backend, sid)?;
    let objective_names = objective_names_for(&directions, metric_names);

    let total_trials = query_scalar_i64(
        backend,
        "SELECT COUNT(*) FROM trials WHERE study_id = ?",
        &[SqlParam::I64(sid)],
        "Failed to count trials",
    )?;

    // ── extras: supplementary info for all trials (all states) — state / datetime / intermediate values ──
    let extras = fetch_study_extras(backend, sid)?;

    // ── trials (COMPLETE only, ascending trial_id) ────────────────────────
    let mut trial_order: Vec<u32> = Vec::new();
    let mut accum: HashMap<u32, TrialAccum> = HashMap::new();
    backend.query_for_each(
        "SELECT trial_id, number FROM trials \
         WHERE study_id = ? AND state = 'COMPLETE' ORDER BY trial_id",
        &[SqlParam::I64(sid)],
        &mut |row| {
            let trial_id = row[0]
                .as_i64()
                .ok_or_else(|| "Failed to read trials: trial_id is not an integer".to_string())?;
            let number = row[1]
                .as_i64()
                .ok_or_else(|| "Failed to read trials: number is not an integer".to_string())?;
            // Skip rows whose id column doesn't fit in u32 (don't silently truncate).
            let (Ok(trial_id), Ok(number)) = (u32::try_from(trial_id), u32::try_from(number))
            else {
                return Ok(());
            };
            trial_order.push(trial_id);
            accum.insert(
                trial_id,
                TrialAccum {
                    trial_number: number,
                    objective_values: Vec::new(),
                    param_display: HashMap::new(),
                    param_category_label: HashMap::new(),
                    user_attrs_numeric: HashMap::new(),
                    user_attrs_string: HashMap::new(),
                    constraint_values: Vec::new(),
                },
            );
            Ok(())
        },
    )?;

    // ── trial_values ──────────────────────────────────────────────────
    backend.query_for_each(
        "SELECT tv.trial_id, tv.objective, tv.value, tv.value_type \
         FROM trial_values tv JOIN trials t ON tv.trial_id = t.trial_id \
         WHERE t.study_id = ? AND t.state = 'COMPLETE'",
        &[SqlParam::I64(sid)],
        &mut |row| {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial_values: trial_id is not an integer".to_string()
            })?;
            let objective = row[1].as_i64().ok_or_else(|| {
                "Failed to read trial_values: objective is not an integer".to_string()
            })?;
            let value = row[2].as_f64();
            let value_type = row[3]
                .as_text()
                .ok_or_else(|| "Failed to read trial_values: value_type is not text".to_string())?;
            let v = match value_type {
                "INF_POS" => f64::INFINITY,
                "INF_NEG" => f64::NEG_INFINITY,
                _ => value.unwrap_or(f64::NAN),
            };
            let Ok(trial_id) = u32::try_from(trial_id) else {
                return Ok(());
            };
            if let Some(trial) = accum.get_mut(&trial_id) {
                trial.objective_values.push((objective, v));
            }
            Ok(())
        },
    )?;

    // ── trial_params ─────────────────────────────────────────────────
    let mut param_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut param_bounds: HashMap<String, (f64, f64)> = HashMap::new();
    backend.query_for_each(
        "SELECT tp.trial_id, tp.param_name, tp.param_value, tp.distribution_json \
         FROM trial_params tp JOIN trials t ON tp.trial_id = t.trial_id \
         WHERE t.study_id = ? AND t.state = 'COMPLETE' ORDER BY tp.trial_id",
        &[SqlParam::I64(sid)],
        &mut |row| {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial_params: trial_id is not an integer".to_string()
            })?;
            let param_name = row[1]
                .as_text()
                .ok_or_else(|| "Failed to read trial_params: param_name is not text".to_string())?
                .to_string();
            let param_value = row[2].as_f64().ok_or_else(|| {
                "Failed to read trial_params: param_value is not numeric".to_string()
            })?;
            let distribution_json = row[3].as_text().ok_or_else(|| {
                "Failed to read trial_params: distribution_json is not text".to_string()
            })?;
            let dist_value: Value = serde_json::from_str(distribution_json).unwrap_or(Value::Null);
            let distribution = Distribution::from_json(&dist_value);

            if let Some(bounds) = distribution.bounds() {
                param_bounds.entry(param_name.clone()).or_insert(bounds);
            }

            param_names.insert(param_name.clone());
            let Ok(trial_id) = u32::try_from(trial_id) else {
                return Ok(());
            };
            if let Some(trial) = accum.get_mut(&trial_id) {
                trial
                    .param_display
                    .insert(param_name.clone(), distribution.to_display_f64(param_value));
                if let Some(label) = distribution.categorical_label(param_value) {
                    trial.param_category_label.insert(param_name, label);
                }
            }
            Ok(())
        },
    )?;

    // ── trial_user_attributes ────────────────────────────────────────
    let mut user_attr_numeric_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut user_attr_string_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    backend.query_for_each(
        "SELECT tua.trial_id, tua.key, tua.value_json \
         FROM trial_user_attributes tua JOIN trials t ON tua.trial_id = t.trial_id \
         WHERE t.study_id = ? AND t.state = 'COMPLETE'",
        &[SqlParam::I64(sid)],
        &mut |row| {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial_user_attributes: trial_id is not an integer".to_string()
            })?;
            let key = row[1].as_text().ok_or_else(|| {
                "Failed to read trial_user_attributes: key is not text".to_string()
            })?;
            let value_json = row[2].as_text().ok_or_else(|| {
                "Failed to read trial_user_attributes: value_json is not text".to_string()
            })?;
            let Ok(value) = serde_json::from_str::<Value>(value_json) else {
                return Ok(());
            };
            let Ok(trial_id) = u32::try_from(trial_id) else {
                return Ok(());
            };
            if let Some(trial) = accum.get_mut(&trial_id) {
                let before_numeric = trial.user_attrs_numeric.len();
                let before_string = trial.user_attrs_string.len();
                classify_user_attr(
                    &value,
                    key,
                    &mut trial.user_attrs_numeric,
                    &mut trial.user_attrs_string,
                );
                if trial.user_attrs_numeric.len() > before_numeric {
                    user_attr_numeric_names.insert(key.to_string());
                } else if trial.user_attrs_string.len() > before_string {
                    user_attr_string_names.insert(key.to_string());
                }
            }
            Ok(())
        },
    )?;

    // ── trial_system_attributes (constraints only) ───────────────────
    let mut has_constraints = false;
    let mut max_constraints = 0usize;
    backend.query_for_each(
        "SELECT tsa.trial_id, tsa.value_json \
         FROM trial_system_attributes tsa JOIN trials t ON tsa.trial_id = t.trial_id \
         WHERE t.study_id = ? AND t.state = 'COMPLETE' AND tsa.key = 'constraints'",
        &[SqlParam::I64(sid)],
        &mut |row| {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial_system_attributes: trial_id is not an integer".to_string()
            })?;
            let value_json = row[1].as_text().ok_or_else(|| {
                "Failed to read trial_system_attributes: value_json is not text".to_string()
            })?;
            let Ok(Value::Array(values)) = serde_json::from_str::<Value>(value_json) else {
                return Ok(());
            };
            let constraints: Vec<f64> = values.iter().filter_map(Value::as_f64).collect();
            let Ok(trial_id) = u32::try_from(trial_id) else {
                return Ok(());
            };
            if let Some(trial) = accum.get_mut(&trial_id) {
                max_constraints = max_constraints.max(constraints.len());
                trial.constraint_values = constraints;
                has_constraints = true;
            }
            Ok(())
        },
    )?;

    // ── assemble TrialRow in trial_id order ──────────────────────────
    let mut rows: Vec<TrialRow> = Vec::with_capacity(trial_order.len());
    for trial_id in trial_order {
        let Some(trial) = accum.remove(&trial_id) else {
            continue;
        };
        let mut objective_values = trial.objective_values;
        objective_values.sort_by_key(|(objective, _)| *objective);
        rows.push(TrialRow {
            trial_id,
            trial_number: trial.trial_number,
            param_display: trial.param_display,
            param_category_label: trial.param_category_label,
            objective_values: objective_values.into_iter().map(|(_, v)| v).collect(),
            user_attrs_numeric: trial.user_attrs_numeric,
            user_attrs_string: trial.user_attrs_string,
            constraint_values: trial.constraint_values,
        });
    }

    let param_names: Vec<String> = param_names.into_iter().collect();
    let user_attr_numeric_names: Vec<String> = user_attr_numeric_names.into_iter().collect();
    let user_attr_string_names: Vec<String> = user_attr_string_names.into_iter().collect();
    let mut user_attr_names = user_attr_numeric_names.clone();
    user_attr_names.extend(user_attr_string_names.iter().cloned());
    user_attr_names.sort();
    user_attr_names.dedup();

    let completed_trials = rows.len() as u32;

    #[allow(clippy::cast_sign_loss)]
    let meta = StudyMeta {
        study_id,
        name,
        directions,
        completed_trials,
        total_trials: total_trials as u32,
        param_names: param_names.clone(),
        objective_names: objective_names.clone(),
        user_attr_names,
        has_constraints,
        param_bounds,
    };

    Ok(RdbStudyRows {
        meta,
        rows,
        param_names,
        objective_names,
        user_attr_numeric_names,
        user_attr_string_names,
        max_constraints,
        extras,
    })
}

/// Reads supplementary info for every trial (all states) of the given study.
///
/// - Reads trial_id / number / state / datetime_start / datetime_complete
///   from `trials`, in ascending trial_id order. Datetimes are read after
///   being stringified via `backend.text_cast()`, then converted to naive
///   unix seconds by `parse_naive_datetime` (SQLite is natively TEXT, but the
///   stringification is applied unconditionally to absorb the difference
///   from PostgreSQL/MySQL's native timestamp types).
/// - Reads intermediate values from `trial_intermediate_values` and attaches
///   them to each trial in ascending step order. This table may not exist in
///   older DBs, so its presence is checked via `backend.table_exists`; if
///   absent, intermediate values are left empty. `value_type` is interpreted
///   with the same semantics as trial_values (FINITE/INF_POS/INF_NEG/NAN).
fn fetch_study_extras(backend: &mut dyn OptunaBackend, sid: i64) -> Result<StudyExtras, String> {
    // trial_id -> index within extras. Used to attach intermediate values.
    let mut index_of: HashMap<u32, usize> = HashMap::new();
    let mut trials: Vec<TrialExtra> = Vec::new();
    {
        let sql = format!(
            "SELECT trial_id, number, state, {}, {} \
             FROM trials WHERE study_id = ? ORDER BY trial_id",
            backend.text_cast("datetime_start"),
            backend.text_cast("datetime_complete"),
        );
        backend.query_for_each(&sql, &[SqlParam::I64(sid)], &mut |row| {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trials for extras: trial_id is not an integer".to_string()
            })?;
            let number = row[1].as_i64().ok_or_else(|| {
                "Failed to read trials for extras: number is not an integer".to_string()
            })?;
            let state = row[2]
                .as_text()
                .ok_or_else(|| "Failed to read trials for extras: state is not text".to_string())?;
            let datetime_start = row[3].as_text();
            let datetime_complete = row[4].as_text();

            // Skip rows whose id column doesn't fit in u32 (don't silently truncate).
            let (Ok(trial_id), Ok(number)) = (u32::try_from(trial_id), u32::try_from(number))
            else {
                return Ok(());
            };

            index_of.insert(trial_id, trials.len());
            trials.push(TrialExtra {
                trial_id,
                trial_number: number,
                state: TrialState::from_rdb_str(state),
                datetime_start: datetime_start.and_then(parse_naive_datetime),
                datetime_complete: datetime_complete.and_then(parse_naive_datetime),
                intermediate_values: Vec::new(),
            });
            Ok(())
        })?;
    }

    // Check whether the trial_intermediate_values table exists (may be absent in older DBs).
    let has_intermediate_table = backend
        .table_exists("trial_intermediate_values")
        .map_err(|e| format!("Failed to inspect intermediate values table: {e}"))?;

    if has_intermediate_table {
        backend.query_for_each(
            "SELECT tiv.trial_id, tiv.step, tiv.intermediate_value, tiv.intermediate_value_type \
             FROM trial_intermediate_values tiv \
             JOIN trials t ON tiv.trial_id = t.trial_id \
             WHERE t.study_id = ? ORDER BY tiv.trial_id, tiv.step",
            &[SqlParam::I64(sid)],
            &mut |row| {
                let trial_id = row[0].as_i64().ok_or_else(|| {
                    "Failed to read trial_intermediate_values: trial_id is not an integer"
                        .to_string()
                })?;
                let step = row[1].as_i64().ok_or_else(|| {
                    "Failed to read trial_intermediate_values: step is not an integer".to_string()
                })?;
                let value = row[2].as_f64();
                let value_type = row[3].as_text().ok_or_else(|| {
                    "Failed to read trial_intermediate_values: intermediate_value_type is not text"
                        .to_string()
                })?;
                let v = match value_type {
                    "INF_POS" => f64::INFINITY,
                    "INF_NEG" => f64::NEG_INFINITY,
                    "NAN" => f64::NAN,
                    _ => value.unwrap_or(f64::NAN),
                };
                let (Ok(trial_id), Ok(step)) = (u32::try_from(trial_id), u64::try_from(step))
                else {
                    return Ok(());
                };
                if let Some(&idx) = index_of.get(&trial_id) {
                    trials[idx].intermediate_values.push((step, v));
                }
                Ok(())
            },
        )?;
    }

    // Sort by ascending step as a safety net (we trust SQL's ORDER BY, but this is idempotent).
    for trial in &mut trials {
        trial.intermediate_values.sort_by_key(|(step, _)| *step);
    }

    Ok(StudyExtras { trials })
}

/// Phase 2: reads all COMPLETE trials for the given study, and returns
/// (finalized metadata, `DataFrame`, `StudyExtras`)
/// (same output contract as journal's `parse_single_study`).
pub fn parse_single_study(
    backend: &mut dyn OptunaBackend,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let RdbStudyRows {
        meta,
        rows,
        param_names,
        objective_names,
        user_attr_numeric_names,
        user_attr_string_names,
        max_constraints,
        extras,
    } = parse_single_study_rows(backend, study_id)?;

    let df = DataFrame::from_trials(
        &rows,
        &param_names,
        &objective_names,
        &user_attr_numeric_names,
        &user_attr_string_names,
        max_constraints,
    );

    Ok((meta, df, extras))
}
