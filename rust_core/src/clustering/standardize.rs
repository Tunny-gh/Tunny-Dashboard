//! Shared column z-score standardization helper for the clustering module.
//!
//! Consolidates the standardization logic that was duplicated across
//! hierarchical / som / pca. Since the variance's degrees-of-freedom
//! correction differs (hierarchical and som use population variance n, pca
//! uses sample variance n-1), it is parameterized via `ddof`.

/// Standardizes each column in place to mean 0, variance 1.
///
/// - `ddof`: Degrees-of-freedom correction for variance. 0 for population variance (denominator n), 1 for sample variance (denominator n-1).
/// - A column whose standard deviation is <= 1e-12 (effectively zero variance) has all its elements mapped to 0.
/// - Returns `(column means, column standard deviations)`. The standard
///   deviation is the raw post-correction value (not rounded to 0 even for a
///   zero-variance column; the inverse-transform side must apply the same threshold check).
///
/// Precondition: all rows must have the same length (validated by the caller).
pub(super) fn standardize_columns(x: &mut [Vec<f64>], ddof: usize) -> (Vec<f64>, Vec<f64>) {
    let n = x.len();
    if n == 0 || x[0].is_empty() {
        return (Vec::new(), Vec::new());
    }
    let p = x[0].len();
    let denom = n.saturating_sub(ddof).max(1) as f64;
    let mut means = vec![0.0f64; p];
    let mut stds = vec![0.0f64; p];
    for j in 0..p {
        let mean = x.iter().map(|r| r[j]).sum::<f64>() / n as f64;
        let var = x.iter().map(|r| (r[j] - mean).powi(2)).sum::<f64>() / denom;
        let std = var.sqrt();
        means[j] = mean;
        stds[j] = std;
        for row in x.iter_mut() {
            row[j] = if std > 1e-12 {
                (row[j] - mean) / std
            } else {
                0.0
            };
        }
    }
    (means, stds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn population_variance_standardizes_to_unit() {
        let mut x = vec![vec![1.0], vec![2.0], vec![3.0]];
        let (means, stds) = standardize_columns(&mut x, 0);
        assert!((means[0] - 2.0).abs() < 1e-12);
        // Population variance = 2/3 -> std = sqrt(2/3)
        assert!((stds[0] - (2.0f64 / 3.0).sqrt()).abs() < 1e-12);
        let mean_after: f64 = x.iter().map(|r| r[0]).sum::<f64>() / 3.0;
        let var_after: f64 = x.iter().map(|r| (r[0] - mean_after).powi(2)).sum::<f64>() / 3.0;
        assert!(mean_after.abs() < 1e-12);
        assert!((var_after - 1.0).abs() < 1e-12);
    }

    #[test]
    fn ddof_one_uses_sample_variance() {
        let mut x = vec![vec![1.0], vec![2.0], vec![3.0]];
        let (_, stds) = standardize_columns(&mut x, 1);
        // Sample variance = 1.0 -> std = 1.0
        assert!((stds[0] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn zero_variance_column_maps_to_zero() {
        let mut x = vec![vec![5.0, 1.0], vec![5.0, 2.0]];
        standardize_columns(&mut x, 0);
        assert_eq!(x[0][0], 0.0);
        assert_eq!(x[1][0], 0.0);
        assert!(x[0][1] != 0.0);
    }

    #[test]
    fn empty_input_is_noop() {
        let mut x: Vec<Vec<f64>> = vec![];
        let (means, stds) = standardize_columns(&mut x, 0);
        assert!(means.is_empty());
        assert!(stds.is_empty());
    }
}
