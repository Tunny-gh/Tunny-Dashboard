//! Study-list scanning: the Phase 1 query-building logic for `RDBStorage`.

use std::collections::HashMap;

use serde_json::Value;

use crate::io::journal::parser::{OptimizationDirection, StudyMeta};
use crate::io::rdb::backend::{OptunaBackend, SqlParam, SqlValue};

use super::ensure_optuna_schema;

pub(super) fn fetch_directions(
    backend: &mut dyn OptunaBackend,
    study_id: i64,
) -> Result<Vec<OptimizationDirection>, String> {
    let rows = backend
        .query(
            "SELECT direction FROM study_directions WHERE study_id = ? ORDER BY objective",
            &[SqlParam::I64(study_id)],
        )
        .map_err(|e| format!("Failed to query study_directions: {e}"))?;
    let mut directions = Vec::with_capacity(rows.len());
    for row in rows {
        let direction = row[0]
            .as_text()
            .ok_or_else(|| "Failed to read study_directions: direction is not text".to_string())?;
        directions.push(direction_from_str(direction));
    }
    Ok(directions)
}

/// Reads objective names from `study_system_attributes`'s `study:metric_names` (empty if absent).
pub(super) fn fetch_metric_names(
    backend: &mut dyn OptunaBackend,
    study_id: i64,
) -> Result<Vec<String>, String> {
    // `key` is a reserved word in MySQL/MariaDB, so as with other queries it is
    // referenced qualified by table name (an alias-qualified reference like
    // `tsa.key` is fine, but unqualified would be a syntax error).
    let rows = backend
        .query(
            "SELECT value_json FROM study_system_attributes \
             WHERE study_id = ? AND study_system_attributes.key = 'study:metric_names'",
            &[SqlParam::I64(study_id)],
        )
        .map_err(|e| format!("Failed to query study_system_attributes: {e}"))?;

    let value_json = rows
        .into_iter()
        .next()
        .and_then(|row| row.into_iter().next())
        .and_then(SqlValue::into_text);

    Ok(value_json
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default())
}

pub(super) fn objective_names_for(
    directions: &[OptimizationDirection],
    metric_names: Vec<String>,
) -> Vec<String> {
    if !metric_names.is_empty() {
        metric_names
    } else {
        (0..directions.len()).map(|i| format!("obj{i}")).collect()
    }
}

fn direction_from_str(direction: &str) -> OptimizationDirection {
    if direction == "MAXIMIZE" {
        OptimizationDirection::Maximize
    } else {
        OptimizationDirection::Minimize
    }
}

/// Fetches the optimization direction for every study, per study_id, in a
/// single query (avoids N+1 in `scan_study_list`). Read in ascending
/// `objective` order, so within each study the directions are ordered by
/// objective index.
fn fetch_directions_by_study(
    backend: &mut dyn OptunaBackend,
) -> Result<HashMap<i64, Vec<OptimizationDirection>>, String> {
    let mut map: HashMap<i64, Vec<OptimizationDirection>> = HashMap::new();
    backend.query_for_each(
        "SELECT study_id, direction FROM study_directions ORDER BY study_id, objective",
        &[],
        &mut |row| {
            let study_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read study_directions: study_id is not an integer".to_string()
            })?;
            let direction = row[1].as_text().ok_or_else(|| {
                "Failed to read study_directions: direction is not text".to_string()
            })?;
            map.entry(study_id)
                .or_default()
                .push(direction_from_str(direction));
            Ok(())
        },
    )?;
    Ok(map)
}

/// Fetches every study's `study:metric_names` (objective names), per
/// study_id, in a single query (avoids N+1 in `scan_study_list`). A study
/// with no value does not appear in the map.
fn fetch_metric_names_by_study(
    backend: &mut dyn OptunaBackend,
) -> Result<HashMap<i64, Vec<String>>, String> {
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    backend.query_for_each(
        "SELECT study_id, value_json FROM study_system_attributes \
         WHERE study_system_attributes.key = 'study:metric_names'",
        &[],
        &mut |row| {
            let study_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read study_system_attributes: study_id is not an integer".to_string()
            })?;
            let names = row[1]
                .as_text()
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .and_then(|v| v.as_array().cloned())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();
            map.insert(study_id, names);
            Ok(())
        },
    )?;
    Ok(map)
}

