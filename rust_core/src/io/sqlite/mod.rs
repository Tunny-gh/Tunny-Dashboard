//! Optuna RDBStorage (SQLite) reader.
//!
//! Reads an Optuna `sqlite:///xxx.db` storage file and exposes the same
//! output contract (`StudyMeta` / `DataFrame`) as the journal parser, so
//! downstream code (UI, export, ...) does not need to distinguish the
//! storage backend.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;

use crate::data::dataframe::{DataFrame, TrialRow};
use crate::io::journal::parser::distribution::Distribution;
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};

#[cfg(test)]
mod tests;

fn open_readonly(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Failed to open SQLite database: {e}"))
}

/// Optuna スキーマかどうかを `studies` テーブルの有無で判定する。
fn ensure_optuna_schema(conn: &Connection) -> Result<(), String> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='studies')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to inspect database schema: {e}"))?;
    if !exists {
        return Err("Not an Optuna SQLite storage: 'studies' table not found".to_string());
    }
    Ok(())
}

fn fetch_directions(
    conn: &Connection,
    study_id: i64,
) -> Result<Vec<OptimizationDirection>, String> {
    let mut stmt = conn
        .prepare("SELECT direction FROM study_directions WHERE study_id = ?1 ORDER BY objective")
        .map_err(|e| format!("Failed to query study_directions: {e}"))?;
    let directions = stmt
        .query_map([study_id], |row| {
            let direction: String = row.get(0)?;
            Ok(if direction == "MAXIMIZE" {
                OptimizationDirection::Maximize
            } else {
                OptimizationDirection::Minimize
            })
        })
        .map_err(|e| format!("Failed to query study_directions: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read study_directions: {e}"))?;
    Ok(directions)
}

