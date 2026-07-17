//! CMA-ES (Covariance Matrix Adaptation Evolution Strategy, the standard form based on
//! Hansen's tutorial).
//!
//! Used for single-objective minimization in normalized space [0,1]^d. Weights are
//! applied only to the top μ individuals (rank-one + rank-μ update). The eigenvalue
//! decomposition of the covariance matrix uses faer's symmetric (self-adjoint)
//! eigenvalue decomposition.

use crate::math::rng::SeededRng;
use rayon::prelude::*;

pub(crate) struct CmaEsConfig {
    /// Initial step size (0.3 is standard for the [0,1] box).
    pub sigma0: f64,
    /// Maximum number of generations (0 = auto-determined from dimensionality).
    pub max_generations: usize,
    pub seed: u64,
}

impl Default for CmaEsConfig {
    fn default() -> Self {
        Self {
            sigma0: 0.3,
            max_generations: 0,
            seed: 42,
        }
    }
}

/// Minimizes `eval` and returns the best point ever evaluated (best-ever).
pub(crate) fn cma_es_minimize<F>(eval: F, start: &[f64], cfg: &CmaEsConfig) -> Vec<f64>
where
    F: Fn(&[f64]) -> f64 + Sync,
{
    let n = start.len();
    if n == 0 {
        return Vec::new();
    }
    let nf = n as f64;
    let mut rng = SeededRng::from_seed(cfg.seed);
    let mut gauss_spare: Option<f64> = None;

    // ── Strategy parameters (Hansen's recommended values) ───────────
    let lambda = 4 + (3.0 * nf.ln()).floor() as usize;
    let mu = lambda / 2;
    let weights: Vec<f64> = {
        let raw: Vec<f64> = (0..mu)
            .map(|i| ((lambda as f64 + 1.0) / 2.0).ln() - ((i + 1) as f64).ln())
            .collect();
        let sum: f64 = raw.iter().sum();
        raw.iter().map(|w| w / sum).collect()
    };
    let mu_eff = 1.0 / weights.iter().map(|w| w * w).sum::<f64>();

    let c_sigma = (mu_eff + 2.0) / (nf + mu_eff + 5.0);
    let d_sigma = 1.0 + 2.0 * (((mu_eff - 1.0) / (nf + 1.0)).sqrt() - 1.0).max(0.0) + c_sigma;
    let c_c = (4.0 + mu_eff / nf) / (nf + 4.0 + 2.0 * mu_eff / nf);
    let c_1 = 2.0 / ((nf + 1.3).powi(2) + mu_eff);
    let c_mu = (2.0 * (mu_eff - 2.0 + 1.0 / mu_eff) / ((nf + 2.0).powi(2) + mu_eff)).min(1.0 - c_1);
    // Approximation of E‖N(0,I)‖.
    let chi_n = nf.sqrt() * (1.0 - 1.0 / (4.0 * nf) + 1.0 / (21.0 * nf * nf));

    let max_gens = if cfg.max_generations > 0 {
        cfg.max_generations
    } else {
        (100 + 20 * n).min(500)
    };

    // ── State ─────────────────────────────────────────────────────────
    let mut mean = start.to_vec();
    let mut sigma = cfg.sigma0;
    let mut cov = identity(n);
    let mut p_sigma = vec![0.0f64; n];
    let mut p_c = vec![0.0f64; n];

    let mut best = start.to_vec();
    let mut best_cost = eval(start);

    for gen in 0..max_gens {
        // C = B diag(d²) Bᵀ (eigenvalues are clamped since numerical error can make them negative).
        // If the eigenvalue decomposition fails, don't panic — return the best-ever found so far and stop.
        let (eigvals, b) = match symmetric_eigen(&cov) {
            Some(decomp) => decomp,
            None => break,
        };
        let d_diag: Vec<f64> = eigvals.iter().map(|&v| v.max(1e-20).sqrt()).collect();

        // ── Sampling and evaluation ─────────────────────────────────
        // y_k = B (d ∘ z_k), x_k = m + σ y_k
        // Sampling is done sequentially since it shares the RNG, while evaluation
        // (surrogate prediction) is mutually independent and parallelized with rayon
        // (par_iter collects in input order, so the random sequence, ranking, and
        // best-update order match sequential execution).
        let mut ys: Vec<Vec<f64>> = Vec::with_capacity(lambda);
        let mut xs: Vec<Vec<f64>> = Vec::with_capacity(lambda);
        for _ in 0..lambda {
            let z: Vec<f64> = (0..n)
                .map(|_| next_gauss(&mut rng, &mut gauss_spare))
                .collect();
            let dz: Vec<f64> = z.iter().zip(&d_diag).map(|(zi, di)| zi * di).collect();
            let y = mat_vec(&b, &dz);
            let x: Vec<f64> = mean.iter().zip(&y).map(|(m, yi)| m + sigma * yi).collect();
            ys.push(y);
            xs.push(x);
        }
        let costs: Vec<f64> = xs.par_iter().map(|x| eval(x)).collect();
        for (x, &cost) in xs.iter().zip(costs.iter()) {
            if cost < best_cost {
                best_cost = cost;
                best = x.clone();
            }
        }
        let mut order: Vec<usize> = (0..lambda).collect();
        order.sort_by(|&a, &b| {
            costs[a]
                .partial_cmp(&costs[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // ── Mean update ────────────────────────────────────────────
        let mut y_w = vec![0.0f64; n];
        for (i, &w) in weights.iter().enumerate() {
            for d in 0..n {
                y_w[d] += w * ys[order[i]][d];
            }
        }
        for d in 0..n {
            mean[d] += sigma * y_w[d];
        }

        // ── Step-size evolution path p_σ (C^{-1/2} y_w = B d^{-1} Bᵀ y_w) ──
        let bt_yw = mat_t_vec(&b, &y_w);
        let dinv_bt: Vec<f64> = bt_yw.iter().zip(&d_diag).map(|(v, di)| v / di).collect();
        let c_inv_sqrt_yw = mat_vec(&b, &dinv_bt);
        let cs_coef = (c_sigma * (2.0 - c_sigma) * mu_eff).sqrt();
        for d in 0..n {
            p_sigma[d] = (1.0 - c_sigma) * p_sigma[d] + cs_coef * c_inv_sqrt_yw[d];
        }
        let p_sigma_norm = p_sigma.iter().map(|v| v * v).sum::<f64>().sqrt();
        let denom = (1.0 - (1.0 - c_sigma).powi(2 * (gen as i32 + 1))).sqrt();
        let h_sigma = if p_sigma_norm / denom.max(1e-12) < (1.4 + 2.0 / (nf + 1.0)) * chi_n {
            1.0
        } else {
            0.0
        };

        // ── Covariance evolution path p_c and update of C (rank-one + rank-μ) ─────────
        let cc_coef = (c_c * (2.0 - c_c) * mu_eff).sqrt();
        for d in 0..n {
            p_c[d] = (1.0 - c_c) * p_c[d] + h_sigma * cc_coef * y_w[d];
        }
        let delta_h = (1.0 - h_sigma) * c_c * (2.0 - c_c);
        // Compute the upper triangle and mirror it to the lower triangle to strictly preserve symmetry.
        for i in 0..n {
            for j in i..n {
                let mut rank_mu = 0.0;
                for (k, &w) in weights.iter().enumerate() {
                    rank_mu += w * ys[order[k]][i] * ys[order[k]][j];
                }
                let updated = (1.0 - c_1 - c_mu) * cov[i][j]
                    + c_1 * (p_c[i] * p_c[j] + delta_h * cov[i][j])
                    + c_mu * rank_mu;
                cov[i][j] = updated;
                cov[j][i] = updated;
            }
        }

        // ── Step-size update and termination check ──────────────────────────
        sigma *= ((c_sigma / d_sigma) * (p_sigma_norm / chi_n - 1.0)).exp();
        if !sigma.is_finite() || sigma * d_diag.iter().cloned().fold(0.0f64, f64::max) < 1e-9 {
            break;
        }
    }

    best
}

/// Standard normal random variate via the Box-Muller method (generates 2 values and
/// keeps 1 in reserve).
///
/// `crate::math::rng::SeededRng::next_gaussian` discards the sine term (the second
/// variate), so it consumes 2 uniform random numbers for every 1 gaussian it produces.
/// CMA-ES draws `n × λ` gaussians per generation, so here we keep the sine term in
/// `spare` to halve the consumption of uniform random numbers (and preserve the
/// existing "golden" random sequence / convergence trajectory). Because this spare-
/// keeping is incompatible with next_gaussian, cma_es keeps its own implementation.
fn next_gauss(rng: &mut SeededRng, spare: &mut Option<f64>) -> f64 {
    if let Some(s) = spare.take() {
        return s;
    }
    // next_f64 is in [0,1), so use 1−u ∈ (0,1] to avoid log(0).
    let u1 = 1.0 - rng.next_f64();
    let u2 = rng.next_f64();
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    *spare = Some(r * theta.sin());
    r * theta.cos()
}

fn identity(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect()
}

/// Matrix-vector product `A v`.
fn mat_vec(a: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    a.iter()
        .map(|row| row.iter().zip(v).map(|(x, y)| x * y).sum())
        .collect()
}

/// Transposed matrix-vector product `Aᵀ v`.
fn mat_t_vec(a: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    let n = a.len();
    (0..n)
        .map(|j| (0..n).map(|i| a[i][j] * v[i]).sum())
        .collect()
}

/// Computes the eigenvalue decomposition of a symmetric matrix (the covariance matrix)
/// using faer's self-adjoint eigenvalue decomposition.
/// Returns `Some((eigenvalues, eigenvector matrix B))` (column j of `B` corresponds to
/// eigenvalue j: `b[i][j]` is component i of eigenvector j). Eigenvalues are in
/// ascending order (per faer's convention).
/// If the decomposition fails, returns `None` instead of panicking (the caller then
/// stops the generation loop).
fn symmetric_eigen(a: &[Vec<f64>]) -> Option<(Vec<f64>, Vec<Vec<f64>>)> {
    let n = a.len();
    if n == 0 {
        return Some((Vec::new(), Vec::new()));
    }
    // Use (A + Aᵀ)/2 to avoid asymmetry from numerical error.
    let mat = faer::Mat::<f64>::from_fn(n, n, |i, j| 0.5 * (a[i][j] + a[j][i]));
    let eigen = mat.self_adjoint_eigen(faer::Side::Lower).ok()?;
    let u = eigen.U();
    let s = eigen.S();
    let eigvals: Vec<f64> = (0..n).map(|i| s[i]).collect();
    let b: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| u[(i, j)]).collect())
        .collect();
    Some((eigvals, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_eigen_known_2x2() {
        // The eigenvalues of [[2,1],[1,2]] are 1 and 3.
        let a = vec![vec![2.0, 1.0], vec![1.0, 2.0]];
        let (mut vals, b) = symmetric_eigen(&a).expect("2x2 decomposition should succeed");
        vals.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert!((vals[0] - 1.0).abs() < 1e-9, "{vals:?}");
        assert!((vals[1] - 3.0).abs() < 1e-9, "{vals:?}");
        // B should be an orthogonal matrix (BᵀB ≈ I).
        for i in 0..2 {
            for j in 0..2 {
                let dot: f64 = (0..2).map(|k| b[k][i] * b[k][j]).sum();
                let expect = if i == j { 1.0 } else { 0.0 };
                assert!((dot - expect).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn symmetric_eigen_reconstructs_matrix() {
        // Reconstruct A = B diag(λ) Bᵀ = B D² Bᵀ (D = diag(√λ)) and check it matches.
        let a = vec![
            vec![4.0, 1.0, 0.5],
            vec![1.0, 3.0, 0.2],
            vec![0.5, 0.2, 2.0],
        ];
        let (vals, b) = symmetric_eigen(&a).expect("3x3 decomposition should succeed");
        for i in 0..3 {
            for j in 0..3 {
                let recon: f64 = (0..3).map(|k| b[i][k] * vals[k] * b[j][k]).sum();
                assert!((recon - a[i][j]).abs() < 1e-9, "({i},{j}): {recon}");
            }
        }
    }

    #[test]
    fn next_gauss_has_zero_mean_unit_variance() {
        let mut rng = SeededRng::from_seed(5);
        let mut spare = None;
        let n = 20_000;
        let samples: Vec<f64> = (0..n).map(|_| next_gauss(&mut rng, &mut spare)).collect();
        let mean = samples.iter().sum::<f64>() / n as f64;
        let var = samples.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        assert!(mean.abs() < 0.05, "mean {mean}");
        assert!((var - 1.0).abs() < 0.05, "var {var}");
    }

    #[test]
    fn cma_es_minimizes_sphere_2d() {
        let cfg = CmaEsConfig::default();
        let best = cma_es_minimize(
            |x| (x[0] - 0.25).powi(2) + (x[1] - 0.75).powi(2),
            &[0.5, 0.5],
            &cfg,
        );
        assert!((best[0] - 0.25).abs() < 1e-3, "x: {}", best[0]);
        assert!((best[1] - 0.75).abs() < 1e-3, "y: {}", best[1]);
    }

    #[test]
    fn cma_es_minimizes_elliptic_5d() {
        // Verify convergence even on an ill-conditioned elliptic function (checks covariance adaptation).
        let cfg = CmaEsConfig::default();
        let target = [0.6, 0.4, 0.5, 0.3, 0.7];
        let best = cma_es_minimize(
            |x| {
                x.iter()
                    .zip(target.iter())
                    .enumerate()
                    .map(|(i, (xi, ti))| 10f64.powi(i as i32) * (xi - ti).powi(2))
                    .sum()
            },
            &[0.5; 5],
            &cfg,
        );
        for (i, (b, t)) in best.iter().zip(target.iter()).enumerate() {
            assert!((b - t).abs() < 0.02, "dim {i}: {b} vs {t}");
        }
    }
}
