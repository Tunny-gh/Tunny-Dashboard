/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone)]
pub struct PdpResult1d {
    /// Documentation.
    pub param_name: String,
    /// Documentation.
    pub objective_name: String,
    /// Documentation.
    pub grid: Vec<f64>,
    /// Documentation.
    pub values: Vec<f64>,
    /// Documentation.
    pub r_squared: f64,
}

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdpResult2d {
    /// Documentation.
    pub param1_name: String,
    /// Documentation.
    pub param2_name: String,
    /// Documentation.
    pub objective_name: String,
    /// Documentation.
    pub grid1: Vec<f64>,
    /// Documentation.
    pub grid2: Vec<f64>,
    /// Documentation.
    pub values: Vec<Vec<f64>>,
    /// Documentation.
    pub r_squared: f64,
}
