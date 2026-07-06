//! `StudyReport` の構築（既存解析 API の合成）。
//!
//! ここでは新規の解析ロジックを発明せず、`multi_objective` / `mcdm` /
//! `statistics` / `math` / `convergence` の既存公開関数を合成してモデルを埋める。
//! チャートに必要な系列はすべてここで計算し、レンダラは描くだけにする。
//!
//! ## 方向の扱い
//!
//! [`OptimizationDirection`] を目的ごとに尊重する。`is_minimize[j]` は
//! `matches!(directions[j], Minimize)` で、既存 MCDM / pareto / convergence の
//! 呼び出し規約に一致させる。

use std::collections::{BTreeMap, BTreeSet};

use crate::convergence::build_best_trial_history;
use crate::data::dataframe::DataFrame;
use crate::data::extras::{StudyExtras, TrialState};
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};
use crate::mcdm::{promethee, topsis, vikor};
use crate::multi_objective::pareto::{compute_hv_history_from_data, nd_sort};
use crate::statistics::{compute_histogram, quantile, BinRule};

use super::findings::{self, FindingInputs};
use super::model::*;
use super::{format_number, ReportOptions, ReportSource};

/// 収束の後半20%判定に用いる、VIKOR の妥協解パラメータ（標準値）。
const VIKOR_V: f64 = 0.5;
/// 収束系列の最大点数（間引き上限）。
const MAX_SERIES_POINTS: usize = 500;
/// ヒストグラムの最大ビン数。
const MAX_HIST_BINS: usize = 20;

