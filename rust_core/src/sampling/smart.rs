use std::collections::HashSet;

use super::common::{full_result, random_sample_fixed_seed, DownsampleResult};
use super::context::SamplingContext;

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
pub fn downsample_smart(
    ctx: &SamplingContext,
    max_points: usize,
    include_pareto: bool,
) -> Option<DownsampleResult> {
    let start = std::time::Instant::now();

    let total_count = crate::dataframe::with_active_df(|df| df.row_count())?;

    if total_count <= max_points {
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        return Some(full_result(total_count, duration_ms));
    }

    let pareto_indices = if include_pareto {
        ctx.get_pareto_rank0_indices()
    } else {
        vec![]
    };
    let pareto_count = pareto_indices.len();

    if pareto_count >= max_points {
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        return Some(DownsampleResult {
            indices: pareto_indices[..max_points].to_vec(),
            pareto_count,
            total_count,
            duration_ms,
        });
    }

    let pareto_set: HashSet<u32> = pareto_indices.iter().copied().collect();
    let non_pareto: Vec<u32> = (0..total_count as u32)
        .filter(|i| !pareto_set.contains(i))
        .collect();

    let remaining_budget = max_points - pareto_count;
    let sampled = random_sample_fixed_seed(&non_pareto, remaining_budget);

    let mut indices = pareto_indices;
    indices.extend_from_slice(&sampled);

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    Some(DownsampleResult {
        indices,
        pareto_count,
        total_count,
        duration_ms,
    })
}
