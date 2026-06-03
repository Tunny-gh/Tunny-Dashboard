//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-101/journal-parser-requirements.md

mod builders;
mod distribution;
mod finalize;
mod state;
mod types;

use serde_json::Value;

use finalize::finalize_state;
use state::{get_str, get_u64, ParserState};

pub use types::{JournalParser, OptimizationDirection, ParseResult, StudyMeta};

#[cfg(test)]
use builders::TrialBuilder;
#[cfg(test)]
use distribution::Distribution;

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

    if data.is_empty() {
        crate::dataframe::store_dataframes(vec![]);
        return Ok(ParseResult {
            studies: vec![],
            duration_ms: 0.0,
        });
    }

    let text = String::from_utf8_lossy(data);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
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
        if let Ok(json) = serde_json::from_str::<Value>(line.trim()) {
            valid_lines += 1;
            if let Some(op) = get_u64(&json, "op_code") {
                #[allow(clippy::cast_possible_truncation)]
                state.process_op(op as u8, &json);
            }
        }
    }

    if valid_lines == 0 {
        return Err("No valid JSON lines found in journal".to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;

    let (studies, dataframes) = finalize_state(state);
    crate::dataframe::store_dataframes(dataframes);

    Ok(ParseResult {
        studies,
        duration_ms,
    })
}

/// Phase 1: op_code=0/3 のみスキャンして Study 一覧を高速取得する。
/// Trial データは一切処理しないため、大規模ファイルでも即座に返る。
/// StudyMeta の completed_trials / param_names 等は 0 / 空（Phase 2 で確定する）。
pub fn scan_study_list(data: &[u8]) -> Result<Vec<StudyMeta>, String> {
    if data.is_empty() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(data);
    let mut studies: Vec<StudyMeta> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Trial 行（op_code 4/5/6/8/9）は全体の99%以上を占める。
        // JSON パースより桁違いに速い文字列検索でまず除外する。
        // op_code フィールドの直後に空白なし・あり両パターンを考慮する。
        let is_create_study = line.contains("\"op_code\":0")
            || line.contains("\"op_code\": 0,")
            || line.contains("\"op_code\": 0}");
        let is_metric_names = !is_create_study
            && (line.contains("\"op_code\":3")
                || line.contains("\"op_code\": 3,")
                || line.contains("\"op_code\": 3}"));
        if !is_create_study && !is_metric_names {
            continue;
        }
        let Ok(json) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(op) = get_u64(&json, "op_code") else {
            continue;
        };
        match op {
            0 => {
                let name = get_str(&json, "study_name").unwrap_or("").to_string();
                if studies.iter().any(|s| s.name == name) {
                    continue;
                }
                let directions = json
                    .get("directions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|d| match d.as_u64() {
                                Some(1) => OptimizationDirection::Minimize,
                                Some(2) => OptimizationDirection::Maximize,
                                _ => OptimizationDirection::Minimize,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let study_id = studies.len() as u32;
                studies.push(StudyMeta {
                    study_id,
                    name,
                    directions,
                    completed_trials: 0,
                    total_trials: 0,
                    param_names: vec![],
                    objective_names: vec![],
                    user_attr_names: vec![],
                    has_constraints: false,
                });
            }
            3 => {
                let study_id = get_u64(&json, "study_id").unwrap_or(0) as usize;
                if study_id >= studies.len() {
                    continue;
                }
                if let Some(attrs) = json.get("system_attr").and_then(|v| v.as_object()) {
                    if let Some(names_arr) =
                        attrs.get("study:metric_names").and_then(|v| v.as_array())
                    {
                        let names: Vec<String> = names_arr
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect();
                        if !names.is_empty() {
                            studies[study_id].objective_names = names;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if studies.is_empty() {
        return Err("No studies found in journal".to_string());
    }
    Ok(studies)
}

/// JSON パース不要で `"field":N` 形式の u32 値を高速抽出する。
/// Phase 2 の string-level フィルタリングに使用する。
fn quick_extract_u32(line: &str, field: &str) -> Option<u32> {
    // 例: `"study_id":2,` → Some(2)
    // フィールド名の長さに応じてスタック上のバッファを使うため alloc なし。
    let key_start = line.find(field)?;
    // field の前に `"` があり、後に `":` が続くことを確認する
    let before = key_start.checked_sub(1)?;
    if line.as_bytes().get(before) != Some(&b'"') {
        return None;
    }
    let after_key = key_start + field.len();
    let rest = line.get(after_key..)?;
    // `":` または `": ` を探す
    let colon_pos = rest.find(':')?;
    let digits = rest[colon_pos + 1..].trim_start();
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    digits[..end].parse().ok()
}

/// op_code が 5/6/8/9 であれば値を返す（試行更新系コマンドの判定用）。
fn detect_trial_update_op(line: &str) -> u8 {
    for (op, pats) in [
        (5u8, ["\"op_code\":5", "\"op_code\": 5,", "\"op_code\": 5}"]),
        (6u8, ["\"op_code\":6", "\"op_code\": 6,", "\"op_code\": 6}"]),
        (8u8, ["\"op_code\":8", "\"op_code\": 8,", "\"op_code\": 8}"]),
        (9u8, ["\"op_code\":9", "\"op_code\": 9,", "\"op_code\": 9}"]),
    ] {
        if pats.iter().any(|p| line.contains(p)) {
            return op;
        }
    }
    0
}

/// Phase 2: 指定 study_id の Trial データのみパースして (StudyMeta, DataFrame) を返す。
///
/// 3-pass 設計で高速化する:
///   Pass 1 (sequential): 全行を string スキャンして対象行を収集・カウンタ管理
///   Pass 2 (rayon parallel): 収集した行を並列 JSON パース
///   Pass 3 (sequential): パース済み結果を順序保持で state に適用
///
/// N Study ファイルで対象 Study が 1 件の場合、JSON パース量は約 1/N に削減される。
/// さらに rayon による並列化でコア数に応じた追加高速化が得られる。
pub fn parse_single_study(
    data: &[u8],
    target_study_id: u32,
) -> Result<(StudyMeta, crate::data::dataframe::DataFrame), String> {
    use rayon::prelude::*;

    if data.is_empty() {
        return Err("Empty journal data".to_string());
    }
    let text = String::from_utf8_lossy(data);
    let mut state = ParserState::new_with_target(target_study_id);
    // 対象 Study に属する trial_id セット（ops 5/6/8/9 のフィルタに使用）
    let mut target_trial_ids = std::collections::HashSet::<u32>::new();
    // Pass 2 用: (line_ref, op_code, pre_trial_id-for-op4)
    let mut deferred: Vec<(&str, u8, Option<u32>)> = Vec::new();
    let mut any_valid = false;

    // ── Pass 1: sequential string scan ──────────────────────────────────
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let is_op0 = line.contains("\"op_code\":0")
            || line.contains("\"op_code\": 0,")
            || line.contains("\"op_code\": 0}");
        let is_op3 = !is_op0
            && (line.contains("\"op_code\":3")
                || line.contains("\"op_code\": 3,")
                || line.contains("\"op_code\": 3}"));
        let is_op4 = !is_op0
            && !is_op3
            && (line.contains("\"op_code\":4")
                || line.contains("\"op_code\": 4,")
                || line.contains("\"op_code\": 4}"));

        if is_op0 || is_op3 {
            // CREATE_STUDY / SET_STUDY_SYSTEM_ATTR: 数が少ないので即パース
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                any_valid = true;
                state.process_op(if is_op0 { 0 } else { 3 }, &json);
            }
        } else if is_op4 {
            any_valid = true;
            let pre_trial_id = state.next_trial_id;
            match quick_extract_u32(line, "study_id") {
                Some(sid) if sid == target_study_id => {
                    // 対象 Study → Pass 2 へ回す（カウンタは先に進める）
                    state.next_trial_id += 1;
                    target_trial_ids.insert(pre_trial_id);
                    deferred.push((line, 4, Some(pre_trial_id)));
                }
                Some(sid) => {
                    // 他 Study → JSON 不要、カウンタのみ更新
                    state.next_trial_id += 1;
                    if (sid as usize) < state.studies.len() {
                        state.studies[sid as usize].total_trials += 1;
                    }
                }
                None => {
                    // 抽出失敗 → 安全のためフルパースにフォールバック
                    let tid = state.next_trial_id;
                    if let Ok(json) = serde_json::from_str::<Value>(line) {
                        state.process_op(4, &json);
                        if state.trial_builders.contains_key(&tid) {
                            target_trial_ids.insert(tid);
                        }
                    }
                }
            }
        } else {
            // op_code 5/6/8/9: 対象 trial_id の行のみ Pass 2 へ
            let op = detect_trial_update_op(line);
            if op != 0 {
                any_valid = true;
                if let Some(tid) = quick_extract_u32(line, "trial_id") {
                    if target_trial_ids.contains(&tid) {
                        deferred.push((line, op, None));
                    }
                }
            }
        }
    }

    if !any_valid {
        return Err("No valid JSON lines found in journal".to_string());
    }

    // ── Pass 2: parallel JSON parse ──────────────────────────────────────
    // &str は text と同じ生存期間を持ち Send なので rayon スレッドへ安全に送れる。
    let parsed: Vec<(u8, Value, Option<u32>)> = deferred
        .par_iter()
        .filter_map(|(line, op, pre_tid)| {
            serde_json::from_str::<Value>(line)
                .ok()
                .map(|v| (*op, v, *pre_tid))
        })
        .collect();
    drop(deferred);

    // ── Pass 3: sequential state update (順序保持) ───────────────────────
    for (op, json, pre_tid) in parsed {
        if op == 4 {
            // op_code=4 は Pass 1 で割り当てた trial_id を使わせる
            if let Some(tid) = pre_tid {
                state.next_trial_id = tid;
            }
        }
        state.process_op(op, &json);
    }

    let (studies, dataframes) = finalize_state(state);

    let pos = studies
        .iter()
        .position(|s| s.study_id == target_study_id)
        .ok_or_else(|| format!("study_id {target_study_id} not found in journal"))?;

    let meta = studies.into_iter().nth(pos).unwrap();
    let df = dataframes.into_iter().nth(pos).unwrap();
    Ok((meta, df))
}

#[cfg(test)]
mod tests;
