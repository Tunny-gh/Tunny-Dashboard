use crate::core::math::stats::column_mean_std;

pub(super) fn col_mean_std(data: &[f64]) -> (f64, f64) {
    column_mean_std(data)
}

/// Compute min and max of a slice in a single pass.
///
/// Returns `(min, max)`. Returns `(INFINITY, NEG_INFINITY)` for an empty slice.
pub(super) fn col_min_max(data: &[f64]) -> (f64, f64) {
    data.iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), &v| {
            (mn.min(v), mx.max(v))
        })
}

/// Normalize x matrix using min-max scaling.
///
/// # Returns
/// - col_stats: Vector of (min, range) tuples. range = (max - min).max(EPSILON)
/// - x_norm: Normalized matrix where each value is in [0, 1]
///
/// Constant columns (range == 0) are clamped with EPSILON to prevent NaN.
pub(super) fn normalize_x_minmax(x_matrix: &[Vec<f64>]) -> (Vec<(f64, f64)>, Vec<Vec<f64>>) {
    let n_dims = x_matrix.first().map(|r| r.len()).unwrap_or(0);
    let col_stats: Vec<(f64, f64)> = (0..n_dims)
        .map(|d| {
            let min = x_matrix.iter().map(|r| r[d]).fold(f64::INFINITY, f64::min);
            let max = x_matrix
                .iter()
                .map(|r| r[d])
                .fold(f64::NEG_INFINITY, f64::max);
            (min, (max - min).max(f64::EPSILON))
        })
        .collect();
    let x_norm = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(d, &v)| {
                    let (min, range) = col_stats[d];
                    (v - min) / range
                })
                .collect()
        })
        .collect();
    (col_stats, x_norm)
}

/// Normalize y using z-score normalization (standardization).
///
/// # Returns
/// (y_mean, y_std, y_norm)
/// - y_std minimum value: f64::EPSILON (zero division guard)
pub(super) fn normalize_y(y: &[f64]) -> (f64, f64, Vec<f64>) {
    let n = y.len();
    if n == 0 {
        return (0.0, f64::EPSILON, vec![]);
    }
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let var = y.iter().map(|&v| (v - y_mean).powi(2)).sum::<f64>() / n as f64;
    let y_std = var.sqrt().max(f64::EPSILON);
    let y_norm = y.iter().map(|&v| (v - y_mean) / y_std).collect();
    (y_mean, y_std, y_norm)
}

/// Calculate R² coefficient of determination.
///
/// If ss_tot < EPSILON (constant y), returns 1.0.
pub(super) fn r_squared(y_actual: &[f64], y_pred: &[f64]) -> f64 {
    let n = y_actual.len();
    if n == 0 {
        return 1.0;
    }
    let y_mean = y_actual.iter().sum::<f64>() / n as f64;
    let ss_tot: f64 = y_actual.iter().map(|&v| (v - y_mean).powi(2)).sum();
    if ss_tot < f64::EPSILON {
        return 1.0;
    }
    let ss_res: f64 = y_actual
        .iter()
        .zip(y_pred.iter())
        .map(|(&a, &p)| (a - p).powi(2))
        .sum();
    1.0 - ss_res / ss_tot
}

