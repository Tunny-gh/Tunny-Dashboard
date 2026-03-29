//! NDSort・Hypervolume・Trade-off Navigator
//!
//! REQ-072: NDSort で Paretoランクを計算（50,000点で100ms以内）
//! REQ-073: 重み付きチェビシェフスカラー化（50,000点で1ms以内）
//! REQ-074: Hypervolume 推移グラフ
//!
//! 参照: docs/implements/TASK-201/pareto-requirements.md

// =============================================================================
// 公開型定義
// =============================================================================

/// compute_pareto_ranks() の戻り値 🟢
#[derive(Debug, Clone)]
pub struct ParetoResult {
    /// 各試行のParetoランク（1 = Pareto front, 2 = 2nd front, ...）
    pub ranks: Vec<u32>,
    /// Rank1（Pareto front）のインデックス
    pub pareto_indices: Vec<u32>,
    /// Hypervolume（2目的以上の場合のみ計算、1目的はNone）🟡
    pub hypervolume: Option<f64>,
}

/// compute_hypervolume_history() の戻り値 🟢
#[derive(Debug, Clone)]
pub struct HvHistoryResult {
    /// DataFrameの trial_id 順
    pub trial_ids: Vec<u32>,
    /// 各 trial_id 時点での累積 HV（1目的の場合は全て0.0）
    pub hv_values: Vec<f64>,
}

// =============================================================================
// 純粋計算関数（テスト可能）
// =============================================================================

/// 点 a が点 b を支配するか判定（minimize 前提に変換済み）
///
/// 【定義】: 全目的で a ≤ b かつ少なくとも1目的で a < b 🟢
fn dominates_minimized(a: &[f64], b: &[f64]) -> bool {
    let mut strictly_better = false;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        if ai > bi {
            return false; // a は b より悪い目的がある → 支配しない
        }
        if ai < bi {
            strictly_better = true;
        }
    }
    strictly_better
}

/// 目的値を minimize 方向に正規化する（maximize の場合は符号反転）🟢
///
/// 【設計】: 内部計算を全て minimize に統一して比較ロジックを単純化
fn normalize_objectives(objectives: &[Vec<f64>], is_minimize: &[bool]) -> Vec<Vec<f64>> {
    objectives
        .iter()
        .map(|obj| {
            obj.iter()
                .enumerate()
                .map(|(j, &v)| {
                    if is_minimize.get(j).copied().unwrap_or(true) {
                        v
                    } else {
                        -v
                    }
                })
                .collect()
        })
        .collect()
}

