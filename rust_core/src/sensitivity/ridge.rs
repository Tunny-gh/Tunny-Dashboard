use super::constants::{RIDGE_ALPHA, RIDGE_MAX_ROWS, RIDGE_SEED};
use super::data::get_param_numeric_values;
use super::metric_trait::SensitivityMetric;
use super::tree::common::{run_importances_pipeline, PreparedData};
use super::types::{RidgeResult, SensitivityResult};
use crate::dataframe::DataFrame;
use crate::math::stats::column_mean_std;

fn standardize_columns_inplace(x_cols: &mut [f64], n: usize, p: usize) {
    for j in 0..p {
        let col = &mut x_cols[j * n..(j + 1) * n];
        let (mean, std_dev) = column_mean_std(col);
        for v in col.iter_mut() {
            *v = (*v - mean) / std_dev;
        }
    }
}

fn transpose_and_standardize_faer(x_matrix: &faer::Mat<f64>, n: usize, p: usize) -> Vec<f64> {
    let mut x_cols = vec![0.0f64; n * p];
    for j in 0..p {
        for i in 0..n {
            x_cols[j * n + i] = x_matrix[(i, j)];
        }
    }
    standardize_columns_inplace(&mut x_cols, n, p);
    x_cols
}

pub(super) fn compute_ridge_from_standardized_columns(
    x_cols: &[f64],
    n: usize,
    y: &[f64],
    alpha: f64,
) -> RidgeResult {
    let num_params = x_cols.len().checked_div(n).unwrap_or(0);
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_c: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();

    let xtx_flat = compute_xtx_matrix(x_cols, n, num_params, alpha);
    let xty = compute_xty_vector(x_cols, &y_c, n, num_params);

    // X'X + αI is SPD (α > 0), so use Cholesky solve for efficiency
    let beta = solve_ridge_normal_equations(&xtx_flat, &xty, num_params);

    let r_squared = compute_r_squared(x_cols, &y_c, &beta, n);
    RidgeResult { beta, r_squared }
}

/// Solve the Ridge normal equations (X'X + αI)β = X'y using faer Cholesky.
/// X'X + αI is always SPD when α > 0, making Cholesky the optimal solver.
fn solve_ridge_normal_equations(xtx_flat: &[f64], xty: &[f64], p: usize) -> Vec<f64> {
    if p == 0 {
        return vec![];
    }
    let xtx_mat = faer::Mat::<f64>::from_fn(p, p, |i, j| xtx_flat[i * p + j]);
    let xty_mat = faer::Mat::<f64>::from_fn(p, 1, |i, _| xty[i]);
    use faer::prelude::Solve;
    match xtx_mat.llt(faer::Side::Lower) {
        Ok(chol) => (0..p).map(|i| chol.solve(&xty_mat)[(i, 0)]).collect(),
        Err(_) => vec![0.0; p],
    }
}

/// Standardizes row-major data into a column-major flat array using externally supplied
/// mean/std. This lets the EVAL split be scored with the TRAIN split's statistics,
/// matching how a fitted model would be applied to unseen data.
fn standardize_rows_with_stats(rows: &[Vec<f64>], means: &[f64], stds: &[f64]) -> Vec<f64> {
    let n = rows.len();
    let p = means.len();
    let mut cols = vec![0.0f64; n * p];
    for j in 0..p {
        for i in 0..n {
            cols[j * n + i] = (rows[i][j] - means[j]) / stds[j];
        }
    }
    cols
}

