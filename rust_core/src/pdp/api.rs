use super::gaussian_process::{compute_pdp_1d_gp_raw, compute_pdp_2d_gp};
use super::ridge::{compute_pdp_2d_from_matrix, compute_pdp_from_matrix};
use super::types::{PdpResult1d, PdpResult2d};
use super::utils::extract_xy;
use crate::gaussian_process::GpMethod;

/// Resolves a `model_type` string into a GP method. Returns `None` for non-GP models.
fn resolve_gp_method(model_type: &str) -> Option<GpMethod> {
    match model_type {
        "gp_fitc" => Some(GpMethod::Fitc),
        "gp_vfe" => Some(GpMethod::Vfe),
        "gp_moe" => Some(GpMethod::Moe),
        _ => None,
    }
}

/// Computes the PDP directly from data extracted beforehand on the main thread.
/// Safe to call from a background thread since it does not use `with_active_df`.
/// `model_type` must be one of "ridge", "gp_fitc", "gp_vfe", "gp_moe", "random_forest".
pub fn compute_pdp_from_data(
    x_matrix: Vec<Vec<f64>>,
    y: Vec<f64>,
    param_names: Vec<String>,
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
    model_type: &str,
) -> PdpResult1d {
    // Ridge (linear) is the common fallback for all models.
    let ridge_fallback = || {
        compute_pdp_from_matrix(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_param_idx,
            n_grid,
        )
    };

    if let Some(method) = resolve_gp_method(model_type) {
        return compute_pdp_1d_gp_raw(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_param_idx,
            n_grid,
            method,
        )
        .unwrap_or_else(ridge_fallback);
    }

    match model_type {
        "random_forest" => {
            let param_name = param_names
                .get(target_param_idx)
                .cloned()
                .unwrap_or_default();
            match crate::lgbm::compute_pdp_1d_lgbm(&x_matrix, &y, target_param_idx, n_grid) {
                Some((grid, values, r_squared)) => PdpResult1d {
                    param_name,
                    objective_name: objective_name.to_string(),
                    grid,
                    values,
                    r_squared,
                    y_upper: None,
                    y_lower: None,
                },
                None => ridge_fallback(),
            }
        }
        _ => ridge_fallback(),
    }
}

pub fn compute_pdp_2d(
    param1_name: &str,
    param2_name: &str,
    objective_name: &str,
    n_grid: usize,
    model_type: &str,
    feasible_only: bool,
) -> Option<PdpResult2d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();

        let p1_idx = param_names.iter().position(|p| p == param1_name)?;
        let p2_idx = param_names.iter().position(|p| p == param2_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        let (x_matrix, y) = extract_xy(df, &param_names, objective_name, feasible_only);

        if let Some(method) = resolve_gp_method(model_type) {
            return Some(compute_pdp_2d_gp(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                p1_idx,
                p2_idx,
                n_grid,
                method,
            ));
        }

        match model_type {
            "random_forest" => {
                let (x_values, y_values, z_values, r_squared) =
                    crate::lgbm::compute_pdp_2d_lgbm(&x_matrix, &y, p1_idx, p2_idx, n_grid)?;
                let p1_name = param_names.get(p1_idx).cloned().unwrap_or_default();
                let p2_name = param_names.get(p2_idx).cloned().unwrap_or_default();
                Some(PdpResult2d {
                    param1_name: p1_name,
                    param2_name: p2_name,
                    objective_name: objective_name.to_string(),
                    x_values,
                    y_values,
                    z_values,
                    r_squared,
                    uncertainties: None,
                })
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
