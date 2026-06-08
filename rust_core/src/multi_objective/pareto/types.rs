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
    /// HV 計算に使用した参照点（正規化空間。最大化目的は符号反転済み）。
    /// 単目的など HV を計算しない場合は空。
    pub ref_point: Vec<f64>,
}
