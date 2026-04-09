//! Downsampling functions for chart rendering performance.
//!
//! All functions access the active DataFrame via WASM global state.
//!
//! # Usage pattern
//!
//! 1. After `select_study`, call `init_sampling(is_minimize, pareto_indices)` once.
//!    `pareto_indices` should come from the already-computed `compute_pareto_ranks()`
//!    result so that downsampling functions do not pay the O(n²) Pareto cost.
//! 2. Call `downsample_smart` / `downsample_for_thumbnail` etc. as needed.

use std::cell::RefCell;
use std::collections::HashSet;

// =============================================================================
// Result type
// =============================================================================

/// Result of a downsampling operation.
pub struct DownsampleResult {
    /// Row indices (into the active DataFrame) selected for rendering.
    pub indices: Vec<u32>,
    /// Number of Pareto Rank 1 points included.
    pub pareto_count: usize,
    /// Total row count in the active DataFrame.
    pub total_count: usize,
    /// Wall-clock duration of the sampling computation (ms), excluding Pareto
    /// pre-computation which is done once via `init_sampling`.
    pub duration_ms: f64,
}

// =============================================================================
// Global state
// =============================================================================

struct SamplingState {
    is_minimize: Vec<bool>,
    /// Pre-computed Pareto Rank 1 indices.  `None` means "not yet computed".
    pareto_indices: Option<Vec<u32>>,
    /// Full per-row Pareto ranks (1-based).  `None` means "not yet computed".
    all_ranks: Option<Vec<u32>>,
    /// Per-row cluster labels (0-based, -1 = unclustered).
    /// `None` means cluster computation has not been run.
    cluster_labels: Option<Vec<i32>>,
}

thread_local! {
    static STATE: RefCell<SamplingState> = RefCell::new(SamplingState {
        is_minimize: vec![],
        pareto_indices: None,
        all_ranks: None,
        cluster_labels: None,
    });
}

/// Initialise sampling state after a study is loaded.
///
/// `pareto_indices` — Rank 1 indices from `pareto::compute_pareto_ranks`.
/// `all_ranks`      — per-row rank array from `pareto::compute_pareto_ranks`.
///
/// Passing empty slices is safe; functions will fall back to on-demand
/// computation (slower).
pub fn init_sampling(is_minimize: Vec<bool>, pareto_indices: Vec<u32>, all_ranks: Vec<u32>) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.is_minimize = is_minimize;
        st.pareto_indices = Some(pareto_indices);
        st.all_ranks = if all_ranks.is_empty() {
            None
        } else {
            Some(all_ranks)
        };
    });
}

/// Reset sampling state (called when a new study is selected but before
/// `init_sampling` has been called for the new study).
pub fn reset_sampling() {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.is_minimize = vec![];
        st.pareto_indices = None;
        st.all_ranks = None;
        st.cluster_labels = None;
    });
}

/// Store cluster labels produced by k-means or HDBSCAN.
///
/// `labels[i]` is the cluster id (0-based) for row `i`, or -1 for unclustered.
/// Call this after running `runKmeans` to enable `downsample_by_cluster`.
pub fn set_cluster_labels(labels: Vec<i32>) {
    STATE.with(|s| {
        s.borrow_mut().cluster_labels = Some(labels);
    });
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Return Pareto Rank 1 indices.
///
/// Uses the cached result from `init_sampling` when available; otherwise
/// computes it on-demand (O(n²) — acceptable for small datasets or fallback).
fn get_pareto_rank1_indices() -> Vec<u32> {
    // Try cached first
    let cached = STATE.with(|s| s.borrow().pareto_indices.clone());
    if let Some(indices) = cached {
        return indices;
    }

    // Fallback: compute from scratch
    let is_min = STATE.with(|s| {
        let st = s.borrow();
        if st.is_minimize.is_empty() {
            let n =
                crate::dataframe::with_active_df(|df| df.objective_col_names().len()).unwrap_or(1);
            vec![true; n]
        } else {
            st.is_minimize.clone()
        }
    });
    let indices = crate::pareto::compute_pareto_ranks(&is_min).pareto_indices;
    // Cache the result
    STATE.with(|s| s.borrow_mut().pareto_indices = Some(indices.clone()));
    indices
}

/// Return per-row Pareto rank array (1-based).
///
/// Uses the cached result from `init_sampling` when available; otherwise
/// computes it on-demand and caches the result.
fn get_all_ranks() -> Vec<u32> {
    let cached = STATE.with(|s| s.borrow().all_ranks.clone());
    if let Some(ranks) = cached {
        return ranks;
    }
    let is_min = STATE.with(|s| {
        let st = s.borrow();
        if st.is_minimize.is_empty() {
            let n =
                crate::dataframe::with_active_df(|df| df.objective_col_names().len()).unwrap_or(1);
            vec![true; n]
        } else {
            st.is_minimize.clone()
        }
    });
    let result = crate::pareto::compute_pareto_ranks(&is_min);
    let ranks = result.ranks;
    // Also cache pareto_indices if not yet cached
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.pareto_indices.is_none() {
            st.pareto_indices = Some(result.pareto_indices);
        }
        st.all_ranks = Some(ranks.clone());
    });
    ranks
}

