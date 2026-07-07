use crate::math::stats::{pearson_correlation, spearman_correlation};

/// Correlation coefficient to use when building a [`CorrelationMatrix`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CorrelationMethod {
    Pearson,
    Spearman,
}

/// A symmetric correlation matrix over a set of labeled columns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CorrelationMatrix {
    pub labels: Vec<String>,
    /// `values[i][j]` is the correlation between `labels[i]` and `labels[j]`.
    /// Symmetric, diagonal is always 1.0, cells that cannot be computed are
    /// `f64::NAN`.
    pub values: Vec<Vec<f64>>,
}

/// Compute a pairwise correlation matrix over `columns`.
///
/// `columns` is a list of `(label, values)` pairs; columns of differing
/// length are compared up to the shorter length. For each pair, only rows
/// where both values are finite are used (pairwise complete-case deletion);
/// pairs with fewer than 2 such rows produce `f64::NAN`. Returns `None` if
/// `columns` is empty.
pub fn compute_correlation_matrix(
    columns: &[(String, Vec<f64>)],
    method: CorrelationMethod,
) -> Option<CorrelationMatrix> {
    if columns.is_empty() {
        return None;
    }

    let k = columns.len();
    let labels: Vec<String> = columns.iter().map(|(label, _)| label.clone()).collect();
    let mut values = vec![vec![0.0f64; k]; k];
    for (i, row) in values.iter_mut().enumerate() {
        row[i] = 1.0;
    }

    for i in 0..k {
        for j in (i + 1)..k {
            let r = pairwise_correlation(&columns[i].1, &columns[j].1, method);
            values[i][j] = r;
            values[j][i] = r;
        }
    }

    Some(CorrelationMatrix { labels, values })
}

/// 2 列間の相関係数（pairwise complete-case: 両方有限な行のみ使用、
/// 有効行 2 未満は NaN）。report 生成などからも再利用するため crate 内公開。
pub(crate) fn pairwise_correlation(x: &[f64], y: &[f64], method: CorrelationMethod) -> f64 {
    let n = x.len().min(y.len());
    let (fx, fy): (Vec<f64>, Vec<f64>) = x[..n]
        .iter()
        .zip(&y[..n])
        .filter(|&(&xi, &yi)| xi.is_finite() && yi.is_finite())
        .map(|(&xi, &yi)| (xi, yi))
        .unzip();

    if fx.len() < 2 {
        return f64::NAN;
    }

    match method {
        CorrelationMethod::Pearson => pearson_correlation(&fx, &fy),
        CorrelationMethod::Spearman => spearman_correlation(&fx, &fy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_correlation_pair() {
        let columns = vec![
            ("a".to_string(), vec![1.0, 2.0, 3.0]),
            ("b".to_string(), vec![2.0, 4.0, 6.0]),
        ];
        let matrix = compute_correlation_matrix(&columns, CorrelationMethod::Pearson).unwrap();
        assert_eq!(matrix.labels, vec!["a", "b"]);
        assert!((matrix.values[0][1] - 1.0).abs() < 1e-10);
        assert!((matrix.values[1][0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn pairwise_nan_exclusion() {
        let columns = vec![
            ("a".to_string(), vec![1.0, f64::NAN, 3.0, 4.0]),
            ("b".to_string(), vec![1.0, 2.0, 3.0, 4.0]),
        ];
        let matrix = compute_correlation_matrix(&columns, CorrelationMethod::Pearson).unwrap();
        assert!((matrix.values[0][1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn diagonal_is_always_one() {
        let columns = vec![
            ("a".to_string(), vec![1.0, 1.0, 1.0]),
            ("b".to_string(), vec![1.0, 2.0, 3.0]),
        ];
        let matrix = compute_correlation_matrix(&columns, CorrelationMethod::Pearson).unwrap();
        assert_eq!(matrix.values[0][0], 1.0);
        assert_eq!(matrix.values[1][1], 1.0);
    }

    #[test]
    fn constant_column_gives_nan_off_diagonal() {
        let columns = vec![
            ("a".to_string(), vec![1.0, 1.0, 1.0]),
            ("b".to_string(), vec![1.0, 2.0, 3.0]),
        ];
        let matrix = compute_correlation_matrix(&columns, CorrelationMethod::Pearson).unwrap();
        assert!(matrix.values[0][1].is_nan());
        assert!(matrix.values[1][0].is_nan());
    }

    #[test]
    fn empty_columns_returns_none() {
        assert!(compute_correlation_matrix(&[], CorrelationMethod::Pearson).is_none());
    }

    #[test]
    fn spearman_method_used_when_requested() {
        let columns = vec![
            ("a".to_string(), vec![1.0, 2.0, 3.0, 4.0]),
            ("b".to_string(), vec![10.0, 20.0, 30.0, 5.0]),
        ];
        let matrix = compute_correlation_matrix(&columns, CorrelationMethod::Spearman).unwrap();
        let expected = spearman_correlation(&columns[0].1, &columns[1].1);
        assert!((matrix.values[0][1] - expected).abs() < 1e-10);
    }
}
