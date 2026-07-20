use super::settings::{ClusterSpace, KMeansInitStrategy, KSelectionMode};

#[derive(Debug, Clone)]
pub struct ClusterComputeRequest {
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    /// Upper bound of k explored in Elbow (auto) mode. Ignored in Manual mode.
    pub elbow_max_k: usize,
}

/// Cache key for clustering results.
/// To share results computed with the same settings (target space, k selection
/// mode, k, init strategy), each chart (2D / 3D / Table) looks up
/// `app_state.cluster_cache` with this key.
///
/// In Elbow (auto) mode, k is chosen by the algorithm, so the input k is normalized
/// to 0 and excluded from the key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClusterCacheKey {
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub k: usize,
    pub init_strategy: KMeansInitStrategy,
    pub elbow_max_k: usize,
}

impl ClusterCacheKey {
    pub fn new(
        target_space: ClusterSpace,
        k_mode: KSelectionMode,
        k: usize,
        init_strategy: KMeansInitStrategy,
        elbow_max_k: usize,
    ) -> Self {
        // In Elbow mode the input k is ignored, so normalize it to 0 to keep cache
        // hit checks stable. Symmetrically, elbow_max_k is unused in Manual mode, so
        // normalize it to 0.
        let (k, elbow_max_k) = match k_mode {
            KSelectionMode::Manual => (k, 0),
            KSelectionMode::ElbowDefault => (0, elbow_max_k),
        };
        Self {
            target_space,
            k_mode,
            k,
            init_strategy,
            elbow_max_k,
        }
    }

    pub fn from_request(req: &ClusterComputeRequest) -> Self {
        Self::new(
            req.target_space,
            req.k_mode,
            req.k,
            req.init_strategy,
            req.elbow_max_k,
        )
    }
}

pub fn validate_cluster_request(
    request: &ClusterComputeRequest,
    trial_count: usize,
) -> Result<(), crate::state::messages::ClusterUiError> {
    if trial_count < 2 {
        return Err(crate::state::messages::cluster_ui_error(
            "At least 2 trials are required.",
            Some(format!("validation: trial_count({trial_count}) < 2")),
            false,
        ));
    }

    if matches!(request.k_mode, KSelectionMode::Manual) {
        if request.k < 2 {
            return Err(crate::state::messages::cluster_ui_error(
                "k must be at least 2.",
                Some("validation: k < 2".to_string()),
                true,
            ));
        }
        if request.k > trial_count {
            return Err(crate::state::messages::cluster_ui_error(
                "k must be less than or equal to the number of trials.",
                Some(format!(
                    "validation: k({}) > trial_count({trial_count})",
                    request.k
                )),
                true,
            ));
        }
    }

    Ok(())
}
