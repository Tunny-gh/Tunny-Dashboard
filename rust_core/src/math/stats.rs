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

/// Rank values with average ranks for ties.
///
/// NaN values are sorted to the end and receive the average of the trailing
/// ranks (they are treated as a single tied group). Ties among finite values
/// receive the average of the ranks they span.
pub fn rank(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }

    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| {
        let va = values[a];
        let vb = values[b];
        match (va.is_nan(), vb.is_nan()) {
            (true, _) => std::cmp::Ordering::Greater,
            (_, true) => std::cmp::Ordering::Less,
            _ => va.partial_cmp(&vb).unwrap(),
        }
    });

    let mut ranks = vec![0.0f64; n];
    let mut i = 0;

    while i < n {
        let val = values[indices[i]];
        if val.is_nan() {
            let avg = (i as f64 + 1.0 + n as f64) / 2.0;
            for k in i..n {
                ranks[indices[k]] = avg;
            }
            break;
        }

        let mut j = i + 1;
        while j < n && values[indices[j]] == val {
            j += 1;
        }

        let avg_rank = (i as f64 + 1.0 + j as f64) / 2.0;
        for k in i..j {
            ranks[indices[k]] = avg_rank;
        }
        i = j;
    }

    ranks
}

/// Spearman rank correlation coefficient of x and y.
///
/// Computed as the Pearson correlation of the ranks of x and y (via [`rank`]).
/// Slice-length mismatch is only checked in debug builds, matching
/// [`pearson_correlation`].
pub fn spearman_correlation(x: &[f64], y: &[f64]) -> f64 {
    debug_assert_eq!(x.len(), y.len());
    let n = x.len().min(y.len());
    let rx = rank(&x[..n]);
    let ry = rank(&y[..n]);
    pearson_correlation(&rx, &ry)
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