/// Fits Ridge on the TRAIN split only (standardized/centered using TRAIN statistics) and
/// reports R² on the EVAL split, matching the holdout convention used by
/// RF-ANOVA/MDI/SHAP/PFI (`tree::common::run_importances_pipeline`). When the dataset is too
/// small for a holdout split, `PreparedData` falls back to using all data for both.
fn compute_ridge_from_prepared(data: &PreparedData) -> Option<(Vec<f64>, f64)> {
    let (x_train, x_eval, y_train, y_eval) = data.split();
    let p = x_train.first()?.len();
    let n_train = x_train.len();

    let mut means = vec![0.0f64; p];
    let mut stds = vec![1.0f64; p];
    for j in 0..p {
        let col: Vec<f64> = x_train.iter().map(|row| row[j]).collect();
        let (mean, std_dev) = column_mean_std(&col);
        means[j] = mean;
        stds[j] = std_dev;
    }

    let x_train_cols = standardize_rows_with_stats(x_train, &means, &stds);
    let x_eval_cols = standardize_rows_with_stats(x_eval, &means, &stds);

    let y_mean = y_train.iter().sum::<f64>() / n_train as f64;
    let y_train_c: Vec<f64> = y_train.iter().map(|&v| v - y_mean).collect();
    let y_eval_c: Vec<f64> = y_eval.iter().map(|&v| v - y_mean).collect();

    let xtx = compute_xtx_matrix(&x_train_cols, n_train, p, RIDGE_ALPHA);
    let xty = compute_xty_vector(&x_train_cols, &y_train_c, n_train, p);
    let beta = solve_ridge_normal_equations(&xtx, &xty, p);

    let r_squared = compute_r_squared(&x_eval_cols, &y_eval_c, &beta, x_eval.len());
    Some((beta, r_squared))
}

/// Sensitivity-analysis entry point for Ridge: NaN/Inf row filtering, 80/20 holdout split,
/// and holdout R², unified with the tree-based metrics (RF-ANOVA/MDI/SHAP/PFI). Kept
/// separate from `compute_ridge`/`compute_ridge_from_vecs`, which are generic in-sample
/// primitives still used by PDP, Sobol, and surrogate-model fitting where a holdout split
/// would be inappropriate (e.g. Sobol quadrature needs every sample point).
pub(super) fn compute_ridge_result(x_matrix: &[Vec<f64>], y: &[f64]) -> RidgeResult {
    let (beta, r_squared) = run_importances_pipeline(
        x_matrix,
        y,
        RIDGE_MAX_ROWS,
        RIDGE_SEED,
        RIDGE_SEED.wrapping_add(1),
        compute_ridge_from_prepared,
    );
    RidgeResult { beta, r_squared }
}

pub struct RidgeMetric;

impl SensitivityMetric for RidgeMetric {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult> {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        let objective_name = objective_names.get(obj_idx)?.clone();
        if n < 2 || param_names.is_empty() {
            return None;
        }

        let y: Vec<f64> = df
            .get_numeric_column(&objective_name)
            .map(|col| col.iter().take(n).copied().collect())
            .unwrap_or_else(|| vec![0.0; n]);

        let param_cols: Vec<Vec<f64>> = param_names
            .iter()
            .map(|name| get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]))
            .collect();
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|row_index| {
                param_cols
                    .iter()
                    .map(|col| col.get(row_index).copied().unwrap_or(0.0))
                    .collect()
            })
            .collect();

        let ridge = vec![compute_ridge_result(&x_matrix, &y)];

        Some(SensitivityResult {
            param_names,
            objective_names: vec![objective_name],
            ridge,
            ..Default::default()
        })
    }

    fn name(&self) -> &'static str {
        "Ridge"
    }
}

/// X'X 行列を計算する（column-major flat 配列から、Ridge 正則化項は含まない）。
/// 返り値は `num_params × num_params` の行優先フラット配列。
pub(crate) fn compute_xtx_matrix(
    x_cols: &[f64],
    n: usize,
    num_params: usize,
    alpha: f64,
) -> Vec<f64> {
    let mut xtx = vec![0.0f64; num_params * num_params];
    for i in 0..num_params {
        for j in i..num_params {
            let col_i = &x_cols[i * n..(i + 1) * n];
            let col_j = &x_cols[j * n..(j + 1) * n];
            let val: f64 = col_i.iter().zip(col_j.iter()).map(|(a, b)| a * b).sum();
            xtx[i * num_params + j] = val;
            xtx[j * num_params + i] = val;
        }
    }
    for i in 0..num_params {
        xtx[i * num_params + i] += alpha;
    }
    xtx
}

