//! Downsampling functions for chart rendering performance.
//!
//! # Usage pattern
//!
//! 1. After `select_study`, call `init_sampling(is_minimize, pareto_indices, all_ranks)`.
//!    This returns a `SamplingContext` value; store it in your app state.
//! 2. To enable cluster-based downsampling, set `ctx.cluster_labels = Some(labels)`.
//! 3. Call `downsample_smart(&ctx, ...)` / `downsample_for_thumbnail(&ctx, ...)` etc.

mod cluster;
mod common;
mod context;
mod smart;
mod state;
mod stratified;
#[cfg(test)]
mod tests;
mod thumbnail;

pub use cluster::downsample_by_cluster;
pub use common::DownsampleResult;
pub use context::SamplingContext;
pub use smart::downsample_smart;
pub use state::init_sampling;
pub use stratified::downsample_stratified_by_rank;
pub use thumbnail::downsample_for_thumbnail;
