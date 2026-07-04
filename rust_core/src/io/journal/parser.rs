//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-101/journal-parser-requirements.md

mod builders;
pub(crate) mod distribution;
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
        // op_code は Optuna journal の各行で必ず先頭フィールド (`{"op_code":N,...`)。
        // 1 回だけ抽出して分岐し、行全体を何度も走査する contains を排除する。
        // Trial 行（op_code 4/5/6/8/9）は全体の 99% 以上を占めるため、ここで即除外する。
        let op = match quick_extract_u32(line, "op_code") {
            Some(op) => u64::from(op),
            None => continue,
        };
        if op != 0 && op != 3 {
            continue;
        }
        // op3 の大半は巨大な sampler 属性配列。必要なのは metric_names を持つ行だけなので、
        // それ以外は JSON パースを完全に回避する。
        if op == 3 && !line.contains("study:metric_names") {
            continue;
        }
        let Ok(json) = serde_json::from_str::<Value>(line) else {
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
                    param_bounds: std::collections::HashMap::new(),
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
    // op_code は各行の先頭フィールド。1 回だけ抽出して match で分岐する。
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let op = match quick_extract_u32(line, "op_code") {
            #[allow(clippy::cast_possible_truncation)]
            Some(op) => op as u8,
            None => continue,
        };

        match op {
            0 => {
                // CREATE_STUDY: 数が少ないので即パース
                if let Ok(json) = serde_json::from_str::<Value>(line) {
                    any_valid = true;
                    state.process_op(0, &json);
                }
            }
            3 => {
                // SET_STUDY_SYSTEM_ATTR: 必要なのは metric_names を持つ行のみ。
                // 巨大な sampler 属性行の JSON パースを回避する。
                any_valid = true;
                if line.contains("study:metric_names") {
                    if let Ok(json) = serde_json::from_str::<Value>(line) {
                        state.process_op(3, &json);
                    }
                }
            }
            4 => {
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
            }
            5 | 6 | 8 | 9 => {
                // 試行更新系: 対象 trial_id の行のみ Pass 2 へ
                any_valid = true;
                if let Some(tid) = quick_extract_u32(line, "trial_id") {
                    if target_trial_ids.contains(&tid) {
                        deferred.push((line, op, None));
                    }
                }
            }
            _ => {}
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

/// `parse_single_study_streaming` が逐次コールバックへ渡すバッチ。
///
/// 完了 Trial を `batch_size` 件ずつ前方ストリーミングで送出する。各バッチには
/// その時点までの累積メタ（列名集合・件数）を同梱するため、UI 側は新規列の追加も
/// 含めて DataFrame を再構築できる。最終バッチは `is_final = true`（残り 0 件でも送出）。
pub struct StudyStreamBatch {
    /// その時点までの累積 StudyMeta（user_attr_names はマージ済みソート）。
    pub meta: StudyMeta,
    /// 今回のバッチで新たに完了した Trial 行。
    pub new_rows: Vec<crate::dataframe::TrialRow>,
    /// 累積パラメータ列名（ソート済み、DataFrame 構築用）。
    pub param_names: Vec<String>,
    /// 目的列名。
    pub objective_names: Vec<String>,
    /// 累積 user_attr 数値列名（ソート済み）。
    pub user_attr_numeric_names: Vec<String>,
    /// 累積 user_attr 文字列列名（ソート済み）。
    pub user_attr_string_names: Vec<String>,
    /// これまでに観測した制約数の最大値。
    pub max_constraints: usize,
    /// 最初のバッチか（UI 側で StudyContext を新規生成する判定に使う）。
    pub is_first: bool,
    /// 最終バッチか（UI 側で Pareto を確定計算しローディングを終える）。
    pub is_final: bool,
}

/// 累積集合から StudyMeta スナップショットを生成する。
#[allow(clippy::too_many_arguments)]
fn stream_build_meta(
    state: &ParserState,
    target: u32,
    param_set: &std::collections::BTreeSet<String>,
    uan_set: &std::collections::BTreeSet<String>,
    uas_set: &std::collections::BTreeSet<String>,
    derived_objective_names: &[String],
    has_constraints: bool,
    completed: u32,
) -> StudyMeta {
    let builder = &state.studies[target as usize];
    let objective_names = if builder.objective_names.is_empty() {
        derived_objective_names.to_vec()
    } else {
        builder.objective_names.clone()
    };
    // user_attr_names は数値・文字列をマージしソート（既存 finalize と同等）。
    let mut user_attr_names: Vec<String> = uan_set.iter().chain(uas_set.iter()).cloned().collect();
    user_attr_names.sort();
    user_attr_names.dedup();
    StudyMeta {
        study_id: target,
        name: builder.name.clone(),
        directions: builder.directions.clone(),
        completed_trials: completed,
        total_trials: builder.total_trials,
        param_names: param_set.iter().cloned().collect(),
        objective_names,
        user_attr_names,
        has_constraints,
        param_bounds: builder.param_bounds.clone(),
    }
}

/// 完了 Trial（state==1）の builder をバッチへ取り込み、列名集合・カウンタを更新する。
/// バッチが満杯になったら `on_batch` で送出する。
///
/// in-memory ストレージは op_code=4 にすべての Trial データ（state / values / params）を
/// インラインで持ち、後続の op_code=6 が来ない。一方ファイルストレージは op_code=6 で完了する。
/// 両者の完了処理を共通化するためのヘルパー。
#[allow(clippy::too_many_arguments)]
fn stream_emit_completed_trial<F>(
    tid: u32,
    builder: builders::TrialBuilder,
    state: &ParserState,
    target_study_id: u32,
    batch: &mut Vec<crate::dataframe::TrialRow>,
    batch_size: usize,
    param_set: &mut std::collections::BTreeSet<String>,
    uan_set: &mut std::collections::BTreeSet<String>,
    uas_set: &mut std::collections::BTreeSet<String>,
    derived_objective_names: &mut Vec<String>,
    has_constraints: &mut bool,
    max_constraints: &mut usize,
    completed: &mut u32,
    first_sent: &mut bool,
    on_batch: &mut F,
) where
    F: FnMut(StudyStreamBatch),
{
    use crate::dataframe::TrialRow;

    let b = builder;
    for name in b.param_display.keys() {
        param_set.insert(name.clone());
    }
    for name in b.param_category_label.keys() {
        param_set.insert(name.clone());
    }
    for name in b.user_attrs_numeric.keys() {
        uan_set.insert(name.clone());
    }
    for name in b.user_attrs_string.keys() {
        uas_set.insert(name.clone());
    }
    if b.has_constraints {
        *has_constraints = true;
    }
    *max_constraints = (*max_constraints).max(b.constraint_values.len());
    if derived_objective_names.is_empty() {
        if let Some(values) = &b.values {
            *derived_objective_names = (0..values.len()).map(|i| format!("obj{i}")).collect();
        }
    }
    *completed += 1;
    batch.push(TrialRow {
        trial_id: tid,
        trial_number: b.trial_number,
        param_display: b.param_display,
        param_category_label: b.param_category_label,
        objective_values: b.values.unwrap_or_default(),
        user_attrs_numeric: b.user_attrs_numeric,
        user_attrs_string: b.user_attrs_string,
        constraint_values: b.constraint_values,
    });

    if batch.len() >= batch_size {
        let meta = stream_build_meta(
            state,
            target_study_id,
            param_set,
            uan_set,
            uas_set,
            derived_objective_names,
            *has_constraints,
            *completed,
        );
        let objective_names = if !state.studies[target_study_id as usize]
            .objective_names
            .is_empty()
        {
            state.studies[target_study_id as usize]
                .objective_names
                .clone()
        } else {
            derived_objective_names.clone()
        };
        on_batch(StudyStreamBatch {
            meta,
            new_rows: std::mem::take(batch),
            param_names: param_set.iter().cloned().collect(),
            objective_names,
            user_attr_numeric_names: uan_set.iter().cloned().collect(),
            user_attr_string_names: uas_set.iter().cloned().collect(),
            max_constraints: *max_constraints,
            is_first: !*first_sent,
            is_final: false,
        });
        *first_sent = true;
    }
}

/// 対象 study の完了 Trial を前方 1 パスで解析し、`batch_size` 件ごとに `on_batch` へ送出する。
///
/// `parse_single_study` と異なり、全件パース完了を待たずに完了 Trial を逐次出力するため、
/// UI 側で「読み込みながら描画」できる。完了は op_code=6 / state==1 の時点で確定し、
/// それまでに設定された params/attrs/constraints を取り込む（live_update と同じ逐次セマンティクス）。
///
/// 列名集合（param/user_attr）は Trial をまたいで累積され、各バッチへ同梱する。
pub fn parse_single_study_streaming<F>(
    data: &[u8],
    target_study_id: u32,
    batch_size: usize,
    mut on_batch: F,
) -> Result<(), String>
where
    F: FnMut(StudyStreamBatch),
{
    use crate::dataframe::TrialRow;
    use std::collections::BTreeSet;

    if data.is_empty() {
        return Err("Empty journal data".to_string());
    }
    let text = String::from_utf8_lossy(data);
    let mut state = ParserState::new_with_target(target_study_id);

    let batch_size = batch_size.max(1);
    let mut batch: Vec<TrialRow> = Vec::with_capacity(batch_size);

    let mut param_set: BTreeSet<String> = BTreeSet::new();
    let mut uan_set: BTreeSet<String> = BTreeSet::new();
    let mut uas_set: BTreeSet<String> = BTreeSet::new();
    let mut derived_objective_names: Vec<String> = Vec::new();
    let mut has_constraints = false;
    let mut max_constraints = 0usize;
    let mut completed = 0u32;
    let mut any_valid = false;
    let mut first_sent = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let op = match quick_extract_u32(line, "op_code") {
            #[allow(clippy::cast_possible_truncation)]
            Some(op) => op as u8,
            None => continue,
        };

        match op {
            0 => {
                if let Ok(json) = serde_json::from_str::<Value>(line) {
                    any_valid = true;
                    state.process_op(0, &json);
                }
            }
            3 => {
                any_valid = true;
                if line.contains("study:metric_names") {
                    if let Ok(json) = serde_json::from_str::<Value>(line) {
                        state.process_op(3, &json);
                    }
                }
            }
            4 => {
                any_valid = true;
                let mut parsed_target = false;
                match quick_extract_u32(line, "study_id") {
                    Some(sid) if sid == target_study_id => {
                        if let Ok(json) = serde_json::from_str::<Value>(line) {
                            state.process_op(4, &json);
                            parsed_target = true;
                        }
                    }
                    Some(sid) => {
                        // 他 study: trial_id カウンタの整合のみ維持（JSON 不要）。
                        state.next_trial_id += 1;
                        if (sid as usize) < state.studies.len() {
                            state.studies[sid as usize].total_trials += 1;
                        }
                    }
                    None => {
                        if let Ok(json) = serde_json::from_str::<Value>(line) {
                            state.process_op(4, &json);
                            parsed_target = true;
                        }
                    }
                }
                // in-memory ストレージは op_code=4 に state/values/params をインラインで持ち、
                // 後続の op_code=6 が来ない。ここで完了 Trial（state==1）を確定・送出する。
                // ファイルストレージの op_code=4 は state==0 のため builder を残して 5/6 を待つ。
                if parsed_target {
                    let tid = state.next_trial_id.wrapping_sub(1);
                    let trial_state = state.trial_builders.get(&tid).map(|b| b.state).unwrap_or(0);
                    if trial_state == 1 {
                        let b = state.trial_builders.remove(&tid).unwrap();
                        stream_emit_completed_trial(
                            tid,
                            b,
                            &state,
                            target_study_id,
                            &mut batch,
                            batch_size,
                            &mut param_set,
                            &mut uan_set,
                            &mut uas_set,
                            &mut derived_objective_names,
                            &mut has_constraints,
                            &mut max_constraints,
                            &mut completed,
                            &mut first_sent,
                            &mut on_batch,
                        );
                    } else if trial_state == 2 || trial_state == 3 {
                        state.trial_builders.remove(&tid);
                    }
                }
            }
            5 | 6 | 8 | 9 => {
                any_valid = true;
                let Some(tid) = quick_extract_u32(line, "trial_id") else {
                    continue;
                };
                if !state.trial_builders.contains_key(&tid) {
                    continue; // 対象 study 以外の trial → 無視
                }
                let Ok(json) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                state.process_op(op, &json);

                if op != 6 {
                    continue;
                }
                // op6 完了判定: state==1 で確定行を送出、2/3（prune/fail）は破棄。
                let trial_state = state.trial_builders.get(&tid).map(|b| b.state).unwrap_or(0);
                if trial_state == 1 {
                    let b = state.trial_builders.remove(&tid).unwrap();
                    stream_emit_completed_trial(
                        tid,
                        b,
                        &state,
                        target_study_id,
                        &mut batch,
                        batch_size,
                        &mut param_set,
                        &mut uan_set,
                        &mut uas_set,
                        &mut derived_objective_names,
                        &mut has_constraints,
                        &mut max_constraints,
                        &mut completed,
                        &mut first_sent,
                        &mut on_batch,
                    );
                } else if trial_state == 2 || trial_state == 3 {
                    state.trial_builders.remove(&tid);
                }
            }
            _ => {}
        }
    }

    if !any_valid {
        return Err("No valid JSON lines found in journal".to_string());
    }
    if (target_study_id as usize) >= state.studies.len() {
        return Err(format!("study_id {target_study_id} not found in journal"));
    }

    // 最終バッチ（残り）を送出。完了 0 件でも is_final を通知する。
    let objective_names = if !state.studies[target_study_id as usize]
        .objective_names
        .is_empty()
    {
        state.studies[target_study_id as usize]
            .objective_names
            .clone()
    } else {
        derived_objective_names.clone()
    };
    let meta = stream_build_meta(
        &state,
        target_study_id,
        &param_set,
        &uan_set,
        &uas_set,
        &derived_objective_names,
        has_constraints,
        completed,
    );
    on_batch(StudyStreamBatch {
        meta,
        new_rows: std::mem::take(&mut batch),
        param_names: param_set.iter().cloned().collect(),
        objective_names,
        user_attr_numeric_names: uan_set.iter().cloned().collect(),
        user_attr_string_names: uas_set.iter().cloned().collect(),
        max_constraints,
        is_first: !first_sent,
        is_final: true,
    });

    Ok(())
}

#[cfg(test)]
mod tests;