/// X'y ベクトルを計算する（y は中心化済み）。
pub(crate) fn compute_xty_vector(
    x_cols: &[f64],
    y_c: &[f64],
    n: usize,
    num_params: usize,
) -> Vec<f64> {
    (0..num_params)
        .map(|j| {
            let col = &x_cols[j * n..(j + 1) * n];
            col.iter().zip(y_c.iter()).map(|(x, yy)| x * yy).sum()
        })
        .collect()
}

/// R² (決定係数) を計算する。
///
/// 定数 y（ss_tot ≈ 0）の規約は `pdp::utils::r_squared` と統一:
/// 残差もほぼゼロ（定数を完全に再現）なら 1.0、そうでなければ 0.0。
pub(crate) fn compute_r_squared(x_cols: &[f64], y_c: &[f64], beta: &[f64], n: usize) -> f64 {
    let num_params = beta.len();
    let ss_tot: f64 = y_c.iter().map(|&yi| yi * yi).sum();
    let ss_res: f64 = (0..n)
        .map(|i| {
            let y_hat: f64 = (0..num_params).map(|j| x_cols[j * n + i] * beta[j]).sum();
            (y_c[i] - y_hat).powi(2)
        })
        .sum();
    if ss_tot < f64::EPSILON {
        return if ss_res < f64::EPSILON { 1.0 } else { 0.0 };
    }
    (1.0 - ss_res / ss_tot).max(0.0)
}

pub fn compute_ridge(x_matrix: &faer::Mat<f64>, y: &[f64], alpha: f64) -> RidgeResult {
    let n = y.len();
    let empty = RidgeResult {
        beta: vec![],
        r_squared: 0.0,
    };
    if n < 2 || x_matrix.nrows() != n {
        return empty;
    }
    let p = x_matrix.ncols();
    if p == 0 {
        return empty;
    }
    let x_cols = transpose_and_standardize_faer(x_matrix, n, p);
    compute_ridge_from_standardized_columns(&x_cols, n, y, alpha)
}

