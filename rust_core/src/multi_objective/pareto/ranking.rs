use rayon::prelude::*;

use super::helpers::{compute_ref_point, normalize_objectives};
use super::hypervolume::hypervolume_nd;
use super::types::ParetoResult;

/// Non-dominated Sorting（FNDS: Fast Non-dominated Sort）🟢
///
/// Documentation.
/// Documentation.
pub fn nd_sort(objectives: &[Vec<f64>], is_minimize: &[bool]) -> Vec<u32> {
    let n = objectives.len();
    if n == 0 {
        return vec![];
    }
    let m = objectives[0].len();
    if m == 0 {
        return vec![0u32; n];
    }
    if m == 1 {
        return vec![0u32; n];
    }

    // 目的本数が m に満たない行（不揃い入力）は NaN 行として扱い、支配判定から除外する。
    // これがないと下流で norm_flat[i*m..(i+1)*m] がスライス範囲外になり panic する。
    let nan_mask: Vec<bool> = objectives
        .iter()
        .map(|obj| obj.len() < m || obj.iter().any(|v| v.is_nan()))
        .collect();

    let signs: Vec<f64> = (0..m)
        .map(|j| {
            if is_minimize.get(j).copied().unwrap_or(true) {
                1.0
            } else {
                -1.0
            }
        })
        .collect();
    // 各行を必ず m 要素にそろえてフラット化する（不足分は NaN で埋める）。
    let mut norm_flat: Vec<f64> = Vec::with_capacity(n * m);
    for obj in objectives {
        for (j, &sign) in signs.iter().enumerate() {
            let v = obj.get(j).copied().unwrap_or(f64::NAN);
            norm_flat.push(sign * v);
        }
    }

    let mut ranks = vec![0u32; n];
    let init_cap = (n / 4).clamp(4, 128);

    // 並列フェーズ: 各 i について j > i との支配関係を並列計算
    // pair_results[i] = (i が支配する j のリスト, i を支配する j のリスト)
    let pair_results: Vec<(Vec<usize>, Vec<usize>)> = (0..n)
        .into_par_iter()
        .map(|i| {
            if nan_mask[i] {
                return (vec![], vec![]);
            }
            let oi = &norm_flat[i * m..(i + 1) * m];
            let mut i_dom_j = Vec::new();
            let mut j_dom_i = Vec::new();
            for j in (i + 1)..n {
                if nan_mask[j] {
                    continue;
                }
                let oj = &norm_flat[j * m..(j + 1) * m];
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
                    i_dom_j.push(j);
                } else if j_better && !i_better {
                    j_dom_i.push(j);
                }
            }
            (i_dom_j, j_dom_i)
        })
        .collect();

    // 集約フェーズ: O(n + edges) で dominates_list と domination_count を構築
    let mut domination_count = vec![0u32; n];
    let mut dominates_list: Vec<Vec<usize>> =
        (0..n).map(|_| Vec::with_capacity(init_cap)).collect();

    for (i, (i_dom_j, j_dom_i)) in pair_results.into_iter().enumerate() {
        for j in i_dom_j {
            dominates_list[i].push(j);
            domination_count[j] += 1;
        }
        // j_dom_i: j > i で j が i を支配 → dominates_list[j] に i を追加
        for dom in j_dom_i {
            dominates_list[dom].push(i);
            domination_count[i] += 1;
        }
    }

    let mut current_front: Vec<usize> = (0..n)
        .filter(|&i| !nan_mask[i] && domination_count[i] == 0)
        .collect();
    let mut rank = 0u32;

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

    let max_rank = ranks.iter().max().copied().unwrap_or(0);
    for i in 0..n {
        if nan_mask[i] {
            ranks[i] = max_rank + 1;
        }
    }

    ranks
}

fn compute_hypervolume(
    pareto_indices: &[u32],
    objectives: &[Vec<f64>],
    is_minimize: &[bool],
    m: usize,
) -> Option<f64> {
    if m >= 2 && pareto_indices.len() >= 2 {
        let pareto_objs: Vec<Vec<f64>> = pareto_indices
            .iter()
            .map(|&i| objectives[i as usize].clone())
            .collect();
        let norm_pareto = normalize_objectives(&pareto_objs, is_minimize);
        let ref_pt = compute_ref_point(&norm_pareto, m);
        Some(hypervolume_nd(&norm_pareto, &ref_pt))
    } else {
        None
    }
}

/// Documentation.
///
/// Documentation.
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

        let obj_cols: Vec<Option<&[f64]>> = obj_names
            .iter()
            .map(|name| df.get_numeric_column(name))
            .collect();

        let objectives: Vec<Vec<f64>> = (0..n)
            .map(|row| {
                obj_cols
                    .iter()
                    .map(|col| col.and_then(|c| c.get(row)).copied().unwrap_or(f64::NAN))
                    .collect()
            })
            .collect();

        let feas = df.feasibility();
        let constraint_sum_col = df.get_numeric_column("constraint_sum");

        if !feas.has_constraints() {
            // 制約なし: 従来フロー
            let ranks = nd_sort(&objectives, is_minimize);
            let pareto_indices: Vec<u32> = ranks
                .iter()
                .enumerate()
                .filter(|(_, &r)| r == 0)
                .map(|(i, _)| i as u32)
                .collect();
            let hypervolume = compute_hypervolume(&pareto_indices, &objectives, is_minimize, m);
            return ParetoResult {
                ranks,
                pareto_indices,
                hypervolume,
            };
        }

        // 制約あり: feasible/infeasible 分離フロー
        let (feasible_indices, infeasible_indices) = feas.partition_indices(n);
        let feasible_objectives: Vec<Vec<f64>> = feasible_indices
            .iter()
            .map(|&i| objectives[i].clone())
            .collect();

        let mut ranks = vec![0u32; n];

        let max_feasible_rank = if feasible_objectives.is_empty() {
            0u32
        } else {
            let feasible_ranks = nd_sort(&feasible_objectives, is_minimize);
            let max_r = feasible_ranks.iter().max().copied().unwrap_or(0);
            for (k, &orig_idx) in feasible_indices.iter().enumerate() {
                ranks[orig_idx] = feasible_ranks[k];
            }
            max_r
        };

        let mut infeasible_with_sum: Vec<(usize, f64)> = infeasible_indices
            .into_iter()
            .map(|i| {
                let sum = constraint_sum_col
                    .and_then(|col| col.get(i))
                    .copied()
                    .unwrap_or(0.0);
                (i, sum)
            })
            .collect();
        infeasible_with_sum
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (violation_rank, (orig_idx, _)) in infeasible_with_sum.iter().enumerate() {
            ranks[*orig_idx] = max_feasible_rank + 1 + violation_rank as u32;
        }

        let pareto_indices: Vec<u32> = feasible_indices
            .iter()
            .filter(|&&i| ranks[i] == 0)
            .map(|&i| i as u32)
            .collect();

        let hypervolume = compute_hypervolume(&pareto_indices, &objectives, is_minimize, m);

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