/// `study_system_attributes` の `study:metric_names` から目的名を読む（無ければ空）。
fn fetch_metric_names(conn: &Connection, study_id: i64) -> Result<Vec<String>, String> {
    let value_json: Option<String> = conn
        .query_row(
            "SELECT value_json FROM study_system_attributes \
             WHERE study_id = ?1 AND key = 'study:metric_names'",
            [study_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to query study_system_attributes: {e}"))?;

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

fn objective_names_for(
    directions: &[OptimizationDirection],
    metric_names: Vec<String>,
) -> Vec<String> {
    if !metric_names.is_empty() {
        metric_names
    } else {
        (0..directions.len()).map(|i| format!("obj{i}")).collect()
    }
}

/// Phase 1: DB を開いて Study 一覧を返す（journal の `scan_study_list` と同じ役割）。
/// completed_trials / total_trials は SQLite では安価に取れるため実数を入れる
/// （journal scan と異なり 0 埋めではない）。param_names 等の詳細は Phase 2 で確定する。
pub fn scan_study_list(path: &Path) -> Result<Vec<StudyMeta>, String> {
    let conn = open_readonly(path)?;
    ensure_optuna_schema(&conn)?;

    let mut stmt = conn
        .prepare("SELECT study_id, study_name FROM studies ORDER BY study_id")
        .map_err(|e| format!("Failed to query studies: {e}"))?;
    let studies_raw: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("Failed to query studies: {e}"))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Failed to read studies: {e}"))?;
    drop(stmt);

    if studies_raw.is_empty() {
        return Err("No studies found in database".to_string());
    }

    let mut studies = Vec::with_capacity(studies_raw.len());
    for (study_id, name) in studies_raw {
        let directions = fetch_directions(&conn, study_id)?;
        let metric_names = fetch_metric_names(&conn, study_id)?;
        let objective_names = objective_names_for(&directions, metric_names);

        let completed_trials: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM trials WHERE study_id = ?1 AND state = 'COMPLETE'",
                [study_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count completed trials: {e}"))?;
        let total_trials: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM trials WHERE study_id = ?1",
                [study_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count trials: {e}"))?;
        let has_constraints: bool = conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM trial_system_attributes tsa
                    JOIN trials t ON tsa.trial_id = t.trial_id
                    WHERE t.study_id = ?1 AND tsa.key = 'constraints'
                )",
                [study_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to check constraints: {e}"))?;

        #[allow(clippy::cast_sign_loss)]
        studies.push(StudyMeta {
            study_id: study_id as u32,
            name,
            directions,
            completed_trials,
            total_trials,
            param_names: vec![],
            objective_names,
            user_attr_names: vec![],
            has_constraints,
            param_bounds: HashMap::new(),
        });
    }

    Ok(studies)
}

/// 数値/文字列を journal パーサ (`state.rs`) と同じ意味論で振り分ける。
/// Number → numeric、String → string。それ以外（bool, array, object, null）は破棄する
/// （journal の `process_set_trial_user_attr` も同様に破棄しており、to_string にはフォールバックしない）。
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

/// `parse_single_study_rows` の戻り値。`DataFrame` 組み立て前の行指向データを保持する。
///
/// egui-app 側が journal の `StudyStreamBatch` と同じ形（meta + `Vec<TrialRow>` + 列名集合）で
/// 単一チャンクとして扱えるように、`DataFrame::from_trials` を呼ぶ前の中間表現を公開する。
pub struct SqliteStudyRows {
    /// 確定済み `StudyMeta`（param_bounds / param_names / user_attr_names 含む）。
    pub meta: StudyMeta,
    /// COMPLETE trial の行データ（trial_id 昇順）。
    pub rows: Vec<TrialRow>,
    /// パラメータ列名（ソート済み）。
    pub param_names: Vec<String>,
    /// 目的列名。
    pub objective_names: Vec<String>,
    /// user_attr 数値列名（ソート済み）。
    pub user_attr_numeric_names: Vec<String>,
    /// user_attr 文字列列名（ソート済み）。
    pub user_attr_string_names: Vec<String>,
    /// 観測した制約数の最大値。
    pub max_constraints: usize,
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、確定メタと行データを返す。
/// `DataFrame` 組み立て前の中間表現なので、`parse_single_study` はこれをそのまま
/// `DataFrame::from_trials` に渡すだけのラッパーになる。
pub fn parse_single_study_rows(path: &Path, study_id: u32) -> Result<SqliteStudyRows, String> {
    let conn = open_readonly(path)?;
    ensure_optuna_schema(&conn)?;

    let sid = i64::from(study_id);

    let name: String = conn
        .query_row(
            "SELECT study_name FROM studies WHERE study_id = ?1",
            [sid],
            |row| row.get(0),
        )
        .map_err(|_| format!("study_id {study_id} not found in database"))?;

    let directions = fetch_directions(&conn, sid)?;
    let metric_names = fetch_metric_names(&conn, sid)?;
    let objective_names = objective_names_for(&directions, metric_names);

    let total_trials: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM trials WHERE study_id = ?1",
            [sid],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count trials: {e}"))?;

    // ── trials (COMPLETE のみ、trial_id 昇順) ────────────────────────────
    let mut trial_order: Vec<u32> = Vec::new();
    let mut accum: HashMap<u32, TrialAccum> = HashMap::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT trial_id, number FROM trials \
                 WHERE study_id = ?1 AND state = 'COMPLETE' ORDER BY trial_id",
            )
            .map_err(|e| format!("Failed to query trials: {e}"))?;
        #[allow(clippy::cast_sign_loss)]
        let rows = stmt
            .query_map([sid], |row| {
                let trial_id: i64 = row.get(0)?;
                let number: i64 = row.get(1)?;
                Ok((trial_id as u32, number as u32))
            })
            .map_err(|e| format!("Failed to query trials: {e}"))?;
        for row in rows {
            let (trial_id, number) = row.map_err(|e| format!("Failed to read trials: {e}"))?;
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
        }
    }

    // ── trial_values ──────────────────────────────────────────────────
    {
        let mut stmt = conn
            .prepare(
                "SELECT tv.trial_id, tv.objective, tv.value, tv.value_type \
                 FROM trial_values tv JOIN trials t ON tv.trial_id = t.trial_id \
                 WHERE t.study_id = ?1 AND t.state = 'COMPLETE'",
            )
            .map_err(|e| format!("Failed to query trial_values: {e}"))?;
        #[allow(clippy::cast_sign_loss)]
        let rows = stmt
            .query_map([sid], |row| {
                let trial_id: i64 = row.get(0)?;
                let objective: i64 = row.get(1)?;
                let value: Option<f64> = row.get(2)?;
                let value_type: String = row.get(3)?;
                Ok((trial_id as u32, objective, value, value_type))
            })
            .map_err(|e| format!("Failed to query trial_values: {e}"))?;
        for row in rows {
            let (trial_id, objective, value, value_type) =
                row.map_err(|e| format!("Failed to read trial_values: {e}"))?;
            let v = match value_type.as_str() {
                "INF_POS" => f64::INFINITY,
                "INF_NEG" => f64::NEG_INFINITY,
                _ => value.unwrap_or(f64::NAN),
            };
            if let Some(trial) = accum.get_mut(&trial_id) {
                trial.objective_values.push((objective, v));
            }
        }
    }

    // ── trial_params ─────────────────────────────────────────────────
    let mut param_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut param_bounds: HashMap<String, (f64, f64)> = HashMap::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT tp.trial_id, tp.param_name, tp.param_value, tp.distribution_json \
                 FROM trial_params tp JOIN trials t ON tp.trial_id = t.trial_id \
                 WHERE t.study_id = ?1 AND t.state = 'COMPLETE' ORDER BY tp.trial_id",
            )
            .map_err(|e| format!("Failed to query trial_params: {e}"))?;
        #[allow(clippy::cast_sign_loss)]
        let rows = stmt
            .query_map([sid], |row| {
                let trial_id: i64 = row.get(0)?;
                let param_name: String = row.get(1)?;
                let param_value: f64 = row.get(2)?;
                let distribution_json: String = row.get(3)?;
                Ok((trial_id as u32, param_name, param_value, distribution_json))
            })
            .map_err(|e| format!("Failed to query trial_params: {e}"))?;
        for row in rows {
            let (trial_id, param_name, param_value, distribution_json) =
                row.map_err(|e| format!("Failed to read trial_params: {e}"))?;
            let dist_value: Value = serde_json::from_str(&distribution_json).unwrap_or(Value::Null);
            let distribution = Distribution::from_json(&dist_value);

            if let Some(bounds) = distribution.bounds() {
                param_bounds.entry(param_name.clone()).or_insert(bounds);
            }

            param_names.insert(param_name.clone());
            if let Some(trial) = accum.get_mut(&trial_id) {
                trial
                    .param_display
                    .insert(param_name.clone(), distribution.to_display_f64(param_value));
                if let Some(label) = distribution.categorical_label(param_value) {
                    trial.param_category_label.insert(param_name, label);
                }
            }
        }
    }

    // ── trial_user_attributes ────────────────────────────────────────
    let mut user_attr_numeric_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut user_attr_string_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT tua.trial_id, tua.key, tua.value_json \
                 FROM trial_user_attributes tua JOIN trials t ON tua.trial_id = t.trial_id \
                 WHERE t.study_id = ?1 AND t.state = 'COMPLETE'",
            )
            .map_err(|e| format!("Failed to query trial_user_attributes: {e}"))?;
        #[allow(clippy::cast_sign_loss)]
        let rows = stmt
            .query_map([sid], |row| {
                let trial_id: i64 = row.get(0)?;
                let key: String = row.get(1)?;
                let value_json: String = row.get(2)?;
                Ok((trial_id as u32, key, value_json))
            })
            .map_err(|e| format!("Failed to query trial_user_attributes: {e}"))?;
        for row in rows {
            let (trial_id, key, value_json) =
                row.map_err(|e| format!("Failed to read trial_user_attributes: {e}"))?;
            let Ok(value) = serde_json::from_str::<Value>(&value_json) else {
                continue;
            };
            if let Some(trial) = accum.get_mut(&trial_id) {
                let before_numeric = trial.user_attrs_numeric.len();
                let before_string = trial.user_attrs_string.len();
                classify_user_attr(
                    &value,
                    &key,
                    &mut trial.user_attrs_numeric,
                    &mut trial.user_attrs_string,
                );
                if trial.user_attrs_numeric.len() > before_numeric {
                    user_attr_numeric_names.insert(key);
                } else if trial.user_attrs_string.len() > before_string {
                    user_attr_string_names.insert(key);
                }
            }
        }
    }

    // ── trial_system_attributes (constraints only) ───────────────────
    let mut has_constraints = false;
    let mut max_constraints = 0usize;
    {
        let mut stmt = conn
            .prepare(
                "SELECT tsa.trial_id, tsa.value_json \
                 FROM trial_system_attributes tsa JOIN trials t ON tsa.trial_id = t.trial_id \
                 WHERE t.study_id = ?1 AND t.state = 'COMPLETE' AND tsa.key = 'constraints'",
            )
            .map_err(|e| format!("Failed to query trial_system_attributes: {e}"))?;
        #[allow(clippy::cast_sign_loss)]
        let rows = stmt
            .query_map([sid], |row| {
                let trial_id: i64 = row.get(0)?;
                let value_json: String = row.get(1)?;
                Ok((trial_id as u32, value_json))
            })
            .map_err(|e| format!("Failed to query trial_system_attributes: {e}"))?;
        for row in rows {
            let (trial_id, value_json) =
                row.map_err(|e| format!("Failed to read trial_system_attributes: {e}"))?;
            let Ok(Value::Array(values)) = serde_json::from_str::<Value>(&value_json) else {
                continue;
            };
            let constraints: Vec<f64> = values.iter().filter_map(Value::as_f64).collect();
            if let Some(trial) = accum.get_mut(&trial_id) {
                max_constraints = max_constraints.max(constraints.len());
                trial.constraint_values = constraints;
                has_constraints = true;
            }
        }
    }

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

    let meta = StudyMeta {
        study_id,
        name,
        directions,
        completed_trials,
        total_trials,
        param_names: param_names.clone(),
        objective_names: objective_names.clone(),
        user_attr_names,
        has_constraints,
        param_bounds,
    };

    Ok(SqliteStudyRows {
        meta,
        rows,
        param_names,
        objective_names,
        user_attr_numeric_names,
        user_attr_string_names,
        max_constraints,
    })
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、(確定メタ, `DataFrame`) を返す
/// （journal の `parse_single_study` と同じ出力契約）。
pub fn parse_single_study(path: &Path, study_id: u32) -> Result<(StudyMeta, DataFrame), String> {
    let SqliteStudyRows {
        meta,
        rows,
        param_names,
        objective_names,
        user_attr_numeric_names,
        user_attr_string_names,
        max_constraints,
    } = parse_single_study_rows(path, study_id)?;

    let df = DataFrame::from_trials(
        &rows,
        &param_names,
        &objective_names,
        &user_attr_numeric_names,
        &user_attr_string_names,
        max_constraints,
    );

    Ok((meta, df))
}