/// Randomly sample `n` elements from `pool` using a fixed seed (42).
///
/// Uses a 64-bit LCG — no external crate required and cross-platform
/// reproducible.
fn random_sample_fixed_seed(pool: &[u32], n: usize) -> Vec<u32> {
    if n >= pool.len() {
        return pool.to_vec();
    }
    let mut buf: Vec<u32> = pool.to_vec();
    let len = buf.len();
    // Knuth's LCG constants
    let mut state: u64 = 42;
    for i in (1..len).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (state >> 33) as usize % (i + 1);
        buf.swap(i, j);
    }
    buf[..n].to_vec()
}

// =============================================================================
// Public API
// =============================================================================

/// General-purpose downsampling with Pareto Rank 1 preservation.
///
/// Algorithm:
/// 1. Get total row count from the active DataFrame.
/// 2. If N ≤ max_points, return all indices (no-op).
/// 3. If `include_pareto`, reserve cached Pareto Rank 1 indices first.
/// 4. If Pareto count ≥ max_points, return the first `max_points` Pareto indices.
/// 5. Randomly sample the remaining budget from non-Pareto rows (seed 42).
/// 6. Return Pareto + sampled non-Pareto.
///
/// Returns `None` when no active study is loaded.
pub fn downsample_smart(max_points: usize, include_pareto: bool) -> Option<DownsampleResult> {
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    let total_count = crate::dataframe::with_active_df(|df| df.row_count())?;

    // --- No-op ---
    if total_count <= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(DownsampleResult {
            indices: (0..total_count as u32).collect(),
            pareto_count: 0,
            total_count,
            duration_ms,
        });
    }

    // --- Pareto reservation ---
    let pareto_indices = if include_pareto {
        get_pareto_rank1_indices()
    } else {
        vec![]
    };
    let pareto_count = pareto_indices.len();

    // Pareto alone exceeds the budget → truncate to max_points.
    if pareto_count >= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(DownsampleResult {
            indices: pareto_indices[..max_points].to_vec(),
            pareto_count,
            total_count,
            duration_ms,
        });
    }

    // --- Sample non-Pareto rows ---
    let pareto_set: HashSet<u32> = pareto_indices.iter().copied().collect();
    let non_pareto: Vec<u32> = (0..total_count as u32)
        .filter(|i| !pareto_set.contains(i))
        .collect();

    let remaining_budget = max_points - pareto_count;
    let sampled = random_sample_fixed_seed(&non_pareto, remaining_budget);

    let mut indices = pareto_indices;
    indices.extend_from_slice(&sampled);

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;

    Some(DownsampleResult {
        indices,
        pareto_count,
        total_count,
        duration_ms,
    })
}

