#[derive(Debug, Clone)]
pub struct SamplingContext {
    pub is_minimize: Vec<bool>,
    pub pareto_indices: Option<Vec<u32>>,
    pub all_ranks: Option<Vec<u32>>,
    pub cluster_labels: Option<Vec<i32>>,
}

impl SamplingContext {
    pub(crate) fn get_pareto_rank0_indices(&self) -> Vec<u32> {
        if let Some(ref indices) = self.pareto_indices {
            return indices.clone();
        }
        let n_obj =
            crate::dataframe::with_active_df(|df| df.objective_col_names().len()).unwrap_or(1);
        let is_min = if self.is_minimize.is_empty() {
            vec![true; n_obj]
        } else {
            self.is_minimize.clone()
        };
        crate::pareto::compute_pareto_ranks(&is_min).pareto_indices
    }

    pub(crate) fn get_all_ranks(&self) -> Vec<u32> {
        if let Some(ref ranks) = self.all_ranks {
            return ranks.clone();
        }
        let n_obj =
            crate::dataframe::with_active_df(|df| df.objective_col_names().len()).unwrap_or(1);
        let is_min = if self.is_minimize.is_empty() {
            vec![true; n_obj]
        } else {
            self.is_minimize.clone()
        };
        crate::pareto::compute_pareto_ranks(&is_min).ranks
    }
}