/// `(StudyMeta, DataFrame, StudyExtras)` から [`StudyReport`] を構築する。
///
/// `df` は COMPLETE trial のみを列指向で保持する前提。`extras` は全 state の
/// 付帯情報（無ければ実行時セクションと枝刈り finding を省略）。決定論性のため、
/// 出力に現れる集合はすべてソート済み / BTree で構成する。
pub fn build_study_report(
    meta: &StudyMeta,
    df: &DataFrame,
    extras: Option<&StudyExtras>,
    source: &ReportSource,
    opts: &ReportOptions,
) -> StudyReport {
    let directions: Vec<Direction> = meta
        .directions
        .iter()
        .map(|d| match d {
            OptimizationDirection::Minimize => Direction::Minimize,
            OptimizationDirection::Maximize => Direction::Maximize,
        })
        .collect();
    let is_minimize: Vec<bool> = directions.iter().map(|d| d.is_minimize()).collect();
    let m = meta.objective_names.len();
    let is_multi = m >= 2;
    let n = df.row_count();

    // 目的列と目的値行列（NaN 許容）。
    let obj_cols: Vec<Option<&[f64]>> = meta
        .objective_names
        .iter()
        .map(|name| df.get_numeric_column(name))
        .collect();
    let objectives: Vec<Vec<f64>> = (0..n)
        .map(|row| {
            obj_cols
                .iter()
                .map(|c| c.and_then(|c| c.get(row)).copied().unwrap_or(f64::NAN))
                .collect()
        })
        .collect();
    let trial_numbers: Vec<u32> = (0..n)
        .map(|row| df.get_trial_number(row).unwrap_or(row as u32))
        .collect();

    // 有効行（全目的が有限）。
    let valid_row: Vec<bool> = objectives
        .iter()
        .map(|o| o.len() == m && o.iter().all(|v| v.is_finite()))
        .collect();
    let valid_count = valid_row.iter().filter(|&&v| v).count();
    let nan_count = n - valid_count;

    // state 内訳・FAIL 数・実測所要時間。
    let (state_counts, fail_count, wall_clock) = state_summary(extras, n);

    // パレート前面（多目的のみ意味を持つ）。制約付きスタディでは feasible 行
    // のみで非劣ソートする（Optuna の constrained 最適化の意味論に合わせる）。
    // feasible 行が 1 件も無い場合のみ、全行の目的空間非劣解へフォールバック
    // する（前面が空になるのを避け、レンダラ側の違反注記で透明化する）。
    let feas = df.feasibility();
    let front_rows: Vec<usize> = if is_multi && n > 0 {
        let feasible_rows: Vec<usize> = (0..n)
            .filter(|&r| valid_row[r] && feas.is_feasible(r))
            .collect();
        if feas.has_constraints() && !feasible_rows.is_empty() {
            let sub: Vec<Vec<f64>> = feasible_rows
                .iter()
                .map(|&r| objectives[r].clone())
                .collect();
            let sub_ranks = nd_sort(&sub, &is_minimize);
            feasible_rows
                .iter()
                .zip(&sub_ranks)
                .filter(|&(_, &rank)| rank == 0)
                .map(|(&r, _)| r)
                .collect()
        } else {
            let ranks = nd_sort(&objectives, &is_minimize);
            (0..n).filter(|&r| ranks[r] == 0 && valid_row[r]).collect()
        }
    } else {
        Vec::new()
    };
    let mut on_front = vec![false; n];
    for &r in &front_rows {
        on_front[r] = true;
    }

    // ---- Overview ----
    let mut param_bounds: Vec<(String, f64, f64)> = meta
        .param_bounds
        .iter()
        .map(|(k, (lo, hi))| (k.clone(), *lo, *hi))
        .collect();
    param_bounds.sort_by(|a, b| a.0.cmp(&b.0));

    let overview = Overview {
        name: meta.name.clone(),
        directions: directions.clone(),
        objective_names: meta.objective_names.clone(),
        param_names: meta.param_names.clone(),
        user_attr_names: meta.user_attr_names.clone(),
        state_counts: state_counts.clone(),
        complete_trials: n,
        total_trials: meta.total_trials as usize,
        wall_clock_seconds: wall_clock,
        param_bounds,
        has_constraints: meta.has_constraints,
    };

    // ---- Convergence ----
    let convergence = if is_multi {
        build_convergence_multi(&objectives, &trial_numbers, &is_minimize, valid_count)
    } else {
        build_convergence_single(&objectives, &trial_numbers, &valid_row, &is_minimize, m)
    };

    // ---- Outcome ----
    let (outcome, mcdm) = if is_multi {
        build_outcome_multi(
            df,
            meta,
            &objectives,
            &trial_numbers,
            &valid_row,
            &on_front,
            &front_rows,
            &directions,
            &is_minimize,
            valid_count,
            m,
            opts,
        )
    } else {
        (
            build_outcome_single(
                df,
                meta,
                &objectives,
                &trial_numbers,
                &valid_row,
                &is_minimize,
                opts,
            ),
            None,
        )
    };

    // ---- Importance ----
    let importance = build_importance(df, meta, &objectives, n);

    // ---- Objective stats ----
    let objective_stats = build_objective_stats(&objectives, meta, &directions, m);

    // ---- Correlations ----
    let correlations = build_correlations(df, meta, &objectives, n, opts);

    // ---- Execution ----
    let execution = extras.map(|ex| build_execution(ex, &state_counts, wall_clock));

    // ---- Findings ----
    let best_single = if !is_multi {
        best_single_fact(&objectives, &trial_numbers, &valid_row, &is_minimize)
    } else {
        None
    };
    let trade_off = if is_multi {
        trade_off_fact(&objectives, meta, m)
    } else {
        None
    };
    let feasibility = if meta.has_constraints {
        feasibility_fact(
            df,
            &objectives,
            &trial_numbers,
            &valid_row,
            &is_minimize,
            is_multi,
            n,
        )
    } else {
        None
    };
    let pruning = extras.map(pruning_fact);
    let top_importance: Vec<(String, f64)> = importance
        .as_ref()
        .map(|s| s.scores.iter().take(3).cloned().collect())
        .unwrap_or_default();
    let importance_method = importance.as_ref().map(|s| s.method.clone());

    let finding_inputs = FindingInputs {
        is_multi,
        best_single,
        pareto: if is_multi {
            Some((front_rows.len(), valid_count))
        } else {
            None
        },
        convergence_status: convergence.status,
        top_importance,
        importance_method,
        trade_off,
        feasibility,
        pruning,
        data_quality: Some((nan_count, fail_count)),
    };
    let key_findings = findings::generate_findings(&finding_inputs);

    StudyReport {
        schema_version: SCHEMA_VERSION,
        source: ReportSourceInfo {
            storage_display: source.storage_display.clone(),
            generated_at_unix: source.generated_at_unix,
        },
        overview,
        key_findings,
        outcome,
        convergence,
        importance,
        objective_stats,
        correlations,
        mcdm,
        execution,
        reproduction: Reproduction {
            study_id: meta.study_id,
            storage_display: source.storage_display.clone(),
            top_n: opts.top_n,
            max_heatmap_params: opts.max_heatmap_params,
            schema_version: SCHEMA_VERSION,
        },
    }
}

// =============================================================================
// 共通ヘルパー
// =============================================================================

