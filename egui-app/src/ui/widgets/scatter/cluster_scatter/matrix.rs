use crate::state::types::StudyView;

use super::settings::ClusterSpace;

#[derive(Debug, Clone)]
pub struct ClusterMatrix {
    pub flat_data: Vec<f64>,
    /// Number of rows to cluster (Pareto front), i.e. the row count passed to k-means.
    pub n_rows: usize,
    pub n_cols: usize,
    /// Total number of trials (including solutions outside the clustering target).
    pub total_trials: usize,
    /// Mapping from matrix row index to original trial index (Pareto-front rows).
    pub target_indices: Vec<usize>,
}

impl ClusterMatrix {
    pub fn is_valid_for_clustering(&self) -> bool {
        self.n_rows >= 2 && self.n_cols > 0
    }
}

fn build_cluster_matrix_data(
    view: &StudyView,
    param_names: &[String],
    obj_names: &[String],
    target_space: ClusterSpace,
) -> ClusterMatrix {
    let total_trials = view.row_count();
    let n_cols = target_space.feature_count(param_names.len(), obj_names.len());

    // The clustering target is limited to Pareto-front solutions (pareto_rank == 0).
    // For Studies with constraints, rank 0 is already only feasible non-dominated
    // solutions, so a separate feasibility check isn't needed.
    let target_indices: Vec<usize> = (0..total_trials)
        .filter(|&i| view.pareto_rank.get(i).copied().unwrap_or(u32::MAX) == 0)
        .collect();

    let n_rows = target_indices.len();

    // Build the feature matrix using only Pareto-front solutions
    let flat_data = match target_space {
        ClusterSpace::Objective => {
            let cols = view.numeric_columns(obj_names);
            target_indices
                .iter()
                .flat_map(|&i| {
                    cols.iter()
                        .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                })
                .collect()
        }
        ClusterSpace::Variable => {
            let cols = view.numeric_columns(param_names);
            target_indices
                .iter()
                .flat_map(|&i| {
                    cols.iter()
                        .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                })
                .collect()
        }
        ClusterSpace::Combined => {
            let param_cols = view.numeric_columns(param_names);
            let obj_cols = view.numeric_columns(obj_names);
            target_indices
                .iter()
                .flat_map(|&i| {
                    param_cols
                        .iter()
                        .chain(obj_cols.iter())
                        .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                })
                .collect()
        }
    };

    ClusterMatrix {
        flat_data,
        n_rows,
        n_cols,
        total_trials,
        target_indices,
    }
}

pub fn build_cluster_matrix(
    view: &StudyView,
    param_names: &[String],
    obj_names: &[String],
    target_space: ClusterSpace,
) -> Result<ClusterMatrix, crate::state::messages::ClusterUiError> {
    let matrix = build_cluster_matrix_data(view, param_names, obj_names, target_space);
    if !matrix.is_valid_for_clustering() {
        return Err(crate::state::messages::cluster_ui_error(
            "At least 2 trials and one feature are required.",
            Some(format!(
                "validation: trial_count({}), n_cols({})",
                matrix.n_rows, matrix.n_cols
            )),
            false,
        ));
    }
    Ok(matrix)
}

/// Returns the first two objective-value axes for the scatter plot.
/// If there's only one objective function, the Y axis is fixed at 0.0.
pub(super) fn compute_obj_axes_2d(view: &StudyView, obj_names: &[String]) -> Vec<[f32; 2]> {
    let n = view.row_count();
    let col0 = obj_names.first().and_then(|name| view.numeric_column(name));
    let col1 = obj_names.get(1).and_then(|name| view.numeric_column(name));
    (0..n)
        .map(|i| {
            let x = col0.and_then(|c| c.get(i)).copied().unwrap_or(0.0) as f32;
            let y = col1.and_then(|c| c.get(i)).copied().unwrap_or(0.0) as f32;
            [x, y]
        })
        .collect()
}
