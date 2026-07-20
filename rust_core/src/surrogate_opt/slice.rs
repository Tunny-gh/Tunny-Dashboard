//! Response-surface slicing: 2D grid slices through a point (for the 3D
//! response-surface viewer) and 1D line slices along one parameter (for the
//! surrogate comparison view).

use super::models;
use super::types::{SurfaceSlice, TrainedSurrogate};
use crate::math::grid::linspace;

/// Evaluates a response-surface slice of a fitted surrogate (for the 3D
/// response-surface viewer).
///
/// Passes through `anchor_orig` (original units, same order as `param_names`)
/// and evaluates an `n_grid` x `n_grid` grid over the full declared range in
/// the `param_x_idx` x `param_y_idx` plane. Unlike a PDP, it does not marginalize
/// the other parameters — it returns a "raw cross-section" with them fixed at
/// the anchor point.
pub fn surface_slice_at(
    trained: &TrainedSurrogate,
    anchor_orig: &[f64],
    param_x_idx: usize,
    param_y_idx: usize,
    n_grid: usize,
) -> Option<SurfaceSlice> {
    let surrogate = &trained.surrogate;
    let n_dims = surrogate.col_stats.len();
    if anchor_orig.len() != n_dims {
        return None;
    }
    let anchor_norm = surrogate.to_norm_x(anchor_orig);
    build_slice(
        surrogate,
        &anchor_norm,
        param_x_idx,
        param_y_idx,
        n_grid.max(2),
        n_dims,
    )
}

/// Row index of the best observed value (minimum when minimizing, maximum when
/// maximizing).
pub(crate) fn best_observed_index(y: &[f64], minimize: bool) -> usize {
    let mut best = 0usize;
    for (i, &v) in y.iter().enumerate() {
        let better = if minimize { v < y[best] } else { v > y[best] };
        if better {
            best = i;
        }
    }
    best
}

/// Evaluates a 2D slice grid through the optimum `t_best` (normalized space)
/// using the surrogate.
pub(crate) fn build_slice(
    surrogate: &models::FittedSurrogate,
    t_best: &[f64],
    param_x_idx: usize,
    param_y_idx: usize,
    n_grid: usize,
    n_dims: usize,
) -> Option<SurfaceSlice> {
    if param_x_idx >= n_dims || param_y_idx >= n_dims || param_x_idx == param_y_idx {
        return None;
    }
    let (min_x, range_x) = surrogate.col_stats[param_x_idx];
    let (min_y, range_y) = surrogate.col_stats[param_y_idx];
    let x_values = linspace(min_x, min_x + range_x, n_grid);
    let y_values = linspace(min_y, min_y + range_y, n_grid);

    // Evaluate the mean (original units) at each grid point, plus the
    // original-unit standard deviation derived from the posterior variance
    // where available. z_std holds Some only when the model has a posterior
    // variance (GP family).
    let mut z_values: Vec<Vec<f64>> = Vec::with_capacity(x_values.len());
    let mut z_std_grid: Vec<Vec<f64>> = Vec::with_capacity(x_values.len());
    let mut has_std = true;
    for &vx in &x_values {
        let mut z_row = Vec::with_capacity(y_values.len());
        let mut std_row = Vec::with_capacity(y_values.len());
        for &vy in &y_values {
            let mut pt = t_best.to_vec();
            pt[param_x_idx] = (vx - min_x) / range_x;
            pt[param_y_idx] = (vy - min_y) / range_y;
            z_row.push(surrogate.to_original_y(surrogate.predict_norm(&pt)));
            match surrogate.predict_var_norm(&pt) {
                // Normalized-space variance -> original-unit standard deviation
                // (scaled by y_std).
                Some(var) => std_row.push(var.max(0.0).sqrt() * surrogate.y_std),
                None => has_std = false,
            }
        }
        z_values.push(z_row);
        z_std_grid.push(std_row);
    }
    let z_std = has_std.then_some(z_std_grid);

    Some(SurfaceSlice {
        param_x_idx,
        param_y_idx,
        x_values,
        y_values,
        z_values,
        z_std,
    })
}

/// A predicted slice along one parameter direction, through the anchor point
/// (for the surrogate comparison view).
#[derive(Debug, Clone)]
pub struct LineSlice {
    /// Column index of the parameter being sliced.
    pub param_idx: usize,
    /// Grid values (original units).
    pub x_values: Vec<f64>,
    /// Predicted values (original units).
    pub y_values: Vec<f64>,
    /// Predicted standard deviation (original units). Some only for models with
    /// a posterior variance (GP family).
    pub y_std: Option<Vec<f64>>,
}

/// Evaluates a 1D predicted slice through the anchor point (original units)
/// using the surrogate.
///
/// Fixes every dimension other than `param_idx` at the anchor value, and
/// evaluates `param_idx` over its declared range (falling back to the observed
/// range) at `n_grid` points (minimum 2). Returns `None` on a dimension
/// mismatch or out-of-range index.
pub fn line_slice_at(
    trained: &TrainedSurrogate,
    anchor_orig: &[f64],
    param_idx: usize,
    n_grid: usize,
) -> Option<LineSlice> {
    let surrogate = &trained.surrogate;
    let n_dims = surrogate.col_stats.len();
    if anchor_orig.len() != n_dims || param_idx >= n_dims {
        return None;
    }
    let anchor_norm = surrogate.to_norm_x(anchor_orig);
    let (min_x, range_x) = surrogate.col_stats[param_idx];
    let x_values = linspace(min_x, min_x + range_x, n_grid.max(2));

    let mut y_values = Vec::with_capacity(x_values.len());
    let mut std_values = Vec::with_capacity(x_values.len());
    let mut has_std = true;
    for &vx in &x_values {
        let mut pt = anchor_norm.clone();
        pt[param_idx] = (vx - min_x) / range_x;
        y_values.push(surrogate.to_original_y(surrogate.predict_norm(&pt)));
        match surrogate.predict_var_norm(&pt) {
            // Normalized-space variance -> original-unit standard deviation
            // (scaled by y_std).
            Some(var) => std_values.push(var.max(0.0).sqrt() * surrogate.y_std),
            None => has_std = false,
        }
    }

    Some(LineSlice {
        param_idx,
        x_values,
        y_values,
        y_std: has_std.then_some(std_values),
    })
}