/// 2 系列のペアワイズ Spearman（両側有限の行のみ使用）。
fn spearman_pairwise(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    let (fx, fy): (Vec<f64>, Vec<f64>) = x[..n]
        .iter()
        .zip(&y[..n])
        .filter(|&(&a, &b)| a.is_finite() && b.is_finite())
        .map(|(&a, &b)| (a, b))
        .unzip();
    if fx.len() < 2 {
        return f64::NAN;
    }
    crate::math::stats::spearman_correlation(&fx, &fy)
}

/// 系列を最大 `max` 点へ均等間引き（先頭・末尾を保持）。
fn downsample<T: Clone>(pts: &[T], max: usize) -> Vec<T> {
    if pts.len() <= max || max < 2 {
        return pts.to_vec();
    }
    let last = pts.len() - 1;
    (0..max)
        .map(|k| pts[k * last / (max - 1)].clone())
        .collect()
}

/// 単調 best-so-far 系列で、最後に改善（更新）が起きた index を返す。
/// 先頭（index 0）は初回として常に改善とみなす。
fn last_improve_index(best_series: &[f64], minimize: bool) -> usize {
    let mut last = 0;
    for i in 1..best_series.len() {
        let improved = if minimize {
            best_series[i] < best_series[i - 1]
        } else {
            best_series[i] > best_series[i - 1]
        };
        if improved {
            last = i;
        }
    }
    last
}

/// state 内訳（全 state）・FAIL 数・実測所要時間（秒）を返す。
fn state_summary(
    extras: Option<&StudyExtras>,
    complete_n: usize,
) -> (BTreeMap<String, usize>, usize, Option<f64>) {
    let mut counts = BTreeMap::new();
    let Some(ex) = extras else {
        counts.insert("COMPLETE".to_string(), complete_n);
        return (counts, 0, None);
    };
    for t in &ex.trials {
        *counts.entry(t.state.label().to_string()).or_insert(0) += 1;
    }
    let fail = counts.get("FAIL").copied().unwrap_or(0);

    let mut min_start = f64::INFINITY;
    let mut max_complete = f64::NEG_INFINITY;
    for t in &ex.trials {
        if let Some(s) = t.datetime_start {
            if s.is_finite() {
                min_start = min_start.min(s);
            }
        }
        if let Some(c) = t.datetime_complete {
            if c.is_finite() {
                max_complete = max_complete.max(c);
            }
        }
    }
    let wall = if min_start.is_finite() && max_complete.is_finite() && max_complete >= min_start {
        Some(max_complete - min_start)
    } else {
        None
    };
    (counts, fail, wall)
}

/// 1 行分の [`TrialSummary`] を構築する。
fn build_trial_summary(df: &DataFrame, meta: &StudyMeta, row: usize) -> TrialSummary {
    let trial_number = df.get_trial_number(row).unwrap_or(row as u32);
    let objectives: Vec<f64> = meta
        .objective_names
        .iter()
        .map(|name| {
            df.get_numeric_column(name)
                .and_then(|c| c.get(row))
                .copied()
                .unwrap_or(f64::NAN)
        })
        .collect();

    let params: Vec<(String, ParamValue)> = meta
        .param_names
        .iter()
        .map(|name| {
            if let Some(col) = df.get_numeric_column(name) {
                (
                    name.clone(),
                    ParamValue::Num(col.get(row).copied().unwrap_or(f64::NAN)),
                )
            } else if let Some(col) = df.get_string_column(name) {
                (
                    name.clone(),
                    ParamValue::Cat(col.get(row).cloned().unwrap_or_default()),
                )
            } else {
                (name.clone(), ParamValue::Cat(String::new()))
            }
        })
        .collect();

    // 生の制約値の行内最大。合計（constraint_sum）は負のマージンが正の違反を
    // 打ち消して feasible に見えてしまうため使わない（max ≤ 0 ⟺ 全制約充足）。
    let max_constraint = if meta.has_constraints {
        df.constraint_col_names()
            .iter()
            .filter_map(|name| {
                df.get_numeric_column(name)
                    .and_then(|c| c.get(row))
                    .copied()
            })
            .reduce(f64::max)
    } else {
        None
    };

    let mut user_attrs: Vec<(String, String)> = Vec::new();
    for name in df.user_attr_string_col_names() {
        if let Some(col) = df.get_string_column(name) {
            user_attrs.push((name.clone(), col.get(row).cloned().unwrap_or_default()));
        }
    }
    for name in df.user_attr_numeric_col_names() {
        if let Some(col) = df.get_numeric_column(name) {
            let v = col.get(row).copied().unwrap_or(f64::NAN);
            user_attrs.push((name.clone(), format_number(v)));
        }
    }
    user_attrs.sort_by(|a, b| a.0.cmp(&b.0));

    TrialSummary {
        trial_number,
        objectives,
        params,
        max_constraint,
        user_attrs,
        duplicate_of: None,
    }
}

