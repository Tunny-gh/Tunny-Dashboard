//! Robustness analysis: Monte Carlo propagation of input noise through a
//! trained surrogate.
//!
//! Applies input noise around a candidate design point (1σ = relative noise
//! level × each parameter's normalized box range, with the distribution
//! chosen from normal / uniform / Weibull) and estimates, via the surrogate's
//! predictions, the output distribution, constraint satisfaction rate, and
//! success probability against spec limits (with sigma-level and Cpk
//! conversion). Theoretical background is in
//! theory/{en,ja}/optimization/robustness-analysis.md.

use super::feasibility;
use super::TrainedSurrogate;
use crate::math::rng::SeededRng;
use crate::math::special::{ln_gamma, norm_ppf};

/// Distribution shape of the input noise.
///
/// All variants sample a **variate standardized to mean 0, variance 1**, which
/// is then scaled by `1σ = relative_sigma × range` and added to the center.
/// So switching distributions keeps the input noise standard deviation
/// identical; only the shape (tails / skew) changes, keeping comparisons fair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoiseDistribution {
    /// Normal distribution N(0, 1).
    Normal,
    /// Uniform distribution U(-√3, √3) (variance 1).
    Uniform,
    /// Weibull distribution (shape k) standardized to mean 0, variance 1.
    /// Skews right for k < 3.6 and becomes nearly symmetric around k ≈ 3.6.
    /// Used to model skewed input uncertainty such as material strength or
    /// fatigue life.
    Weibull {
        /// Shape parameter k (> 0).
        shape: f64,
    },
}

impl NoiseDistribution {
    /// Samples one variate with mean 0, variance 1.
    fn sample_standardized(&self, rng: &mut SeededRng) -> f64 {
        match *self {
            NoiseDistribution::Normal => rng.next_gaussian(),
            NoiseDistribution::Uniform => (rng.next_f64() * 2.0 - 1.0) * 3.0_f64.sqrt(),
            NoiseDistribution::Weibull { shape } => {
                // Inverse transform sampling: W = (-ln(1-u))^(1/k) (λ = 1).
                // u = 1 would give ln(0), so we use next_f64's [0,1) range as-is.
                let u = rng.next_f64();
                let w = (-(1.0 - u).ln()).powf(1.0 / shape);
                let (mean, std) = weibull_mean_std(shape);
                (w - mean) / std
            }
        }
    }

    /// Validates the distribution spec (range of the Weibull shape parameter).
    fn validate(&self) -> Result<(), String> {
        if let NoiseDistribution::Weibull { shape } = *self {
            // If k is extremely small, the variance tends to diverge and
            // standardization breaks down numerically.
            if !(shape.is_finite() && (0.2..=20.0).contains(&shape)) {
                return Err(format!(
                    "Weibull shape must be finite and within [0.2, 20], got {shape}"
                ));
            }
        }
        Ok(())
    }
}

/// Mean and standard deviation of the standard Weibull (λ = 1):
/// μ = Γ(1+1/k), σ² = Γ(1+2/k) − μ².
fn weibull_mean_std(shape: f64) -> (f64, f64) {
    let mean = ln_gamma(1.0 + 1.0 / shape).exp();
    let m2 = ln_gamma(1.0 + 2.0 / shape).exp();
    let var = (m2 - mean * mean).max(f64::MIN_POSITIVE);
    (mean, var.sqrt())
}

/// Input spec for robustness analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessSpec {
    /// Candidate design point (original units, same order as
    /// `TrainedSurrogate::param_names`).
    pub center: Vec<f64>,
    /// 1σ of the input noise, specified as a fraction of each parameter's
    /// range (e.g., 0.02 = ±2%).
    pub relative_sigma: f64,
    /// Distribution shape of the input noise (shared across all parameters).
    pub distribution: NoiseDistribution,
    /// Number of Monte Carlo samples.
    pub n_samples: usize,
    /// If true, also includes GP posterior variance (epistemic uncertainty)
    /// in the sampling. Ignored for models that don't provide variance
    /// (e.g., Ridge).
    pub include_epistemic: bool,
    /// Random seed (for reproducible results given the same inputs).
    pub seed: u64,
    /// Lower spec limit (LSL, original units of the objective). Samples below
    /// this value count as failures.
    pub lower_spec: Option<f64>,
    /// Upper spec limit (USL, original units of the objective). Samples above
    /// this value count as failures.
    pub upper_spec: Option<f64>,
}

