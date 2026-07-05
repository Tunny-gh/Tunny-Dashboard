//! Optuna RDBStorage の共通クエリ・組み立てロジック。
//!
//! SQLite / PostgreSQL / MySQL で共通のクエリ文字列・行組み立てロジックをここに
//! 集約する。接続や値表現の方言差は `OptunaBackend` trait 経由でのみバックエンドへ
//! 問い合わせるため、本体はバックエンド非依存になっている。

use std::collections::HashMap;

use serde_json::Value;

use crate::data::dataframe::{DataFrame, TrialRow};
use crate::data::extras::{StudyExtras, TrialExtra, TrialState};
use crate::io::datetime::parse_naive_datetime;
use crate::io::journal::parser::distribution::Distribution;
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};

use super::backend::{OptunaBackend, SqlParam, SqlValue};

/// Optuna スキーマかどうかを `studies` テーブルの有無で判定する。
fn ensure_optuna_schema(backend: &mut dyn OptunaBackend) -> Result<(), String> {
    let exists = backend
        .table_exists("studies")
        .map_err(|e| format!("Failed to inspect database schema: {e}"))?;
    if !exists {
        return Err("Not an Optuna SQLite storage: 'studies' table not found".to_string());
    }
    Ok(())
}

/// 1 行 1 列の集計クエリ（`COUNT`/`MAX` 等）を実行し `i64` として返す。
fn query_scalar_i64(
    backend: &mut dyn OptunaBackend,
    sql: &str,
    params: &[SqlParam],
    context: &str,
) -> Result<i64, String> {
    let rows = backend
        .query(sql, params)
        .map_err(|e| format!("{context}: {e}"))?;
    rows.into_iter()
        .next()
        .and_then(|row| row.into_iter().next())
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("{context}: expected a single integer row"))
}

/// `SELECT 1 ... LIMIT 1` の行の有無で真偽を判定する。
///
/// `EXISTS(...)` は PostgreSQL では bool 型で返るなど方言差が出るため、代わりに
/// 行の有無判定（3 バックエンドで可搬）を使う。
fn query_exists(
    backend: &mut dyn OptunaBackend,
    sql: &str,
    params: &[SqlParam],
    context: &str,
) -> Result<bool, String> {
    let rows = backend
        .query(sql, params)
        .map_err(|e| format!("{context}: {e}"))?;
    Ok(!rows.is_empty())
}

fn fetch_directions(
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
        directions.push(if direction == "MAXIMIZE" {
            OptimizationDirection::Maximize
        } else {
            OptimizationDirection::Minimize
        });
    }
    Ok(directions)
}