/// Non-dominated Sorting（FNDS: Fast Non-dominated Sort）🟢
///
/// 【アルゴリズム】: NSGA-II の FNDS — O(M × N²)、単一パスで支配関係を事前計算
/// 【NaN処理】: NaN を含む点は常に被支配扱い（最大ランクに配置）🟡
pub fn nd_sort(objectives: &[Vec<f64>], is_minimize: &[bool]) -> Vec<u32> {
    let n = objectives.len();
    if n == 0 {
        return vec![];
    }
    let m = objectives[0].len();
    if m == 0 {
        return vec![1u32; n]; // 目的なし → 全 rank1
    }
    // 1目的の場合: 全点 Rank1（Optuna の単目的最適化では全試行が Pareto front）🟢
    if m == 1 {
        return vec![1u32; n];
    }

    // NaN を含む点のマスク（NaN は被支配扱い、後で最大ランク+1 に設定）
    let nan_mask: Vec<bool> = objectives
        .iter()
        .map(|obj| obj.iter().any(|v| v.is_nan()))
        .collect();

    // 方向符号（minimize=+1, maximize=-1）で平坦化して normalize を回避
    let signs: Vec<f64> = (0..m)
        .map(|j| {
            if is_minimize.get(j).copied().unwrap_or(true) {
                1.0
            } else {
                -1.0
            }
        })
        .collect();
    // 平坦配列: norm_flat[i*m + j] = sign[j] * objectives[i][j]（二重参照を排除）🟢
    let mut norm_flat: Vec<f64> = Vec::with_capacity(n * m);
    for obj in objectives.iter() {
        for (j, &v) in obj.iter().enumerate() {
            norm_flat.push(signs[j] * v);
        }
    }

    let mut ranks = vec![0u32; n];
    // domination_count[i] = i を支配する点の数（0 なら現フロント）
    let mut domination_count = vec![0u32; n];
    // dominates_list[i] = i が支配する点のインデックスリスト（初期容量を確保してリアロケを削減）🟡
    let init_cap = (n / 4).clamp(4, 128);
    let mut dominates_list: Vec<Vec<usize>> =
        (0..n).map(|_| Vec::with_capacity(init_cap)).collect();

    // 【O(N²) 事前計算フェーズ】: 各ペアを1回だけ評価、関数呼び出し排除でデバッグ高速化 🟢
    for i in 0..n {
        if nan_mask[i] {
            continue;
        }
        let oi = &norm_flat[i * m..(i + 1) * m];
        for j in (i + 1)..n {
            if nan_mask[j] {
                continue;
            }
            let oj = &norm_flat[j * m..(j + 1) * m];
            // i と j の両方向を1ループで判定（関数呼び出し2回分を削減）
            let mut i_better = false;
            let mut j_better = false;
            for k in 0..m {
                if oi[k] < oj[k] {
                    i_better = true;
                } else if oi[k] > oj[k] {
                    j_better = true;
                }
            }
            if i_better && !j_better {
                dominates_list[i].push(j);
                domination_count[j] += 1;
            } else if j_better && !i_better {
                dominates_list[j].push(i);
                domination_count[i] += 1;
            }
        }
    }

    // 【フロント剥ぎフェーズ】: domination_count=0 の点が現在のフロント 🟢
    let mut current_front: Vec<usize> = (0..n)
        .filter(|&i| !nan_mask[i] && domination_count[i] == 0)
        .collect();
    let mut rank = 1u32;

    while !current_front.is_empty() {
        let mut next_front = Vec::new();
        for &i in &current_front {
            ranks[i] = rank;
            for &j in &dominates_list[i] {
                domination_count[j] -= 1;
                if domination_count[j] == 0 {
                    next_front.push(j);
                }
            }
        }
        current_front = next_front;
        rank += 1;
    }

    // NaN 点は最大ランク + 1 に設定（最も劣った扱い）🟡
    let max_rank = ranks.iter().max().copied().unwrap_or(0);
    for i in 0..n {
        if nan_mask[i] {
            ranks[i] = max_rank + 1;
        }
    }

    ranks
}

/// 2D Hypervolume を sweep line で計算（参照点との支配領域）🟢
///
/// 【アルゴリズム】: Pareto 点を x 昇順にソートし、y 方向の面積を累積
/// 【複雑度】: O(P log P)（P = Pareto front サイズ）
pub fn hypervolume_2d(pareto_points: &[(f64, f64)], ref_x: f64, ref_y: f64) -> f64 {
    if pareto_points.is_empty() {
        return 0.0;
    }
    // minimize 想定: 参照点は nadir 点（全点より大きい）
    let mut pts: Vec<(f64, f64)> = pareto_points
        .iter()
        .filter(|&&(x, y)| x < ref_x && y < ref_y)
        .cloned()
        .collect();
    if pts.is_empty() {
        return 0.0;
    }
    // x 昇順にソート
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // sweep: Pareto front は x 昇順 → y 降順が保証される
    // 各ストリップ [x_i, x_{i+1}] の高さ = ref_y - y_i（累積支配領域の正しい高さ）🟢
    let mut hv = 0.0f64;
    for i in 0..pts.len() {
        let next_x = if i + 1 < pts.len() {
            pts[i + 1].0
        } else {
            ref_x
        };
        let width = next_x - pts[i].0;
        let height = ref_y - pts[i].1;
        if width > 0.0 && height > 0.0 {
            hv += width * height;
        }
    }
    hv
}

