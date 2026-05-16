//! Module documentation.
//!
//! Module documentation.
//! Design:
//! Module documentation.
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Module documentation.
//! REQ-081: k-means run_kmeans() — Lloyd's algorithm
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-901

mod kmeans;
mod pca;
mod stats;
mod types;

pub use kmeans::{estimate_k_elbow, run_kmeans};
pub use pca::run_pca;
pub use stats::{
    compute_cluster_centroid_std, compute_cluster_stats, compute_global_stats,
    compute_significant_features,
};
pub use types::{ClusterStat, ElbowResult, InitStrategy, KmeansResult, PcaResult, PcaSpace};

#[cfg(test)]
pub(crate) use kmeans::{estimate_k_elbow_on_data, run_kmeans_on_data};
#[cfg(test)]
pub(crate) use pca::run_pca_on_matrix;
#[cfg(test)]
pub(crate) use stats::compute_cluster_stats_on_data;

#[cfg(test)]
mod tests;
