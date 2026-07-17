//! Special functions (gamma function, normal quantile function).
//!
//! Used for robustness-analysis Weibull standardization (Γ(1+1/k) / Γ(1+2/k))
//! and sigma-level conversion (Φ⁻¹). To avoid external dependencies, well-known
//! approximation formulas are implemented in-house. Accuracy is sufficient for
//! display/judgment purposes in both cases
//! (ln_gamma: relative error ~1e-13, norm_ppf: absolute error ~1e-9).

/// ln Γ(x) (x > 0). Lanczos approximation (g = 7, 9 coefficients).
pub(crate) fn ln_gamma(x: f64) -> f64 {
    // Lanczos (g=7, n=9) coefficients. Standard values from the Numerical Recipes family.
    const COEFFS: [f64; 8] = [
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    debug_assert!(x > 0.0, "ln_gamma requires x > 0, got {x}");

    // The reflection formula isn't needed elsewhere (only x > 0 is handled). x < 0.5 uses reflection to preserve accuracy.
    if x < 0.5 {
        // Γ(x)Γ(1-x) = π / sin(πx)
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }

    let x = x - 1.0;
    let mut a = 0.999_999_999_999_809_9_f64;
    for (i, &c) in COEFFS.iter().enumerate() {
        a += c / (x + (i + 1) as f64);
    }
    let t = x + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Quantile function of the standard normal distribution Φ⁻¹(p) (0 < p < 1). Acklam's rational approximation.
///
/// Absolute error is roughly 1.15e-9. Returns NaN if p is outside (0,1).
pub(crate) fn norm_ppf(p: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) || p == 0.0 || p == 1.0 {
        return f64::NAN;
    }

    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const P_LOW: f64 = 0.024_25;
    const P_HIGH: f64 = 1.0 - P_LOW;

    if p < P_LOW {
        // Lower tail
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= P_HIGH {
        // Central region
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        // Upper tail (symmetric to the lower tail)
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ln_gamma_matches_known_values() {
        // Γ(1) = Γ(2) = 1, Γ(3) = 2, Γ(4) = 6, Γ(0.5) = √π
        assert!((ln_gamma(1.0)).abs() < 1e-12);
        assert!((ln_gamma(2.0)).abs() < 1e-12);
        assert!((ln_gamma(3.0) - 2.0_f64.ln()).abs() < 1e-12);
        assert!((ln_gamma(4.0) - 6.0_f64.ln()).abs() < 1e-12);
        assert!((ln_gamma(0.5) - std::f64::consts::PI.sqrt().ln()).abs() < 1e-12);
        // The region used by Weibull standardization: Γ(1 + 1/k). Γ(1.5) = √π/2.
        let expected = (std::f64::consts::PI.sqrt() / 2.0).ln();
        assert!((ln_gamma(1.5) - expected).abs() < 1e-12);
    }

    #[test]
    fn norm_ppf_matches_known_quantiles() {
        // Standard z-values (constants verified against scipy.stats.norm.ppf)
        let cases = [
            (0.5, 0.0),
            (0.841_344_746_068_542_9, 1.0),
            (0.977_249_868_051_820_8, 2.0),
            (0.998_650_101_968_369_9, 3.0),
            (0.999_968_328_758_167, 4.0),
            (0.05, -1.644_853_626_951_472_7),
            (0.95, 1.644_853_626_951_472_7),
        ];
        for (p, z) in cases {
            assert!(
                (norm_ppf(p) - z).abs() < 1e-6,
                "ppf({p}) = {} != {z}",
                norm_ppf(p)
            );
        }
    }

    #[test]
    fn norm_ppf_out_of_domain_is_nan() {
        assert!(norm_ppf(0.0).is_nan());
        assert!(norm_ppf(1.0).is_nan());
        assert!(norm_ppf(-0.1).is_nan());
        assert!(norm_ppf(1.1).is_nan());
    }
}
