use std::collections::HashMap;

use super::common::{full_result, random_sample_fixed_seed, DownsampleResult};
use super::context::SamplingContext;
use super::smart::downsample_smart;

/// Cluster-equalised downsampling for ClusterScatter / DimReductionScatter.
///
/// Algorithm:
/// 1. Read cluster labels from context.
/// 2. If no labels are stored, fall back to `downsample_smart(max_points, true)`.
/// 3. Otherwise, assign a budget of `max_points / K` to each of the K clusters.
/// 4. Sample from each cluster (seed 42); the largest cluster absorbs any
///    remaining points from the integer division.
///
/// Returns `None` when no active study is loaded.
pub fn downsample_by_cluster(ctx: &SamplingContext, max_points: usize) -> Option<DownsampleResult> {
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

    let Some(labels) = ctx.cluster_labels.as_ref() else {
        return downsample_smart(ctx, max_points, true);
    };

    let mut clusters: HashMap<i32, Vec<u32>> = HashMap::new();
    for (idx, &label) in labels.iter().enumerate() {
        if label >= 0 {
            clusters.entry(label).or_default().push(idx as u32);
        }
    }
    if clusters.is_empty() {
        return downsample_smart(ctx, max_points, true);
    }

    let k = clusters.len();
    let per_cluster = max_points / k;
    let mut remainder = max_points - per_cluster * k;

    let mut sorted_ids: Vec<i32> = clusters.keys().copied().collect();
    sorted_ids.sort_unstable();

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

    let pareto_indices = ctx.get_pareto_rank0_indices();
    let pareto_count = pareto_indices
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
