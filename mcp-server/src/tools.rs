//! MCP ツールの定義と実装。
//!
//! 各ツールは `storage`（journal / SQLite パス、または PostgreSQL / MySQL の
//! 接続 URL）を受け取り、tunny-core のヘッドレス API で読み込んで分析結果を
//! 返す。返り値はテキスト 1 件（Markdown または JSON 文字列）。
//!
//! ツール一覧:
//! - `list_studies`   — ストレージ内の study 一覧（JSON）
//! - `study_summary`  — 1 study の要約（overview + key findings、JSON）
//! - `study_report`   — 完全な分析レポート（Markdown / JSON、日英対応）
//! - `trials`         — trial データのスライス（JSON、limit/offset）

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
use tunny_core::io::journal::parser::OptimizationDirection;
use tunny_core::report::{render_markdown, Outcome};
use tunny_core::{build_study_report, ReportLang, ReportOptions, ReportSource};

use crate::storage;

/// ツール呼び出しの失敗種別。
pub enum ToolError {
    /// ツール名が未知（JSON-RPC の Invalid params として返す）。
    UnknownTool,
    /// 実行時エラー（MCP の isError: true ツール結果として返す）。
    Execution(String),
}

/// `tools/list` に返すツール定義（MCP 形式、inputSchema は JSON Schema）。
pub fn definitions() -> Vec<Value> {
    let storage_prop = json!({
        "type": "string",
        "description": "Path to an Optuna storage: journal file (.log/.journal), \
                        SQLite database file, or a PostgreSQL/MySQL connection URL \
                        (e.g. postgresql://user:pass@host:5432/db).",
    });
    let study_id_prop = json!({
        "type": "integer",
        "description": "Target study_id (from list_studies).",
    });

    vec![
        json!({
            "name": "list_studies",
            "description": "List all studies in an Optuna storage with their id, name, \
                            objective directions, parameter names, and trial counts.",
            "inputSchema": {
                "type": "object",
                "properties": { "storage": storage_prop },
                "required": ["storage"],
            },
        }),
        json!({
            "name": "study_summary",
            "description": "Compact summary of one study: overview (objectives, parameters, \
                            trial states) and key findings (Pareto front size, convergence \
                            status, top parameter importance, feasibility). Skips the MCDM \
                            and correlation computations and returns far less text than \
                            study_report; use it first.",
            "inputSchema": {
                "type": "object",
                "properties": { "storage": storage_prop, "study_id": study_id_prop },
                "required": ["storage", "study_id"],
            },
        }),
        json!({
            "name": "study_report",
            "description": "Full analysis report of one study: key findings, Pareto front \
                            (constraint-aware) with parameter values, convergence, parameter \
                            importance, objective statistics, correlations, and MCDM rankings \
                            (TOPSIS/VIKOR/PROMETHEE II with consensus). Format 'markdown' is \
                            optimized for LLM consumption; 'json' returns the full structured \
                            report model.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "storage": storage_prop,
                    "study_id": study_id_prop,
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "json"],
                        "description": "Output format (default: markdown).",
                    },
                    "lang": {
                        "type": "string",
                        "enum": ["en", "ja"],
                        "description": "Report language (default: en).",
                    },
                    "top_n": {
                        "type": "integer",
                        "description": "Row cap for top-N tables (default: 10).",
                    },
                },
                "required": ["storage", "study_id"],
            },
        }),
        json!({
            "name": "trials",
            "description": "Raw trial data of one study as JSON rows (trial number, \
                            objective values, parameter values, constraint values). Only \
                            COMPLETE trials are included. Supports offset/limit pagination; \
                            rows are capped at 200 per call.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "storage": storage_prop,
                    "study_id": study_id_prop,
                    "offset": {
                        "type": "integer",
                        "description": "Rows to skip (default: 0).",
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max rows to return (default: 50, cap: 200).",
                    },
                },
                "required": ["storage", "study_id"],
            },
        }),
    ]
}