/// `study_system_attributes` の `study:metric_names` から目的名を読む（無ければ空）。
fn fetch_metric_names(
    backend: &mut dyn OptunaBackend,
    study_id: i64,
) -> Result<Vec<String>, String> {
    // `key` は MySQL/MariaDB の予約語のため、他クエリと同様テーブル修飾して参照する
    // （`tsa.key` のようにエイリアス修飾されていれば問題ないが、無修飾だと構文エラーになる）。
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

/// ライブ更新のポーリングで変化検出に使う軽量フィンガープリント。
///
/// journal と異なり RDB (SQLite 等) は trial の状態がインプレースで更新される
/// （RUNNING → COMPLETE 等）ため、バイトオフセット差分方式が使えない。
/// 代わりに本フィンガープリントで変化の有無だけを安価に検出し、変化を検出したら
/// 対象 study を丸ごと再パースする（`parse_single_study`）方式を取る。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StudyFingerprint {
    pub total_trials: u32,
    pub completed_trials: u32,
    pub max_trial_id: i64,
    /// 中間値レコード総数（テーブルが無ければ 0）。RUNNING trial の進捗検出用。
    pub intermediate_count: i64,
    /// state 文字列の集計ハッシュ（state 遷移の検出用）。
    pub state_digest: u64,
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a でバイト列をハッシュへ畳み込む（pure std、追加依存なし）。
fn fnv1a_fold(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// ライブ更新のポーリングで呼ぶ軽量フィンガープリント取得。
/// `total_trials` / `completed_trials` / `max_trial_id` は集計クエリで、
/// `intermediate_count` は `trial_intermediate_values` の有無を確認した上で数える
/// （`fetch_study_extras` と同じガード）。`state_digest` は `trials` を trial_id 昇順で
/// 読み、各行の trial_id と state 文字列を FNV-1a で畳み込んだもの（state 遷移の検出用）。
pub fn study_fingerprint(
    backend: &mut dyn OptunaBackend,
    study_id: u32,
) -> Result<StudyFingerprint, String> {
    ensure_optuna_schema(backend)?;

    let sid = i64::from(study_id);

    let total_trials = query_scalar_i64(
        backend,
        "SELECT COUNT(*) FROM trials WHERE study_id = ?",
        &[SqlParam::I64(sid)],
        "Failed to count trials",
    )?;

    let completed_trials = query_scalar_i64(
        backend,
        "SELECT COUNT(*) FROM trials WHERE study_id = ? AND state = 'COMPLETE'",
        &[SqlParam::I64(sid)],
        "Failed to count completed trials",
    )?;

    let max_trial_id = query_scalar_i64(
        backend,
        "SELECT COALESCE(MAX(trial_id), 0) FROM trials WHERE study_id = ?",
        &[SqlParam::I64(sid)],
        "Failed to read max trial_id",
    )?;

    // trial_intermediate_values テーブルの存在確認（古い DB では欠落しうる）。
    let has_intermediate_table = backend
        .table_exists("trial_intermediate_values")
        .map_err(|e| format!("Failed to inspect intermediate values table: {e}"))?;

    let intermediate_count = if has_intermediate_table {
        query_scalar_i64(
            backend,
            "SELECT COUNT(*) FROM trial_intermediate_values tiv \
             JOIN trials t ON tiv.trial_id = t.trial_id WHERE t.study_id = ?",
            &[SqlParam::I64(sid)],
            "Failed to count intermediate values",
        )?
    } else {
        0
    };

    let mut state_digest = FNV_OFFSET_BASIS;
    {
        let rows = backend
            .query(
                "SELECT trial_id, state FROM trials WHERE study_id = ? ORDER BY trial_id",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trial states: {e}"))?;
        for row in rows {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial states: trial_id is not an integer".to_string()
            })?;
            let state = row[1]
                .as_text()
                .ok_or_else(|| "Failed to read trial states: state is not text".to_string())?;
            state_digest = fnv1a_fold(state_digest, &trial_id.to_le_bytes());
            state_digest = fnv1a_fold(state_digest, state.as_bytes());
        }
    }

    #[allow(clippy::cast_sign_loss)]
    Ok(StudyFingerprint {
        total_trials: total_trials as u32,
        completed_trials: completed_trials as u32,
        max_trial_id,
        intermediate_count,
        state_digest,
    })
}

/// Phase 1: DB を開いて Study 一覧を返す（journal の `scan_study_list` と同じ役割）。
/// completed_trials / total_trials は安価に取れるため実数を入れる
/// （journal scan と異なり 0 埋めではない）。param_names 等の詳細は Phase 2 で確定する。
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

    let mut studies = Vec::with_capacity(studies_raw.len());
    for (study_id, name) in studies_raw {
        let directions = fetch_directions(backend, study_id)?;
        let metric_names = fetch_metric_names(backend, study_id)?;
        let objective_names = objective_names_for(&directions, metric_names);

        let completed_trials = query_scalar_i64(
            backend,
            "SELECT COUNT(*) FROM trials WHERE study_id = ? AND state = 'COMPLETE'",
            &[SqlParam::I64(study_id)],
            "Failed to count completed trials",
        )?;
        let total_trials = query_scalar_i64(
            backend,
            "SELECT COUNT(*) FROM trials WHERE study_id = ?",
            &[SqlParam::I64(study_id)],
            "Failed to count trials",
        )?;
        let has_constraints = query_exists(
            backend,
            "SELECT 1 FROM trial_system_attributes tsa \
             JOIN trials t ON tsa.trial_id = t.trial_id \
             WHERE t.study_id = ? AND tsa.key = 'constraints' LIMIT 1",
            &[SqlParam::I64(study_id)],
            "Failed to check constraints",
        )?;

        #[allow(clippy::cast_sign_loss)]
        studies.push(StudyMeta {
            study_id: study_id as u32,
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
pub struct RdbStudyRows {
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
    /// 全 trial（全 state）の付帯情報（state / 日時 / 中間値）。trial_id 昇順。
    pub extras: StudyExtras,
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、確定メタと行データを返す。
/// `DataFrame` 組み立て前の中間表現なので、`parse_single_study` はこれをそのまま
/// `DataFrame::from_trials` に渡すだけのラッパーになる。
pub fn parse_single_study_rows(
    backend: &mut dyn OptunaBackend,
    study_id: u32,
) -> Result<RdbStudyRows, String> {
    ensure_optuna_schema(backend)?;

    let sid = i64::from(study_id);

    // 元の rusqlite 実装と同様、study 未存在・クエリ失敗のいずれも
    // 「study_id {id} not found in database」に丸める。
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

    // ── extras: 全 trial（全 state）の付帯情報（state / 日時 / 中間値） ──────
    let extras = fetch_study_extras(backend, sid)?;

    // ── trials (COMPLETE のみ、trial_id 昇順) ────────────────────────────
    let mut trial_order: Vec<u32> = Vec::new();
    let mut accum: HashMap<u32, TrialAccum> = HashMap::new();
    {
        let rows = backend
            .query(
                "SELECT trial_id, number FROM trials \
                 WHERE study_id = ? AND state = 'COMPLETE' ORDER BY trial_id",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trials: {e}"))?;
        for row in rows {
            let trial_id = row[0]
                .as_i64()
                .ok_or_else(|| "Failed to read trials: trial_id is not an integer".to_string())?;
            let number = row[1]
                .as_i64()
                .ok_or_else(|| "Failed to read trials: number is not an integer".to_string())?;
            #[allow(clippy::cast_sign_loss)]
            let trial_id = trial_id as u32;
            #[allow(clippy::cast_sign_loss)]
            let number = number as u32;
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
        let rows = backend
            .query(
                "SELECT tv.trial_id, tv.objective, tv.value, tv.value_type \
                 FROM trial_values tv JOIN trials t ON tv.trial_id = t.trial_id \
                 WHERE t.study_id = ? AND t.state = 'COMPLETE'",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trial_values: {e}"))?;
        for row in rows {
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
            #[allow(clippy::cast_sign_loss)]
            let trial_id = trial_id as u32;
            if let Some(trial) = accum.get_mut(&trial_id) {
                trial.objective_values.push((objective, v));
            }
        }
    }

    // ── trial_params ─────────────────────────────────────────────────
    let mut param_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut param_bounds: HashMap<String, (f64, f64)> = HashMap::new();
    {
        let rows = backend
            .query(
                "SELECT tp.trial_id, tp.param_name, tp.param_value, tp.distribution_json \
                 FROM trial_params tp JOIN trials t ON tp.trial_id = t.trial_id \
                 WHERE t.study_id = ? AND t.state = 'COMPLETE' ORDER BY tp.trial_id",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trial_params: {e}"))?;
        for row in rows {
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
            #[allow(clippy::cast_sign_loss)]
            let trial_id = trial_id as u32;
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
        let rows = backend
            .query(
                "SELECT tua.trial_id, tua.key, tua.value_json \
                 FROM trial_user_attributes tua JOIN trials t ON tua.trial_id = t.trial_id \
                 WHERE t.study_id = ? AND t.state = 'COMPLETE'",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trial_user_attributes: {e}"))?;
        for row in rows {
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
                continue;
            };
            #[allow(clippy::cast_sign_loss)]
            let trial_id = trial_id as u32;
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
        }
    }

    // ── trial_system_attributes (constraints only) ───────────────────
    let mut has_constraints = false;
    let mut max_constraints = 0usize;
    {
        let rows = backend
            .query(
                "SELECT tsa.trial_id, tsa.value_json \
                 FROM trial_system_attributes tsa JOIN trials t ON tsa.trial_id = t.trial_id \
                 WHERE t.study_id = ? AND t.state = 'COMPLETE' AND tsa.key = 'constraints'",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trial_system_attributes: {e}"))?;
        for row in rows {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial_system_attributes: trial_id is not an integer".to_string()
            })?;
            let value_json = row[1].as_text().ok_or_else(|| {
                "Failed to read trial_system_attributes: value_json is not text".to_string()
            })?;
            let Ok(Value::Array(values)) = serde_json::from_str::<Value>(value_json) else {
                continue;
            };
            let constraints: Vec<f64> = values.iter().filter_map(Value::as_f64).collect();
            #[allow(clippy::cast_sign_loss)]
            let trial_id = trial_id as u32;
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

/// 指定 study の全 trial（全 state）の付帯情報を読む。
///
/// - `trials` から trial_id / number / state / datetime_start / datetime_complete を trial_id 昇順で読む。
///   日時は `backend.text_cast()` でテキスト化した上で読み、`parse_naive_datetime` により
///   naive unix 秒へ変換する（SQLite は元々 TEXT だが、PostgreSQL/MySQL のネイティブ
///   timestamp 型との差を吸収するため常にテキスト化を通す）。
/// - `trial_intermediate_values` から中間値を読み、各 trial に step 昇順で紐付ける。
///   このテーブルは古い DB では存在しない場合があるため `backend.table_exists` で存在を確認し、
///   無ければ中間値は空とする。value_type は trial_values と同じ意味論
///   (FINITE/INF_POS/INF_NEG/NAN) で解釈する。
fn fetch_study_extras(backend: &mut dyn OptunaBackend, sid: i64) -> Result<StudyExtras, String> {
    // trial_id → extras 内の index。中間値の紐付けに使う。
    let mut index_of: HashMap<u32, usize> = HashMap::new();
    let mut trials: Vec<TrialExtra> = Vec::new();
    {
        let sql = format!(
            "SELECT trial_id, number, state, {}, {} \
             FROM trials WHERE study_id = ? ORDER BY trial_id",
            backend.text_cast("datetime_start"),
            backend.text_cast("datetime_complete"),
        );
        let rows = backend
            .query(&sql, &[SqlParam::I64(sid)])
            .map_err(|e| format!("Failed to query trials for extras: {e}"))?;
        for row in rows {
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

            #[allow(clippy::cast_sign_loss)]
            let trial_id = trial_id as u32;
            #[allow(clippy::cast_sign_loss)]
            let number = number as u32;

            index_of.insert(trial_id, trials.len());
            trials.push(TrialExtra {
                trial_id,
                trial_number: number,
                state: TrialState::from_rdb_str(state),
                datetime_start: datetime_start.and_then(parse_naive_datetime),
                datetime_complete: datetime_complete.and_then(parse_naive_datetime),
                intermediate_values: Vec::new(),
            });
        }
    }

    // trial_intermediate_values テーブルの存在確認（古い DB では欠落しうる）。
    let has_intermediate_table = backend
        .table_exists("trial_intermediate_values")
        .map_err(|e| format!("Failed to inspect intermediate values table: {e}"))?;

    if has_intermediate_table {
        let rows = backend
            .query(
                "SELECT tiv.trial_id, tiv.step, tiv.intermediate_value, tiv.intermediate_value_type \
                 FROM trial_intermediate_values tiv \
                 JOIN trials t ON tiv.trial_id = t.trial_id \
                 WHERE t.study_id = ? ORDER BY tiv.trial_id, tiv.step",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trial_intermediate_values: {e}"))?;
        for row in rows {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to read trial_intermediate_values: trial_id is not an integer".to_string()
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
            #[allow(clippy::cast_sign_loss)]
            let trial_id = trial_id as u32;
            #[allow(clippy::cast_sign_loss)]
            let step = step as u64;
            if let Some(&idx) = index_of.get(&trial_id) {
                trials[idx].intermediate_values.push((step, v));
            }
        }
    }

    // 保険として step 昇順にそろえる（SQL の ORDER BY を信頼するが冪等）。
    for trial in &mut trials {
        trial.intermediate_values.sort_by_key(|(step, _)| *step);
    }

    Ok(StudyExtras { trials })
}

/// Phase 2: 指定 study の COMPLETE trial を全件読み、(確定メタ, `DataFrame`, `StudyExtras`) を返す
/// （journal の `parse_single_study` と同じ出力契約）。
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
