use super::kriging::{
    compute_pdp_1d_kriging_raw, compute_pdp_1d_sparse_kriging_raw, compute_pdp_2d_kriging,
    compute_pdp_2d_sparse_kriging,
};
use super::ridge::{compute_pdp_2d_from_matrix, compute_pdp_from_matrix};
use super::types::{PdpResult1d, PdpResult2d};
use super::utils::extract_xy;

/// メインスレッド側で事前に抽出したデータを直接受け取って PDP を計算する。
/// `with_active_df` を使わないため、バックグラウンドスレッドから安全に呼べる。
/// `model_type` には "ridge", "kriging", "sparse_kriging" のいずれかを指定する。
pub fn compute_pdp_from_data(
    x_matrix: Vec<Vec<f64>>,
    y: Vec<f64>,
    param_names: Vec<String>,
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
    model_type: &str,
) -> PdpResult1d {
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
                None => compute_pdp_from_matrix(
                    &x_matrix,
                    &y,
                    &param_names,
                    objective_name,
                    target_param_idx,
                    n_grid,
                ),
            }
        }
        "kriging" => compute_pdp_1d_kriging_raw(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_param_idx,
            n_grid,
        )
        .unwrap_or_else(|| {
            compute_pdp_from_matrix(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                target_param_idx,
                n_grid,
            )
        }),
        "sparse_kriging" => compute_pdp_1d_sparse_kriging_raw(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_param_idx,
            n_grid,
        )
        .unwrap_or_else(|| {
            compute_pdp_from_matrix(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                target_param_idx,
                n_grid,
            )
        }),
        _ => compute_pdp_from_matrix(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            target_param_idx,
            n_grid,
        ),
    }
}

pub fn compute_pdp(
    param_name: &str,
    objective_name: &str,
    n_grid: usize,
    _n_samples: usize,
) -> Option<PdpResult1d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();

        let target_idx = param_names.iter().position(|p| p == param_name)?;
        let _ = objective_names.iter().position(|o| o == objective_name)?;

        let (x_matrix, y) = extract_xy(df, &param_names, objective_name, false);

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
            "kriging" => Some(compute_pdp_2d_kriging(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                p1_idx,
                p2_idx,
                n_grid,
            )),
            "sparse_kriging" => Some(compute_pdp_2d_sparse_kriging(
                &x_matrix,
                &y,
                &param_names,
                objective_name,
                p1_idx,
                p2_idx,
                n_grid,
            )),
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

/// Compute a 2D response surface from raw data without using the thread-local dataframe.
/// Suitable for calling from background threads.
/// `model_type` accepts "ridge" (default), "kriging", "sparse_kriging".
#[allow(clippy::too_many_arguments)]
pub fn compute_surface_from_data(
    x_matrix: Vec<Vec<f64>>,
    y: Vec<f64>,
    param_names: Vec<String>,
    objective_name: &str,
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
    model_type: &str,
) -> PdpResult2d {
    match model_type {
        "kriging" => compute_pdp_2d_kriging(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            param1_idx,
            param2_idx,
            n_grid,
        ),
        "sparse_kriging" => compute_pdp_2d_sparse_kriging(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            param1_idx,
            param2_idx,
            n_grid,
        ),
        _ => compute_pdp_2d_from_matrix(
            &x_matrix,
            &y,
            &param_names,
            objective_name,
            param1_idx,
            param2_idx,
            n_grid,
        ),
    }
}
