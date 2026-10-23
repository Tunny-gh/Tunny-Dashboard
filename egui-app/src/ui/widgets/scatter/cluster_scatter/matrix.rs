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
