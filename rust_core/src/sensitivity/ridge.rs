use crate::core::math::stats::column_mean_std;
use crate::dataframe::DataFrame;
use super::data::get_param_numeric_values;
use super::metric_trait::SensitivityMetric;
use super::types::{RidgeResult, SensitivityResult};

fn transpose_and_standardize(x_matrix: &[Vec<f64>], n: usize, p: usize) -> Vec<f64> {
    let mut x_cols = vec![0.0f64; n * p];

    for (i, row) in x_matrix.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            x_cols[j * n + i] = v;
        }
    }

    for j in 0..p {
        let col = &mut x_cols[j * n..(j + 1) * n];
        let (mean, std_dev) = column_mean_std(col);
        for v in col.iter_mut() {
            *v = (*v - mean) / std_dev;
        }
    }

    x_cols
}

pub(super) fn gaussian_elimination(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let p = b.len();
    if p == 0 {
        return Some(vec![]);
    }

    for col in 0..p {
        let pivot_row = (col..p)
            .max_by(|&i, &j| {
                a[i][col]
                    .abs()
                    .partial_cmp(&a[j][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);

        a.swap(col, pivot_row);
        b.swap(col, pivot_row);

        let pivot = a[col][col];
        if pivot.abs() < 1e-12 {
            return None;
        }

        for row in (col + 1)..p {
            let factor = a[row][col] / pivot;
            #[allow(clippy::needless_range_loop)]
            for k in col..p {
                let v = a[col][k] * factor;
                a[row][k] -= v;
            }
            b[row] -= b[col] * factor;
        }
    }

    let mut x = vec![0.0f64; p];
    for i in (0..p).rev() {
        let mut sum = b[i];
        for j in (i + 1)..p {
            sum -= a[i][j] * x[j];
        }
        x[i] = sum / a[i][i];
    }

    Some(x)
}

pub(super) fn compute_ridge_from_standardized_columns(
    x_cols: &[f64],
    n: usize,
    y: &[f64],
    alpha: f64,
) -> RidgeResult {
    let num_params = if n == 0 { 0 } else { x_cols.len() / n };
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_c: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();

    let xtx_flat = compute_xtx_matrix(x_cols, n, num_params, alpha);
    let xty = compute_xty_vector(x_cols, &y_c, n, num_params);

    let xtx_2d: Vec<Vec<f64>> = (0..num_params)
        .map(|i| xtx_flat[i * num_params..(i + 1) * num_params].to_vec())
        .collect();
    let beta = gaussian_elimination(xtx_2d, xty).unwrap_or_else(|| vec![0.0; num_params]);

    let r_squared = compute_r_squared(x_cols, &y_c, &beta, n);
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
            .map(|col| col[..n].to_vec())
            .unwrap_or_else(|| vec![0.0; n]);

        let num_params = param_names.len();
        let mut x_cols_flat = vec![0.0f64; n * num_params];
        for (j, param_name) in param_names.iter().enumerate() {
            if let Some(col) = get_param_numeric_values(df, param_name, n) {
                for (i, &value) in col.iter().enumerate().take(n) {
                    x_cols_flat[j * n + i] = value;
                }
            }
        }
        for j in 0..num_params {
            let col_slice = &mut x_cols_flat[j * n..(j + 1) * n];
            let (mean, std_dev) = column_mean_std(col_slice);
            for value in col_slice.iter_mut() {
                *value = (*value - mean) / std_dev;
            }
        }

        let ridge = vec![compute_ridge_from_standardized_columns(&x_cols_flat, n, &y, 1.0)];

        Some(SensitivityResult {
            param_names,
            objective_names: vec![objective_name],
            spearman: vec![],
            ridge,
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
        })
    }

    fn name(&self) -> &'static str {
        "Ridge"
    }
}

/// X'X 行列を計算する（column-major flat 配列から、Ridge 正則化項は含まない）。
/// 返り値は `num_params × num_params` の行優先フラット配列。
pub(crate) fn compute_xtx_matrix(x_cols: &[f64], n: usize, num_params: usize, alpha: f64) -> Vec<f64> {
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
pub(crate) fn compute_xty_vector(x_cols: &[f64], y_c: &[f64], n: usize, num_params: usize) -> Vec<f64> {
    (0..num_params)
        .map(|j| {
            let col = &x_cols[j * n..(j + 1) * n];
            col.iter().zip(y_c.iter()).map(|(x, yy)| x * yy).sum()
        })
        .collect()
}

/// R² (決定係数) を計算する。ss_tot < EPSILON の場合は 0.0 を返す。
pub(crate) fn compute_r_squared(x_cols: &[f64], y_c: &[f64], beta: &[f64], n: usize) -> f64 {
    let num_params = beta.len();
    let ss_tot: f64 = y_c.iter().map(|&yi| yi * yi).sum();
    if ss_tot < f64::EPSILON {
        return 0.0;
    }
    let ss_res: f64 = (0..n)
        .map(|i| {
            let y_hat: f64 = (0..num_params).map(|j| x_cols[j * n + i] * beta[j]).sum();
            (y_c[i] - y_hat).powi(2)
        })
        .sum();
    (1.0 - ss_res / ss_tot).max(0.0)
}

pub fn compute_ridge(x_matrix: &[Vec<f64>], y: &[f64], alpha: f64) -> RidgeResult {
    let n = y.len();
    let empty = RidgeResult {
        beta: vec![],
        r_squared: 0.0,
    };

    if n < 2 || x_matrix.len() != n {
        return empty;
    }
    let p = x_matrix[0].len();
    if p == 0 {
        return empty;
    }

    let x_cols = transpose_and_standardize(x_matrix, n, p);
    compute_ridge_from_standardized_columns(&x_cols, n, y, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_x_cols() -> (Vec<f64>, usize, usize) {
        // 4 rows × 2 params, column-major standardized:
        // col0: [-1, -0.333, 0.333, 1], col1: [1, 0.333, -0.333, -1] (approx)
        let x = vec![-1.5, -0.5, 0.5, 1.5,  1.5, 0.5, -0.5, -1.5];
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
        let x = vec![1.0, 1.0, 1.0];
        let y_c = vec![0.0, 0.0, 0.0]; // ss_tot = 0
        let beta = vec![0.0];
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
        let result = compute_ridge(&x_matrix, &y, 1.0);
        assert!(!result.beta.is_empty());
        assert!(result.r_squared >= 0.0 && result.r_squared <= 1.0);
    }
}
