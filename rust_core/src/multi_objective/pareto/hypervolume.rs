use super::helpers::{add_to_pareto_front, compute_ref_point, normalize_objectives};
use super::types::HvHistoryResult;

/// N-dimensional hypervolume (assumes minimization, exact value).
///
/// Only points strictly smaller than `ref_point` in every dimension are valid.
/// m=1/2 use dedicated fast paths; m>=3 uses the WFG algorithm
/// (While, Bradstreet, Barone 2012). The input may include dominated or
/// duplicate points (they are reduced to the non-dominated set internally).
/// See theory/ja/optimization/hypervolume.md for method details.
pub fn hypervolume_nd(points: &[Vec<f64>], ref_point: &[f64]) -> f64 {
    let m = ref_point.len();
    if points.is_empty() || m == 0 {
        return 0.0;
    }

    let valid: Vec<Vec<f64>> = points
        .iter()
        .filter(|p| p.len() >= m && p.iter().zip(ref_point.iter()).all(|(pi, ri)| *pi < *ri))
        .cloned()
        .collect();

    if valid.is_empty() {
        return 0.0;
    }

    if m == 1 {
        let min_v = valid.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        return ref_point[0] - min_v;
    }

    if m == 2 {
        let pts_2d: Vec<(f64, f64)> = valid.iter().map(|p| (p[0], p[1])).collect();
        return hypervolume_2d(&pts_2d, ref_point[0], ref_point[1]);
    }

    // WFG's recursion cost grows with the number of points, so reduce to the
    // non-dominated set first. Sorting by the last objective ascending is a
    // heuristic to increase dominated points within the limitset and improve
    // pruning (correctness does not depend on the sort order).
    let mut front: Vec<Vec<f64>> = Vec::new();
    for p in valid {
        add_to_pareto_front(&mut front, p);
    }
    front.sort_by(|a, b| {
        a[m - 1]
            .partial_cmp(&b[m - 1])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    wfg(&front, ref_point)
}

/// WFG core: computes the HV of a non-dominated set as the sum of each
/// point's exclusive contribution (exclhv). Counts of 0/1/2 are handled in
/// closed form to keep the recursion shallow.
fn wfg(front: &[Vec<f64>], ref_point: &[f64]) -> f64 {
    match front.len() {
        0 => 0.0,
        1 => inclhv(&front[0], ref_point),
        2 => {
            // Inclusion-exclusion: |A∪B| = |A| + |B| − |A∩B|. The intersection
            // is the component-wise max.
            let joint: Vec<f64> = front[0]
                .iter()
                .zip(front[1].iter())
                .map(|(a, b)| a.max(*b))
                .collect();
            inclhv(&front[0], ref_point) + inclhv(&front[1], ref_point) - inclhv(&joint, ref_point)
        }
        _ => (0..front.len()).map(|i| exclhv(front, i, ref_point)).sum(),
    }
}

/// Inclusive HV of a single point p: Π_k (ref_k − p_k).
fn inclhv(p: &[f64], ref_point: &[f64]) -> f64 {
    p.iter()
        .zip(ref_point.iter())
        .map(|(pi, ri)| ri - pi)
        .product()
}

/// Exclusive contribution of front[i]: subtract from inclhv(front[i]) the HV
/// of the "shadow" (limitset = component-wise max) that the subsequent
/// points front[i+1..] cast inside front[i]'s box.
/// The shadow is reduced to a non-dominated set before recursing (this is
/// the core pruning step of WFG).
fn exclhv(front: &[Vec<f64>], i: usize, ref_point: &[f64]) -> f64 {
    let p = &front[i];
    let mut limit: Vec<Vec<f64>> = Vec::new();
    for q in &front[i + 1..] {
        let shadow: Vec<f64> = p.iter().zip(q.iter()).map(|(pi, qi)| pi.max(*qi)).collect();
        add_to_pareto_front(&mut limit, shadow);
    }
    inclhv(p, ref_point) - wfg(&limit, ref_point)
}

/// 2D hypervolume (assumes minimization, exact value).
///
/// Only points strictly smaller than the ref point in both dimensions are
/// valid. The input may include dominated or duplicate points (they are
/// reduced to a non-dominated front internally, then summed over x-ascending
/// intervals).
pub fn hypervolume_2d(pareto_points: &[(f64, f64)], ref_x: f64, ref_y: f64) -> f64 {
    if pareto_points.is_empty() {
        return 0.0;
    }
    let mut pts: Vec<(f64, f64)> = pareto_points
        .iter()
        .filter(|&&(x, y)| x < ref_x && y < ref_y)
        .cloned()
        .collect();
    if pts.is_empty() {
        return 0.0;
    }
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });

    // The interval sum assumes a non-dominated front where y strictly
    // decreases as x increases, so dominated/duplicate points are removed
    // here. Without this reduction, dominated points' strips would be
    // double-counted and the HV would be overestimated.
    let mut front: Vec<(f64, f64)> = Vec::with_capacity(pts.len());
    for &(x, y) in &pts {
        if front.last().is_none_or(|&(_, last_y)| y < last_y) {
            front.push((x, y));
        }
    }

    let mut hv = 0.0f64;
    for i in 0..front.len() {
        let next_x = if i + 1 < front.len() {
            front[i + 1].0
        } else {
            ref_x
        };
        let width = next_x - front[i].0;
        let height = ref_y - front[i].1;
        if width > 0.0 && height > 0.0 {
            hv += width * height;
        }
    }
    hv
}

