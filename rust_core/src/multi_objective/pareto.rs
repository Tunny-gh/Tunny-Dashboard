//! NDSort・Hypervolume
//!
//! パレートランク（Fast Non-dominated Sort）、パレート前面の Hypervolume
//! （2D は区間和、3D 以上は WFG）、および試行順の HV 推移を提供する。
//! 最大化目的は内部で符号反転し、最小化に統一した空間で計算する。
//!
//! Reference: docs/implements/TASK-201/pareto-requirements.md

mod helpers;
mod hypervolume;
mod ranking;
mod types;

pub(crate) use helpers::{add_to_pareto_front, compute_ref_point, normalize_objectives};
pub use hypervolume::{
    compute_hv_history_from_data, compute_hv_history_with_ref, compute_hypervolume_history,
    hypervolume_2d, hypervolume_nd,
};
pub use ranking::{compute_pareto_ranks, nd_sort};
pub use types::{HvHistoryResult, ParetoResult};

#[cfg(test)]
mod tests;