/// ツールを実行する。返り値はテキストコンテンツ 1 件。
pub fn call(name: &str, args: &Value) -> Result<String, ToolError> {
    match name {
        "list_studies" => list_studies(args),
        "study_summary" => study_summary(args),
        "study_report" => study_report(args),
        "trials" => trials(args),
        _ => Err(ToolError::UnknownTool),
    }
}

// =============================================================================
// 引数ヘルパー
// =============================================================================

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::Execution(format!("missing required argument: {key}")))
}

fn arg_u32(args: &Value, key: &str) -> Result<u32, ToolError> {
    match args.get(key) {
        None => Err(ToolError::Execution(format!(
            "missing required argument: {key}"
        ))),
        // 存在するが型・範囲が不正（負数・小数・u32 超過）は区別して伝える。
        Some(v) => v
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| {
                ToolError::Execution(format!(
                    "invalid argument: {key} (expected a non-negative integer, got {v})"
                ))
            }),
    }
}

fn exec_err(e: String) -> ToolError {
    ToolError::Execution(e)
}

/// レポート生成の共通入力（`ReportSource` + 生成時刻）を組み立てる。
/// `study_report` / `study_summary` で重複していた組み立てを一元化する。
fn report_source(storage_display: String) -> ReportSource {
    ReportSource {
        storage_display,
        generated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64),
    }
}

fn direction_label(d: &OptimizationDirection) -> &'static str {
    match d {
        OptimizationDirection::Minimize => "minimize",
        OptimizationDirection::Maximize => "maximize",
    }
}

// =============================================================================
// list_studies
// =============================================================================

fn list_studies(args: &Value) -> Result<String, ToolError> {
    let storage = arg_str(args, "storage")?;
    let studies = storage::scan_studies(storage).map_err(exec_err)?;

    let rows: Vec<Value> = studies
        .iter()
        .map(|m| {
            json!({
                "study_id": m.study_id,
                "name": m.name,
                "directions": m.directions.iter().map(direction_label).collect::<Vec<_>>(),
                "objective_names": m.objective_names,
                "param_names": m.param_names,
                "completed_trials": m.completed_trials,
                "total_trials": m.total_trials,
                "has_constraints": m.has_constraints,
            })
        })
        .collect();

    serde_json::to_string_pretty(&json!({ "studies": rows }))
        .map_err(|e| exec_err(format!("JSON serialization failed: {e}")))
}

// =============================================================================
// study_report
// =============================================================================

fn study_report(args: &Value) -> Result<String, ToolError> {
    let storage_str = arg_str(args, "storage")?;
    let study_id = arg_u32(args, "study_id")?;
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("markdown");
    let lang = match args.get("lang").and_then(Value::as_str).unwrap_or("en") {
        "ja" => ReportLang::Ja,
        _ => ReportLang::En,
    };
    let top_n = args
        .get("top_n")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(10)
        .max(1);

    let (meta, df, extras, storage_display) =
        storage::load_study(storage_str, study_id).map_err(exec_err)?;

    let source = report_source(storage_display);
    let opts = ReportOptions {
        lang,
        top_n,
        ..ReportOptions::default()
    };
    let report = build_study_report(&meta, &df, Some(&extras), &source, &opts);

    match format {
        "json" => serde_json::to_string_pretty(&report)
            .map_err(|e| exec_err(format!("JSON serialization failed: {e}"))),
        "markdown" => Ok(render_markdown(&report, lang)),
        other => Err(exec_err(format!(
            "invalid format: {other} (expected markdown|json)"
        ))),
    }
}

// =============================================================================
// study_summary
// =============================================================================

/// `trials` の limit の既定値。
const TRIALS_DEFAULT_LIMIT: usize = 50;
/// `trials` の limit の上限（超えた分は黙ってこの値にクランプする）。
const TRIALS_MAX_LIMIT: usize = 200;