/// Computes the HV history by taking data directly, without thread-local
/// state. Use this variant when calling from a background thread.
pub fn compute_hv_history_from_data(
    trial_ids: &[u32],
    objectives: &[Vec<f64>],
    is_minimize: &[bool],
) -> HvHistoryResult {
    compute_hv_history_with_ref(trial_ids, objectives, is_minimize, None)
}

/// Computes the HV history with an optional explicit reference point.
///
/// `ref_point_override` is a reference point in normalized space (maximize
/// objectives already sign-flipped). If `None`, it is auto-computed from the
/// observed points' nadir plus a 10% margin. The returned `ref_point` holds
/// the reference point actually used (in normalized space).
pub fn compute_hv_history_with_ref(
    trial_ids: &[u32],
    objectives: &[Vec<f64>],
    is_minimize: &[bool],
    ref_point_override: Option<&[f64]>,
) -> HvHistoryResult {
    let n = objectives.len();
    let m = if n > 0 { objectives[0].len() } else { 0 };

    // Empty result for cases where HV is not computed (single objective, no valid points).
    let empty = || HvHistoryResult {
        trial_ids: trial_ids.to_vec(),
        hv_values: vec![0.0; n],
        ref_point: Vec::new(),
    };

    if m < 2 {
        return empty();
    }

    let norm_all = normalize_objectives(objectives, is_minimize);
    let valid_objs: Vec<Vec<f64>> = norm_all
        .iter()
        .filter(|obj| !obj.iter().any(|v| v.is_nan()))
        .cloned()
        .collect();
    if valid_objs.is_empty() {
        return empty();
    }
    // Use the override if it's provided, matches the dimension, and all
    // elements are finite; otherwise auto-compute.
    let ref_pt = match ref_point_override {
        Some(r) if r.len() == m && r.iter().all(|v| v.is_finite()) => r.to_vec(),
        _ => compute_ref_point(&valid_objs, m),
    };

    let mut current_pareto: Vec<Vec<f64>> = Vec::new();
    let mut hv_values = Vec::with_capacity(n);

    for obj in norm_all.iter().take(n) {
        if obj.iter().any(|v| v.is_nan()) {
            hv_values.push(hv_values.last().copied().unwrap_or(0.0));
            continue;
        }
        add_to_pareto_front(&mut current_pareto, obj.clone());
        hv_values.push(hypervolume_nd(&current_pareto, &ref_pt));
    }

    HvHistoryResult {
        trial_ids: trial_ids.to_vec(),
        hv_values,
        ref_point: ref_pt,
    }
}

/// Thread-local version that computes the HV history from the active
/// DataFrame's objective columns. Returns an empty result if there is no
/// active study.
pub fn compute_hypervolume_history(is_minimize: &[bool]) -> HvHistoryResult {
    crate::dataframe::with_active_df(|df| {
        let n = df.row_count();
        let obj_names = df.objective_col_names();
        let trial_ids: Vec<u32> = (0..n).filter_map(|i| df.get_trial_id(i)).collect();
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
        compute_hv_history_from_data(&trial_ids, &all_objs, is_minimize)
    })
    .unwrap_or(HvHistoryResult {
        trial_ids: vec![],
        hv_values: vec![],
        ref_point: vec![],
    })
}

#[cfg(test)]
mod wfg_tests {
    use super::*;

