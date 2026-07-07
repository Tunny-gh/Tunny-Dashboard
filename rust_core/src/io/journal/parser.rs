//! Optuna JournalStorage（JSON Lines）のパーサ。
//!
//! 一括解析（`parse_journal`）、Study 一覧の高速スキャン（`scan_study_list`）、
//! 指定 study のオンデマンド解析（`parse_single_study` / `parse_single_study_streaming`）を提供する。
//!
//! Reference: docs/implements/TASK-101/journal-parser-requirements.md

mod builders;
pub(crate) mod distribution;
mod finalize;
mod state;
mod types;

use serde_json::Value;

use super::line_u32_field;
use finalize::finalize_state;
use state::{get_str, get_u64, ParserState};

pub use types::{OptimizationDirection, ParseResult, StudyMeta};

#[cfg(test)]
use builders::TrialBuilder;
#[cfg(test)]
use distribution::Distribution;

/// Journal 全体を一括パースし、全 study の `StudyMeta` を返す。
///
/// 併せて全 study の `DataFrame`（COMPLETE trial）と `StudyExtras`（全 state の付帯情報）を
/// 構築し、共有ストア（`crate::dataframe`）へ格納する。不正な JSON 行は読み飛ばし、
/// 有効行が 1 行も無い場合はエラーを返す。
pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
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

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let finalized = finalize_state(state);
    let mut studies = Vec::with_capacity(finalized.len());
    let mut dataframes = Vec::with_capacity(finalized.len());
    let mut extras = Vec::with_capacity(finalized.len());
    for study in finalized {
        studies.push(study.meta);
        dataframes.push(study.dataframe);
        extras.push(study.extras);
    }
    crate::dataframe::store_dataframes(dataframes);
    crate::dataframe::store_extras(extras);

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
    // study 名の重複チェック用（Vec の線形探索を避ける）。
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // op_code は Optuna journal の各行で必ず先頭フィールド (`{"op_code":N,...`)。
        // 1 回だけ抽出して分岐し、行全体を何度も走査する contains を排除する。
        // Trial 行（op_code 4/5/6/8/9）は全体の 99% 以上を占めるため、ここで即除外する。
        let op = match line_u32_field(line, "op_code") {
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
                // 同名 study の重複 create_study 行はスキップ（HashSet で O(1) 判定）。
                if !seen_names.insert(name.clone()) {
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

/// op_code=0（CREATE_STUDY）/ 3（SET_STUDY_SYSTEM_ATTR）の共通処理
/// （`parse_single_study` Pass 1 と `parse_single_study_streaming` で共用）。
///
/// op0 は数が少ないため即 JSON パースする。op3 の大半は巨大な sampler 属性行のため、
/// `study:metric_names` を含む行のみパースする。戻り値は「有効行として数えるか」
/// （op0 はパース成功時のみ、op3 は常に true）。
fn process_study_meta_op(state: &mut ParserState, op: u8, line: &str) -> bool {
    match op {
        0 => {
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                state.process_op(0, &json);
                true
            } else {
                false
            }
        }
        3 => {
            if line.contains("study:metric_names") {
                if let Ok(json) = serde_json::from_str::<Value>(line) {
                    state.process_op(3, &json);
                }
            }
            true
        }
        _ => false,
    }
}

/// op_code=4 の非対象 study 行の共通処理: JSON パースせず、trial_id カウンタの整合と
/// 該当 study の total_trials のみ更新する（`parse_single_study` / streaming で共用）。
fn count_other_study_trial(state: &mut ParserState, sid: u32) {
    state.next_trial_id += 1;
    if (sid as usize) < state.studies.len() {
        state.studies[sid as usize].total_trials += 1;
    }
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
) -> Result<
    (
        StudyMeta,
        crate::data::dataframe::DataFrame,
        crate::data::extras::StudyExtras,
    ),
    String,
> {
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

        let op = match line_u32_field(line, "op_code") {
            #[allow(clippy::cast_possible_truncation)]
            Some(op) => op as u8,
            None => continue,
        };

        match op {
            0 | 3 => {
                if process_study_meta_op(&mut state, op, line) {
                    any_valid = true;
                }
            }
            4 => {
                any_valid = true;
                let pre_trial_id = state.next_trial_id;
                match line_u32_field(line, "study_id") {
                    Some(sid) if sid == target_study_id => {
                        // 対象 Study → Pass 2 へ回す（カウンタは先に進める）
                        state.next_trial_id += 1;
                        target_trial_ids.insert(pre_trial_id);
                        deferred.push((line, 4, Some(pre_trial_id)));
                    }
                    Some(sid) => {
                        // 他 Study → JSON 不要、カウンタのみ更新
                        count_other_study_trial(&mut state, sid);
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
            5..=9 => {
                // 試行更新系（op7 の中間値含む）: 対象 trial_id の行のみ Pass 2 へ
                any_valid = true;
                if let Some(tid) = line_u32_field(line, "trial_id") {
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

    let study = finalize_state(state)
        .into_iter()
        .find(|s| s.meta.study_id == target_study_id)
        .ok_or_else(|| format!("study_id {target_study_id} not found in journal"))?;

    Ok((study.meta, study.dataframe, study.extras))
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

/// `parse_single_study_streaming` の累積状態（バッチ・列名集合・カウンタ）。
///
/// 旧 `stream_emit_completed_trial` が約 15 個の引数で受け渡していた可変状態を
/// 1 つの struct に集約したもの（挙動は不変）。
struct StreamAccum {
    batch: Vec<crate::dataframe::TrialRow>,
    batch_size: usize,
    param_set: std::collections::BTreeSet<String>,
    uan_set: std::collections::BTreeSet<String>,
    uas_set: std::collections::BTreeSet<String>,
    derived_objective_names: Vec<String>,
    has_constraints: bool,
    max_constraints: usize,
    completed: u32,
    first_sent: bool,
}

impl StreamAccum {
    fn new(batch_size: usize) -> Self {
        let batch_size = batch_size.max(1);
        StreamAccum {
            batch: Vec::with_capacity(batch_size),
            batch_size,
            param_set: std::collections::BTreeSet::new(),
            uan_set: std::collections::BTreeSet::new(),
            uas_set: std::collections::BTreeSet::new(),
            derived_objective_names: Vec::new(),
            has_constraints: false,
            max_constraints: 0,
            completed: 0,
            first_sent: false,
        }
    }

    /// 目的名を選ぶ: study builder 側の確定名があればそれ、無ければ完了 trial の
    /// values 数から導出した `obj{i}`。
    fn objective_names(&self, state: &ParserState, target: u32) -> Vec<String> {
        let builder_names = &state.studies[target as usize].objective_names;
        if builder_names.is_empty() {
            self.derived_objective_names.clone()
        } else {
            builder_names.clone()
        }
    }

    /// 累積集合から StudyMeta スナップショットを生成する。
    fn snapshot_meta(&self, state: &ParserState, target: u32) -> StudyMeta {
        let builder = &state.studies[target as usize];
        // user_attr_names は数値・文字列をマージしソート（既存 finalize と同等）。
        let mut user_attr_names: Vec<String> = self
            .uan_set
            .iter()
            .chain(self.uas_set.iter())
            .cloned()
            .collect();
        user_attr_names.sort();
        user_attr_names.dedup();
        StudyMeta {
            study_id: target,
            name: builder.name.clone(),
            directions: builder.directions.clone(),
            completed_trials: self.completed,
            total_trials: builder.total_trials,
            param_names: self.param_set.iter().cloned().collect(),
            objective_names: self.objective_names(state, target),
            user_attr_names,
            has_constraints: self.has_constraints,
            param_bounds: builder.param_bounds.clone(),
        }
    }

    /// 現在の累積内容から `StudyStreamBatch` を組み立てて送出する
    /// （`batch` は take されて空になる）。
    fn send_batch<F>(&mut self, state: &ParserState, target: u32, is_final: bool, on_batch: &mut F)
    where
        F: FnMut(StudyStreamBatch),
    {
        let meta = self.snapshot_meta(state, target);
        let objective_names = self.objective_names(state, target);
        on_batch(StudyStreamBatch {
            meta,
            new_rows: std::mem::take(&mut self.batch),
            param_names: self.param_set.iter().cloned().collect(),
            objective_names,
            user_attr_numeric_names: self.uan_set.iter().cloned().collect(),
            user_attr_string_names: self.uas_set.iter().cloned().collect(),
            max_constraints: self.max_constraints,
            is_first: !self.first_sent,
            is_final,
        });
        self.first_sent = true;
    }

    /// 完了 Trial（state==1）の builder をバッチへ取り込み、列名集合・カウンタを更新する。
    /// バッチが満杯になったら `on_batch` で送出する。
    fn emit_completed_trial<F>(
        &mut self,
        tid: u32,
        builder: builders::TrialBuilder,
        state: &ParserState,
        target: u32,
        on_batch: &mut F,
    ) where
        F: FnMut(StudyStreamBatch),
    {
        use crate::dataframe::TrialRow;

        let b = builder;
        for name in b.param_display.keys() {
            self.param_set.insert(name.clone());
        }
        for name in b.param_category_label.keys() {
            self.param_set.insert(name.clone());
        }
        for name in b.user_attrs_numeric.keys() {
            self.uan_set.insert(name.clone());
        }
        for name in b.user_attrs_string.keys() {
            self.uas_set.insert(name.clone());
        }
        if b.has_constraints {
            self.has_constraints = true;
        }
        self.max_constraints = self.max_constraints.max(b.constraint_values.len());
        if self.derived_objective_names.is_empty() {
            if let Some(values) = &b.values {
                self.derived_objective_names =
                    (0..values.len()).map(|i| format!("obj{i}")).collect();
            }
        }
        self.completed += 1;
        self.batch.push(TrialRow {
            trial_id: tid,
            trial_number: b.trial_number,
            param_display: b.param_display,
            param_category_label: b.param_category_label,
            objective_values: b.values.unwrap_or_default(),
            user_attrs_numeric: b.user_attrs_numeric,
            user_attrs_string: b.user_attrs_string,
            constraint_values: b.constraint_values,
        });

        if self.batch.len() >= self.batch_size {
            self.send_batch(state, target, false, on_batch);
        }
    }

    /// trial の状態確定処理（op_code=4 のインライン state / op_code=6 で共通）。
    ///
    /// in-memory ストレージは op_code=4 にすべての Trial データ（state / values / params）を
    /// インラインで持ち、後続の op_code=6 が来ない。一方ファイルストレージは op_code=6 で
    /// 完了する。state==1（完了）なら確定行を送出し、2/3（prune/fail）は builder を破棄して
    /// extras のみ記録する。それ以外（RUNNING）は builder を残して後続 op を待つ。
    fn resolve_trial_state<F>(
        &mut self,
        state: &mut ParserState,
        tid: u32,
        target: u32,
        extras_trials: &mut Vec<crate::data::extras::TrialExtra>,
        on_batch: &mut F,
    ) where
        F: FnMut(StudyStreamBatch),
    {
        let trial_state = state.trial_builders.get(&tid).map(|b| b.state).unwrap_or(0);
        if trial_state == 1 {
            let b = state.trial_builders.remove(&tid).unwrap();
            extras_trials.push(trial_extra_from_builder(tid, &b));
            self.emit_completed_trial(tid, b, state, target, on_batch);
        } else if trial_state == 2 || trial_state == 3 {
            if let Some(b) = state.trial_builders.remove(&tid) {
                extras_trials.push(trial_extra_from_builder(tid, &b));
            }
        }
    }
}

/// 除去（完了/prune/fail 確定または EOF 時点）される `TrialBuilder` から
/// `TrialExtra` を構築する。中間値は step 昇順にそろえる。
fn trial_extra_from_builder(
    trial_id: u32,
    b: &builders::TrialBuilder,
) -> crate::data::extras::TrialExtra {
    let mut intermediate_values = b.intermediate_values.clone();
    intermediate_values.sort_by_key(|(step, _)| *step);
    crate::data::extras::TrialExtra {
        trial_id,
        trial_number: b.trial_number,
        state: crate::data::extras::TrialState::from_journal(b.state),
        datetime_start: b.datetime_start,
        datetime_complete: b.datetime_complete,
        intermediate_values,
    }
}

/// 対象 study の完了 Trial を前方 1 パスで解析し、`batch_size` 件ごとに `on_batch` へ送出する。
///
/// `parse_single_study` と異なり、全件パース完了を待たずに完了 Trial を逐次出力するため、
/// UI 側で「読み込みながら描画」できる。完了は op_code=6 / state==1 の時点で確定し、
/// それまでに設定された params/attrs/constraints を取り込む（live_update と同じ逐次セマンティクス）。
///
/// 列名集合（param/user_attr）は Trial をまたいで累積され、各バッチへ同梱する。
///
/// 全 trial（全 state）の付帯情報（[`crate::data::extras::StudyExtras`]）もあわせて収集し、
/// 完了時に `store_extras_for` で共有ストアへ格納する（`parse_journal` が
/// `store_extras` で格納するのと対になる、単一 study ロード経路の格納口）。
pub fn parse_single_study_streaming<F>(
    data: &[u8],
    target_study_id: u32,
    batch_size: usize,
    mut on_batch: F,
) -> Result<(), String>
where
    F: FnMut(StudyStreamBatch),
{
    if data.is_empty() {
        return Err("Empty journal data".to_string());
    }
    let text = String::from_utf8_lossy(data);
    let mut state = ParserState::new_with_target(target_study_id);
    let mut acc = StreamAccum::new(batch_size);
    let mut any_valid = false;
    // 全 trial（全 state）の付帯情報。除去時（完了/prune/fail/EOF）に随時追加する。
    let mut extras_trials: Vec<crate::data::extras::TrialExtra> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let op = match line_u32_field(line, "op_code") {
            #[allow(clippy::cast_possible_truncation)]
            Some(op) => op as u8,
            None => continue,
        };

        match op {
            0 | 3 => {
                if process_study_meta_op(&mut state, op, line) {
                    any_valid = true;
                }
            }
            4 => {
                any_valid = true;
                let mut parsed_target = false;
                match line_u32_field(line, "study_id") {
                    Some(sid) if sid == target_study_id => {
                        if let Ok(json) = serde_json::from_str::<Value>(line) {
                            state.process_op(4, &json);
                            parsed_target = true;
                        }
                    }
                    Some(sid) => {
                        // 他 study: trial_id カウンタの整合のみ維持（JSON 不要）。
                        count_other_study_trial(&mut state, sid);
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
                    acc.resolve_trial_state(
                        &mut state,
                        tid,
                        target_study_id,
                        &mut extras_trials,
                        &mut on_batch,
                    );
                }
            }
            5..=9 => {
                any_valid = true;
                let Some(tid) = line_u32_field(line, "trial_id") else {
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
                acc.resolve_trial_state(
                    &mut state,
                    tid,
                    target_study_id,
                    &mut extras_trials,
                    &mut on_batch,
                );
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

    // EOF 時点でまだ残っている trial_builders（対象 study のみ生成される）は
    // 完了/prune/fail 未確定＝ Running のまま。extras に取り込む。
    for (tid, b) in state.trial_builders.drain() {
        extras_trials.push(trial_extra_from_builder(tid, &b));
    }
    extras_trials.sort_by_key(|t| t.trial_id);
    crate::dataframe::store_extras_for(
        target_study_id,
        crate::data::extras::StudyExtras {
            trials: extras_trials,
        },
    );

    // 最終バッチ（残り）を送出。完了 0 件でも is_final を通知する。
    acc.send_batch(&state, target_study_id, true, &mut on_batch);

    Ok(())
}

#[cfg(test)]
mod tests;