// =============================================================================
// Convergence
// =============================================================================

fn build_convergence_single(
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
    m: usize,
) -> ConvergenceSection {
    let minimize = is_minimize.first().copied().unwrap_or(true);
    // 有効行を trial.number 昇順に並べる。
    let mut seq: Vec<usize> = (0..objectives.len()).filter(|&r| valid_row[r]).collect();
    seq.sort_by_key(|&r| trial_numbers[r]);

    if m == 0 || seq.is_empty() {
        return ConvergenceSection {
            metric: ConvergenceMetric::BestSoFar,
            series: Vec::new(),
            found_at_trial_number: None,
            improved_in_last_20pct: false,
            status: findings::convergence_status(seq.len(), 0.0),
        };
    }

    let ids: Vec<u32> = seq.iter().map(|&r| trial_numbers[r]).collect();
    let vals: Vec<f64> = seq.iter().map(|&r| objectives[r][0]).collect();
    let history = build_best_trial_history(&ids, &vals, minimize);
    let best_series: Vec<f64> = history.iter().map(|&(_, v)| v).collect();

    let last_idx = last_improve_index(&best_series, minimize);
    let len = history.len();
    let frac = if len <= 1 {
        0.0
    } else {
        last_idx as f64 / (len - 1) as f64
    };

    let series: Vec<ConvergencePoint> = downsample(&history, MAX_SERIES_POINTS)
        .into_iter()
        .map(|(tn, v)| ConvergencePoint {
            trial_number: tn,
            value: v,
        })
        .collect();

    ConvergenceSection {
        metric: ConvergenceMetric::BestSoFar,
        series,
        found_at_trial_number: Some(history[last_idx].0),
        improved_in_last_20pct: frac >= findings::STILL_IMPROVING_FRACTION,
        status: findings::convergence_status(len, frac),
    }
}

fn build_convergence_multi(
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    is_minimize: &[bool],
    valid_count: usize,
) -> ConvergenceSection {
    let n = objectives.len();
    // trial.number 昇順に並べ替えて HV 推移を計算する。
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&r| trial_numbers[r]);
    let ord_ids: Vec<u32> = order.iter().map(|&r| trial_numbers[r]).collect();
    let ord_objs: Vec<Vec<f64>> = order.iter().map(|&r| objectives[r].clone()).collect();

    let hv = compute_hv_history_from_data(&ord_ids, &ord_objs, is_minimize);

    if hv.hv_values.is_empty() {
        return ConvergenceSection {
            metric: ConvergenceMetric::Hypervolume,
            series: Vec::new(),
            found_at_trial_number: None,
            improved_in_last_20pct: false,
            status: findings::convergence_status(valid_count, 0.0),
        };
    }

    // HV は単調非減少なので「改善 = 前値より大きい」で最後の更新位置を判定する。
    let last_idx = last_improve_index(&hv.hv_values, false);
    let len = hv.hv_values.len();
    let frac = if len <= 1 {
        0.0
    } else {
        last_idx as f64 / (len - 1) as f64
    };

    let points: Vec<(u32, f64)> = ord_ids
        .iter()
        .copied()
        .zip(hv.hv_values.iter().copied())
        .collect();
    let series: Vec<ConvergencePoint> = downsample(&points, MAX_SERIES_POINTS)
        .into_iter()
        .map(|(tn, v)| ConvergencePoint {
            trial_number: tn,
            value: v,
        })
        .collect();

    ConvergenceSection {
        metric: ConvergenceMetric::Hypervolume,
        series,
        found_at_trial_number: Some(ord_ids[last_idx]),
        improved_in_last_20pct: frac >= findings::STILL_IMPROVING_FRACTION,
        status: findings::convergence_status(valid_count, frac),
    }
}

// =============================================================================
// Outcome
// =============================================================================

#[allow(clippy::too_many_arguments)]
fn build_outcome_single(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
    opts: &ReportOptions,
) -> Outcome {
    if meta.objective_names.is_empty() {
        return Outcome::SingleObj {
            best_trial: None,
            top_n: Vec::new(),
        };
    }
    let minimize = is_minimize.first().copied().unwrap_or(true);
    let mut order: Vec<usize> = (0..objectives.len()).filter(|&r| valid_row[r]).collect();
    order.sort_by(|&a, &b| {
        let (va, vb) = (objectives[a][0], objectives[b][0]);
        let ord = va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal);
        let ord = if minimize { ord } else { ord.reverse() };
        ord.then(trial_numbers[a].cmp(&trial_numbers[b]))
    });

    let best_trial = order.first().map(|&r| build_trial_summary(df, meta, r));
    let top_n: Vec<TrialSummary> = order
        .iter()
        .take(opts.top_n)
        .map(|&r| build_trial_summary(df, meta, r))
        .collect();

    Outcome::SingleObj { best_trial, top_n }
}

