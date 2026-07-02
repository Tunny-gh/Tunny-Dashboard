/// Pearson correlation coefficient of x and y.
///
/// Returns `f64::NAN` when either slice is empty or when both variances are
/// near zero (constant data). Slice-length mismatch is only checked in debug
/// builds; release builds zip and silently process the shorter length.
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    debug_assert_eq!(x.len(), y.len());
    let n = x.len().min(y.len());
    if n == 0 {
        return f64::NAN;
    }
    let nf = n as f64;
    let mean_x: f64 = x.iter().sum::<f64>() / nf;
    let mean_y: f64 = y.iter().sum::<f64>() / nf;

    let mut cov = 0.0f64;
    let mut var_x = 0.0f64;
    let mut var_y = 0.0f64;
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < f64::EPSILON {
        return f64::NAN;
    }
    cov / denom
}

/// Column mean and standard deviation.
/// The std is the population standard deviation (denominator = n, no Bessel's
/// correction), matching the usual convention for feature standardization
/// (e.g. sklearn's StandardScaler). Cluster dispersion statistics in
/// `clustering::stats` intentionally use the sample std (n-1) instead.
/// - Empty slice: returns (0.0, 1.0)
/// - std < EPSILON: std is fixed to 1.0 (zero-division guard)
pub(crate) fn column_mean_std(vals: &[f64]) -> (f64, f64) {
    let n = vals.len();
    if n == 0 {
        return (0.0, 1.0);
    }
    let nf = n as f64;
    let mean = vals.iter().sum::<f64>() / nf;
    let var = vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf;
    let std_dev = var.sqrt();
    let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
    (mean, std_dev)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- column_mean_std ---

    #[test]
    fn normal_data() {
        let (mean, std) = column_mean_std(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((mean - 3.0).abs() < 1e-10);
        assert!((std - std::f64::consts::SQRT_2).abs() < 0.01);
    }

    #[test]
    fn constant_values() {
        let (mean, std) = column_mean_std(&[5.0, 5.0, 5.0]);
        assert!((mean - 5.0).abs() < 1e-10);
        assert!((std - 1.0).abs() < 1e-10);
    }

    #[test]
    fn empty_slice() {
        let (mean, std) = column_mean_std(&[]);
        assert!((mean - 0.0).abs() < 1e-10);
        assert!((std - 1.0).abs() < 1e-10);
    }

    #[test]
    fn single_element() {
        let (mean, std) = column_mean_std(&[3.0]);
        assert!((mean - 3.0).abs() < 1e-10);
        assert!((std - 1.0).abs() < 1e-10);
    }

    // --- pearson_correlation (TASK-2261) ---

    #[test]
    fn tc_2261_01_pearson_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0];
        let r = pearson_correlation(&x, &y);
        assert!((r - 1.0).abs() < 1e-10, "expected 1.0, got {}", r);
    }

    #[test]
    fn tc_2261_02_pearson_proportional_positive() {
        let x = vec![1.0, -1.0];
        let y = vec![2.0, -2.0];
        let r = pearson_correlation(&x, &y);
        assert!((r - 1.0).abs() < 1e-10, "expected 1.0, got {}", r);
    }

    #[test]
    fn tc_2261_03_pearson_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![3.0, 2.0, 1.0];
        let r = pearson_correlation(&x, &y);
        assert!((r - (-1.0)).abs() < 1e-10, "expected -1.0, got {}", r);
    }

    #[test]
    fn tc_2261_04_pearson_zero_variance_returns_nan() {
        let x = vec![1.0, 1.0, 1.0];
        let y = vec![1.0, 2.0, 3.0];
        let r = pearson_correlation(&x, &y);
        assert!(r.is_nan(), "expected NaN for zero-variance x, got {}", r);
    }

    #[test]
    fn tc_2261_05_pearson_both_zero_variance_returns_nan() {
        let x = vec![2.0, 2.0, 2.0];
        let y = vec![3.0, 3.0, 3.0];
        let r = pearson_correlation(&x, &y);
        assert!(r.is_nan(), "expected NaN for both zero-variance, got {}", r);
    }

    #[test]
    fn tc_2261_06_pearson_empty_returns_nan() {
        let r = pearson_correlation(&[], &[]);
        assert!(r.is_nan(), "expected NaN for empty slices, got {}", r);
    }
}