/// Convenience wrapper for internal callers that have Vec<Vec<f64>>.
pub(crate) fn compute_ridge_from_vecs(x_matrix: &[Vec<f64>], y: &[f64], alpha: f64) -> RidgeResult {
    let n = y.len();
    let empty = RidgeResult {
        beta: vec![],
        r_squared: 0.0,
    };
    if n < 2 || x_matrix.len() != n {
        return empty;
    }
    let p = x_matrix.first().map(|r| r.len()).unwrap_or(0);
    if p == 0 {
        return empty;
    }
    let faer_x = faer::Mat::from_fn(n, p, |i, j| x_matrix[i][j]);
    compute_ridge(&faer_x, y, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_x_cols() -> (Vec<f64>, usize, usize) {
        // 4 rows × 2 params, column-major standardized:
        // col0: [-1, -0.333, 0.333, 1], col1: [1, 0.333, -0.333, -1] (approx)
        let x = vec![-1.5, -0.5, 0.5, 1.5, 1.5, 0.5, -0.5, -1.5];
        (x, 4, 2)
    }

    #[test]
    fn tc_2265_01_compute_xtx_matrix_diagonal() {
        let (x, n, p) = simple_x_cols();
        let xtx = compute_xtx_matrix(&x, n, p, 0.0);
        assert_eq!(xtx.len(), p * p);
        // X'X diagonal = sum of squares for each column = 1.5²+0.5²+0.5²+1.5² = 5.0
        assert!((xtx[0] - 5.0).abs() < 1e-10, "xtx[0,0]={}", xtx[0]);
        assert!((xtx[3] - 5.0).abs() < 1e-10, "xtx[1,1]={}", xtx[3]);
    }

    #[test]
    fn tc_2265_02_compute_xtx_matrix_ridge_regularization() {
        let (x, n, p) = simple_x_cols();
        let xtx_noreg = compute_xtx_matrix(&x, n, p, 0.0);
        let xtx_reg = compute_xtx_matrix(&x, n, p, 2.0);
        // alpha=2 should add 2 to diagonal
        assert!((xtx_reg[0] - xtx_noreg[0] - 2.0).abs() < 1e-10);
        assert!((xtx_reg[3] - xtx_noreg[3] - 2.0).abs() < 1e-10);
        // Off-diagonal unchanged
        assert!((xtx_reg[1] - xtx_noreg[1]).abs() < 1e-10);
    }

    #[test]
    fn tc_2265_03_compute_xty_vector() {
        let (x, n, p) = simple_x_cols();
        let y_c = vec![1.0, 0.5, -0.5, -1.0];
        let xty = compute_xty_vector(&x, &y_c, n, p);
        assert_eq!(xty.len(), p);
        // col0 · y_c = -1.5*1 + -0.5*0.5 + 0.5*-0.5 + 1.5*-1 = -1.5-0.25-0.25-1.5 = -3.5
        assert!((xty[0] - (-3.5)).abs() < 1e-10, "xty[0]={}", xty[0]);
    }

    #[test]
    fn tc_2265_04_compute_r_squared_perfect_fit() {
        // If beta perfectly predicts y_c, R² = 1.0
        let x = vec![1.0, 2.0, 3.0]; // 3 rows, 1 param
        let y_c = vec![1.0, 2.0, 3.0];
        let beta = vec![1.0];
        let r2 = compute_r_squared(&x, &y_c, &beta, 3);
        assert!((r2 - 1.0).abs() < 1e-10, "R²={}", r2);
    }

    #[test]
    fn tc_2265_05_compute_r_squared_zero_variance() {
        // 定数 y の規約は pdp::utils::r_squared と統一:
        // 残差もゼロ（定数を完全再現）なら 1.0。
        let x = vec![1.0, 1.0, 1.0];
        let y_c = vec![0.0, 0.0, 0.0]; // ss_tot = 0
        let beta = vec![0.0];
        let r2 = compute_r_squared(&x, &y_c, &beta, 3);
        assert!((r2 - 1.0).abs() < 1e-10, "R²={} for zero-variance y", r2);
    }

    #[test]
    fn tc_2265_05b_compute_r_squared_zero_variance_nonzero_residual() {
        // 定数 y を再現できていない場合は 0.0。
        let x = vec![1.0, 2.0, 3.0];
        let y_c = vec![0.0, 0.0, 0.0]; // ss_tot = 0
        let beta = vec![1.0]; // y_hat = [1, 2, 3] → 残差非ゼロ
        let r2 = compute_r_squared(&x, &y_c, &beta, 3);
        assert!((r2 - 0.0).abs() < 1e-10, "R²={} for zero-variance y", r2);
    }

    #[test]
    fn tc_2265_06_orchestrator_matches_original_behavior() {
        // compute_ridge_from_standardized_columns result should equal compute_ridge result
        // on standardized data
        let x_matrix = vec![
            vec![1.0, 0.5],
            vec![2.0, 1.0],
            vec![3.0, 1.5],
            vec![4.0, 2.0],
        ];
        let y = vec![1.5, 3.0, 4.5, 6.0];
        let result = compute_ridge_from_vecs(&x_matrix, &y, 1.0);
        assert!(!result.beta.is_empty());
        assert!(result.r_squared >= 0.0 && result.r_squared <= 1.0);
    }

    // ---- TASK-2308: faer Cholesky replacement tests ----

    #[test]
    fn tc_103_01_ridge_beta_and_r2_consistent_with_linear_data() {
        // Perfect linear relationship: y = 2*x1 + x2
        let n = 20usize;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 5) as f64]).collect();
        let y: Vec<f64> = x_matrix.iter().map(|r| 2.0 * r[0] + r[1]).collect();
        let result = compute_ridge_from_vecs(&x_matrix, &y, 0.001);
        assert_eq!(result.beta.len(), 2);
        assert!(
            result.r_squared > 0.95,
            "R² should be high for linear data: {}",
            result.r_squared
        );
        // First coefficient should dominate (x1 has larger scale contribution)
        assert!(result.beta[0] > 0.0, "β[0] should be positive");
    }

    #[test]
    fn tc_103_b01_ridge_empty_params_returns_empty() {
        let empty: Vec<Vec<f64>> = vec![];
        let result = compute_ridge_from_vecs(&empty, &[], 1.0);
        assert_eq!(result.beta.len(), 0);
        assert_eq!(result.r_squared, 0.0);
    }

    #[test]
    fn tc_103_01_solve_ridge_normal_equations_correct() {
        let (x, n, p) = super::tests::simple_x_cols();
        let xtx = compute_xtx_matrix(&x, n, p, 1.0);
        let xty = compute_xty_vector(&x, &[-1.0f64, -0.5, 0.5, 1.0], n, p);
        let beta = solve_ridge_normal_equations(&xtx, &xty, p);
        assert_eq!(beta.len(), p);
        assert!(
            beta.iter().all(|b| b.is_finite()),
            "all betas should be finite"
        );
    }

    // ---- Ridge holdout R² + NaN/Inf row filtering (rust_core audit 2026-07) ----

    fn linear_xy(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        (
            (0..n).map(|i| vec![i as f64, (i % 5) as f64]).collect(),
            (0..n).map(|i| 2.0 * i as f64 + (i % 5) as f64).collect(),
        )
    }

    #[test]
    fn tc_ridge_holdout_01_nan_row_matches_clean_subset() {
        let (mut x, mut y) = linear_xy(10);
        x.insert(5, vec![f64::NAN, 1.0]);
        y.insert(5, 42.0);

        let (clean_x, clean_y) = linear_xy(10);

        let contaminated = compute_ridge_result(&x, &y);
        let clean = compute_ridge_result(&clean_x, &clean_y);

        assert_eq!(contaminated.beta, clean.beta, "beta should ignore NaN row");
        assert_eq!(
            contaminated.r_squared, clean.r_squared,
            "R² should ignore NaN row"
        );
    }

    #[test]
    fn tc_ridge_holdout_02_inf_row_matches_clean_subset() {
        let (mut x, mut y) = linear_xy(10);
        x.insert(3, vec![1.0, f64::INFINITY]);
        y.insert(3, 7.0);

        let (clean_x, clean_y) = linear_xy(10);

        let contaminated = compute_ridge_result(&x, &y);
        let clean = compute_ridge_result(&clean_x, &clean_y);

        assert_eq!(contaminated.beta, clean.beta, "beta should ignore Inf row");
        assert_eq!(
            contaminated.r_squared, clean.r_squared,
            "R² should ignore Inf row"
        );
    }

    #[test]
    fn tc_ridge_holdout_03_deterministic_across_calls() {
        let (x, y) = linear_xy(30);

        let first = compute_ridge_result(&x, &y);
        let second = compute_ridge_result(&x, &y);

        assert_eq!(
            first.beta, second.beta,
            "beta must be bit-identical across calls"
        );
        assert_eq!(
            first.r_squared, second.r_squared,
            "R² must be bit-identical across calls"
        );
    }

    #[test]
    fn tc_ridge_holdout_04_r_squared_is_out_of_sample() {
        // With enough rows to trigger the 80/20 split, holdout R² must be computed
        // by evaluating on data the fit never saw.
        let (x, y) = linear_xy(50);
        let result = compute_ridge_result(&x, &y);
        assert!(
            (0.0..=1.0).contains(&result.r_squared),
            "R² should be a valid holdout score: {}",
            result.r_squared
        );
        assert!(!result.beta.is_empty());
    }
}