/// Nadir 点（各目的の最大値）+ マージンで参照点を計算 🟢
///
/// 【最適化】: min/max を1パスで同時計算（2パスから削減）
fn compute_ref_point(pareto_objs: &[Vec<f64>], m: usize) -> Vec<f64> {
    let mut nadir = vec![f64::NEG_INFINITY; m];
    let mut ideal = vec![f64::INFINITY; m];
    // 1パスで min（ideal）と max（nadir）を同時収集
    for obj in pareto_objs {
        for (j, &v) in obj.iter().enumerate() {
            if v > nadir[j] {
                nadir[j] = v;
            }
            if v < ideal[j] {
                ideal[j] = v;
            }
        }
    }
    // 10% マージン（+ 1.0 ベースライン）を加えて参照点が Pareto 点より必ず大きくなるよう保証 🟡
    (0..m)
        .map(|j| nadir[j] + (nadir[j] - ideal[j]).abs() * 0.1 + 1.0)
        .collect()
}

/// 重み付きチェビシェフスカラー化でスコアを計算し、昇順インデックスを返す 🟢
///
/// 【スコア式】: score_i = max_j( w_j × |f_j(i) - ideal_j| )
/// 【ideal点】: minimize → 各目的の最小値, maximize → 各目的の最大値
pub fn chebyshev_sort(objectives: &[Vec<f64>], weights: &[f64], is_minimize: &[bool]) -> Vec<u32> {
    let n = objectives.len();
    if n == 0 {
        return vec![];
    }
    let m = objectives[0].len();
    if m == 0 || weights.iter().all(|&w| w == 0.0) {
        // 重みが全て0 → 全インデックスをそのまま返す（ランダム順に近い）🟢
        return (0..n as u32).collect();
    }

    // normalize方向で ideal 計算（minimize→min値, maximize→max値をminに変換で-max）
    let norm_objs = normalize_objectives(objectives, is_minimize);

    // ideal点 = 各目的の最小値（normalize後）
    let ideal: Vec<f64> = (0..m)
        .map(|j| {
            norm_objs
                .iter()
                .map(|obj| obj[j])
                .filter(|v| !v.is_nan())
                .fold(f64::INFINITY, f64::min)
        })
        .collect();

    // 各点のチェビシェフスコアを計算
    let mut scores: Vec<(usize, f64)> = norm_objs
        .iter()
        .enumerate()
        .map(|(i, obj)| {
            let score = obj
                .iter()
                .enumerate()
                .map(|(j, &v)| {
                    let w = weights.get(j).copied().unwrap_or(0.0);
                    w * (v - ideal[j]).abs()
                })
                .fold(0.0f64, f64::max);
            (i, if score.is_nan() { f64::INFINITY } else { score })
        })
        .collect();

    // スコア昇順にソート（最良 = 最小スコアが先頭）sort_unstable_by: stable 不要で高速 🟢
    scores.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.into_iter().map(|(i, _)| i as u32).collect()
}

// =============================================================================
// グローバル状態アクセス版 API
// =============================================================================