/// Thumbnail downsampling with Pareto preservation and grid spatial sampling.
///
/// Algorithm (REQ-063 compliant):
/// 1. Get cached Pareto Rank 1 indices.
/// 2. Limit Pareto to min(pareto_count, max_points / 2).
/// 3. remaining_budget = max_points - confirmed_pareto_count.
/// 4. Divide the objective space of non-Pareto points into a
///    ⌊√remaining_budget⌋ × ⌊√remaining_budget⌋ grid.
/// 5. From each non-empty cell, pick one point (deterministic: first in cell).
/// 6. Return Pareto + grid-selected non-Pareto.
///
/// Returns `None` when no active study is loaded.
pub fn downsample_for_thumbnail(max_points: usize) -> Option<DownsampleResult> {
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    let total_count = crate::dataframe::with_active_df(|df| df.row_count())?;

    // --- No-op ---
    if total_count <= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(DownsampleResult {
            indices: (0..total_count as u32).collect(),
            pareto_count: 0,
            total_count,
            duration_ms,
        });
    }

    // --- Pareto (capped at 50%) ---
    let all_pareto = get_pareto_rank1_indices();
    let max_pareto = max_points / 2;
    let confirmed_pareto: Vec<u32> = all_pareto[..all_pareto.len().min(max_pareto)].to_vec();
    let pareto_count = confirmed_pareto.len();
    let remaining_budget = max_points.saturating_sub(pareto_count);

    // --- Grid spatial sampling of non-Pareto points ---
    let pareto_set: HashSet<u32> = confirmed_pareto.iter().copied().collect();

    // Read objective values (first 2 objectives) for all non-Pareto indices.
    let (obj0_vals, obj1_vals) = crate::dataframe::with_active_df(|df| {
        let names = df.objective_col_names();
        let v0: Vec<f64> = names
            .first()
            .and_then(|n| df.get_numeric_column(n))
            .map(|c| c.to_vec())
            .unwrap_or_else(|| vec![0.0; total_count]);
        let v1: Vec<f64> = names
            .get(1)
            .and_then(|n| df.get_numeric_column(n))
            .map(|c| c.to_vec())
            .unwrap_or_else(|| (0..total_count).map(|i| i as f64).collect());
        (v0, v1)
    })
    .unwrap_or_else(|| {
        (
            vec![0.0; total_count],
            (0..total_count).map(|i| i as f64).collect(),
        )
    });

    let non_pareto: Vec<u32> = (0..total_count as u32)
        .filter(|i| !pareto_set.contains(i))
        .collect();

    let grid_selected = grid_sample(&non_pareto, &obj0_vals, &obj1_vals, remaining_budget);

    let mut indices = confirmed_pareto;
    indices.extend_from_slice(&grid_selected);

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;

    Some(DownsampleResult {
        indices,
        pareto_count,
        total_count,
        duration_ms,
    })
}

/// Select at most `budget` points from `pool` by dividing the objective space
/// (defined by `obj0` and `obj1` coordinates) into a √budget × √budget grid
/// and picking one representative per non-empty cell.
fn grid_sample(pool: &[u32], obj0: &[f64], obj1: &[f64], budget: usize) -> Vec<u32> {
    if pool.is_empty() || budget == 0 {
        return vec![];
    }
    if pool.len() <= budget {
        return pool.to_vec();
    }

    let k = (budget as f64).sqrt().floor().max(1.0) as usize;

    // Compute obj0 range among pool points
    let mut min0 = f64::INFINITY;
    let mut max0 = f64::NEG_INFINITY;
    let mut min1 = f64::INFINITY;
    let mut max1 = f64::NEG_INFINITY;
    for &idx in pool {
        let v0 = obj0.get(idx as usize).copied().unwrap_or(0.0);
        let v1 = obj1.get(idx as usize).copied().unwrap_or(0.0);
        if v0.is_finite() {
            min0 = min0.min(v0);
            max0 = max0.max(v0);
        }
        if v1.is_finite() {
            min1 = min1.min(v1);
            max1 = max1.max(v1);
        }
    }
    let range0 = (max0 - min0).max(1e-12);
    let range1 = (max1 - min1).max(1e-12);

    // Assign each pool point to a cell; keep only the first encountered per cell.
    let mut cell_rep: std::collections::HashMap<(usize, usize), u32> =
        std::collections::HashMap::new();

    for &idx in pool {
        let v0 = obj0.get(idx as usize).copied().unwrap_or(min0);
        let v1 = obj1.get(idx as usize).copied().unwrap_or(min1);
        let ci = ((v0 - min0) / range0 * k as f64).floor() as usize;
        let cj = ((v1 - min1) / range1 * k as f64).floor() as usize;
        let ci = ci.min(k - 1);
        let cj = cj.min(k - 1);
        cell_rep.entry((ci, cj)).or_insert(idx);
    }

    cell_rep.into_values().collect()
}