/// Fetches the trial count (completed/total) for every study, per study_id, in a single query.
///
/// To avoid the N+1 pattern where `scan_study_list` issues two `COUNT(*)`
/// queries per study, this reads all studies at once with
/// `GROUP BY study_id, state` and folds them into a
/// `study_id -> (completed_trials, total_trials)` map.
fn fetch_trial_counts_by_study(
    backend: &mut dyn OptunaBackend,
) -> Result<HashMap<i64, (i64, i64)>, String> {
    let mut counts: HashMap<i64, (i64, i64)> = HashMap::new();
    backend.query_for_each(
        "SELECT study_id, state, COUNT(*) FROM trials GROUP BY study_id, state",
        &[],
        &mut |row| {
            let study_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial counts: study_id is not an integer".to_string()
            })?;
            let state = row[1]
                .as_text()
                .ok_or_else(|| "Failed to read trial counts: state is not text".to_string())?;
            let count = row[2].as_i64().ok_or_else(|| {
                "Failed to read trial counts: count is not an integer".to_string()
            })?;

            let entry = counts.entry(study_id).or_insert((0, 0));
            entry.1 += count; // total_trials
            if state == "COMPLETE" {
                entry.0 += count; // completed_trials
            }
            Ok(())
        },
    )?;
    Ok(counts)
}

/// Fetches, in a single query, the set of study_ids that have at least one
/// trial with a `constraints` system attribute (avoids N+1 in
/// `scan_study_list`, used for the `has_constraints` check).
fn fetch_studies_with_constraints(
    backend: &mut dyn OptunaBackend,
) -> Result<std::collections::HashSet<i64>, String> {
    let mut study_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    backend.query_for_each(
        "SELECT DISTINCT t.study_id FROM trial_system_attributes tsa \
         JOIN trials t ON tsa.trial_id = t.trial_id WHERE tsa.key = 'constraints'",
        &[],
        &mut |row| {
            let study_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read constraints: study_id is not an integer".to_string()
            })?;
            study_ids.insert(study_id);
            Ok(())
        },
    )?;
    Ok(study_ids)
}

/// Phase 1: opens the DB and returns the list of studies (plays the same role
/// as journal's `scan_study_list`). Since completed_trials / total_trials can
/// be obtained cheaply, they are filled with real values (unlike journal
/// scan, they are not zero-filled). Details such as param_names are finalized in Phase 2.
pub fn scan_study_list(backend: &mut dyn OptunaBackend) -> Result<Vec<StudyMeta>, String> {
    ensure_optuna_schema(backend)?;

    let rows = backend
        .query(
            "SELECT study_id, study_name FROM studies ORDER BY study_id",
            &[],
        )
        .map_err(|e| format!("Failed to query studies: {e}"))?;

    if rows.is_empty() {
        return Err("No studies found in database".to_string());
    }

    let mut studies_raw = Vec::with_capacity(rows.len());
    for row in rows {
        let study_id = row[0]
            .as_i64()
            .ok_or_else(|| "Failed to read studies: study_id is not an integer".to_string())?;
        let name = row[1]
            .as_text()
            .ok_or_else(|| "Failed to read studies: study_name is not text".to_string())?
            .to_string();
        studies_raw.push((study_id, name));
    }

    // To avoid per-study N+1 queries (directions, metric_names,
    // completed/total counts, presence of constraints), fetch all studies'
    // worth up front with one query each.
    let mut directions_by_study = fetch_directions_by_study(backend)?;
    let mut metric_names_by_study = fetch_metric_names_by_study(backend)?;
    let trial_counts = fetch_trial_counts_by_study(backend)?;
    let studies_with_constraints = fetch_studies_with_constraints(backend)?;

    let mut studies = Vec::with_capacity(studies_raw.len());
    for (study_id, name) in studies_raw {
        // An out-of-range study_id does not fit in StudyMeta.study_id (u32), so skip it.
        let Ok(study_id_u32) = u32::try_from(study_id) else {
            continue;
        };
        let directions = directions_by_study.remove(&study_id).unwrap_or_default();
        let metric_names = metric_names_by_study.remove(&study_id).unwrap_or_default();
        let objective_names = objective_names_for(&directions, metric_names);

        let (completed_trials, total_trials) =
            trial_counts.get(&study_id).copied().unwrap_or((0, 0));
        let has_constraints = studies_with_constraints.contains(&study_id);

        #[allow(clippy::cast_sign_loss)]
        studies.push(StudyMeta {
            study_id: study_id_u32,
            name,
            directions,
            completed_trials: completed_trials as u32,
            total_trials: total_trials as u32,
            param_names: vec![],
            objective_names,
            user_attr_names: vec![],
            has_constraints,
            param_bounds: HashMap::new(),
        });
    }

    Ok(studies)
}