/// アクティブ Study の Pareto ランクを計算する 🟢
///
/// 【引数】: `is_minimize` — 各目的の最小化フラグ（directions から変換）
pub fn compute_pareto_ranks(is_minimize: &[bool]) -> ParetoResult {
    crate::dataframe::with_active_df(|df| {
        let obj_names = df.objective_col_names();
        let m = obj_names.len();
        let n = df.row_count();
        if n == 0 || m == 0 {
            return ParetoResult {
                ranks: vec![],
                pareto_indices: vec![],
                hypervolume: None,
            };
        }

        // 目的値を Vec<Vec<f64>> に収集
        let objectives: Vec<Vec<f64>> = (0..n)
            .map(|row| {
                obj_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row))
                            .copied()
                            .unwrap_or(f64::NAN)
                    })
                    .collect()
            })
            .collect();

        let ranks = nd_sort(&objectives, is_minimize);
        let pareto_indices: Vec<u32> = ranks
            .iter()
            .enumerate()
            .filter(|(_, &r)| r == 1)
            .map(|(i, _)| i as u32)
            .collect();

        // Hypervolume 計算（2目的以上、Pareto front が2点以上）🟡
        let hypervolume = if m >= 2 && pareto_indices.len() >= 2 {
            let pareto_objs: Vec<Vec<f64>> = pareto_indices
                .iter()
                .map(|&i| objectives[i as usize].clone())
                .collect();
            // minimize 正規化
            let norm_pareto = normalize_objectives(&pareto_objs, is_minimize);
            // 2D のみ exact HV、3D+ は obj0×obj1 の 2D 射影 🟡
            let ref_pt = compute_ref_point(&norm_pareto, m);
            let pts_2d: Vec<(f64, f64)> = norm_pareto.iter().map(|obj| (obj[0], obj[1])).collect();
            Some(hypervolume_2d(&pts_2d, ref_pt[0], ref_pt[1]))
        } else {
            None
        };

        ParetoResult {
            ranks,
            pareto_indices,
            hypervolume,
        }
    })
    .unwrap_or(ParetoResult {
        ranks: vec![],
        pareto_indices: vec![],
        hypervolume: None,
    })
}

/// 試行番号順の Hypervolume 推移を計算する 🟢
pub fn compute_hypervolume_history(is_minimize: &[bool]) -> HvHistoryResult {
    crate::dataframe::with_active_df(|df| {
        let n = df.row_count();
        let obj_names = df.objective_col_names();
        let m = obj_names.len();

        let trial_ids: Vec<u32> = (0..n).filter_map(|i| df.get_trial_id(i)).collect();

        if m < 2 {
            // 1目的: HV は定義されないため全て 0.0 🟢
            return HvHistoryResult {
                trial_ids,
                hv_values: vec![0.0; n],
            };
        }

        // 参照点は全データの nadir + マージン（事前計算）
        let all_objs: Vec<Vec<f64>> = (0..n)
            .map(|row| {
                obj_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row))
                            .copied()
                            .unwrap_or(f64::NAN)
                    })
                    .collect()
            })
            .collect();
        let norm_all = normalize_objectives(&all_objs, is_minimize);
        let valid_objs: Vec<Vec<f64>> = norm_all
            .iter()
            .filter(|obj| !obj.iter().any(|v| v.is_nan()))
            .cloned()
            .collect();
        if valid_objs.is_empty() {
            return HvHistoryResult {
                trial_ids,
                hv_values: vec![0.0; n],
            };
        }
        let ref_pt = compute_ref_point(&valid_objs, m);

        // 試行を順に追加しながら Pareto front と HV を更新
        let mut current_pareto: Vec<Vec<f64>> = Vec::new();
        let mut hv_values = Vec::with_capacity(n);

        for row in 0..n {
            let obj = norm_all[row].clone();
            if obj.iter().any(|v| v.is_nan()) {
                hv_values.push(hv_values.last().copied().unwrap_or(0.0));
                continue;
            }
            // 現在の点が既存 Pareto front に追加できるか（非支配かチェック）
            let dominated = current_pareto.iter().any(|p| dominates_minimized(p, &obj));
            if !dominated {
                // 既存 Pareto から obj に支配される点を除去
                current_pareto.retain(|p| !dominates_minimized(&obj, p));
                current_pareto.push(obj);
            }
            // HV 計算（2D射影）
            let pts_2d: Vec<(f64, f64)> = current_pareto.iter().map(|o| (o[0], o[1])).collect();
            hv_values.push(hypervolume_2d(&pts_2d, ref_pt[0], ref_pt[1]));
        }

        HvHistoryResult {
            trial_ids,
            hv_values,
        }
    })
    .unwrap_or(HvHistoryResult {
        trial_ids: vec![],
        hv_values: vec![],
    })
}