fn study_summary(args: &Value) -> Result<String, ToolError> {
    let storage_str = arg_str(args, "storage")?;
    let study_id = arg_u32(args, "study_id")?;

    let (meta, df, extras, storage_display) =
        storage::load_study(storage_str, study_id).map_err(exec_err)?;

    let source = report_source(storage_display);
    // 要約はレポート全体を出力しないため、MCDM・相関の計算を省略する
    // （Key Findings とパレート表の TOPSIS 順は維持される）。
    let opts = ReportOptions {
        skip_decision_sections: true,
        ..ReportOptions::default()
    };
    let report = build_study_report(&meta, &df, Some(&extras), &source, &opts);

    let overview = serde_json::to_value(&report.overview)
        .map_err(|e| exec_err(format!("JSON serialization failed: {e}")))?;
    let key_findings = serde_json::to_value(&report.key_findings)
        .map_err(|e| exec_err(format!("JSON serialization failed: {e}")))?;
    let convergence_status = serde_json::to_value(report.convergence.status)
        .map_err(|e| exec_err(format!("JSON serialization failed: {e}")))?;

    let mut out = Map::new();
    out.insert("overview".to_string(), overview);
    out.insert("key_findings".to_string(), key_findings);
    out.insert(
        "convergence".to_string(),
        json!({
            "status": convergence_status,
            "found_at_trial_number": report.convergence.found_at_trial_number,
            "improved_in_last_20pct": report.convergence.improved_in_last_20pct,
        }),
    );

    match &report.outcome {
        Outcome::MultiObj {
            pareto_size,
            complete_count,
            ..
        } => {
            out.insert(
                "pareto".to_string(),
                json!({ "front_size": pareto_size, "complete_count": complete_count }),
            );
        }
        Outcome::SingleObj { best_trial, .. } => {
            let best = best_trial
                .as_ref()
                .map(|t| json!({ "trial_number": t.trial_number, "objectives": t.objectives }));
            out.insert("best".to_string(), best.unwrap_or(Value::Null));
        }
    }

    serde_json::to_string_pretty(&Value::Object(out))
        .map_err(|e| exec_err(format!("JSON serialization failed: {e}")))
}

// =============================================================================
// trials
// =============================================================================

/// `offset`/`limit` 引数を解析する（純粋なパース処理、テスト容易にするため分離）。
///
/// `offset` 省略時は 0、`limit` 省略時は [`TRIALS_DEFAULT_LIMIT`]。`limit` は
/// [`TRIALS_MAX_LIMIT`] を超えると黙ってクランプする（エラーにしない）。
fn parse_offset_limit(args: &Value) -> (usize, usize) {
    let offset = args
        .get("offset")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(0);
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(TRIALS_DEFAULT_LIMIT)
        .min(TRIALS_MAX_LIMIT);
    (offset, limit)
}

fn trials(args: &Value) -> Result<String, ToolError> {
    let storage_str = arg_str(args, "storage")?;
    let study_id = arg_u32(args, "study_id")?;
    let (offset, limit) = parse_offset_limit(args);

    let (_meta, df, _extras, _storage_display) =
        storage::load_study(storage_str, study_id).map_err(exec_err)?;

    let total = df.row_count();
    let end = total.min(offset.saturating_add(limit));
    let objective_names = df.objective_col_names();
    let param_names = df.param_col_names();
    let constraint_names = df.constraint_col_names();

    let mut rows = Vec::new();
    for row in offset..end {
        let Some(trial_number) = df.get_trial_number(row) else {
            continue;
        };

        let mut objectives = Map::new();
        for name in objective_names {
            if let Some(col) = df.get_numeric_column(name) {
                let v = col.get(row).copied().unwrap_or(f64::NAN);
                objectives.insert(name.clone(), finite_or_null(v));
            }
        }

        let mut params = Map::new();
        for name in param_names {
            if let Some(col) = df.get_numeric_column(name) {
                let v = col.get(row).copied().unwrap_or(f64::NAN);
                params.insert(name.clone(), finite_or_null(v));
            } else if let Some(col) = df.get_string_column(name) {
                let v = col.get(row).cloned().unwrap_or_default();
                params.insert(name.clone(), Value::String(v));
            }
        }

        let mut row_obj = Map::new();
        row_obj.insert("trial_number".to_string(), json!(trial_number));
        row_obj.insert("objectives".to_string(), Value::Object(objectives));
        row_obj.insert("params".to_string(), Value::Object(params));

        if !constraint_names.is_empty() {
            let mut constraints = Map::new();
            for name in constraint_names {
                if let Some(col) = df.get_numeric_column(name) {
                    let v = col.get(row).copied().unwrap_or(f64::NAN);
                    constraints.insert(name.clone(), finite_or_null(v));
                }
            }
            row_obj.insert("constraints".to_string(), Value::Object(constraints));
        }

        rows.push(Value::Object(row_obj));
    }

    serde_json::to_string_pretty(&json!({
        "total_complete_trials": total,
        "offset": offset,
        "returned": rows.len(),
        "trials": rows,
    }))
    .map_err(|e| exec_err(format!("JSON serialization failed: {e}")))
}