/// Result of robustness analysis (all values in original units).
#[derive(Debug, Clone)]
pub struct RobustnessResult {
    /// Surrogate-predicted value at the candidate point itself.
    pub nominal: f64,
    /// Empirical mean of the output samples.
    pub mean: f64,
    /// Standard deviation of the output samples (square root of the
    /// population variance).
    pub std: f64,
    /// 5th percentile.
    pub p05: f64,
    /// Median.
    pub median: f64,
    /// 95th percentile.
    pub p95: f64,
    /// All output samples, for histogram rendering.
    pub samples: Vec<f64>,
    /// Estimated constraint satisfaction probability (`None` if there are no
    /// constraint surrogates). The sample average of `P(all constraints ≤ 0)`
    /// at each sample point.
    pub feasibility_rate: Option<f64>,
    /// Fraction of samples clipped to the declared range boundary in any
    /// dimension. If large, the reported distribution is conditional on
    /// "staying inside the box."
    pub clipped_fraction: f64,
    /// Fraction of samples that fell within the spec limits (LSL/USL).
    /// `None` if no limits are specified.
    pub success_rate: Option<f64>,
    /// One-sided normal-equivalent z-score of the success probability,
    /// z = Φ⁻¹(success_rate).
    /// The empirical probability is clamped to [1/(2n), 1−1/(2n)] so this is
    /// always finite (even with all samples passing, we don't claim more than
    /// what "n samples could have observed").
    pub sigma_level: Option<f64>,
    /// Process capability index Cpk = min((USL−μ)/3σ, (μ−LSL)/3σ) (only the
    /// specified side(s)). `None` if the output distribution's standard
    /// deviation is 0.
    pub cpk: Option<f64>,
}

/// Runs Monte Carlo propagation of input noise through a trained surrogate.
///
/// `spec.center` must have the same number of dimensions as the surrogate's
/// parameters. The noise 1σ is `spec.relative_sigma` times the width of the
/// normalized box (declared range, or observed range if none is declared),
/// and samples are clipped to stay within the box.
pub fn robustness_analysis(
    trained: &TrainedSurrogate,
    spec: &RobustnessSpec,
) -> Result<RobustnessResult, String> {
    let surrogate = &trained.surrogate;
    let n_dims = surrogate.col_stats.len();
    if spec.center.len() != n_dims {
        return Err(format!(
            "center has {} dims but surrogate expects {}",
            spec.center.len(),
            n_dims
        ));
    }
    if spec.n_samples == 0 {
        return Err("n_samples must be positive".to_string());
    }
    if !(spec.relative_sigma.is_finite() && spec.relative_sigma >= 0.0) {
        return Err("relative_sigma must be a non-negative finite value".to_string());
    }
    spec.distribution.validate()?;
    if let (Some(lsl), Some(usl)) = (spec.lower_spec, spec.upper_spec) {
        if lsl >= usl {
            return Err(format!(
                "lower_spec ({lsl}) must be less than upper_spec ({usl})"
            ));
        }
    }

    let mut rng = SeededRng::from_seed(spec.seed);

    let center_norm = surrogate.to_norm_x(&spec.center);
    let nominal = surrogate.to_original_y(surrogate.predict_norm(&center_norm));

    let has_constraints = !trained.constraint_models.is_empty();
    let mut samples = Vec::with_capacity(spec.n_samples);
    let mut feas_sum = 0.0;
    let mut clipped_count = 0usize;

    for _ in 0..spec.n_samples {
        // Apply noise in original units, clipping to the box (col_stats' (min, range)).
        let mut x = Vec::with_capacity(n_dims);
        let mut clipped = false;
        for (d, &(low, range)) in surrogate.col_stats.iter().enumerate() {
            let sigma = spec.relative_sigma * range;
            let raw = spec.center[d] + spec.distribution.sample_standardized(&mut rng) * sigma;
            let clamped = raw.clamp(low, low + range);
            if clamped != raw {
                clipped = true;
            }
            x.push(clamped);
        }
        if clipped {
            clipped_count += 1;
        }

        let x_norm = surrogate.to_norm_x(&x);
        let mut y_norm = surrogate.predict_norm(&x_norm);
        if spec.include_epistemic {
            if let Some(var) = surrogate.predict_var_norm(&x_norm) {
                y_norm += rng.next_gaussian() * var.max(0.0).sqrt();
            }
        }
        samples.push(surrogate.to_original_y(y_norm));

        if has_constraints {
            feas_sum += feasibility::feasibility_probability(&trained.constraint_models, &x_norm);
        }
    }

    // Fail closed: if non-finite predictions sneak in, mean/std/percentile and
    // the success rate would all be silently skewed (NaN evaluates false on
    // both sides of the spec check, so it only survives in the denominator),
    // so we return an explicit error instead.
    if samples.iter().any(|y| !y.is_finite()) {
        return Err(
            "surrogate produced non-finite predictions around this center; \
             the model may be degenerate here"
                .to_string(),
        );
    }

    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();

    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let (success_rate, sigma_level, cpk) =
        spec_metrics(&samples, mean, std, spec.lower_spec, spec.upper_spec);

    Ok(RobustnessResult {
        nominal,
        mean,
        std,
        p05: crate::statistics::quantile(&sorted, 0.05),
        median: crate::statistics::quantile(&sorted, 0.5),
        p95: crate::statistics::quantile(&sorted, 0.95),
        samples,
        feasibility_rate: has_constraints.then(|| feas_sum / n),
        clipped_fraction: clipped_count as f64 / n,
        success_rate,
        sigma_level,
        cpk,
    })
}

