use crate::core::random_forest;

use super::kriging_core::{compute_pdp_2d_kriging, compute_pdp_2d_sparse_kriging_raw};
use super::ridge_core::{compute_pdp_2d_from_matrix, compute_pdp_from_matrix};
use super::types::{PdpResult1d, PdpResult2d};

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
pub fn compute_pdp(
    param_name: &str,
    objective_name: &str,
    n_grid: usize,
    _n_samples: usize,
) -> Option<PdpResult1d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        let target_idx = param_names.iter().position(|p| p == param_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                param_names
                    .iter()
                    .map(|p| {
                        df.get_numeric_column(p)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        let y: Vec<f64> = (0..n)
            .map(|i| {
                df.get_numeric_column(objective_name)
                    .and_then(|c| c.get(i))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();

        Some(compute_pdp_from_matrix(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_idx,
            n_grid,
        ))
    })
    .flatten()
}

/// Documentation.
///
/// Documentation.
pub fn compute_pdp_2d(
    param1_name: &str,
    param2_name: &str,
    objective_name: &str,
    n_grid: usize,
    model_type: &str,
) -> Option<PdpResult2d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        let p1_idx = param_names.iter().position(|p| p == param1_name)?;
        let p2_idx = param_names.iter().position(|p| p == param2_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                param_names
                    .iter()
                    .map(|p| {
                        df.get_numeric_column(p)
                            .and_then(|c| c.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();
        let y: Vec<f64> = (0..n)
            .map(|i| {
                df.get_numeric_column(objective_name)
                    .and_then(|c| c.get(i))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();

        match model_type {
            "random_forest" => {
                let (grid1, grid2, values, r_squared) =
                    random_forest::compute_pdp_2d_rf(&x_matrix, &y, p1_idx, p2_idx, n_grid)?;
                let p1_name = param_names.get(p1_idx).cloned().unwrap_or_default();
                let p2_name = param_names.get(p2_idx).cloned().unwrap_or_default();
                Some(PdpResult2d {
                    param1_name: p1_name,
                    param2_name: p2_name,
                    objective_name: objective_name.to_string(),
                    grid1,
                    grid2,
                    values,
                    r_squared,
                })
            }
            "kriging" => Some(compute_pdp_2d_kriging(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                p1_idx,
                p2_idx,
                n_grid,
            )),
            "sparse_kriging" => {
                let p1_name = param_names.get(p1_idx).cloned().unwrap_or_default();
                let p2_name = param_names.get(p2_idx).cloned().unwrap_or_default();
                let x_2d: Vec<Vec<f64>> = x_matrix
                    .iter()
                    .map(|row| vec![row[p1_idx], row[p2_idx]])
                    .collect();
                let mut result = compute_pdp_2d_sparse_kriging_raw(&x_2d, &y, n_grid)?;
                result.param1_name = p1_name;
                result.param2_name = p2_name;
                result.objective_name = objective_name.to_string();
                Some(result)
            }
            _ => Some(compute_pdp_2d_from_matrix(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                p1_idx,
                p2_idx,
                n_grid,
            )),
        }
    })
    .flatten()
}
