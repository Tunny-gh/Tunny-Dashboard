//! NDSort / Hypervolume
//!
//! Provides Pareto rank (Fast Non-dominated Sort), the Hypervolume of the Pareto front
//! (interval sum for 2D, WFG for 3D and above), and the HV trajectory in trial order.
//! Maximize objectives have their sign flipped internally, and computation happens in a
//! space unified to minimization.
//!
//! Reference: docs/implements/TASK-201/pareto-requirements.md

mod helpers;
mod hypervolume;
mod ranking;
mod types;

pub(crate) use helpers::{
    add_to_pareto_front, compute_ref_point, dominates_minimized, normalize_objectives,
};
pub use hypervolume::{
    compute_hv_history_from_data, compute_hv_history_with_ref, compute_hypervolume_history,
    hypervolume_2d, hypervolume_nd,
};
pub use ranking::{compute_pareto_ranks, nd_sort};
pub use types::{HvHistoryResult, ParetoResult};

#[cfg(test)]
mod tests;
