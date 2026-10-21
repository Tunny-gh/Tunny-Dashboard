use crate::state::types::StudyView;

#[derive(Debug, Clone)]
pub struct ClusterMatrix {
    pub flat_data: Vec<f64>,
    pub n_rows: usize,
    pub n_cols: usize,
    pub total_trials: usize,
    pub target_indices: Vec<usize>,
}

impl ClusterMatrix {
    pub fn is_valid_for_clustering(&self) -> bool {
        self.n_rows >= 2 && self.n_cols > 0
    }
}

#[cfg(test)]
pub fn build_cluster_matrix(
    view: &StudyView,
    parameters: &[String],
    objectives: &[String],
    target: super::settings::ClusterSpace,
) -> Result<ClusterMatrix, crate::state::messages::ClusterUiError> {
    use super::settings::ClusterSpace;
    use tdashboard_analysis::preparation::{cluster_features, FeatureSpace};
    let space = match target {
        ClusterSpace::Objective => FeatureSpace::Objectives,
        ClusterSpace::Variable => FeatureSpace::Parameters,
        ClusterSpace::Combined => FeatureSpace::Both,
    };
    let data = cluster_features(&view.df, parameters, objectives, space, &view.pareto_rank);
    if data.rows.len() < 2 || data.names.is_empty() {
        return Err(crate::state::messages::cluster_ui_error(
            "At least 2 trials and one feature are required.",
            Some(format!(
                "validation: trial_count({}), n_cols({})",
                data.rows.len(),
                data.names.len()
            )),
            false,
        ));
    }
    Ok(ClusterMatrix {
        flat_data: data.matrix.into_iter().flatten().collect(),
        n_rows: data.rows.len(),
        n_cols: data.names.len(),
        total_trials: data.total_rows,
        target_indices: data.rows,
    })
}

/// Display axes are presentation data, not clustering input selection.
pub(super) fn compute_obj_axes_2d(view: &StudyView, obj_names: &[String]) -> Vec<[f32; 2]> {
    let col0 = obj_names.first().and_then(|name| view.numeric_column(name));
    let col1 = obj_names.get(1).and_then(|name| view.numeric_column(name));
    (0..view.row_count())
        .map(|i| {
            [
                col0.and_then(|c| c.get(i)).copied().unwrap_or(0.0) as f32,
                col1.and_then(|c| c.get(i)).copied().unwrap_or(0.0) as f32,
            ]
        })
        .collect()
}
