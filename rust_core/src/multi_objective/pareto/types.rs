/// パレートランク計算の結果。
#[derive(Debug, Clone)]
pub struct ParetoResult {
    /// 各試行のパレートランク（0 = 非支配前面）。行順は DataFrame と同じ。
    pub ranks: Vec<u32>,
    /// ランク 0（パレート前面）に属する試行の行インデックス。
    pub pareto_indices: Vec<u32>,
    /// パレート前面の Hypervolume。目的数 2 未満や前面 2 点未満では `None`。
    pub hypervolume: Option<f64>,
}

/// Hypervolume 推移（試行を追加するごとの HV 値）の計算結果。
#[derive(Debug, Clone)]
pub struct HvHistoryResult {
    /// 各点の trial_id（`hv_values` と同じ順序・要素数）。
    pub trial_ids: Vec<u32>,
    /// 試行順に前面へ点を加えたときの HV 値の推移。
    pub hv_values: Vec<f64>,
    /// HV 計算に使用した参照点（正規化空間。最大化目的は符号反転済み）。
    /// 単目的など HV を計算しない場合は空。
    pub ref_point: Vec<f64>,
}
