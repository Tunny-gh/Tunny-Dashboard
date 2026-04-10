use super::common::{full_result, random_sample_fixed_seed, DownsampleResult};
use super::state::get_all_ranks;

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

    if total_count <= max_points {
        #[cfg(not(target_arch = "wasm32"))]
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        #[cfg(target_arch = "wasm32")]
        let duration_ms = 0.0_f64;

        return Some(full_result(total_count, duration_ms));
    }

    let all_ranks = get_all_ranks();
    let max_rank = n_strata.max(1);
    let mut by_rank: Vec<Vec<u32>> = vec![vec![]; max_rank + 1];

    for (idx, &rank) in all_ranks.iter().enumerate() {
        let r = rank as usize;
        if r >= 1 && r <= max_rank {
            by_rank[r].push(idx as u32);
        }
    }

    let rank1 = &by_rank[1];
    let pareto_count = rank1.len();

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
