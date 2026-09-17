use super::boxplot::quantile;

/// Upper bound on the number of samples used for the KDE. Larger inputs are
/// deterministically subsampled (uniformly spaced indices) so that the O(n *
/// grid) density evaluation stays bounded. `ViolinCurve::n` reports the number
/// of samples actually used after subsampling.
const MAX_KDE_SAMPLES: usize = 50_000;

/// A Gaussian kernel density estimate plus the summary needed to draw one violin.
#[derive(Debug, Clone)]
pub struct ViolinCurve {
    /// Number of finite samples used for the KDE (after the subsample cap).
    pub n: usize,
    /// Median of the (sorted) samples used for the KDE.
    pub median: f64,
    /// Smallest finite sample value.
    pub data_min: f64,
    /// Largest finite sample value.
    pub data_max: f64,
    /// Evenly spaced evaluation grid (ascending).
    pub grid: Vec<f64>,
    /// Kernel density at each `grid` point (same length as `grid`).
    pub density: Vec<f64>,
    /// Bandwidth `h` used for the kernels (a robust variant of Scott's rule).
    pub bandwidth: f64,
}

/// Compute a Gaussian kernel density estimate over `values`.
///
/// Non-finite values (NaN/Inf) are excluded. Returns `None` if fewer than two
/// finite values remain, or if the bandwidth is not finite/positive (i.e. every
/// value identical).
///
/// The bandwidth is a robust variant of Scott's rule:
/// `h = 1.06 * spread * n^(-1/5)`, using the population standard deviation and
/// the sample IQR (via [`quantile`]) with `spread = min(sd, IQR/1.34)`. The
/// classic normal-reference Scott bandwidth uses `sd` alone; taking the minimum
/// with the IQR term makes the estimate robust to skewed or heavy-tailed
/// samples. When the robust term is zero but `sd > 0` (a non-constant sample
/// whose type-7 IQR collapses to 0, e.g. `[1,1,1,1,2]`), `spread` falls back to
/// `sd` so such samples are still rendered.
///
/// The grid spans `[data_min - 2h, data_max + 2h]` with `max(grid_points, 2)`
/// evenly spaced points (both ends inclusive). The density at a grid point `g`
/// is `1/(n*h) * sum_j exp(-0.5*((g - x_j)/h)^2) / sqrt(2*pi)`.
///
/// To bound cost, inputs with more than 50,000 finite values are deterministically
/// subsampled to 50,000 (uniformly spaced indices) before the bandwidth and
/// density are computed; `ViolinCurve::n` reflects the number actually used.
pub fn compute_violin(values: &[f64], grid_points: usize) -> Option<ViolinCurve> {
    let mut finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.len() < 2 {
        return None;
    }

    // Deterministic uniform subsample to cap the KDE cost.
    if finite.len() > MAX_KDE_SAMPLES {
        let n_orig = finite.len();
        finite = (0..MAX_KDE_SAMPLES)
            .map(|i| finite[i * n_orig / MAX_KDE_SAMPLES])
            .collect();
    }

    let n = finite.len();
    finite.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let data_min = finite[0];
    let data_max = finite[n - 1];
    let nf = n as f64;
    let mean = finite.iter().sum::<f64>() / nf;
    // Population standard deviation (Scott's rule uses the population estimate).
    let sd = (finite.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf).sqrt();
    let iqr = quantile(&finite, 0.75) - quantile(&finite, 0.25);

    let spread = if iqr > 0.0 { sd.min(iqr / 1.34) } else { sd };
    let h = 1.06 * spread * nf.powf(-1.0 / 5.0);
    if !h.is_finite() || h <= 0.0 {
        return None;
    }

    let grid_points = grid_points.max(2);
    let lo = data_min - 2.0 * h;
    let hi = data_max + 2.0 * h;
    let step = (hi - lo) / (grid_points - 1) as f64;
    let grid: Vec<f64> = (0..grid_points).map(|i| lo + step * i as f64).collect();

    let norm = 1.0 / (nf * h * (2.0 * std::f64::consts::PI).sqrt());
    let density: Vec<f64> = grid
        .iter()
        .map(|&g| {
            finite
                .iter()
                .map(|&x| (-0.5 * ((g - x) / h).powi(2)).exp())
                .sum::<f64>()
                * norm
        })
        .collect();

    Some(ViolinCurve {
        n,
        median: quantile(&finite, 0.5),
        data_min,
        data_max,
        grid,
        density,
        bandwidth: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fewer_than_two_finite_values_returns_none() {
        assert!(compute_violin(&[], 128).is_none());
        assert!(compute_violin(&[1.0], 128).is_none());
        assert!(compute_violin(&[f64::NAN, f64::INFINITY], 128).is_none());
    }

    #[test]
    fn constant_input_returns_none() {
        assert!(compute_violin(&[2.0, 2.0, 2.0], 128).is_none());
    }

    #[test]
    fn non_finite_values_are_filtered() {
        let values = [1.0, f64::NAN, 2.0, f64::INFINITY, 3.0, f64::NEG_INFINITY];
        let curve = compute_violin(&values, 32).unwrap();
        assert_eq!(curve.n, 3);
        assert!((curve.data_min - 1.0).abs() < 1e-12);
        assert!((curve.data_max - 3.0).abs() < 1e-12);
    }

    #[test]
    fn grid_and_range_are_reported() {
        let values = [0.0, 1.0, 2.0, 3.0, 4.0];
        let curve = compute_violin(&values, 128).unwrap();
        assert_eq!(curve.n, 5);
        assert_eq!(curve.grid.len(), 128);
        assert_eq!(curve.density.len(), 128);
        assert!((curve.data_min - 0.0).abs() < 1e-12);
        assert!((curve.data_max - 4.0).abs() < 1e-12);
        // The grid extends 2h beyond the data at both ends.
        assert!(curve.grid[0] < curve.data_min);
        assert!(*curve.grid.last().unwrap() > curve.data_max);
        // The median of 0..=4 is 2.
        assert!((curve.median - 2.0).abs() < 1e-12);
    }

    #[test]
    fn grid_points_is_clamped_to_two() {
        let curve = compute_violin(&[0.0, 1.0], 0).unwrap();
        assert_eq!(curve.grid.len(), 2);
        assert_eq!(curve.density.len(), 2);
    }

    #[test]
    fn density_is_finite_and_non_negative() {
        let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let curve = compute_violin(&values, 128).unwrap();
        assert!(curve.density.iter().all(|d| d.is_finite() && *d >= 0.0));
    }

    #[test]
    fn density_integrates_to_approximately_one() {
        // The KDE integrates to 1 over the full line. The grid is truncated at 2h
        // beyond the data extremes, so a small tail outside it is lost; with a
        // smooth sample the rectangular sum is close to 1.
        let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let curve = compute_violin(&values, 512).unwrap();
        let step = curve.grid[1] - curve.grid[0];
        let integral: f64 = curve.density.iter().sum::<f64>() * step;
        assert!((integral - 1.0).abs() < 0.01, "integral = {integral}");
    }

    #[test]
    fn symmetric_sample_peaks_near_mean() {
        // Evenly spaced values over [-3, 3], symmetric about 0.
        let values: Vec<f64> = (0..=60).map(|i| (i as f64 - 30.0) / 10.0).collect();
        let curve = compute_violin(&values, 128).unwrap();
        let (max_i, _) =
            curve
                .density
                .iter()
                .enumerate()
                .fold((0, f64::NEG_INFINITY), |(bi, bv), (i, &v)| {
                    if v > bv {
                        (i, v)
                    } else {
                        (bi, bv)
                    }
                });
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        assert!(
            (curve.grid[max_i] - mean).abs() < 0.5,
            "argmax {} is not near the mean {mean}",
            curve.grid[max_i]
        );
    }

    #[test]
    fn bimodal_sample_has_two_peaks() {
        // Two separated uniform clusters around -3 and +3.
        let mut values = Vec::new();
        for i in 0..50 {
            values.push(-3.0 + i as f64 / 50.0);
            values.push(3.0 + i as f64 / 50.0);
        }
        let curve = compute_violin(&values, 256).unwrap();
        let local_maxima: Vec<usize> = (1..curve.density.len() - 1)
            .filter(|&i| {
                curve.density[i] > curve.density[i - 1] && curve.density[i] > curve.density[i + 1]
            })
            .collect();
        assert!(
            local_maxima.len() >= 2,
            "expected two peaks, got {local_maxima:?}"
        );
        let first = *local_maxima.first().unwrap();
        let last = *local_maxima.last().unwrap();
        let interior_min = curve.density[first..=last]
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let peak_min = curve.density[first].min(curve.density[last]);
        assert!(
            interior_min < peak_min,
            "expected a dip between the peaks ({interior_min} >= {peak_min})"
        );
    }

    /// Population sd, sample IQR, and the `1.06 * n^(-1/5)` Scott factor for
    /// `values` — the raw ingredients, not the implementation's combined rule.
    fn scott_terms(values: &[f64]) -> (f64, f64, f64) {
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let sd = (values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let iqr = quantile(&sorted, 0.75) - quantile(&sorted, 0.25);
        (sd, iqr, 1.06 * n.powf(-1.0 / 5.0))
    }

    #[test]
    fn scott_bandwidth_matches_formula() {
        // Robust term smaller: the bandwidth must use IQR/1.34, not sd.
        let robust = [0.0, 0.0, 1.0, 2.0, 3.0, 100.0];
        let (sd, iqr, factor) = scott_terms(&robust);
        assert!(iqr / 1.34 < sd, "fixture must let the robust term win");
        let expected = factor * (iqr / 1.34);
        assert!((compute_violin(&robust, 64).unwrap().bandwidth - expected).abs() < 1e-12);

        // sd smaller: the bandwidth must use sd, not IQR/1.34.
        let narrow = [1.0, 2.0, 3.0, 4.0, 5.0];
        let (sd, iqr, factor) = scott_terms(&narrow);
        assert!(sd < iqr / 1.34, "fixture must let sd win");
        let expected = factor * sd;
        assert!((compute_violin(&narrow, 64).unwrap().bandwidth - expected).abs() < 1e-12);
    }

    #[test]
    fn zero_iqr_non_constant_input_falls_back_to_sd() {
        // The type-7 IQR of [1,1,1,1,2] is exactly 0, but the sample is not
        // constant. The sd fallback must keep it (regression: h used to collapse
        // to 0 and the column was dropped as if constant).
        let values = [1.0, 1.0, 1.0, 1.0, 2.0];
        let Some(curve) = compute_violin(&values, 128) else {
            panic!("zero-IQR non-constant input must not be dropped");
        };
        let (sd, iqr, factor) = scott_terms(&values);
        assert_eq!(iqr, 0.0);
        let expected = factor * sd;
        assert!((curve.bandwidth - expected).abs() < 1e-12);
    }

    #[test]
    fn large_input_is_subsampled_to_the_cap() {
        let values: Vec<f64> = (0..60_000).map(|i| i as f64).collect();
        let curve = compute_violin(&values, 32).unwrap();
        assert_eq!(curve.n, MAX_KDE_SAMPLES);
    }
}
