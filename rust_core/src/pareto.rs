//! NDSort・Hypervolume・Trade-off Navigator
//!
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-201/pareto-requirements.md

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
#[derive(Debug, Clone)]
pub struct ParetoResult {
    /// Documentation.
    pub ranks: Vec<u32>,
    /// Documentation.
    pub pareto_indices: Vec<u32>,
    /// Documentation.
    pub hypervolume: Option<f64>,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct HvHistoryResult {
    /// Documentation.
    pub trial_ids: Vec<u32>,
    /// Documentation.
    pub hv_values: Vec<f64>,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
fn dominates_minimized(a: &[f64], b: &[f64]) -> bool {
    let mut strictly_better = false;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        if ai > bi {
            return false; // Documentation.
        }
        if ai < bi {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Documentation.
///
/// Documentation.
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
/// Documentation.
/// Documentation.
pub fn nd_sort(objectives: &[Vec<f64>], is_minimize: &[bool]) -> Vec<u32> {
    let n = objectives.len();
    if n == 0 {
        return vec![];
    }
    let m = objectives[0].len();
    if m == 0 {
        return vec![1u32; n]; // Documentation.
    }
    // Documentation.
    if m == 1 {
        return vec![1u32; n];
    }

    // Documentation.
    let nan_mask: Vec<bool> = objectives
        .iter()
        .map(|obj| obj.iter().any(|v| v.is_nan()))
        .collect();

    // Documentation.
    let signs: Vec<f64> = (0..m)
        .map(|j| {
            if is_minimize.get(j).copied().unwrap_or(true) {
                1.0
            } else {
                -1.0
            }
        })
        .collect();
    // Documentation.
    let mut norm_flat: Vec<f64> = Vec::with_capacity(n * m);
    for obj in objectives.iter() {
        for (j, &v) in obj.iter().enumerate() {
            norm_flat.push(signs[j] * v);
        }
    }

    let mut ranks = vec![0u32; n];
    // Documentation.
    let mut domination_count = vec![0u32; n];
    // Documentation.
    let init_cap = (n / 4).clamp(4, 128);
    let mut dominates_list: Vec<Vec<usize>> =
        (0..n).map(|_| Vec::with_capacity(init_cap)).collect();

    // Documentation.
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
            // Documentation.
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

    // Documentation.
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

    // Documentation.
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
/// Documentation.
pub fn hypervolume_2d(pareto_points: &[(f64, f64)], ref_x: f64, ref_y: f64) -> f64 {
    if pareto_points.is_empty() {
        return 0.0;
    }
    // Documentation.
    let mut pts: Vec<(f64, f64)> = pareto_points
        .iter()
        .filter(|&&(x, y)| x < ref_x && y < ref_y)
        .cloned()
        .collect();
    if pts.is_empty() {
        return 0.0;
    }
    // Documentation.
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Documentation.
    // Documentation.
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

/// Documentation.
///
/// Documentation.
fn compute_ref_point(pareto_objs: &[Vec<f64>], m: usize) -> Vec<f64> {
    let mut nadir = vec![f64::NEG_INFINITY; m];
    let mut ideal = vec![f64::INFINITY; m];
    // Documentation.
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
    // Documentation.
    (0..m)
        .map(|j| nadir[j] + (nadir[j] - ideal[j]).abs() * 0.1 + 1.0)
        .collect()
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn chebyshev_sort(objectives: &[Vec<f64>], weights: &[f64], is_minimize: &[bool]) -> Vec<u32> {
    let n = objectives.len();
    if n == 0 {
        return vec![];
    }
    let m = objectives[0].len();
    if m == 0 || weights.iter().all(|&w| w == 0.0) {
        // Documentation.
        return (0..n as u32).collect();
    }

    // Documentation.
    let norm_objs = normalize_objectives(objectives, is_minimize);

    // Documentation.
    let ideal: Vec<f64> = (0..m)
        .map(|j| {
            norm_objs
                .iter()
                .map(|obj| obj[j])
                .filter(|v| !v.is_nan())
                .fold(f64::INFINITY, f64::min)
        })
        .collect();

    // Documentation.
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

    // Documentation.
    scores.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.into_iter().map(|(i, _)| i as u32).collect()
}

// =============================================================================
// Documentation.
// =============================================================================

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

        // Documentation.
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

        // Documentation.
        let hypervolume = if m >= 2 && pareto_indices.len() >= 2 {
            let pareto_objs: Vec<Vec<f64>> = pareto_indices
                .iter()
                .map(|&i| objectives[i as usize].clone())
                .collect();
            // Documentation.
            let norm_pareto = normalize_objectives(&pareto_objs, is_minimize);
            // Documentation.
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

/// Documentation.
pub fn compute_hypervolume_history(is_minimize: &[bool]) -> HvHistoryResult {
    crate::dataframe::with_active_df(|df| {
        let n = df.row_count();
        let obj_names = df.objective_col_names();
        let m = obj_names.len();

        let trial_ids: Vec<u32> = (0..n).filter_map(|i| df.get_trial_id(i)).collect();

        if m < 2 {
            // Documentation.
            return HvHistoryResult {
                trial_ids,
                hv_values: vec![0.0; n],
            };
        }

        // Documentation.
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

        // Documentation.
        let mut current_pareto: Vec<Vec<f64>> = Vec::new();
        let mut hv_values = Vec::with_capacity(n);

        for row in 0..n {
            let obj = norm_all[row].clone();
            if obj.iter().any(|v| v.is_nan()) {
                hv_values.push(hv_values.last().copied().unwrap_or(0.0));
                continue;
            }
            // Documentation.
            let dominated = current_pareto.iter().any(|p| dominates_minimized(p, &obj));
            if !dominated {
                // Documentation.
                current_pareto.retain(|p| !dominates_minimized(&obj, p));
                current_pareto.push(obj);
            }
            // Documentation.
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

/// Documentation.
///
/// Documentation.
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

        // Documentation.
        let cols: Vec<&[f64]> = obj_names
            .iter()
            .filter_map(|name| df.get_numeric_column(name))
            .collect();
        if cols.len() != m {
            return (0..n as u32).collect();
        }

        // Documentation.
        let sign: Vec<f64> = (0..m)
            .map(|j| {
                if is_minimize.get(j).copied().unwrap_or(true) {
                    1.0
                } else {
                    -1.0
                }
            })
            .collect();

        // Documentation.
        let ideal: Vec<f64> = (0..m)
            .map(|j| {
                cols[j]
                    .iter()
                    .filter(|v| !v.is_nan())
                    .map(|&v| sign[j] * v)
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();

        // Documentation.
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

        // Documentation.
        scores.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.into_iter().map(|(i, _)| i as u32).collect()
    })
    .unwrap_or_default()
}

// =============================================================================
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // Documentation.
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
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_201_01_two_obj_all_nondominated() {
        // Documentation.
        // Documentation.
        let objs = vec![
            vec![1.0, 4.0],
            vec![2.0, 3.0],
            vec![3.0, 2.0],
            vec![4.0, 1.0],
        ];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // Documentation.
        assert_eq!(ranks, vec![1, 1, 1, 1]);
    }

    #[test]
    fn tc_201_02_two_obj_clear_domination() {
        // Documentation.
        let objs = vec![vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // Documentation.
        assert_eq!(ranks, vec![1, 2, 3]);
    }

    #[test]
    fn tc_201_03_four_objectives() {
        // Documentation.
        // Documentation.
        let objs = vec![
            vec![1.0, 1.0, 1.0, 1.0], // rank 1
            vec![2.0, 2.0, 2.0, 2.0], // rank 3 (dominated by both)
            vec![1.0, 2.0, 1.0, 2.0], // rank 2 (dominated by [0])
        ];
        let is_min = [true, true, true, true];
        let ranks = nd_sort(&objs, &is_min);
        assert_eq!(ranks[0], 1); // Documentation.
        assert_eq!(ranks[2], 2); // Documentation.
        assert_eq!(ranks[1], 3); // Documentation.
    }

    #[test]
    fn tc_201_04_single_objective_all_rank1() {
        // Documentation.
        let objs = vec![vec![3.0], vec![1.0], vec![4.0], vec![1.5], vec![2.0]];
        let is_min = [true];
        let ranks = nd_sort(&objs, &is_min);
        // Documentation.
        assert!(ranks.iter().all(|&r| r == 1));
    }

    #[test]
    fn tc_201_05_maximize_direction() {
        // Documentation.
        // Documentation.
        let objs = vec![
            vec![1.0], // Documentation.
            vec![2.0],
            vec![3.0],
        ];
        let is_min_single = [true];
        let ranks_single = nd_sort(&objs, &is_min_single);
        assert!(ranks_single.iter().all(|&r| r == 1)); // Documentation.

        // Documentation.
        // Documentation.
        // Documentation.
        // Documentation.
        // Documentation.
        let objs2 = vec![
            vec![1.0, 3.0], // Documentation.
            vec![2.0, 2.0], // [-2, 2]
            vec![3.0, 1.0], // Documentation.
        ];
        let is_min2 = [false, true]; // obj0 maximize, obj1 minimize
        let ranks2 = nd_sort(&objs2, &is_min2);
        // normalize: (-1,3),(-2,2),(-3,1)
        // Documentation.
        // Documentation.
        // Documentation.
        assert_eq!(ranks2[2], 1); // Documentation.
        assert_eq!(ranks2[1], 2);
        assert_eq!(ranks2[0], 3);
    }

    #[test]
    fn tc_201_06_hypervolume_2d_known_value() {
        // Documentation.
        // Documentation.
        // Documentation.
        //      = 1 + 3 + 8 = 12
        let pts = vec![(1.0, 4.0), (2.0, 2.0), (3.0, 1.0)];
        let hv = hypervolume_2d(&pts, 5.0, 5.0);
        // Documentation.
        assert!((hv - 12.0).abs() < 1e-9, "HV = {}, expected 12.0", hv);
    }

    #[test]
    fn tc_201_07_hypervolume_single_objective_none() {
        // Documentation.
        let rows = vec![
            make_row_obj(0, vec![1.0]),
            make_row_obj(1, vec![2.0]),
            make_row_obj(2, vec![3.0]),
        ];
        setup_study(rows, &["obj0"]);
        let result = compute_pareto_ranks(&[true]);
        // Documentation.
        assert!(result.hypervolume.is_none());
    }

    #[test]
    fn tc_201_08_tradeoff_navigator_order() {
        // Documentation.
        // Documentation.
        // score0 = max(0.5×|1-1|, 0.5×|4-1|) = max(0, 1.5) = 1.5
        // score1 = max(0.5×|2-1|, 0.5×|2-1|) = max(0.5, 0.5) = 0.5
        // score2 = max(0.5×|4-1|, 0.5×|1-1|) = max(1.5, 0) = 1.5
        let objs = vec![vec![1.0, 4.0], vec![2.0, 2.0], vec![4.0, 1.0]];
        let is_min = [true, true];
        let weights = [0.5, 0.5];
        let result = chebyshev_sort(&objs, &weights, &is_min);
        // Documentation.
        assert_eq!(result[0], 1); // Documentation.
    }

    #[test]
    fn tc_201_09_hypervolume_history_single_obj() {
        // Documentation.
        let rows = vec![
            make_row_obj(0, vec![2.0]),
            make_row_obj(1, vec![1.0]),
            make_row_obj(2, vec![3.0]),
        ];
        setup_study(rows, &["obj0"]);
        let result = compute_hypervolume_history(&[true]);
        // Documentation.
        assert!(result.hv_values.iter().all(|&v| v == 0.0));
        assert_eq!(result.trial_ids.len(), 3);
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_201_e01_zero_weights_fallback() {
        // Documentation.
        let objs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let is_min = [true, true];
        let weights = [0.0, 0.0]; // Documentation.
        let result = chebyshev_sort(&objs, &weights, &is_min);
        // Documentation.
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn tc_201_e02_empty_dataframe_returns_empty() {
        // Documentation.
        store_dataframes(vec![DataFrame::empty()]);
        select_study(0).unwrap();
        let result = compute_pareto_ranks(&[true]);
        // Documentation.
        assert!(result.ranks.is_empty());
        assert!(result.pareto_indices.is_empty());
        assert!(result.hypervolume.is_none());
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_201_b01_all_same_coords() {
        // Documentation.
        let objs = vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // Documentation.
        assert!(ranks.iter().all(|&r| r == 1));
    }

    #[test]
    fn tc_201_b02_chain_dominance() {
        // Documentation.
        let objs = vec![
            vec![1.0, 1.0], // rank 1
            vec![2.0, 2.0], // rank 2
            vec![3.0, 3.0], // rank 3
            vec![4.0, 4.0], // rank 4
        ];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        // Documentation.
        assert_eq!(ranks, vec![1, 2, 3, 4]);
    }

    #[test]
    fn tc_201_b03_single_point() {
        // Documentation.
        let rows = vec![make_row_obj(0, vec![1.0, 2.0])];
        setup_study(rows, &["obj0", "obj1"]);
        let result = compute_pareto_ranks(&[true, true]);
        // Documentation.
        assert_eq!(result.ranks, vec![1]);
        assert_eq!(result.pareto_indices, vec![0]);
        assert!(result.hypervolume.is_none()); // pareto_indices.len() < 2
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_201_p01_ndsort_1000_points_under_100ms() {
        // Documentation.
        // Documentation.
        // Documentation.
        let n = 1_000usize;
        let objs: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                // Documentation.
                let x = ((i.wrapping_mul(7_919).wrapping_add(1_234_567)) % n) as f64 / n as f64;
                let y = ((i.wrapping_mul(6_271).wrapping_add(9_876_543)) % n) as f64 / n as f64;
                vec![x, y]
            })
            .collect();
        let is_min = [true, true];

        let start = std::time::Instant::now();
        let ranks = nd_sort(&objs, &is_min);
        let elapsed = start.elapsed();

        // Documentation.
        assert!(
            elapsed.as_millis() <= 100,
            "NDSort translated {}ms translated（translated: ≤100ms）",
            elapsed.as_millis()
        );
        assert_eq!(ranks.len(), n);
        assert!(ranks.iter().all(|&r| r >= 1));
    }

    #[test]
    fn tc_201_p02_tradeoff_50000_points_under_1ms() {
        // Documentation.
        // Documentation.
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

        // Documentation.
        #[cfg(debug_assertions)]
        assert!(
            elapsed.as_millis() <= 50,
            "Trade-off Navigator translated {}ms translated（translated: ≤50ms）",
            elapsed.as_millis()
        );
        #[cfg(not(debug_assertions))]
        assert!(
            elapsed.as_millis() <= 1,
            "Trade-off Navigator translated {}ms translated（translated: ≤1ms）",
            elapsed.as_millis()
        );
        assert_eq!(result.len(), n);
    }
}
