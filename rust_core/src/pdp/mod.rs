//! Module documentation.
//!
//! Module documentation.
//!
//! Design:
//!
//! Module documentation.
//!   - f̄_j(v) = y_mean + β_j * (v - mean_j) / std_j
//!
//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-803

mod api;
mod kriging_core;
mod ridge_core;
mod types;
mod utils;

pub use api::{compute_pdp, compute_pdp_2d};
pub use types::{PdpResult1d, PdpResult2d};

#[cfg(any(test, feature = "wasm"))]
pub(crate) use kriging_core::{compute_pdp_2d_kriging_raw, compute_pdp_2d_sparse_kriging_raw};
#[cfg(test)]
pub(crate) use ridge_core::{compute_pdp_2d_from_matrix, compute_pdp_from_matrix};

#[cfg(test)]
mod tests;
