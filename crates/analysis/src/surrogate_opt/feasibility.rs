//! Computation of feasibility probability based on constraint surrogates.
//!
//! Optuna's constraint convention: a value ≤ 0 is feasible.
//!
//! ## Formulas
//!
//! The constraint surrogate `cm` predicts in the normalized x → z-score space.
//! The constraint in original units is `c_orig(x) = mu_norm(x) * c_std + c_mean`.
//! Rewriting the feasibility condition `c_orig(x) ≤ 0` in normalized space gives:
//!
//! ```text
//! mu_norm(x) ≤ z0    where z0 = (0 - c_mean) / c_std
//! ```
//!
//! ### GP model (with posterior variance)
//!
//! ```text
//! P(c ≤ 0 | x) = Φ(z)    where z = (z0 - mu_norm(x)) / sigma_norm(x)
//! ```
//!
//! sigma_norm(x) = sqrt(max(predict_var_norm(x), 0))
//!
//! ### Non-GP model (no posterior variance)
//!
//! Hard indicator: 1.0 if `mu_orig(x) ≤ 0`, otherwise 0.0.
//!
//! ### Multiple constraints
//!
//! Constraints are assumed independent and combined as a product:
//!
//! ```text
//! P_feas(x) = ∏_i P(c_i ≤ 0 | x)
//! ```

use super::acquisition::normal_cdf;
use super::models::FittedSurrogate;

/// Computes the feasibility probability at point `x_norm` in normalized space.
///
/// Returns 1.0 when `models` is empty (no constraints = always feasible).
pub(crate) fn feasibility_probability(models: &[FittedSurrogate], x_norm: &[f64]) -> f64 {
    models
        .iter()
        .fold(1.0, |acc, cm| acc * single_prob(cm, x_norm))
}

/// Computes P(c ≤ 0 | x) for a single constraint model.
fn single_prob(cm: &FittedSurrogate, x_norm: &[f64]) -> f64 {
    let mu_norm = cm.predict_norm(x_norm);

    // Convert the feasibility boundary to normalized space: z0 = (0 - c_mean) / c_std
    // Handle the degenerate case where c_std is near 0 (constant constraint value).
    let z0 = if cm.y_std > 1e-12 {
        (0.0 - cm.y_mean) / cm.y_std
    } else {
        // Constant constraint: always feasible if c_mean ≤ 0, otherwise always violated.
        if cm.y_mean <= 0.0 {
            return 1.0;
        } else {
            return 0.0;
        }
    };

    match cm.predict_var_norm(x_norm) {
        Some(var) => {
            let sigma_norm = var.max(0.0).sqrt();
            if sigma_norm < 1e-12 {
                // No variance → decide with a hard indicator.
                if mu_norm <= z0 {
                    1.0
                } else {
                    0.0
                }
            } else {
                // P(mu_norm(x) ≤ z0) under N(mu_norm, sigma_norm²)
                let z = (z0 - mu_norm) / sigma_norm;
                normal_cdf(z)
            }
        }
        None => {
            // Non-GP model: hard indicator.
            // mu_norm(x) ≤ z0  ⟺  mu_orig(x) ≤ 0
            if mu_norm <= z0 {
                1.0
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surrogate_opt::models::FittedSurrogate;

    // Feasibility probability is a pure formula that doesn't depend on GP fit quality,
    // so we verify it exactly by injecting analytic mocks with a known constraint
    // surface c(x) and constant variance σ² (with identity normalization, z0 = 0,
    // so the feasibility boundary c(x) ≤ 0 is used directly).
    // The "fitting" of GP / Ridge constraint surrogates itself is verified by
    // surrogate_opt::tests (constrained_fit_validation_succeeds, etc.).

    /// Builds an analytic mock constraint surrogate from a known constraint surface
    /// `c` and its constant variance σ² (None means no posterior variance = hard indicator).
    fn analytic_constraint(c: fn(&[f64]) -> f64, var: Option<f64>) -> FittedSurrogate {
        let v: Option<crate::surrogate_opt::models::AnalyticFn> = var
            .map(|s2| Box::new(move |_x: &[f64]| s2) as crate::surrogate_opt::models::AnalyticFn);
        FittedSurrogate::analytic(1, c, v)
    }

    /// For a GP (with posterior variance), P(c ≤ 0 | x) exactly matches Φ((0 − c)/σ).
    #[test]
    fn gp_feasibility_matches_normal_cdf_exactly() {
        // c(x) = x0 − 0.5, σ = 0.1.
        let cm = analytic_constraint(|x| x[0] - 0.5, Some(0.01));
        for &x0 in &[0.0_f64, 0.3, 0.5, 0.7, 1.0] {
            let p = feasibility_probability(std::slice::from_ref(&cm), &[x0]);
            let expected = normal_cdf((0.0 - (x0 - 0.5)) / 0.1);
            assert!(
                (p - expected).abs() < 1e-12,
                "x0={x0}: P_feas {p} should equal Φ {expected}"
            );
        }
    }

    /// In regions where everything is violated (c > 0), P_feas closely approaches 0;
    /// where everything is feasible (c < 0), it closely approaches 1.
    #[test]
    fn p_feas_saturates_at_extremes() {
        // c = 5 (strongly violated) → Φ(−50) ≈ 0.
        let infeasible = analytic_constraint(|_x| 5.0, Some(0.01));
        let p_low = feasibility_probability(std::slice::from_ref(&infeasible), &[0.5]);
        assert!(
            p_low < 1e-9,
            "strongly infeasible should give P≈0, got {p_low}"
        );

        // c = −5 (strongly feasible) → Φ(50) ≈ 1.
        let feasible = analytic_constraint(|_x| -5.0, Some(0.01));
        let p_high = feasibility_probability(std::slice::from_ref(&feasible), &[0.5]);
        assert!(
            p_high > 1.0 - 1e-9,
            "strongly feasible should give P≈1, got {p_high}"
        );
    }

    /// Multiple constraints are assumed independent and combine as a product.
    #[test]
    fn multiple_constraints_multiply() {
        let c1 = analytic_constraint(|x| x[0] - 0.5, Some(0.01)); // σ = 0.1
        let c2 = analytic_constraint(|x| 0.3 - x[0], Some(0.04)); // σ = 0.2
        let x = [0.4_f64];
        let p = feasibility_probability(&[c1, c2], &x);
        let e1 = normal_cdf((0.0 - (0.4 - 0.5)) / 0.1);
        let e2 = normal_cdf((0.0 - (0.3 - 0.4)) / 0.2);
        assert!(
            (p - e1 * e2).abs() < 1e-12,
            "joint P_feas {p} should equal product {}",
            e1 * e2
        );
    }

    /// Hard indicator path (no posterior variance): violated → 0.0, feasible → 1.0.
    #[test]
    fn no_variance_uses_hard_indicator() {
        let cm = analytic_constraint(|x| x[0] - 0.5, None);
        // x0=0.4 → c = −0.1 ≤ 0 → feasible → 1.0.
        assert_eq!(
            feasibility_probability(std::slice::from_ref(&cm), &[0.4]),
            1.0,
            "feasible point → 1.0"
        );
        // x0=0.7 → c = 0.2 > 0 → violated → 0.0.
        assert_eq!(
            feasibility_probability(std::slice::from_ref(&cm), &[0.7]),
            0.0,
            "infeasible point → 0.0"
        );
    }

    /// No constraints (empty models) → P_feas = 1.0.
    #[test]
    fn empty_models_returns_one() {
        let p = feasibility_probability(&[], &[0.5, 0.5]);
        assert_eq!(p, 1.0);
    }
}
