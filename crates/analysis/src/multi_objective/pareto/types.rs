/// Result of the Pareto rank computation.
#[derive(Debug, Clone)]
pub struct ParetoResult {
    /// Pareto rank of each trial (0 = non-dominated front). Row order matches the DataFrame.
    pub ranks: Vec<u32>,
    /// Row indices of trials belonging to rank 0 (the Pareto front).
    pub pareto_indices: Vec<u32>,
    /// Hypervolume of the Pareto front. `None` when fewer than 2 objectives or fewer than
    /// 2 points on the front.
    pub hypervolume: Option<f64>,
}

/// Result of computing the Hypervolume trajectory (the HV value as each trial is added).
#[derive(Debug, Clone)]
pub struct HvHistoryResult {
    /// trial_id of each point (same order and length as `hv_values`).
    pub trial_ids: Vec<u32>,
    /// Trajectory of HV values as points are added to the front in trial order.
    pub hv_values: Vec<f64>,
    /// Reference point used for the HV computation (normalized space; maximize objectives
    /// already have their sign flipped). Empty when HV is not computed, e.g. single-objective.
    pub ref_point: Vec<f64>,
}
