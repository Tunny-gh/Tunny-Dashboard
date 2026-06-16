//! NDSort・Hypervolume・Trade-off Navigator
//!
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-201/pareto-requirements.md

mod helpers;
mod hypervolume;
mod ranking;
mod tradeoff;
mod types;

pub(crate) use helpers::{add_to_pareto_front, compute_ref_point, normalize_objectives};
pub use hypervolume::{
    compute_hv_history_from_data, compute_hv_history_with_ref, compute_hypervolume_history,
    hypervolume_2d, hypervolume_nd,
};
pub use ranking::{compute_pareto_ranks, nd_sort};
pub use tradeoff::chebyshev_sort;
pub use types::{HvHistoryResult, ParetoResult};

#[cfg(test)]
mod tests;
