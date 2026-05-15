use std::collections::HashMap;
use std::collections::HashSet;

use super::common::{full_result, DownsampleResult};
use super::context::SamplingContext;

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
pub fn downsample_for_thumbnail(ctx: &SamplingContext, max_points: usize) -> Option<DownsampleResult> {
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    let total_count = crate::dataframe::with_active_df(|df| df.row_count())?;

    if total_count <= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(full_result(total_count, duration_ms));
    }

    let all_pareto = ctx.get_pareto_rank0_indices();
    let max_pareto = max_points / 2;
    let confirmed_pareto: Vec<u32> = all_pareto[..all_pareto.len().min(max_pareto)].to_vec();
    let pareto_count = confirmed_pareto.len();
    let remaining_budget = max_points.saturating_sub(pareto_count);

    let pareto_set: HashSet<u32> = confirmed_pareto.iter().copied().collect();

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

    let mut cell_rep: HashMap<(usize, usize), u32> = HashMap::new();

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
