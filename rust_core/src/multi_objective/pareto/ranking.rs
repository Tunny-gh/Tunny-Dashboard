use super::helpers::{compute_ref_point, normalize_objectives};
use super::hypervolume::hypervolume_2d;
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
        return vec![1u32; n];
    }
    if m == 1 {
        return vec![1u32; n];
    }

    let nan_mask: Vec<bool> = objectives
        .iter()
        .map(|obj| obj.iter().any(|v| v.is_nan()))
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
    let mut norm_flat: Vec<f64> = Vec::with_capacity(n * m);
    for obj in objectives {
        for (j, &v) in obj.iter().enumerate() {
            norm_flat.push(signs[j] * v);
        }
    }

    let mut ranks = vec![0u32; n];
    let mut domination_count = vec![0u32; n];
    let init_cap = (n / 4).clamp(4, 128);
    let mut dominates_list: Vec<Vec<usize>> =
        (0..n).map(|_| Vec::with_capacity(init_cap)).collect();

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

    let max_rank = ranks.iter().max().copied().unwrap_or(0);
    for i in 0..n {
        if nan_mask[i] {
            ranks[i] = max_rank + 1;
        }
    }

    ranks
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

        let hypervolume = if m >= 2 && pareto_indices.len() >= 2 {
            let pareto_objs: Vec<Vec<f64>> = pareto_indices
                .iter()
                .map(|&i| objectives[i as usize].clone())
                .collect();
            let norm_pareto = normalize_objectives(&pareto_objs, is_minimize);
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