/// Computes the success probability, sigma level, and Cpk against spec limits.
///
/// - `success_rate`: fraction of samples with LSL ≤ y ≤ USL (only the
///   specified side(s) are checked).
/// - `sigma_level`: Φ⁻¹(success probability). The empirical probability is
///   clamped to [1/(2n), 1−1/(2n)], so even with all samples passing it stays
///   at a finite value determined by n (e.g., n=1024 → about 3.3σ).
/// - `cpk`: min((USL−μ)/3σ, (μ−LSL)/3σ), taking the min over only the
///   specified side(s).
///
/// If both limits are `None`, all three return values are `None`.
fn spec_metrics(
    samples: &[f64],
    mean: f64,
    std: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    if lower_spec.is_none() && upper_spec.is_none() {
        return (None, None, None);
    }

    let n = samples.len() as f64;
    let ok = samples
        .iter()
        .filter(|&&y| lower_spec.is_none_or(|l| y >= l) && upper_spec.is_none_or(|u| y <= u))
        .count() as f64;
    let rate = ok / n;

    // Clamp the empirical probability (prevents z from diverging at 0/1 while
    // preserving the information content of n).
    let half = 1.0 / (2.0 * n);
    let clamped = rate.clamp(half, 1.0 - half);
    let sigma_level = norm_ppf(clamped);

    let cpk = if std > 0.0 {
        let upper_side = upper_spec.map(|u| (u - mean) / (3.0 * std));
        let lower_side = lower_spec.map(|l| (mean - l) / (3.0 * std));
        Some(match (lower_side, upper_side) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => unreachable!("guarded above"),
        })
    } else {
        None
    };

    (Some(rate), Some(sigma_level), cpk)
}

#[cfg(test)]
mod tests {
    use super::super::{fit_surrogate_with_validation, SurrogateFitRequest};
    use super::*;
    use crate::surrogate_opt::{ConstraintData, SurrogateModelKind};