/// Extract feature matrix and objective variable from DataFrame.
///
/// Missing values (non-existent column or index out of bounds) fallback to 0.0.
pub(super) fn extract_xy(
    df: &crate::dataframe::DataFrame,
    param_names: &[String],
    objective_name: &str,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = df.row_count();
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            param_names
                .iter()
                .map(|p| {
                    df.get_numeric_column(p)
                        .and_then(|c| c.get(i).copied())
                        .unwrap_or(0.0)
                })
                .collect()
        })
        .collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            df.get_numeric_column(objective_name)
                .and_then(|c| c.get(i).copied())
                .unwrap_or(0.0)
        })
        .collect();
    (x_matrix, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc_101_01_normal_data_normalization() {
        // Given: x_matrix with 3 rows and 2 columns
        let x_matrix = vec![vec![0.0, 10.0], vec![0.5, 20.0], vec![1.0, 30.0]];

        // When: normalize_x_minmax is called
        let (col_stats, x_norm) = normalize_x_minmax(&x_matrix);

        // Then: Check column stats
        assert_eq!(col_stats.len(), 2);
        assert!((col_stats[0].0 - 0.0).abs() < 1e-10); // min of col 0
        assert!((col_stats[0].1 - 1.0).abs() < 1e-10); // range of col 0
        assert!((col_stats[1].0 - 10.0).abs() < 1e-10); // min of col 1
        assert!((col_stats[1].1 - 20.0).abs() < 1e-10); // range of col 1

        // Check normalized values
        assert_eq!(x_norm.len(), 3);
        assert!((x_norm[0][0] - 0.0).abs() < 1e-10); // (0.0 - 0.0) / 1.0
        assert!((x_norm[0][1] - 0.0).abs() < 1e-10); // (10.0 - 10.0) / 20.0
        assert!((x_norm[1][0] - 0.5).abs() < 1e-10); // (0.5 - 0.0) / 1.0
        assert!((x_norm[1][1] - 0.5).abs() < 1e-10); // (20.0 - 10.0) / 20.0
        assert!((x_norm[2][0] - 1.0).abs() < 1e-10); // (1.0 - 0.0) / 1.0
        assert!((x_norm[2][1] - 1.0).abs() < 1e-10); // (30.0 - 10.0) / 20.0
    }

    #[test]
    fn tc_101_e01_constant_column_clamping() {
        // Given: x_matrix with constant column
        let x_matrix = vec![vec![5.0], vec![5.0], vec![5.0]];

        // When: normalize_x_minmax is called
        let (col_stats, x_norm) = normalize_x_minmax(&x_matrix);

        // Then: Should not panic, range should be clamped to EPSILON
        assert_eq!(col_stats.len(), 1);
        assert!((col_stats[0].0 - 5.0).abs() < 1e-10); // min
        assert!((col_stats[0].1 - f64::EPSILON).abs() < 1e-15); // range clamped to EPSILON

        // All normalized values should be 0 (since (5.0 - 5.0) / EPSILON = 0 effectively)
        assert_eq!(x_norm.len(), 3);
        assert!(x_norm[0][0].is_finite());
        assert!(x_norm[1][0].is_finite());
        assert!(x_norm[2][0].is_finite());
    }

    #[test]
    fn tc_102_01_y_normalization_accuracy() {
        // Given: y = [1.0, 2.0, 3.0]
        let y = vec![1.0, 2.0, 3.0];

        // When: normalize_y is called
        let (y_mean, y_std, y_norm) = normalize_y(&y);

        // Then: Check y_mean
        assert!((y_mean - 2.0).abs() < 1e-10);

        // Check y_std is positive
        assert!(y_std > 0.0);

        // Check sum of normalized y is approximately 0
        let sum_norm: f64 = y_norm.iter().sum();
        assert!(sum_norm.abs() < 1e-10);

        // Check length
        assert_eq!(y_norm.len(), 3);
    }

    #[test]
    fn tc_102_e01_empty_slice() {
        // Given: empty slice
        let y: Vec<f64> = vec![];

        // When: normalize_y is called
        let (y_mean, y_std, y_norm) = normalize_y(&y);

        // Then: Should return safe defaults without panicking
        assert_eq!(y_mean, 0.0);
        assert_eq!(y_std, f64::EPSILON);
        assert_eq!(y_norm.len(), 0);
    }

    #[test]
    fn tc_201_01_perfect_prediction() {
        // Given: y_actual == y_pred
        let y_actual = vec![1.0, 2.0, 3.0];
        let y_pred = vec![1.0, 2.0, 3.0];

        // When: r_squared is called
        let r2 = r_squared(&y_actual, &y_pred);

        // Then: Should return 1.0 (perfect fit)
        assert!((r2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tc_201_02_constant_prediction() {
        // Given: y_actual = [1.0, 2.0, 3.0], y_pred = [2.0, 2.0, 2.0]
        let y_actual = vec![1.0, 2.0, 3.0];
        let y_pred = vec![2.0, 2.0, 2.0];

        // When: r_squared is called
        let r2 = r_squared(&y_actual, &y_pred);

        // Then: Should return 0.0 (predictions are constant = mean)
        assert!((r2 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn tc_201_e01_constant_y_zero_division_guard() {
        // Given: constant y (all same values)
        let y_actual = vec![5.0, 5.0, 5.0];
        let y_pred = vec![5.0, 5.0, 5.0];

        // When: r_squared is called
        let r2 = r_squared(&y_actual, &y_pred);

        // Then: Should return 1.0 without panicking (zero division guard)
        assert_eq!(r2, 1.0);
    }

    #[test]
    fn tc_301_01_extract_xy_basic() {
        // Given: DataFrame with 2 rows, 2 params, 1 objective
        use crate::dataframe::DataFrame;
        use crate::dataframe::TrialRow;
        use std::collections::HashMap;

        let rows = vec![
            TrialRow {
                trial_id: 0,
                param_display: vec![("x".to_string(), 0.5), ("y".to_string(), 2.0)]
                    .into_iter()
                    .collect(),
                param_category_label: HashMap::new(),
                objective_values: vec![1.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            TrialRow {
                trial_id: 1,
                param_display: vec![("x".to_string(), 1.5), ("y".to_string(), 3.0)]
                    .into_iter()
                    .collect(),
                param_category_label: HashMap::new(),
                objective_values: vec![2.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];

        let df = DataFrame::from_trials(
            &rows,
            &["x".to_string(), "y".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );

        // When: extract_xy is called
        let (x_matrix, y) = extract_xy(&df, &["x".to_string(), "y".to_string()], "obj0");

        // Then: Check extracted values
        assert_eq!(x_matrix.len(), 2);
        assert_eq!(x_matrix[0].len(), 2);
        assert!((x_matrix[0][0] - 0.5).abs() < 1e-9);
        assert!((x_matrix[0][1] - 2.0).abs() < 1e-9);
        assert!((x_matrix[1][0] - 1.5).abs() < 1e-9);
        assert!((x_matrix[1][1] - 3.0).abs() < 1e-9);

        assert_eq!(y.len(), 2);
        assert!((y[0] - 1.0).abs() < 1e-9);
        assert!((y[1] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn tc_301_e01_missing_column_fallback() {
        // Given: DataFrame where requested column doesn't exist
        use crate::dataframe::DataFrame;
        use crate::dataframe::TrialRow;
        use std::collections::HashMap;

        let rows = vec![TrialRow {
            trial_id: 0,
            param_display: vec![("x".to_string(), 0.5)].into_iter().collect(),
            param_category_label: HashMap::new(),
            objective_values: vec![1.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }];

        let df = DataFrame::from_trials(
            &rows,
            &["x".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );

        // When: extract_xy requests non-existent column "z"
        let (x_matrix, _y) = extract_xy(&df, &["x".to_string(), "z".to_string()], "obj0");

        // Then: Should fallback to 0.0 for missing column
        assert_eq!(x_matrix.len(), 1);
        assert_eq!(x_matrix[0].len(), 2);
        assert!((x_matrix[0][0] - 0.5).abs() < 1e-9);
        assert_eq!(x_matrix[0][1], 0.0); // z doesn't exist, so 0.0
    }
}