#[allow(clippy::too_many_arguments)]
fn build_outcome_multi(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    on_front: &[bool],
    front_rows: &[usize],
    directions: &[Direction],
    is_minimize: &[bool],
    valid_count: usize,
    m: usize,
    opts: &ReportOptions,
) -> (Outcome, Option<McdmSection>) {
    // 目的ごとの極値。
    let mut per_objective_extremes = Vec::with_capacity(m);
    for j in 0..m {
        let minimize = is_minimize[j];
        let mut best_v = if minimize {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
        let mut worst_v = if minimize {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut best_row: Option<usize> = None;
        for row in 0..objectives.len() {
            if !valid_row[row] {
                continue;
            }
            let v = objectives[row][j];
            let is_best = if minimize { v < best_v } else { v > best_v };
            if best_row.is_none() || is_best {
                best_v = v;
                best_row = Some(row);
            }
            let is_worst = if minimize { v > worst_v } else { v < worst_v };
            if is_worst {
                worst_v = v;
            }
        }
        if let Some(br) = best_row {
            per_objective_extremes.push(ObjectiveExtreme {
                objective_index: j,
                objective_name: meta.objective_names[j].clone(),
                direction: directions[j],
                best_value: best_v,
                best_trial_number: trial_numbers[br],
                best_feasible: df.feasibility().is_feasible(br),
                worst_value: worst_v,
            });
        }
    }

    // 散布図（全 COMPLETE、先頭2目的軸）。
    let feas = df.feasibility();
    let scatter: Vec<ParetoPoint> = (0..objectives.len())
        .filter(|&r| valid_row[r])
        .map(|r| ParetoPoint {
            trial_number: trial_numbers[r],
            x: objectives[r][0],
            y: if m >= 2 { objectives[r][1] } else { f64::NAN },
            on_front: on_front[r],
            feasible: feas.is_feasible(r),
        })
        .collect();

    // MCDM（等重み、パレート前面部分集合）。
    let mcdm = build_mcdm(
        df,
        meta,
        objectives,
        trial_numbers,
        front_rows,
        is_minimize,
        m,
        opts,
    );

    // pareto_table は front 行全体を TOPSIS ランキングで並べ直す
    // （ランキング未計算なら front 行順）。top_n*2 で cap。
    let cap = opts.top_n.saturating_mul(2);
    let pareto_table_rows = mcdm_ordered_front(df, meta, objectives, front_rows, is_minimize, m)
        .unwrap_or_else(|| front_rows.to_vec());
    let mut pareto_table: Vec<TrialSummary> = pareto_table_rows
        .iter()
        .take(cap.max(1))
        .map(|&r| build_trial_summary(df, meta, r))
        .collect();
    mark_duplicate_objectives(&mut pareto_table);

    let outcome = Outcome::MultiObj {
        pareto_size: front_rows.len(),
        complete_count: valid_count,
        objective_count: m,
        per_objective_extremes,
        pareto_table,
        scatter,
        scatter_axes: (0, 1),
    };

    (outcome, mcdm)
}

/// 同一目的値ベクトルの trial を初出（最小 trial 番号）でマークする。
///
/// パレート前面では同一パラメータの再サンプル等で目的値が完全一致する
/// trial が並び得る。各グループの最小 trial 番号を正とし、それ以外の
/// `duplicate_of` に正の trial 番号を入れる。比較はビットパターン一致
/// （NaN 目的値同士も同一視、-0.0 と 0.0 は区別）で決定論的に行う。
fn mark_duplicate_objectives(table: &mut [TrialSummary]) {
    use std::collections::HashMap;
    let mut first_of: HashMap<Vec<u64>, u32> = HashMap::new();
    for t in table.iter() {
        let key: Vec<u64> = t.objectives.iter().map(|v| v.to_bits()).collect();
        first_of
            .entry(key)
            .and_modify(|n| *n = (*n).min(t.trial_number))
            .or_insert(t.trial_number);
    }
    for t in table.iter_mut() {
        let key: Vec<u64> = t.objectives.iter().map(|v| v.to_bits()).collect();
        let first = first_of[&key];
        if first != t.trial_number {
            t.duplicate_of = Some(first);
        }
    }
}

/// パレート前面行を TOPSIS ランキング順で返す（等重み）。front が空なら `None`。
fn mcdm_ordered_front(
    _df: &DataFrame,
    _meta: &StudyMeta,
    objectives: &[Vec<f64>],
    front_rows: &[usize],
    is_minimize: &[bool],
    m: usize,
) -> Option<Vec<usize>> {
    if front_rows.is_empty() {
        return None;
    }
    let k = front_rows.len();
    let values = flatten_front(objectives, front_rows, m);
    let weights = vec![1.0 / m as f64; m];
    let res = topsis::compute_topsis(&values, k, m, &weights, is_minimize).ok()?;
    Some(
        res.ranked_indices
            .iter()
            .map(|&sub| front_rows[sub as usize])
            .collect(),
    )
}

/// パレート前面部分集合の目的値を row-major に平坦化する。
fn flatten_front(objectives: &[Vec<f64>], front_rows: &[usize], m: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(front_rows.len() * m);
    for &r in front_rows {
        values.extend_from_slice(&objectives[r][..m]);
    }
    values
}

#[allow(clippy::too_many_arguments)]
fn build_mcdm(
    _df: &DataFrame,
    _meta: &StudyMeta,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    front_rows: &[usize],
    is_minimize: &[bool],
    m: usize,
    _opts: &ReportOptions,
) -> Option<McdmSection> {
    if front_rows.is_empty() || m == 0 {
        return None;
    }
    let k = front_rows.len();
    let values = flatten_front(objectives, front_rows, m);
    let weights = vec![1.0 / m as f64; m];

    let ts = topsis::compute_topsis(&values, k, m, &weights, is_minimize).ok()?;
    let vk = vikor::compute_vikor(&values, k, m, &weights, is_minimize, VIKOR_V).ok()?;
    let pr = promethee::compute_promethee(&values, k, m, &weights, is_minimize).ok()?;

    let entry = |ranked: &[u32], rank_i: usize| -> McdmEntry {
        let row = front_rows[ranked[rank_i] as usize];
        McdmEntry {
            rank: rank_i + 1,
            trial_number: trial_numbers[row],
            objectives: objectives[row].clone(),
        }
    };
    let top_entries = |ranked: &[u32]| -> Vec<McdmEntry> {
        (0..ranked.len().min(5)).map(|i| entry(ranked, i)).collect()
    };
    let top_set = |ranked: &[u32]| -> BTreeSet<u32> {
        ranked
            .iter()
            .take(10)
            .map(|&sub| trial_numbers[front_rows[sub as usize]])
            .collect()
    };

    let t10 = top_set(&ts.ranked_indices);
    let v10 = top_set(&vk.ranked_indices);
    let p10 = top_set(&pr.ranked_indices_ii);
    let consensus_trials: Vec<u32> = t10
        .iter()
        .filter(|x| v10.contains(x) && p10.contains(x))
        .copied()
        .collect();

    Some(McdmSection {
        weight_scheme: "equal".to_string(),
        weights,
        topsis_top: top_entries(&ts.ranked_indices),
        vikor_top: top_entries(&vk.ranked_indices),
        promethee_top: top_entries(&pr.ranked_indices_ii),
        consensus_trials,
    })
}

// =============================================================================
// Importance / Correlations / Stats
// =============================================================================

/// 数値パラメータのみを対象に、目的0 に対する |Spearman| を重要度とする。
fn build_importance(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    n: usize,
) -> Option<ImportanceSection> {
    if n < 2 || meta.objective_names.is_empty() {
        return None;
    }
    let y: Vec<f64> = objectives.iter().map(|o| o[0]).collect();
    let mut scores: Vec<(String, f64)> = Vec::new();
    for name in &meta.param_names {
        if let Some(col) = df.get_numeric_column(name) {
            let s = spearman_pairwise(col, &y).abs();
            if s.is_finite() {
                scores.push((name.clone(), s));
            }
        }
    }
    if scores.is_empty() {
        return None;
    }
    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    Some(ImportanceSection {
        method: "spearman_abs".to_string(),
        objective_name: meta.objective_names[0].clone(),
        scores,
    })
}

fn build_correlations(
    df: &DataFrame,
    meta: &StudyMeta,
    objectives: &[Vec<f64>],
    n: usize,
    opts: &ReportOptions,
) -> Option<CorrelationSection> {
    if n < 2 || meta.objective_names.is_empty() {
        return None;
    }
    // 数値パラメータ列のみ。
    let numeric_params: Vec<String> = meta
        .param_names
        .iter()
        .filter(|name| df.get_numeric_column(name).is_some())
        .cloned()
        .collect();
    if numeric_params.is_empty() {
        return None;
    }

    let obj_cols: Vec<Vec<f64>> = (0..meta.objective_names.len())
        .map(|j| objectives.iter().map(|o| o[j]).collect())
        .collect();

    // 各パラメータ×各目的の Spearman 行列と、|ρ| の最大値。
    let mut rows: Vec<(String, Vec<f64>, f64)> = numeric_params
        .iter()
        .map(|name| {
            let x = df.get_numeric_column(name).unwrap();
            let row: Vec<f64> = obj_cols.iter().map(|y| spearman_pairwise(x, y)).collect();
            let max_abs = row
                .iter()
                .filter(|v| v.is_finite())
                .fold(0.0f64, |acc, v| acc.max(v.abs()));
            (name.clone(), row, max_abs)
        })
        .collect();

    // |ρ| 最大値降順（同点は名前昇順）で cap。
    rows.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    rows.truncate(opts.max_heatmap_params.max(1));

    Some(CorrelationSection {
        method: "spearman".to_string(),
        params: rows.iter().map(|(n, _, _)| n.clone()).collect(),
        objectives: meta.objective_names.clone(),
        matrix: rows.into_iter().map(|(_, r, _)| r).collect(),
    })
}

fn build_objective_stats(
    objectives: &[Vec<f64>],
    meta: &StudyMeta,
    directions: &[Direction],
    m: usize,
) -> Vec<ObjectiveStats> {
    (0..m)
        .map(|j| {
            let mut finite: Vec<f64> = objectives
                .iter()
                .map(|o| o[j])
                .filter(|v| v.is_finite())
                .collect();
            let name = meta.objective_names[j].clone();
            let direction = directions[j];
            if finite.is_empty() {
                return ObjectiveStats {
                    name,
                    direction,
                    n: 0,
                    mean: 0.0,
                    std: 0.0,
                    min: 0.0,
                    q1: 0.0,
                    median: 0.0,
                    q3: 0.0,
                    max: 0.0,
                    histogram: None,
                };
            }
            finite.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let cnt = finite.len();
            let mean = finite.iter().sum::<f64>() / cnt as f64;
            let var = finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / cnt as f64;
            let bins = sturges_bins(cnt).clamp(1, MAX_HIST_BINS);
            let histogram =
                compute_histogram(&finite, BinRule::Manual(bins)).map(|h| HistogramData {
                    bin_edges: h.bin_edges,
                    counts: h.counts,
                });
            ObjectiveStats {
                name,
                direction,
                n: cnt,
                mean,
                std: var.sqrt(),
                min: finite[0],
                q1: quantile(&finite, 0.25),
                median: quantile(&finite, 0.5),
                q3: quantile(&finite, 0.75),
                max: finite[cnt - 1],
                histogram,
            }
        })
        .collect()
}

fn sturges_bins(n: usize) -> usize {
    ((n as f64).log2().ceil() as usize).saturating_add(1)
}

// =============================================================================
// Execution
// =============================================================================

fn build_execution(
    ex: &StudyExtras,
    state_counts: &BTreeMap<String, usize>,
    wall_clock: Option<f64>,
) -> ExecutionSection {
    let complete = state_counts.get("COMPLETE").copied().unwrap_or(0);
    let pruned = state_counts.get("PRUNED").copied().unwrap_or(0);
    let fail = state_counts.get("FAIL").copied().unwrap_or(0);
    let finished = complete + pruned + fail;
    let pruned_rate = if finished > 0 {
        pruned as f64 / finished as f64
    } else {
        0.0
    };

    // PRUNED trial の最終中間値 step の中央値。
    let mut prune_steps: Vec<f64> = ex
        .trials
        .iter()
        .filter(|t| t.state == TrialState::Pruned)
        .filter_map(|t| t.intermediate_values.iter().map(|&(s, _)| s).max())
        .map(|s| s as f64)
        .collect();
    let median_prune_step = if prune_steps.is_empty() {
        None
    } else {
        prune_steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Some(quantile(&prune_steps, 0.5))
    };

    // trial 所要時間。
    let durations: Vec<f64> = ex
        .trials
        .iter()
        .filter_map(|t| match (t.datetime_start, t.datetime_complete) {
            (Some(s), Some(c)) if s.is_finite() && c.is_finite() && c >= s => Some(c - s),
            _ => None,
        })
        .collect();
    let (mean_trial_seconds, std_trial_seconds) = if durations.is_empty() {
        (None, None)
    } else {
        let mean = durations.iter().sum::<f64>() / durations.len() as f64;
        let var =
            durations.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / durations.len() as f64;
        (Some(mean), Some(var.sqrt()))
    };

    ExecutionSection {
        state_counts: state_counts.clone(),
        pruned_rate,
        median_prune_step,
        mean_trial_seconds,
        std_trial_seconds,
        total_seconds: wall_clock,
    }
}

// =============================================================================
// Finding facts
// =============================================================================

fn best_single_fact(
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
) -> Option<(f64, u32, f64)> {
    let minimize = is_minimize.first().copied().unwrap_or(true);
    let mut seq: Vec<usize> = (0..objectives.len())
        .filter(|&r| valid_row[r] && !objectives[r].is_empty())
        .collect();
    if seq.is_empty() {
        return None;
    }
    seq.sort_by_key(|&r| trial_numbers[r]);
    let vals: Vec<f64> = seq.iter().map(|&r| objectives[r][0]).collect();
    let ids: Vec<u32> = seq.iter().map(|&r| trial_numbers[r]).collect();
    let history = build_best_trial_history(&ids, &vals, minimize);
    let best_series: Vec<f64> = history.iter().map(|&(_, v)| v).collect();
    let last_idx = last_improve_index(&best_series, minimize);
    let best = best_series[last_idx];
    let found_pct = (last_idx + 1) as f64 / history.len() as f64 * 100.0;
    Some((best, history[last_idx].0, found_pct))
}

fn trade_off_fact(
    objectives: &[Vec<f64>],
    meta: &StudyMeta,
    m: usize,
) -> Option<(String, String, f64)> {
    let mut worst: Option<(String, String, f64)> = None;
    let cols: Vec<Vec<f64>> = (0..m)
        .map(|j| objectives.iter().map(|o| o[j]).collect())
        .collect();
    for a in 0..m {
        for b in (a + 1)..m {
            let rho = spearman_pairwise(&cols[a], &cols[b]);
            if rho.is_finite() && rho < worst.as_ref().map(|w| w.2).unwrap_or(f64::INFINITY) {
                worst = Some((
                    meta.objective_names[a].clone(),
                    meta.objective_names[b].clone(),
                    rho,
                ));
            }
        }
    }
    worst.filter(|w| w.2 < findings::TRADEOFF_RHO_THRESHOLD)
}

#[allow(clippy::too_many_arguments)]
fn feasibility_fact(
    df: &DataFrame,
    objectives: &[Vec<f64>],
    trial_numbers: &[u32],
    valid_row: &[bool],
    is_minimize: &[bool],
    is_multi: bool,
    n: usize,
) -> Option<(f64, usize, usize, Option<u32>)> {
    let feas = df.feasibility();
    let feasible: Vec<usize> = (0..n).filter(|&r| feas.is_feasible(r)).collect();
    let feasible_count = feasible.len();
    let rate = if n > 0 {
        feasible_count as f64 / n as f64
    } else {
        0.0
    };
    // 単目的のみ最良 feasible trial を求める。
    let best_trial = if !is_multi {
        let minimize = is_minimize.first().copied().unwrap_or(true);
        let mut best: Option<(f64, u32)> = None;
        for &r in &feasible {
            if !valid_row[r] {
                continue;
            }
            let v = objectives[r][0];
            let better = match best {
                None => true,
                Some((bv, _)) => {
                    if minimize {
                        v < bv
                    } else {
                        v > bv
                    }
                }
            };
            if better {
                best = Some((v, trial_numbers[r]));
            }
        }
        best.map(|(_, t)| t)
    } else {
        None
    };
    Some((rate, feasible_count, n, best_trial))
}

fn pruning_fact(ex: &StudyExtras) -> (f64, usize, Option<f64>) {
    let mut complete = 0usize;
    let mut pruned = 0usize;
    let mut fail = 0usize;
    for t in &ex.trials {
        match t.state {
            TrialState::Complete => complete += 1,
            TrialState::Pruned => pruned += 1,
            TrialState::Fail => fail += 1,
            _ => {}
        }
    }
    let finished = complete + pruned + fail;
    let rate = if finished > 0 {
        pruned as f64 / finished as f64
    } else {
        0.0
    };
    let mut steps: Vec<f64> = ex
        .trials
        .iter()
        .filter(|t| t.state == TrialState::Pruned)
        .filter_map(|t| t.intermediate_values.iter().map(|&(s, _)| s).max())
        .map(|s| s as f64)
        .collect();
    let median = if steps.is_empty() {
        None
    } else {
        steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Some(quantile(&steps, 0.5))
    };
    (rate, pruned, median)
}

#[cfg(test)]
mod tests;
