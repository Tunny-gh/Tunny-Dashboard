use super::context::SamplingContext;

/// Initialise a SamplingContext after a study is loaded.
///
/// `pareto_indices` — Rank 0 indices from `pareto::compute_pareto_ranks`.
/// `all_ranks`      — per-row rank array from `pareto::compute_pareto_ranks`.
pub fn init_sampling(
    is_minimize: Vec<bool>,
    pareto_indices: Vec<u32>,
    all_ranks: Vec<u32>,
) -> SamplingContext {
    SamplingContext {
        is_minimize,
        pareto_indices: Some(pareto_indices),
        all_ranks: if all_ranks.is_empty() { None } else { Some(all_ranks) },
        cluster_labels: None,
    }
}