    /// Trains a surrogate on simple two-variable training data (minimal setup
    /// for wiring verification).
    fn train_simple(with_constraint: bool) -> TrainedSurrogate {
        let n = 20;
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                vec![t * 10.0, 5.0 - t * 10.0]
            })
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|r| r[0] * 2.0 + r[1]).collect();
        let constraints = if with_constraint {
            // Linear constraint value equivalent to x0 - 5 <= 0
            vec![ConstraintData {
                name: "c0".to_string(),
                values: x_matrix.iter().map(|r| r[0] - 5.0).collect(),
            }]
        } else {
            vec![]
        };
        let req = SurrogateFitRequest {
            x_matrix,
            y,
            param_names: vec!["x0".to_string(), "x1".to_string()],
            objective_name: "obj".to_string(),
            model: SurrogateModelKind::Ridge,
            auto_select: false,
            constraints,
            priority_rows: vec![],
            param_bounds: Some(vec![Some((0.0, 10.0)), Some((-5.0, 5.0))]),
        };
        fit_surrogate_with_validation(&req).expect("fit")
    }

    fn spec(center: Vec<f64>) -> RobustnessSpec {
        RobustnessSpec {
            center,
            relative_sigma: 0.05,
            distribution: NoiseDistribution::Normal,
            n_samples: 256,
            include_epistemic: false,
            seed: 42,
            lower_spec: None,
            upper_spec: None,
        }
    }

    #[test]
    fn same_seed_is_reproducible() {
        let trained = train_simple(false);
        let s = spec(vec![5.0, 0.0]);
        let a = robustness_analysis(&trained, &s).unwrap();
        let b = robustness_analysis(&trained, &s).unwrap();
        assert_eq!(a.samples, b.samples);
        assert_eq!(a.mean, b.mean);
    }

    #[test]
    fn zero_noise_collapses_to_nominal() {
        let trained = train_simple(false);
        let mut s = spec(vec![5.0, 0.0]);
        s.relative_sigma = 0.0;
        let r = robustness_analysis(&trained, &s).unwrap();
        assert!(r.samples.iter().all(|&y| (y - r.nominal).abs() < 1e-9));
        assert!(r.std < 1e-9);
        assert_eq!(r.clipped_fraction, 0.0);
    }

    #[test]
    fn result_shape_and_ordering() {
        let trained = train_simple(false);
        let r = robustness_analysis(&trained, &spec(vec![5.0, 0.0])).unwrap();
        assert_eq!(r.samples.len(), 256);
        assert!(r.p05 <= r.median && r.median <= r.p95);
        assert!(r.feasibility_rate.is_none(), "no constraints -> None");
    }

    #[test]
    fn feasibility_rate_present_with_constraints() {
        let trained = train_simple(true);
        // Constraint x0 - 5 <= 0: x0=1 comfortably satisfies it, x0=9 nearly violates it
        let feas = robustness_analysis(&trained, &spec(vec![1.0, 0.0])).unwrap();
        let infeas = robustness_analysis(&trained, &spec(vec![9.0, 0.0])).unwrap();
        let f = feas.feasibility_rate.expect("Some with constraints");
        let i = infeas.feasibility_rate.expect("Some with constraints");
        assert!((0.0..=1.0).contains(&f) && (0.0..=1.0).contains(&i));
        assert!(f > i, "feasible center must score higher: {f} vs {i}");
    }

    #[test]
    fn center_near_bound_reports_clipping() {
        let trained = train_simple(false);
        let mut s = spec(vec![0.0, 0.0]); // x0 is exactly at the lower bound
        s.relative_sigma = 0.10;
        let r = robustness_analysis(&trained, &s).unwrap();
        assert!(r.clipped_fraction > 0.0, "half the noise should clip");
    }

    #[test]
    fn dimension_mismatch_errors() {
        let trained = train_simple(false);
        let err = robustness_analysis(&trained, &spec(vec![5.0])).unwrap_err();
        assert!(err.contains("dims"));
    }

    #[test]
    fn surface_slice_at_returns_expected_grid() {
        // Placed here to reuse train_simple (wiring check for surface_slice_at).
        let trained = train_simple(false);
        let slice =
            crate::surrogate_opt::surface_slice_at(&trained, &[5.0, 0.0], 0, 1, 10).unwrap();
        assert_eq!(slice.x_values.len(), 10);
        assert_eq!(slice.y_values.len(), 10);
        assert_eq!(slice.z_values.len(), 10);
        assert!(slice.z_values.iter().all(|row| row.len() == 10));
        // Dimension mismatch / same axis -> None
        assert!(crate::surrogate_opt::surface_slice_at(&trained, &[5.0], 0, 1, 10).is_none());
        assert!(crate::surrogate_opt::surface_slice_at(&trained, &[5.0, 0.0], 0, 0, 10).is_none());
    }

    #[test]
    fn spec_metrics_none_without_limits() {
        let trained = train_simple(false);
        let r = robustness_analysis(&trained, &spec(vec![5.0, 0.0])).unwrap();
        assert!(r.success_rate.is_none() && r.sigma_level.is_none() && r.cpk.is_none());
    }

    #[test]
    fn success_rate_and_sigma_level_with_limits() {
        let trained = train_simple(false);
        // y = 2*x0 + x1, center (5,0) → nominal ≈ 10.
        // Setting the upper limit far above the distribution gives all successes;
        // at the median it gives about half.
        let mut s = spec(vec![5.0, 0.0]);
        s.upper_spec = Some(1e6);
        let all_ok = robustness_analysis(&trained, &s).unwrap();
        assert_eq!(all_ok.success_rate, Some(1.0));
        // Even with all successes, clamping keeps it finite (n=256 → Φ⁻¹(1−1/512) ≈ 2.88).
        let z = all_ok.sigma_level.unwrap();
        assert!(z > 2.5 && z < 3.2, "z = {z}");
        assert!(all_ok.cpk.unwrap() > 10.0, "limit far away -> huge Cpk");

        let mut s2 = spec(vec![5.0, 0.0]);
        s2.upper_spec = Some(all_ok.median);
        let half = robustness_analysis(&trained, &s2).unwrap();
        let rate = half.success_rate.unwrap();
        assert!((0.4..=0.6).contains(&rate), "rate = {rate}");
        assert!(half.sigma_level.unwrap().abs() < 0.3);
    }

    #[test]
    fn two_sided_limits_and_cpk_direction() {
        let trained = train_simple(false);
        let base = robustness_analysis(&trained, &spec(vec![5.0, 0.0])).unwrap();
        let mut s = spec(vec![5.0, 0.0]);
        // Two-sided limits straddling the mean. The upper side is closer → Cpk is
        // determined by the upper side.
        s.lower_spec = Some(base.mean - 10.0 * base.std);
        s.upper_spec = Some(base.mean + 2.0 * base.std);
        let r = robustness_analysis(&trained, &s).unwrap();
        let cpk = r.cpk.unwrap();
        assert!((cpk - 2.0 / 3.0).abs() < 0.05, "cpk = {cpk} (≈ 2σ/3σ)");
        // LSL >= USL is an input error.
        let mut bad = spec(vec![5.0, 0.0]);
        bad.lower_spec = Some(1.0);
        bad.upper_spec = Some(0.0);
        assert!(robustness_analysis(&trained, &bad).is_err());
    }

    #[test]
    fn distributions_share_scale_but_differ_in_shape() {
        let trained = train_simple(false);
        let mk = |d: NoiseDistribution| {
            let mut s = spec(vec![5.0, 0.0]);
            s.distribution = d;
            s.n_samples = 4096;
            robustness_analysis(&trained, &s).unwrap()
        };
        let normal = mk(NoiseDistribution::Normal);
        let uniform = mk(NoiseDistribution::Uniform);
        let weibull = mk(NoiseDistribution::Weibull { shape: 1.5 });

        // Because of standardization, the output distribution's std is nearly
        // identical across distributions (y is linear → output std = |coefficient|
        // norm × input std).
        let ratio_u = uniform.std / normal.std;
        let ratio_w = weibull.std / normal.std;
        assert!((0.9..=1.1).contains(&ratio_u), "uniform ratio = {ratio_u}");
        assert!(
            (0.85..=1.15).contains(&ratio_w),
            "weibull ratio = {ratio_w}"
        );

        // Uniform distribution has finite tails: smaller extreme values than normal.
        let max_dev_n = normal
            .samples
            .iter()
            .map(|y| (y - normal.mean).abs())
            .fold(0.0, f64::max);
        let max_dev_u = uniform
            .samples
            .iter()
            .map(|y| (y - uniform.mean).abs())
            .fold(0.0, f64::max);
        assert!(max_dev_u < max_dev_n, "{max_dev_u} vs {max_dev_n}");
    }

    #[test]
    fn weibull_shape_out_of_range_errors() {
        let trained = train_simple(false);
        let mut s = spec(vec![5.0, 0.0]);
        s.distribution = NoiseDistribution::Weibull { shape: 0.0 };
        assert!(robustness_analysis(&trained, &s).is_err());
        s.distribution = NoiseDistribution::Weibull { shape: f64::NAN };
        assert!(robustness_analysis(&trained, &s).is_err());
    }

    #[test]
    fn weibull_standardization_is_zero_mean_unit_std() {
        // Verifies the standardization of sample_standardized (pure sampling,
        // no surrogate required).
        for shape in [0.8, 1.5, 3.6, 8.0] {
            let dist = NoiseDistribution::Weibull { shape };
            let mut rng = SeededRng::from_seed(7);
            let n = 200_000;
            let mut sum = 0.0;
            let mut sq = 0.0;
            for _ in 0..n {
                let v = dist.sample_standardized(&mut rng);
                sum += v;
                sq += v * v;
            }
            let mean = sum / n as f64;
            let std = (sq / n as f64 - mean * mean).sqrt();
            assert!(mean.abs() < 0.02, "shape {shape}: mean = {mean}");
            assert!((std - 1.0).abs() < 0.03, "shape {shape}: std = {std}");
        }
    }

    #[test]
    fn percentile_linear_interpolation() {
        // Percentile computation delegates to the shared statistics::quantile (NumPy type-7).
        let sorted = vec![0.0, 1.0, 2.0, 3.0];
        assert_eq!(crate::statistics::quantile(&sorted, 0.5), 1.5);
        assert_eq!(crate::statistics::quantile(&sorted, 0.0), 0.0);
        assert_eq!(crate::statistics::quantile(&sorted, 1.0), 3.0);
    }
}
