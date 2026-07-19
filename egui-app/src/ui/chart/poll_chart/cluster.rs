use crate::state::messages::{AppMessage, ClusterChartSource};
use crate::ui::widgets::cluster_scatter::{
    ClusterCacheKey, ClusterComputeRequest, ClusterMatrix, KSelectionMode,
};

pub(super) fn run_cluster_compute(
    source: ClusterChartSource,
    req: ClusterComputeRequest,
    matrix: ClusterMatrix,
) -> AppMessage {
    let key = ClusterCacheKey::from_request(&req);
    let trial_count = matrix.n_rows; // Number of Pareto-front solutions (rows passed to k-means)
    let n_cols = matrix.n_cols;

    if !matrix.is_valid_for_clustering() {
        return cluster_failed(
            source,
            "At least 2 trials and one feature are required.",
            Some(format!(
                "validation: trial_count({trial_count}), n_cols({n_cols})"
            )),
            false,
        );
    }

    let init_strategy: tunny_core::clustering::InitStrategy = req.init_strategy.into();
    let selected_k = match req.k_mode {
        KSelectionMode::ElbowDefault => {
            let elbow = tunny_core::clustering::estimate_k_elbow(
                &matrix.flat_data,
                n_cols,
                trial_count.min(req.elbow_max_k.clamp(2, 50)),
            );
            elbow.recommended_k.clamp(2, trial_count)
        }
        KSelectionMode::Manual => req.k,
    };

    if selected_k < 2 || selected_k > trial_count {
        return cluster_failed(
            source,
            "k must be in [2, trial_count].",
            Some(format!(
                "validation: k({selected_k}) outside [2, {trial_count}]"
            )),
            true,
        );
    }

    let result =
        tunny_core::clustering::run_kmeans(selected_k, &matrix.flat_data, n_cols, init_strategy);
    if result.labels.len() != trial_count {
        return cluster_failed(
            source,
            "Cluster result is inconsistent. Please run again.",
            Some(format!(
                "validation: labels_len({}) != trial_count({trial_count})",
                result.labels.len()
            )),
            true,
        );
    }

    // Expand Pareto-front labels to cover all trials (solutions not included get -1)
    let mut full_labels = vec![-1i32; matrix.total_trials];
    for (matrix_row, &trial_idx) in matrix.target_indices.iter().enumerate() {
        if let Some(&label) = result.labels.get(matrix_row) {
            full_labels[trial_idx] = label as i32;
        }
    }

    AppMessage::ClusteringDone {
        source,
        key,
        result: crate::state::results::ClusterResult {
            labels: full_labels,
            n_clusters: selected_k,
        },
    }
}

fn cluster_failed(
    source: ClusterChartSource,
    message: &str,
    detail: Option<String>,
    retryable: bool,
) -> AppMessage {
    AppMessage::ClusterFailed {
        source,
        err: crate::state::messages::cluster_ui_error(message, detail, retryable),
    }
}
