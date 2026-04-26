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
    /// 95% confidence upper bound (Kriging/Sparse Kriging only).
    pub y_upper: Option<Vec<f64>>,
    /// 95% confidence lower bound (Kriging/Sparse Kriging only).
    pub y_lower: Option<Vec<f64>>,
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
    /// Grid values for the first parameter axis.
    pub x_values: Vec<f64>,
    /// Grid values for the second parameter axis.
    pub y_values: Vec<f64>,
    /// Predicted mean values on the grid: z_values[i][j] corresponds to (x_values[i], y_values[j]).
    pub z_values: Vec<Vec<f64>>,
    /// Documentation.
    pub r_squared: f64,
    /// Posterior variance grid (Kriging / Sparse Kriging only). Same layout as z_values.
    pub uncertainties: Option<Vec<Vec<f64>>>,
}
