/// One-dimensional Partial Dependence Plot (PDP) result for a single
/// parameter / objective pair.
///
/// `values[i]` is the surrogate's predicted mean objective value when the
/// target parameter is fixed at `grid[i]` and all other parameters are
/// marginalized (averaged) over the observed data, following the standard
/// PDP definition (Friedman, 2001).
#[derive(Debug, Clone)]
pub struct PdpResult1d {
    /// Name of the parameter being varied along `grid`.
    pub param_name: String,
    /// Name of the objective whose predicted value is reported in `values`.
    pub objective_name: String,
    /// Grid values for the target parameter axis.
    pub grid: Vec<f64>,
    /// Predicted mean objective value at each grid point (same length/order as `grid`).
    pub values: Vec<f64>,
    /// Goodness of fit (R²) of the underlying surrogate model used to compute `values`.
    pub r_squared: f64,
    /// 95% confidence upper bound (GP methods only).
    pub y_upper: Option<Vec<f64>>,
    /// 95% confidence lower bound (GP methods only).
    pub y_lower: Option<Vec<f64>>,
}

/// Two-dimensional Partial Dependence Plot (PDP) result for a pair of
/// parameters against a single objective.
///
/// `z_values[i][j]` is the surrogate's predicted mean objective value when
/// the two target parameters are fixed at `(x_values[i], y_values[j])` and
/// all remaining parameters are marginalized over the observed data.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdpResult2d {
    /// Name of the parameter varied along `x_values`.
    pub param1_name: String,
    /// Name of the parameter varied along `y_values`.
    pub param2_name: String,
    /// Name of the objective whose predicted value is reported in `z_values`.
    pub objective_name: String,
    /// Grid values for the first parameter axis.
    pub x_values: Vec<f64>,
    /// Grid values for the second parameter axis.
    pub y_values: Vec<f64>,
    /// Predicted mean values on the grid: z_values[i][j] corresponds to (x_values[i], y_values[j]).
    pub z_values: Vec<Vec<f64>>,
    /// Goodness of fit (R²) of the underlying surrogate model used to compute `z_values`.
    pub r_squared: f64,
    /// Posterior variance grid (GP methods only). Same layout as z_values.
    pub uncertainties: Option<Vec<Vec<f64>>>,
}
