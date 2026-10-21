/// Five-number summary plus Tukey fences, used to render a box plot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoxPlotStats {
    pub n: usize,
    pub mean: f64,
    pub min: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub max: f64,
    /// Smallest data point within the lower Tukey fence (`q1 - 1.5*IQR`).
    pub whisker_low: f64,
    /// Largest data point within the upper Tukey fence (`q3 + 1.5*IQR`).
    pub whisker_high: f64,
    /// Data points outside the Tukey fences, ascending.
    pub outliers: Vec<f64>,
}

/// Compute box plot statistics (Tukey fences, 1.5*IQR).
///
/// Non-finite values (NaN/Inf) are excluded before computing. Returns `None`
/// if no finite values remain. A single finite value produces a degenerate
/// box plot where every statistic equals that value and there are no
/// outliers.
pub fn compute_boxplot(values: &[f64]) -> Option<BoxPlotStats> {
    let mut finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let n = finite.len();
    if n == 0 {
        return None;
    }
    if n == 1 {
        let v = finite[0];
        return Some(BoxPlotStats {
            n: 1,
            mean: v,
            min: v,
            q1: v,
            median: v,
            q3: v,
            max: v,
            whisker_low: v,
            whisker_high: v,
            outliers: vec![],
        });
    }

    finite.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = finite.iter().sum::<f64>() / n as f64;
    let min = finite[0];
    let max = finite[n - 1];
    let q1 = quantile(&finite, 0.25);
    let median = quantile(&finite, 0.5);
    let q3 = quantile(&finite, 0.75);
    let iqr = q3 - q1;
    let low_fence = q1 - 1.5 * iqr;
    let high_fence = q3 + 1.5 * iqr;

    let whisker_low = finite
        .iter()
        .copied()
        .filter(|&v| v >= low_fence && v <= high_fence)
        .fold(f64::INFINITY, f64::min);
    let whisker_high = finite
        .iter()
        .copied()
        .filter(|&v| v >= low_fence && v <= high_fence)
        .fold(f64::NEG_INFINITY, f64::max);
    // Defensive fallback: min/median/max always satisfy min <= q1 <= median
    // <= q3 <= max, so the fenced range is never empty in practice.
    let whisker_low = if whisker_low.is_finite() {
        whisker_low
    } else {
        min
    };
    let whisker_high = if whisker_high.is_finite() {
        whisker_high
    } else {
        max
    };

    let outliers: Vec<f64> = finite
        .iter()
        .copied()
        .filter(|&v| v < low_fence || v > high_fence)
        .collect();

    Some(BoxPlotStats {
        n,
        mean,
        min,
        q1,
        median,
        q3,
        max,
        whisker_low,
        whisker_high,
        outliers,
    })
}

/// Quantile via linear interpolation (numpy's default "linear"/type-7 method).
///
/// `sorted` must be ascending. Returns `f64::NAN` for an empty slice.
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    if n == 1 {
        return sorted[0];
    }

    let h = (n - 1) as f64 * q;
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_quartiles_one_to_nine() {
        let values: Vec<f64> = (1..=9).map(|v| v as f64).collect();
        let stats = compute_boxplot(&values).unwrap();
        assert_eq!(stats.n, 9);
        assert!((stats.q1 - 3.0).abs() < 1e-10);
        assert!((stats.median - 5.0).abs() < 1e-10);
        assert!((stats.q3 - 7.0).abs() < 1e-10);
        assert!(stats.outliers.is_empty());
        assert!((stats.whisker_low - 1.0).abs() < 1e-10);
        assert!((stats.whisker_high - 9.0).abs() < 1e-10);
    }

    #[test]
    fn detects_outliers() {
        let mut values: Vec<f64> = (1..=9).map(|v| v as f64).collect();
        values.push(100.0);
        let stats = compute_boxplot(&values).unwrap();
        assert_eq!(stats.outliers, vec![100.0]);
        assert!((stats.whisker_high - 9.0).abs() < 1e-10);
    }

    #[test]
    fn single_element() {
        let stats = compute_boxplot(&[3.0]).unwrap();
        assert_eq!(stats.n, 1);
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.min, 3.0);
        assert_eq!(stats.q1, 3.0);
        assert_eq!(stats.median, 3.0);
        assert_eq!(stats.q3, 3.0);
        assert_eq!(stats.max, 3.0);
        assert!(stats.outliers.is_empty());
    }

    #[test]
    fn empty_input_returns_none() {
        assert!(compute_boxplot(&[]).is_none());
        assert!(compute_boxplot(&[f64::NAN, f64::INFINITY]).is_none());
    }

    #[test]
    fn quantile_interpolates() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0];
        assert!((quantile(&sorted, 0.5) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn quantile_empty_is_nan() {
        assert!(quantile(&[], 0.5).is_nan());
    }
}