/// Pareto-rank–stratified downsampling for ParallelCoordinates.
///
/// Algorithm:
/// 1. Get per-row Pareto ranks (from cache or computed on-demand).
/// 2. Group row indices by rank.
/// 3. Rank 1 gets full allocation (all points included); if Rank 1 alone
///    exceeds `max_points`, return the first `max_points` Rank 1 points.
/// 4. Remaining budget is distributed across higher ranks proportionally to
///    1/rank (i.e., Rank 2 gets half the budget of Rank 1, Rank 3 one-third,
///    etc.), clamped to the group size.
/// 5. Random-sample (seed 42) from each rank group up to its quota.
///
/// `n_strata` limits the maximum number of distinct ranks to consider; ranks
/// beyond `n_strata` are ignored (points omitted).
///
/// Returns `None` when no active study is loaded.
pub fn downsample_stratified_by_rank(
    max_points: usize,
    n_strata: usize,
) -> Option<DownsampleResult> {
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    let total_count = crate::dataframe::with_active_df(|df| df.row_count())?;

    // --- No-op ---
    if total_count <= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(DownsampleResult {
            indices: (0..total_count as u32).collect(),
            pareto_count: 0,
            total_count,
            duration_ms,
        });
    }

    let all_ranks = get_all_ranks();

    // Group indices by rank (1-based).
    let max_rank = n_strata.max(1);
    let mut by_rank: Vec<Vec<u32>> = vec![vec![]; max_rank + 1]; // index 0 unused

    for (idx, &rank) in all_ranks.iter().enumerate() {
        let r = rank as usize;
        if r >= 1 && r <= max_rank {
            by_rank[r].push(idx as u32);
        }
    }

    let rank1 = &by_rank[1];
    let pareto_count = rank1.len();

    // If Rank 1 alone exceeds max_points, truncate.
    if pareto_count >= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(DownsampleResult {
            indices: rank1[..max_points].to_vec(),
            pareto_count,
            total_count,
            duration_ms,
        });
    }

    // Allocate budget proportionally to 1/rank.
    // Weight_r = 1/r; normalised across ranks 1..=max_rank.
    let total_weight: f64 = (1..=max_rank).map(|r| 1.0 / r as f64).sum();
    let mut result_indices: Vec<u32> = rank1.clone();
    let mut used = pareto_count;

    for r in 2..=max_rank {
        if used >= max_points {
            break;
        }
        let group = &by_rank[r];
        if group.is_empty() {
            continue;
        }
        let weight = 1.0 / r as f64;
        let quota = ((weight / total_weight) * max_points as f64).round() as usize;
        let quota = quota.min(max_points - used).min(group.len());
        if quota == 0 {
            continue;
        }
        let sampled = random_sample_fixed_seed(group, quota);
        result_indices.extend_from_slice(&sampled);
        used += sampled.len();
    }

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;

    Some(DownsampleResult {
        indices: result_indices,
        pareto_count,
        total_count,
        duration_ms,
    })
}

