/// Documentation.
#[derive(Debug, Clone)]
pub struct ParetoResult {
    /// Documentation.
    pub ranks: Vec<u32>,
    /// Documentation.
    pub pareto_indices: Vec<u32>,
    /// Documentation.
    pub hypervolume: Option<f64>,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct HvHistoryResult {
    /// Documentation.
    pub trial_ids: Vec<u32>,
    /// Documentation.
    pub hv_values: Vec<f64>,
}
