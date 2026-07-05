mod hierarchical;
mod kmeans;
mod pca;
mod som;
mod stats;
mod types;

pub use hierarchical::{
    cut_tree, dendrogram_nodes, ward_linkage, DendrogramNode, HierarchicalResult, Merge,
    MAX_HIERARCHICAL_ROWS,
};
pub use kmeans::{estimate_k_elbow, run_kmeans};
pub use pca::{run_pca, run_pca_standardized};
pub use som::{train_som, SomResult, SomSpec};
pub use stats::{compute_cluster_centroid_std, compute_global_stats, compute_significant_features};
pub use types::{ClusterStat, ElbowResult, InitStrategy, KmeansResult, PcaResult, PcaSpace};

#[cfg(test)]
pub(crate) use kmeans::{estimate_k_elbow_on_data, run_kmeans_on_data};
#[cfg(test)]
pub(crate) use pca::run_pca_on_matrix;

#[cfg(test)]
mod tests;