/// 有限値ならそのまま JSON 数値、NaN/inf は JSON に表現できないため `null` にする。
fn finite_or_null(v: f64) -> Value {
    if v.is_finite() {
        json!(v)
    } else {
        Value::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── study_summary: 引数エラー ────────────────────────────────

    #[test]
    fn study_summary_missing_study_id_is_execution_error() {
        let args = json!({ "storage": "/tmp/x.log" });
        let err = study_summary(&args).unwrap_err();
        match err {
            ToolError::Execution(msg) => assert!(msg.contains("study_id"), "{msg}"),
            ToolError::UnknownTool => panic!("expected Execution error"),
        }
    }

    #[test]
    fn study_summary_missing_storage_is_execution_error() {
        let args = json!({ "study_id": 0 });
        let err = study_summary(&args).unwrap_err();
        match err {
            ToolError::Execution(msg) => assert!(msg.contains("storage"), "{msg}"),
            ToolError::UnknownTool => panic!("expected Execution error"),
        }
    }

    // ── trials: 引数エラー ───────────────────────────────────────

    #[test]
    fn trials_missing_study_id_is_execution_error() {
        let args = json!({ "storage": "/tmp/x.log" });
        let err = trials(&args).unwrap_err();
        match err {
            ToolError::Execution(msg) => assert!(msg.contains("study_id"), "{msg}"),
            ToolError::UnknownTool => panic!("expected Execution error"),
        }
    }

    #[test]
    fn trials_missing_storage_is_execution_error() {
        let args = json!({ "study_id": 0 });
        let err = trials(&args).unwrap_err();
        match err {
            ToolError::Execution(msg) => assert!(msg.contains("storage"), "{msg}"),
            ToolError::UnknownTool => panic!("expected Execution error"),
        }
    }

    // ── trials: offset/limit のパース・クランプ ─────────────────

    #[test]
    fn parse_offset_limit_defaults() {
        let (offset, limit) = parse_offset_limit(&json!({}));
        assert_eq!(offset, 0);
        assert_eq!(limit, TRIALS_DEFAULT_LIMIT);
    }

    #[test]
    fn parse_offset_limit_respects_explicit_values() {
        let (offset, limit) = parse_offset_limit(&json!({ "offset": 5, "limit": 20 }));
        assert_eq!(offset, 5);
        assert_eq!(limit, 20);
    }

    #[test]
    fn parse_offset_limit_caps_at_max() {
        let (offset, limit) = parse_offset_limit(&json!({ "limit": 10_000 }));
        assert_eq!(offset, 0);
        assert_eq!(limit, TRIALS_MAX_LIMIT);
    }

    #[test]
    fn parse_offset_limit_exact_cap_boundary_unchanged() {
        let (_, limit) = parse_offset_limit(&json!({ "limit": TRIALS_MAX_LIMIT }));
        assert_eq!(limit, TRIALS_MAX_LIMIT);
    }
}
