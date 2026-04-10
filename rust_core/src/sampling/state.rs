use std::cell::RefCell;

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

fn objective_count_or_default() -> usize {
    crate::dataframe::with_active_df(|df| df.objective_col_names().len()).unwrap_or(1)
}

fn is_minimize() -> Vec<bool> {
    STATE.with(|s| {
        let st = s.borrow();
        if st.is_minimize.is_empty() {
            vec![true; objective_count_or_default()]
        } else {
            st.is_minimize.clone()
        }
    })
}

/// Return Pareto Rank 1 indices.
///
/// Uses the cached result from `init_sampling` when available; otherwise
/// computes it on-demand (O(n²) — acceptable for small datasets or fallback).
pub(crate) fn get_pareto_rank1_indices() -> Vec<u32> {
    let cached = STATE.with(|s| s.borrow().pareto_indices.clone());
    if let Some(indices) = cached {
        return indices;
    }

    let indices = crate::pareto::compute_pareto_ranks(&is_minimize()).pareto_indices;
    STATE.with(|s| s.borrow_mut().pareto_indices = Some(indices.clone()));
    indices
}

/// Return per-row Pareto rank array (1-based).
///
/// Uses the cached result from `init_sampling` when available; otherwise
/// computes it on-demand and caches the result.
pub(crate) fn get_all_ranks() -> Vec<u32> {
    let cached = STATE.with(|s| s.borrow().all_ranks.clone());
    if let Some(ranks) = cached {
        return ranks;
    }

    let result = crate::pareto::compute_pareto_ranks(&is_minimize());
    let ranks = result.ranks;
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.pareto_indices.is_none() {
            st.pareto_indices = Some(result.pareto_indices);
        }
        st.all_ranks = Some(ranks.clone());
    });
    ranks
}

pub(crate) fn cluster_labels() -> Option<Vec<i32>> {
    STATE.with(|s| s.borrow().cluster_labels.clone())
}
