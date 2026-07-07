use super::boxplot::quantile;

/// Number of bins in a histogram.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinRule {
    /// `ceil(log2(n)) + 1`
    Sturges,
    /// Bin width `h = 3.49 * sigma * n^(-1/3)` (sigma: sample std, n-1).
    Scott,
    /// Bin width `h = 2 * IQR * n^(-1/3)`.
    FreedmanDiaconis,
    /// Explicit bin count (0 is treated as 1).
    Manual(usize),
}

/// Histogram bin edges and counts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Histogram {
    /// Bin boundaries, ascending, `len() == counts.len() + 1`.
    pub bin_edges: Vec<f64>,
    pub counts: Vec<usize>,
    /// Number of finite values that went into the histogram.
    pub n: usize,
}

const MIN_BINS: usize = 1;
const MAX_BINS: usize = 200;

/// Compute a histogram over `values` using the given bin rule.
///
/// Non-finite values (NaN/Inf) are excluded before computing. Returns `None`
/// if no finite values remain. Constant data (min == max) collapses to a
/// single bin. Bin counts derived from width-based rules ([`BinRule::Scott`],
/// [`BinRule::FreedmanDiaconis`]) fall back to [`BinRule::Sturges`] when the
/// computed width is non-positive or non-finite. The final bin includes its
/// right edge (matches `numpy.histogram`, which is half-open except for the
/// last bin). Bin count is always clamped to `1..=200`.
pub fn compute_histogram(values: &[f64], rule: BinRule) -> Option<Histogram> {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let n = finite.len();
    if n == 0 {
        return None;
    }

    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    if min == max {
        return Some(Histogram {
            bin_edges: vec![min, min],
            counts: vec![n],
            n,
        });
    }

    let bins = bin_count(&finite, min, max, rule).clamp(MIN_BINS, MAX_BINS);

    let bin_edges: Vec<f64> = (0..=bins)
        .map(|i| min + (max - min) * (i as f64) / (bins as f64))
        .collect();

    let bin_width = (max - min) / bins as f64;
    let mut counts = vec![0usize; bins];
    for &v in &finite {
        let mut idx = ((v - min) / bin_width) as usize;
        if idx >= bins {
            idx = bins - 1;
        }
        counts[idx] += 1;
    }

    Some(Histogram {
        bin_edges,
        counts,
        n,
    })
}

fn bin_count(finite: &[f64], min: f64, max: f64, rule: BinRule) -> usize {
    let n = finite.len();
    match rule {
        BinRule::Sturges => sturges_bins(n),
        BinRule::Manual(k) => {
            if k == 0 {
                1
            } else {
                k
            }
        }
        BinRule::Scott => {
            let sigma = sample_std(finite);
            let h = 3.49 * sigma * (n as f64).powf(-1.0 / 3.0);
            width_based_bins(min, max, h).unwrap_or_else(|| sturges_bins(n))
        }
        BinRule::FreedmanDiaconis => {
            let mut sorted = finite.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let iqr = quantile(&sorted, 0.75) - quantile(&sorted, 0.25);
            let h = 2.0 * iqr * (n as f64).powf(-1.0 / 3.0);
            width_based_bins(min, max, h).unwrap_or_else(|| sturges_bins(n))
        }
    }
}

/// Sturges の公式によるビン数 `ceil(log2(n)) + 1`（最小 1）。
///
/// `report::builder` がビン数の決定（独自上限つき）に再利用するため
/// crate 内公開（以前は report 側に同一実装が重複していた）。
pub(crate) fn sturges_bins(n: usize) -> usize {
    (((n as f64).log2().ceil() as i64) + 1).max(1) as usize
}

fn width_based_bins(min: f64, max: f64, h: f64) -> Option<usize> {
    if !h.is_finite() || h <= 0.0 {
        return None;
    }
    let bins = ((max - min) / h).ceil();
    if !bins.is_finite() || bins < 1.0 {
        return None;
    }
    Some(bins as usize)
}

fn sample_std(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let nf = n as f64;
    let mean = values.iter().sum::<f64>() / nf;
    let var = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (nf - 1.0);
    var.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sturges_bin_count() {
        // n = 8 -> ceil(log2(8)) + 1 = 3 + 1 = 4
        let values: Vec<f64> = (0..8).map(|v| v as f64).collect();
        let hist = compute_histogram(&values, BinRule::Sturges).unwrap();
        assert_eq!(hist.counts.len(), 4);
        assert_eq!(hist.n, 8);
        assert_eq!(hist.counts.iter().sum::<usize>(), 8);
    }

    #[test]
    fn constant_data_single_bin() {
        let values = vec![5.0, 5.0, 5.0];
        let hist = compute_histogram(&values, BinRule::Sturges).unwrap();
        assert_eq!(hist.bin_edges, vec![5.0, 5.0]);
        assert_eq!(hist.counts, vec![3]);
        assert_eq!(hist.n, 3);
    }

    #[test]
    fn excludes_non_finite_values() {
        let values = vec![1.0, 2.0, f64::NAN, f64::INFINITY, 3.0];
        let hist = compute_histogram(&values, BinRule::Manual(3)).unwrap();
        assert_eq!(hist.n, 3);
        assert_eq!(hist.counts.iter().sum::<usize>(), 3);
    }

    #[test]
    fn last_bin_is_right_closed() {
        let values = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let hist = compute_histogram(&values, BinRule::Manual(4)).unwrap();
        // edges: 0,1,2,3,4 -> value 4.0 (== max) should land in the last bin.
        assert_eq!(hist.counts.iter().sum::<usize>(), 5);
        assert_eq!(*hist.counts.last().unwrap(), 2); // values 3.0 and 4.0
    }

    #[test]
    fn freedman_diaconis_falls_back_to_sturges_when_iqr_zero() {
        // Many repeated values collapse IQR to 0, so FD must fall back.
        let values = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 10.0];
        let fd = compute_histogram(&values, BinRule::FreedmanDiaconis).unwrap();
        let sturges = compute_histogram(&values, BinRule::Sturges).unwrap();
        assert_eq!(fd.counts.len(), sturges.counts.len());
    }

    #[test]
    fn empty_or_all_non_finite_returns_none() {
        assert!(compute_histogram(&[], BinRule::Sturges).is_none());
        assert!(compute_histogram(&[f64::NAN, f64::INFINITY], BinRule::Sturges).is_none());
    }

    #[test]
    fn manual_zero_treated_as_one() {
        let values = vec![1.0, 2.0, 3.0];
        let hist = compute_histogram(&values, BinRule::Manual(0)).unwrap();
        assert_eq!(hist.counts.len(), 1);
    }

    #[test]
    fn bin_count_clamped_to_200() {
        let values = vec![1.0, 2.0];
        let hist = compute_histogram(&values, BinRule::Manual(10_000)).unwrap();
        assert_eq!(hist.counts.len(), 200);
    }
}