    /// Old recursive slicing implementation (production code before WFG was
    /// introduced). Kept only in tests as a reference for validating WFG.
    /// Roughly O(n^m), so it's for small inputs only.
    fn hypervolume_nd_slicing(points: &[Vec<f64>], ref_point: &[f64]) -> f64 {
        let m = ref_point.len();
        if points.is_empty() || m == 0 {
            return 0.0;
        }
        let valid: Vec<Vec<f64>> = points
            .iter()
            .filter(|p| p.len() >= m && p.iter().zip(ref_point.iter()).all(|(pi, ri)| *pi < *ri))
            .cloned()
            .collect();
        if valid.is_empty() {
            return 0.0;
        }
        if m == 1 {
            let min_v = valid.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
            return ref_point[0] - min_v;
        }
        if m == 2 {
            let pts_2d: Vec<(f64, f64)> = valid.iter().map(|p| (p[0], p[1])).collect();
            return hypervolume_2d(&pts_2d, ref_point[0], ref_point[1]);
        }
        let last = m - 1;
        let mut sorted = valid;
        sorted.sort_by(|a, b| {
            a[last]
                .partial_cmp(&b[last])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut hv = 0.0f64;
        let mut prev = ref_point[last];
        for i in (0..sorted.len()).rev() {
            let thickness = prev - sorted[i][last];
            if thickness > 0.0 {
                let mut proj_front: Vec<Vec<f64>> = Vec::new();
                for p in &sorted[..=i] {
                    add_to_pareto_front(&mut proj_front, p[..last].to_vec());
                }
                hv += thickness * hypervolume_nd_slicing(&proj_front, &ref_point[..last]);
            }
            prev = sorted[i][last];
        }
        hv
    }

    /// Deterministic pseudo-random number generator (LCG). Seed is fixed for test reproducibility.
    fn lcg_next(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Verifies that WFG matches the reference implementation (old slicing method) on random fronts.
    #[test]
    fn wfg_matches_slicing_reference_on_random_fronts() {
        for m in [3usize, 4, 5] {
            for n in [1usize, 2, 3, 5, 8, 12] {
                let mut seed = (m * 1000 + n) as u64;
                let points: Vec<Vec<f64>> = (0..n)
                    .map(|_| (0..m).map(|_| lcg_next(&mut seed)).collect())
                    .collect();
                let ref_pt = vec![1.1; m];
                let a = hypervolume_nd(&points, &ref_pt);
                let b = hypervolume_nd_slicing(&points, &ref_pt);
                assert!((a - b).abs() < 1e-9, "m={m} n={n}: wfg={a} slicing={b}");
            }
        }
    }

    /// Hand-computed check: points (0,1,1), (1,0,0), ref point (1.1, 1.1, 1.1).
    /// Inclusion-exclusion: 1.1·0.1·0.1 + 0.1·1.1·1.1 − 0.1³ = 0.011 + 0.121 − 0.001 = 0.131
    #[test]
    fn wfg_3d_two_points_hand_computed() {
        let pts = vec![vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 0.0]];
        let ref_pt = vec![1.1, 1.1, 1.1];
        let hv = hypervolume_nd(&pts, &ref_pt);
        assert!((hv - 0.131).abs() < 1e-12, "HV = {hv}, expected 0.131");
    }

    /// Mixing in dominated points does not change the HV (they are reduced to the non-dominated set internally).
    #[test]
    fn wfg_unaffected_by_dominated_points() {
        let front = vec![vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 0.0]];
        let mut with_dominated = front.clone();
        with_dominated.push(vec![1.05, 1.05, 1.05]); // dominated by both points
        let ref_pt = vec![1.1, 1.1, 1.1];
        let a = hypervolume_nd(&front, &ref_pt);
        let b = hypervolume_nd(&with_dominated, &ref_pt);
        assert!((a - b).abs() < 1e-12, "a={a} b={b}");
    }

    /// Duplicate points are counted only once.
    #[test]
    fn wfg_duplicate_points_counted_once() {
        let front = vec![vec![0.2, 0.8, 0.5], vec![0.8, 0.2, 0.5]];
        let mut with_dup = front.clone();
        with_dup.push(vec![0.2, 0.8, 0.5]);
        let ref_pt = vec![1.0, 1.0, 1.0];
        let a = hypervolume_nd(&front, &ref_pt);
        let b = hypervolume_nd(&with_dup, &ref_pt);
        assert!((a - b).abs() < 1e-12, "a={a} b={b}");
    }

    /// A single point's HV equals the inclusive HV (box volume).
    #[test]
    fn wfg_single_point_is_box_volume() {
        let pts = vec![vec![0.25, 0.5, 0.75, 0.5]];
        let ref_pt = vec![1.0, 1.0, 1.0, 1.0];
        let hv = hypervolume_nd(&pts, &ref_pt);
        let expected = 0.75 * 0.5 * 0.25 * 0.5;
        assert!((hv - expected).abs() < 1e-12, "HV = {hv}");
    }
}