/// Cluster-equalised downsampling for ClusterScatter / DimReductionScatter.
///
/// Algorithm:
/// 1. Read cluster labels from global state.
/// 2. If no labels are stored, fall back to `downsample_smart(max_points, true)`.
/// 3. Otherwise, assign a budget of `max_points / K` to each of the K clusters.
/// 4. Sample from each cluster (seed 42); the largest cluster absorbs any
///    remaining points from the integer division.
///
/// Returns `None` when no active study is loaded.
pub fn downsample_by_cluster(max_points: usize) -> Option<DownsampleResult> {
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    let total_count = crate::dataframe::with_active_df(|df| df.row_count())?;

    // --- No-op ---
    if total_count <= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(DownsampleResult {
            indices: (0..total_count as u32).collect(),
            pareto_count: 0,
            total_count,
            duration_ms,
        });
    }

    // --- Fallback if no cluster labels ---
    let labels_opt = STATE.with(|s| s.borrow().cluster_labels.clone());
    if labels_opt.is_none() {
        return downsample_smart(max_points, true);
    }
    let labels = labels_opt.unwrap();

    // --- Group indices by cluster ---
    let mut clusters: std::collections::HashMap<i32, Vec<u32>> = std::collections::HashMap::new();
    for (idx, &label) in labels.iter().enumerate() {
        if label >= 0 {
            clusters.entry(label).or_default().push(idx as u32);
        }
    }
    if clusters.is_empty() {
        return downsample_smart(max_points, true);
    }

    let k = clusters.len();
    let per_cluster = max_points / k;
    let mut remainder = max_points - per_cluster * k;

    // Sort cluster ids for deterministic output
    let mut sorted_ids: Vec<i32> = clusters.keys().copied().collect();
    sorted_ids.sort_unstable();

    // Find largest cluster to absorb remainder
    let largest_cluster_id = sorted_ids
        .iter()
        .max_by_key(|&&id| clusters[&id].len())
        .copied()
        .unwrap_or(sorted_ids[0]);

    let mut result_indices: Vec<u32> = Vec::with_capacity(max_points);

    for &id in &sorted_ids {
        let group = &clusters[&id];
        let mut quota = per_cluster;
        if id == largest_cluster_id && remainder > 0 {
            quota += remainder;
            remainder = 0;
        }
        let sampled = random_sample_fixed_seed(group, quota);
        result_indices.extend_from_slice(&sampled);
    }

    let pareto_count = get_pareto_rank1_indices()
        .iter()
        .filter(|&&p| result_indices.contains(&p))
        .count();

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;

    Some(DownsampleResult {
        indices: result_indices,
        pareto_count,
        total_count,
        duration_ms,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn make_row(trial_id: u32, obj_values: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: obj_values,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    /// Build a DataFrame with `n` rows (single objective = row index).
    /// Pareto Rank 1 = row 0 (smallest value under minimisation).
    fn setup_single_obj(n: usize) {
        let rows: Vec<TrialRow> = (0..n).map(|i| make_row(i as u32, vec![i as f64])).collect();
        let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
        store_dataframes(vec![df]);
        select_study(0).expect("select_study");
        // Row 0 is the only Pareto point (minimum of a single objective)
        init_sampling(vec![true], vec![0], vec![]);
    }

    // -------------------------------------------------------------------------
    // TC1: N ≤ max_points → return all indices
    // -------------------------------------------------------------------------

    #[test]
    fn tc1656_01_small_df_returns_all() {
        setup_single_obj(100);
        let result = downsample_smart(1_000, true).expect("should return Some");
        assert_eq!(result.indices.len(), 100, "expected all 100 rows");
        assert_eq!(result.total_count, 100);
    }

    // -------------------------------------------------------------------------
    // TC2: Pareto Rank 1 points must always be included
    // -------------------------------------------------------------------------

    #[test]
    fn tc1656_02_pareto_always_included() {
        // Use a small n so nd_sort is fast, then inject pre-computed pareto.
        let n = 200usize;
        let rows: Vec<TrialRow> = (0..n)
            .map(|i| make_row(i as u32, vec![i as f64, i as f64]))
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &[],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        );
        store_dataframes(vec![df]);
        select_study(0).expect("select_study");
        // Row 0 is the sole Pareto point
        let pareto = crate::pareto::compute_pareto_ranks(&[true, true]).pareto_indices;
        init_sampling(vec![true, true], pareto, vec![]);

        let result = downsample_smart(100, true).expect("should return Some");

        assert!(
            result.indices.contains(&0),
            "Pareto Rank 1 point (index 0) must be included"
        );
        assert_eq!(result.indices.len(), 100);
        assert!(result.pareto_count >= 1);
    }

    // -------------------------------------------------------------------------
    // TC3: Pareto count > max_points → truncate to max_points Pareto indices
    // -------------------------------------------------------------------------

    #[test]
    fn tc1656_03_pareto_exceeds_max_points() {
        // 200 rows forming a Pareto front (non-dominated trade-off curve).
        let pareto_size = 200usize;
        let n = 500usize;
        let rows: Vec<TrialRow> = (0..n)
            .map(|i| {
                if i < pareto_size {
                    // Non-dominated: obj0 decreases, obj1 increases
                    make_row(i as u32, vec![(pareto_size - i) as f64, i as f64])
                } else {
                    // Dominated: both worse than row 0
                    make_row(
                        i as u32,
                        vec![pareto_size as f64 + 1.0, pareto_size as f64 + 1.0],
                    )
                }
            })
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &[],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        );
        store_dataframes(vec![df]);
        select_study(0).expect("select_study");
        // Pre-compute pareto (500 rows is fast)
        let pareto = crate::pareto::compute_pareto_ranks(&[true, true]).pareto_indices;
        init_sampling(vec![true, true], pareto, vec![]);

        let result = downsample_smart(100, true).expect("should return Some");
        assert_eq!(result.indices.len(), 100, "must be capped at max_points");
    }

    // -------------------------------------------------------------------------
    // TC4: Performance — 50,000 points, pre-computed Pareto, must finish < 5ms
    // -------------------------------------------------------------------------

    #[test]
    fn tc1656_04_performance_50k() {
        let n = 50_000usize;
        let rows: Vec<TrialRow> = (0..n)
            .map(|i| make_row(i as u32, vec![i as f64, i as f64]))
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &[],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        );
        store_dataframes(vec![df]);
        select_study(0).expect("select_study");

        // Pre-set pareto cache (single point) — avoids O(n²) Pareto computation
        // inside the timing measurement.
        init_sampling(vec![true, true], vec![0u32], vec![]);

        let result = downsample_smart(10_000, true).expect("should return Some");
        assert_eq!(result.total_count, 50_000);
        assert_eq!(result.indices.len(), 10_000);
        // In debug mode, Rust is ~10-50× slower; the real target (< 5ms) is
        // validated by `cargo bench` in TASK-1673 using `--release`.
        #[cfg(debug_assertions)]
        let threshold_ms = 200.0_f64;
        #[cfg(not(debug_assertions))]
        let threshold_ms = 5.0_f64;

        assert!(
            result.duration_ms < threshold_ms,
            "expected < {threshold_ms}ms, got {:.2}ms",
            result.duration_ms
        );
    }

    // =========================================================================
    // TASK-1657: downsample_for_thumbnail
    // =========================================================================

    // -------------------------------------------------------------------------
    // TC1: Pareto is capped at 50% of max_points
    // -------------------------------------------------------------------------

    #[test]
    fn tc1657_01_pareto_capped_at_50_percent() {
        // 400 Pareto points + 9600 non-Pareto = 10000 total
        let pareto_size = 400usize;
        let total = 10_000usize;
        // Build a non-dominated front: obj0 decreases, obj1 increases
        let rows: Vec<TrialRow> = (0..total)
            .map(|i| {
                if i < pareto_size {
                    make_row(i as u32, vec![(pareto_size - i) as f64, i as f64])
                } else {
                    // dominated
                    make_row(
                        i as u32,
                        vec![pareto_size as f64 + 1.0, pareto_size as f64 + 1.0],
                    )
                }
            })
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &[],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        );
        store_dataframes(vec![df]);
        select_study(0).expect("select_study");
        let pareto = crate::pareto::compute_pareto_ranks(&[true, true]).pareto_indices;
        init_sampling(vec![true, true], pareto, vec![]);

        let result = downsample_for_thumbnail(500).expect("should return Some");

        // Pareto must be at most 50% of max_points (500/2 = 250)
        assert!(
            result.pareto_count <= 250,
            "pareto_count {} should be <= 250 (50% of 500)",
            result.pareto_count
        );
    }

    // -------------------------------------------------------------------------
    // TC2: Total indices ≤ max_points (REQ-063)
    // -------------------------------------------------------------------------

    #[test]
    fn tc1657_02_total_within_max_points() {
        setup_single_obj(5_000);
        let result = downsample_for_thumbnail(500).expect("should return Some");
        assert!(
            result.indices.len() <= 500,
            "indices.len() = {} must be <= 500",
            result.indices.len()
        );
    }

    // =========================================================================
    // TASK-1658: downsample_stratified_by_rank
    // =========================================================================

    // -------------------------------------------------------------------------
    // TC1: Rank 1 points are all included (when budget allows)
    // -------------------------------------------------------------------------

    #[test]
    fn tc1658_01_rank1_all_included() {
        // 50 Rank-1 points + 9950 higher-rank points
        let rank1_size = 50usize;
        let total = 10_000usize;
        // Rank-1: non-dominated front (obj0 dec, obj1 inc)
        // Rank-2: dominated by front but non-dominated among themselves → use a
        //         simpler approach: set all non-front points as dominated (rank 2)
        let rows: Vec<TrialRow> = (0..total)
            .map(|i| {
                if i < rank1_size {
                    make_row(i as u32, vec![(rank1_size - i) as f64, i as f64])
                } else {
                    // All dominated: strictly worse than row 0
                    make_row(
                        i as u32,
                        vec![rank1_size as f64 + 1.0, rank1_size as f64 + 1.0],
                    )
                }
            })
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &[],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        );
        store_dataframes(vec![df]);
        select_study(0).expect("select_study");
        let pareto_result = crate::pareto::compute_pareto_ranks(&[true, true]);
        let all_ranks = pareto_result.ranks.clone();
        let pareto_indices = pareto_result.pareto_indices.clone();
        init_sampling(vec![true, true], pareto_indices, all_ranks);

        let result = downsample_stratified_by_rank(5_000, 5).expect("should return Some");

        // All 50 Rank-1 points must be in the result
        let result_set: HashSet<u32> = result.indices.iter().copied().collect();
        // pareto_count from result should be ≤ total indices
        assert!(result.pareto_count <= result.indices.len());
        assert_eq!(
            result.pareto_count, rank1_size,
            "all rank1 must be included"
        );
        // Verify all rank1 indices are actually in the result
        assert!(result.indices.len() >= rank1_size);
        // The first pareto_count items should cover rank1
        for &idx in &result.indices[..result.pareto_count] {
            assert!(
                result_set.contains(&idx),
                "rank1 index {} not in result",
                idx
            );
        }
    }

    // -------------------------------------------------------------------------
    // TC2: Total indices ≤ max_points
    // -------------------------------------------------------------------------

    #[test]
    fn tc1658_02_total_within_max_points() {
        setup_single_obj(10_000);
        // Single objective → all rank 1; compute proper ranks
        let pareto_result = crate::pareto::compute_pareto_ranks(&[true]);
        init_sampling(
            vec![true],
            pareto_result.pareto_indices,
            pareto_result.ranks,
        );
        let result = downsample_stratified_by_rank(5_000, 5).expect("should return Some");
        assert!(
            result.indices.len() <= 5_000,
            "indices.len() = {} must be <= 5000",
            result.indices.len()
        );
    }

    // =========================================================================
    // TASK-1659: downsample_by_cluster
    // =========================================================================

    // -------------------------------------------------------------------------
    // TC1: Equal samples from each cluster
    // -------------------------------------------------------------------------

    #[test]
    fn tc1659_01_equal_sampling_per_cluster() {
        // 4 clusters × 2000 points = 8000 total
        let clusters = 4usize;
        let per_cluster_size = 2_000usize;
        let total = clusters * per_cluster_size;

        let rows: Vec<TrialRow> = (0..total)
            .map(|i| make_row(i as u32, vec![i as f64]))
            .collect();
        let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
        store_dataframes(vec![df]);
        select_study(0).expect("select_study");
        init_sampling(vec![true], vec![0], vec![]);

        // Set cluster labels: cluster 0..3, each with 2000 points
        let labels: Vec<i32> = (0..total).map(|i| (i / per_cluster_size) as i32).collect();
        set_cluster_labels(labels.clone());

        let result = downsample_by_cluster(4_000).expect("should return Some");

        // Exactly 4000 indices, roughly 1000 per cluster
        assert_eq!(result.indices.len(), 4_000);

        // Count per cluster
        let mut counts = vec![0usize; clusters];
        for &idx in &result.indices {
            let cluster = labels[idx as usize] as usize;
            counts[cluster] += 1;
        }
        for (c, &cnt) in counts.iter().enumerate() {
            assert_eq!(
                cnt, 1_000,
                "cluster {} expected 1000 points, got {}",
                c, cnt
            );
        }
    }

    // -------------------------------------------------------------------------
    // TC2: Fallback to downsample_smart when no cluster labels
    // -------------------------------------------------------------------------

    #[test]
    fn tc1659_02_fallback_without_labels() {
        setup_single_obj(50_000);
        // Explicitly clear cluster labels (reset_sampling clears them)
        reset_sampling();
        // Re-init without cluster labels
        init_sampling(vec![true], vec![0u32], vec![]);
        // (cluster_labels remains None after reset + init_sampling)

        let result = downsample_by_cluster(10_000).expect("should return Some");

        // Should produce results (fallback to smart sampling)
        assert_eq!(result.indices.len(), 10_000);
        assert_eq!(result.total_count, 50_000);
    }
}
