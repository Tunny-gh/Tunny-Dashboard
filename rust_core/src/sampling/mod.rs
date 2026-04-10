//! Downsampling functions for chart rendering performance.
//!
//! All functions access the active DataFrame via WASM global state.
//!
//! # Usage pattern
//!
//! 1. After `select_study`, call `init_sampling(is_minimize, pareto_indices)` once.
//!    `pareto_indices` should come from the already-computed `compute_pareto_ranks()`
//!    result so that downsampling functions do not pay the O(n²) Pareto cost.
//! 2. Call `downsample_smart` / `downsample_for_thumbnail` etc. as needed.

mod cluster;
mod common;
mod smart;
mod state;
mod stratified;
#[cfg(test)]
mod tests;
mod thumbnail;

pub use cluster::downsample_by_cluster;
pub use common::DownsampleResult;
pub use smart::downsample_smart;
pub use state::{init_sampling, reset_sampling, set_cluster_labels};
pub use stratified::downsample_stratified_by_rank;
pub use thumbnail::downsample_for_thumbnail;