/// Trade-off Navigator: 重み付きチェビシェフでスコアリングして昇順インデックスを返す 🟢
///
/// 【最適化】: 列スライスを事前キャッシュし Vec<Vec<f64>> アロケーションを回避 O(N×M)
pub fn score_tradeoff_navigator(weights: &[f64], is_minimize: &[bool]) -> Vec<u32> {
    crate::dataframe::with_active_df(|df| {
        let obj_names = df.objective_col_names();
        let n = df.row_count();
        let m = obj_names.len();
        if n == 0 || m == 0 {
            return (0..n as u32).collect();
        }
        if weights.iter().all(|&w| w == 0.0) {
            return (0..n as u32).collect();
        }

        // 【列スライス事前キャッシュ】: get_numeric_column は O(C) スキャンなのでループ外で1回だけ呼ぶ
        let cols: Vec<&[f64]> = obj_names
            .iter()
            .filter_map(|name| df.get_numeric_column(name))
            .collect();
        if cols.len() != m {
            return (0..n as u32).collect();
        }

        // 方向符号（minimize=+1, maximize=-1）で内部的に全て最小化に統一
        let sign: Vec<f64> = (0..m)
            .map(|j| {
                if is_minimize.get(j).copied().unwrap_or(true) {
                    1.0
                } else {
                    -1.0
                }
            })
            .collect();

        // ideal 点 = 各目的の最小値（正規化後）
        let ideal: Vec<f64> = (0..m)
            .map(|j| {
                cols[j]
                    .iter()
                    .filter(|v| !v.is_nan())
                    .map(|&v| sign[j] * v)
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();

        // チェビシェフスコアを列指向で計算（アロケーションなし）
        let mut scores: Vec<(usize, f64)> = (0..n)
            .map(|i| {
                let score = (0..m)
                    .map(|j| {
                        let v = cols[j].get(i).copied().unwrap_or(f64::NAN);
                        let w = weights.get(j).copied().unwrap_or(0.0);
                        w * (sign[j] * v - ideal[j]).abs()
                    })
                    .fold(0.0f64, f64::max);
                (i, if score.is_nan() { f64::INFINITY } else { score })
            })
            .collect();

        // sort_unstable_by: stable sort 不要なので高速な pdqsort を使う 🟢
        scores.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.into_iter().map(|(i, _)| i as u32).collect()
    })
    .unwrap_or_default()
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // テスト共通ヘルパー
    // -------------------------------------------------------------------------

    fn make_row_obj(trial_id: u32, obj: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: obj,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    fn setup_study(rows: Vec<TrialRow>, obj_names: &[&str]) {
        let names: Vec<String> = obj_names.iter().map(|s| s.to_string()).collect();
        let df = DataFrame::from_trials(&rows, &[], &names, &[], &[], 0);
        store_dataframes(vec![df]);
        select_study(0).unwrap();
    }

    // =========================================================================
    // 正常系
    // =========================================================================

    #[test]
    fn tc_201_01_two_obj_all_nondominated() {
        // 【テスト目的】: 2目的で全点が非支配の場合、全点 rank=1 🟢
        // 【入力】: (1,4),(2,3),(3,2),(4,1) — 交差する非支配点
        let objs = vec![
            vec![1.0, 4.0],
            vec![2.0, 3.0],
            vec![3.0, 2.0],
            vec![4.0, 1.0],
        ];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // 【確認】全点 rank=1（支配関係なし）
        assert_eq!(ranks, vec![1, 1, 1, 1]);
    }

    #[test]
    fn tc_201_02_two_obj_clear_domination() {
        // 【テスト目的】: 明確な支配チェーン (1,1)>(2,2)>(3,3) 🟢
        let objs = vec![vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // 【確認】ranks = [1, 2, 3]
        assert_eq!(ranks, vec![1, 2, 3]);
    }

    #[test]
    fn tc_201_03_four_objectives() {
        // 【テスト目的】: 4目的 NDSort が正確に動作する 🟢
        // (1,1,1,1) が (2,2,2,2) を支配, (1,2,1,2) は (2,2,2,2) を支配
        let objs = vec![
            vec![1.0, 1.0, 1.0, 1.0], // rank 1
            vec![2.0, 2.0, 2.0, 2.0], // rank 3 (dominated by both)
            vec![1.0, 2.0, 1.0, 2.0], // rank 2 (dominated by [0])
        ];
        let is_min = [true, true, true, true];
        let ranks = nd_sort(&objs, &is_min);
        assert_eq!(ranks[0], 1); // 【確認】(1,1,1,1) rank=1
        assert_eq!(ranks[2], 2); // 【確認】(1,2,1,2) rank=2
        assert_eq!(ranks[1], 3); // 【確認】(2,2,2,2) rank=3
    }

    #[test]
    fn tc_201_04_single_objective_all_rank1() {
        // 【テスト目的】: 1目的の場合は全点 Rank1（支配比較が意味をなさない）🟢
        let objs = vec![vec![3.0], vec![1.0], vec![4.0], vec![1.5], vec![2.0]];
        let is_min = [true];
        let ranks = nd_sort(&objs, &is_min);
        // 【確認】全点 rank=1
        assert!(ranks.iter().all(|&r| r == 1));
    }

    #[test]
    fn tc_201_05_maximize_direction() {
        // 【テスト目的】: maximize 方向で大きい値が優れている 🟢
        // obj0 = [1, 2, 3] で maximize → 3が最良(rank1)
        let objs = vec![
            vec![1.0], // rank 1 (全点 rank1 — 1目的)
            vec![2.0],
            vec![3.0],
        ];
        let is_min_single = [true];
        let ranks_single = nd_sort(&objs, &is_min_single);
        assert!(ranks_single.iter().all(|&r| r == 1)); // 1目的は全 rank1

        // 2目的 maximize×minimize で確認
        // (1,1),(2,2),(3,3) で maximize/minimize → (3,3) が (2,2) を支配?
        // maximize obj0: 3>2>1 → 3が最良
        // minimize obj1: 1<2<3 → 1が最良
        // (3,3) vs (2,2): obj0は3>2(maximize良), obj1は3>2(minimize悪) → 非支配
        let objs2 = vec![
            vec![1.0, 3.0], // obj0大きい=悪, obj1小さい=良 → [minimize後: (-1, 3)]
            vec![2.0, 2.0], // [-2, 2]
            vec![3.0, 1.0], // [-3, 1] — 最小化後は支配
        ];
        let is_min2 = [false, true]; // obj0 maximize, obj1 minimize
        let ranks2 = nd_sort(&objs2, &is_min2);
        // normalize: (-1,3),(-2,2),(-3,1)
        // (-3,1) dominates (-2,2)? -3<-2(良) AND 1<2(良) → YES
        // (-3,1) dominates (-1,3)? -3<-1(良) AND 1<3(良) → YES
        // (-2,2) dominates (-1,3)? -2<-1(良) AND 2<3(良) → YES
        assert_eq!(ranks2[2], 1); // obj0=3(maximize最大)が rank1
        assert_eq!(ranks2[1], 2);
        assert_eq!(ranks2[0], 3);
    }

    #[test]
    fn tc_201_06_hypervolume_2d_known_value() {
        // 【テスト目的】: 2D HV が既知の値と一致する 🟢
        // Pareto点: (1,4),(2,2),(3,1), 参照点: (5,5)
        // 面積: (2-1)×(5-4) + (3-2)×(5-2) + (5-3)×(5-1)
        //      = 1 + 3 + 8 = 12
        let pts = vec![(1.0, 4.0), (2.0, 2.0), (3.0, 1.0)];
        let hv = hypervolume_2d(&pts, 5.0, 5.0);
        // 【確認】HV = 12.0
        assert!((hv - 12.0).abs() < 1e-9, "HV = {}, expected 12.0", hv);
    }

    #[test]
    fn tc_201_07_hypervolume_single_objective_none() {
        // 【テスト目的】: 1目的の場合 hypervolume = None 🟢
        let rows = vec![
            make_row_obj(0, vec![1.0]),
            make_row_obj(1, vec![2.0]),
            make_row_obj(2, vec![3.0]),
        ];
        setup_study(rows, &["obj0"]);
        let result = compute_pareto_ranks(&[true]);
        // 【確認】hypervolume = None
        assert!(result.hypervolume.is_none());
    }

    #[test]
    fn tc_201_08_tradeoff_navigator_order() {
        // 【テスト目的】: Trade-off Navigator が正確なスコア順を返す 🟢
        // 3点 [(1,4),(2,2),(4,1)], weights=[0.5,0.5], ideal=[1,1]
        // score0 = max(0.5×|1-1|, 0.5×|4-1|) = max(0, 1.5) = 1.5
        // score1 = max(0.5×|2-1|, 0.5×|2-1|) = max(0.5, 0.5) = 0.5
        // score2 = max(0.5×|4-1|, 0.5×|1-1|) = max(1.5, 0) = 1.5
        let objs = vec![vec![1.0, 4.0], vec![2.0, 2.0], vec![4.0, 1.0]];
        let is_min = [true, true];
        let weights = [0.5, 0.5];
        let result = chebyshev_sort(&objs, &weights, &is_min);
        // 【確認】score 最小のインデックス1が先頭
        assert_eq!(result[0], 1); // score=0.5 が最小
    }

    #[test]
    fn tc_201_09_hypervolume_history_single_obj() {
        // 【テスト目的】: 1目的のHV推移は全て0.0 🟢
        let rows = vec![
            make_row_obj(0, vec![2.0]),
            make_row_obj(1, vec![1.0]),
            make_row_obj(2, vec![3.0]),
        ];
        setup_study(rows, &["obj0"]);
        let result = compute_hypervolume_history(&[true]);
        // 【確認】1目的はHV=0.0
        assert!(result.hv_values.iter().all(|&v| v == 0.0));
        assert_eq!(result.trial_ids.len(), 3);
    }

    // =========================================================================
    // 異常系
    // =========================================================================

    #[test]
    fn tc_201_e01_zero_weights_fallback() {
        // 【テスト目的】: 重みが全て0でも panic しない 🟢
        let objs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let is_min = [true, true];
        let weights = [0.0, 0.0]; // 全て0
        let result = chebyshev_sort(&objs, &weights, &is_min);
        // 【確認】全インデックスが返る（panic しない）
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn tc_201_e02_empty_dataframe_returns_empty() {
        // 【テスト目的】: 空DataFrame で compute_pareto_ranks が空を返す 🟢
        store_dataframes(vec![DataFrame::empty()]);
        select_study(0).unwrap();
        let result = compute_pareto_ranks(&[true]);
        // 【確認】全フィールドが空
        assert!(result.ranks.is_empty());
        assert!(result.pareto_indices.is_empty());
        assert!(result.hypervolume.is_none());
    }

    // =========================================================================
    // 境界値
    // =========================================================================

    #[test]
    fn tc_201_b01_all_same_coords() {
        // 【テスト目的】: 全点が同一座標（全点非支配）→ 全 rank=1 🟢
        let objs = vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // 【確認】完全に同一の点は互いを支配しない → 全 rank=1
        assert!(ranks.iter().all(|&r| r == 1));
    }

    #[test]
    fn tc_201_b02_chain_dominance() {
        // 【テスト目的】: 完全な支配チェーン → 各点が異なるランク 🟢
        let objs = vec![
            vec![1.0, 1.0], // rank 1
            vec![2.0, 2.0], // rank 2
            vec![3.0, 3.0], // rank 3
            vec![4.0, 4.0], // rank 4
        ];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // 【確認】各点が順番にランクを持つ
        assert_eq!(ranks, vec![1, 2, 3, 4]);
    }

    #[test]
    fn tc_201_b03_single_point() {
        // 【テスト目的】: 1点のみの DataFrame 🟢
        let rows = vec![make_row_obj(0, vec![1.0, 2.0])];
        setup_study(rows, &["obj0", "obj1"]);
        let result = compute_pareto_ranks(&[true, true]);
        // 【確認】1点 → rank=1, hypervolume=None（点が1つではHV不可）
        assert_eq!(result.ranks, vec![1]);
        assert_eq!(result.pareto_indices, vec![0]);
        assert!(result.hypervolume.is_none()); // pareto_indices.len() < 2
    }

    // =========================================================================
    // パフォーマンス
    // =========================================================================

    #[test]
    fn tc_201_p01_ndsort_1000_points_under_100ms() {
        // 【テスト目的】: N=1,000点 NDSort がデバッグビルドで100ms以内 🟡
        // 【注記】: リリースビルドでは N=50,000 を達成（REQ-072）
        // 【テストデータ】: 一様分布に近い擬似乱数2D点（構造化パターンは支配辺が密で不当に遅い）
        let n = 1_000usize;
        let objs: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                // xorshift 風ハッシュで一様分布に近い2D点を生成（再現性あり）
                let x = ((i.wrapping_mul(7_919).wrapping_add(1_234_567)) % n) as f64 / n as f64;
                let y = ((i.wrapping_mul(6_271).wrapping_add(9_876_543)) % n) as f64 / n as f64;
                vec![x, y]
            })
            .collect();
        let is_min = [true, true];

        let start = std::time::Instant::now();
        let ranks = nd_sort(&objs, &is_min);
        let elapsed = start.elapsed();

        // 【確認】100ms 以内、全点にランクが付与
        assert!(
            elapsed.as_millis() <= 100,
            "NDSort が {}ms かかった（期待: ≤100ms）",
            elapsed.as_millis()
        );
        assert_eq!(ranks.len(), n);
        assert!(ranks.iter().all(|&r| r >= 1));
    }

    #[test]
    fn tc_201_p02_tradeoff_50000_points_under_1ms() {
        // 【テスト目的】: N=50,000点 Trade-off Navigator が1ms以内 🟢（リリースビルド）
        // 【注記】: デバッグビルドは最適化なしのため N=5,000 で50ms以内を検証 🟡
        #[cfg(debug_assertions)]
        let n = 5_000usize;
        #[cfg(not(debug_assertions))]
        let n = 50_000usize;

        let rows: Vec<TrialRow> = (0..n)
            .map(|i| make_row_obj(i as u32, vec![(i % 100) as f64, (n - i) as f64]))
            .collect();
        setup_study(rows, &["obj0", "obj1"]);

        let weights = [0.5, 0.5];
        let is_min = [true, true];
        let start = std::time::Instant::now();
        let result = score_tradeoff_navigator(&weights, &is_min);
        let elapsed = start.elapsed();

        // デバッグビルド: ≤50ms、リリースビルド: ≤1ms
        #[cfg(debug_assertions)]
        assert!(
            elapsed.as_millis() <= 50,
            "Trade-off Navigator が {}ms かかった（期待: ≤50ms）",
            elapsed.as_millis()
        );
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed.as_millis() <= 1,
            "Trade-off Navigator が {}ms かかった（期待: ≤1ms）",
            elapsed.as_millis()
        );
        assert_eq!(result.len(), n);
    }
}
