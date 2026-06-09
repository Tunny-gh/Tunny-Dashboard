//! CMA-ES（共分散行列適応進化戦略、Hansen のチュートリアルに基づく標準形）。
//!
//! 正規化空間 [0,1]^d の単一目的最小化に使う。重みは正の上位 μ 個体のみ
//! （rank-one + rank-μ 更新）。共分散行列の固有値分解は次元が小さい
//! （= パラメータ数）前提で巡回 Jacobi 法を自前実装する。

use crate::math::rng::SeededRng;

pub(crate) struct CmaEsConfig {
    /// 初期ステップサイズ（[0,1] 箱に対して 0.3 が標準）。
    pub sigma0: f64,
    /// 最大世代数（0 = 次元から自動決定）。
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

/// `eval` を最小化し、評価済みの最良点（best-ever）を返す。
pub(crate) fn cma_es_minimize<F>(eval: F, start: &[f64], cfg: &CmaEsConfig) -> Vec<f64>
where
    F: Fn(&[f64]) -> f64,
{
    let n = start.len();
    if n == 0 {
        return Vec::new();
    }
    let nf = n as f64;
    let mut rng = SeededRng::from_seed(cfg.seed);
    let mut gauss_spare: Option<f64> = None;

    // ── 戦略パラメータ（Hansen の推奨値） ───────────────────────────
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
    let d_sigma = 1.0 + 2.0 * ((mu_eff - 1.0) / (nf + 1.0)).sqrt().max(0.0) + c_sigma;
    let c_c = (4.0 + mu_eff / nf) / (nf + 4.0 + 2.0 * mu_eff / nf);
    let c_1 = 2.0 / ((nf + 1.3).powi(2) + mu_eff);
    let c_mu = (2.0 * (mu_eff - 2.0 + 1.0 / mu_eff) / ((nf + 2.0).powi(2) + mu_eff)).min(1.0 - c_1);
    // E‖N(0,I)‖ の近似。
    let chi_n = nf.sqrt() * (1.0 - 1.0 / (4.0 * nf) + 1.0 / (21.0 * nf * nf));

    let max_gens = if cfg.max_generations > 0 {
        cfg.max_generations
    } else {
        (100 + 20 * n).min(500)
    };

    // ── 状態 ────────────────────────────────────────────────────────
    let mut mean = start.to_vec();
    let mut sigma = cfg.sigma0;
    let mut cov = identity(n);
    let mut p_sigma = vec![0.0f64; n];
    let mut p_c = vec![0.0f64; n];

    let mut best = start.to_vec();
    let mut best_cost = eval(start);

    for gen in 0..max_gens {
        // C = B diag(d²) Bᵀ（固有値は数値誤差で負になり得るためクランプ）。
        let (eigvals, b) = jacobi_eigen(&cov);
        let d_diag: Vec<f64> = eigvals.iter().map(|&v| v.max(1e-20).sqrt()).collect();

        // ── サンプリングと評価 ─────────────────────────────────────
        // y_k = B (d ∘ z_k), x_k = m + σ y_k
        let mut ys: Vec<Vec<f64>> = Vec::with_capacity(lambda);
        let mut costs: Vec<f64> = Vec::with_capacity(lambda);
        for _ in 0..lambda {
            let z: Vec<f64> = (0..n)
                .map(|_| next_gauss(&mut rng, &mut gauss_spare))
                .collect();
            let dz: Vec<f64> = z.iter().zip(&d_diag).map(|(zi, di)| zi * di).collect();
            let y = mat_vec(&b, &dz);
            let x: Vec<f64> = mean.iter().zip(&y).map(|(m, yi)| m + sigma * yi).collect();
            let cost = eval(&x);
            if cost < best_cost {
                best_cost = cost;
                best = x;
            }
            ys.push(y);
            costs.push(cost);
        }
        let mut order: Vec<usize> = (0..lambda).collect();
        order.sort_by(|&a, &b| {
            costs[a]
                .partial_cmp(&costs[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // ── 平均の更新 ─────────────────────────────────────────────
        let mut y_w = vec![0.0f64; n];
        for (i, &w) in weights.iter().enumerate() {
            for d in 0..n {
                y_w[d] += w * ys[order[i]][d];
            }
        }
        for d in 0..n {
            mean[d] += sigma * y_w[d];
        }

        // ── ステップサイズパス p_σ（C^{-1/2} y_w = B d^{-1} Bᵀ y_w） ──
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

        // ── 共分散パス p_c と C の更新（rank-one + rank-μ） ─────────
        let cc_coef = (c_c * (2.0 - c_c) * mu_eff).sqrt();
        for d in 0..n {
            p_c[d] = (1.0 - c_c) * p_c[d] + h_sigma * cc_coef * y_w[d];
        }
        let delta_h = (1.0 - h_sigma) * c_c * (2.0 - c_c);
        // 上三角を計算して下三角へ鏡映し、対称性を厳密に保つ。
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

        // ── ステップサイズの更新と停止判定 ──────────────────────────
        sigma *= ((c_sigma / d_sigma) * (p_sigma_norm / chi_n - 1.0)).exp();
        if !sigma.is_finite() || sigma * d_diag.iter().cloned().fold(0.0f64, f64::max) < 1e-9 {
            break;
        }
    }

    best
}

/// Box-Muller 法による標準正規乱数（2 個生成して 1 個を温存する）。
fn next_gauss(rng: &mut SeededRng, spare: &mut Option<f64>) -> f64 {
    if let Some(s) = spare.take() {
        return s;
    }
    // next_f64 は [0,1) のため、log(0) を避けるよう 1−u ∈ (0,1] を使う。
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

/// 行列ベクトル積 `A v`。
fn mat_vec(a: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    a.iter()
        .map(|row| row.iter().zip(v).map(|(x, y)| x * y).sum())
        .collect()
}

/// 転置行列ベクトル積 `Aᵀ v`。
fn mat_t_vec(a: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    let n = a.len();
    (0..n)
        .map(|j| (0..n).map(|i| a[i][j] * v[i]).sum())
        .collect()
}

/// 対称行列の巡回 Jacobi 法による固有値分解。
/// `(固有値, 固有ベクトル行列 B)` を返す（`B` の列 j が固有値 j に対応:
/// `b[i][j]` は固有ベクトル j の成分 i）。
fn jacobi_eigen(a: &[Vec<f64>]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a.to_vec();
    let mut v = identity(n);
    if n <= 1 {
        return (m.first().map(|r| r[0]).into_iter().collect(), v);
    }

    for _sweep in 0..100 {
        let off: f64 = (0..n)
            .flat_map(|p| ((p + 1)..n).map(move |q| (p, q)))
            .map(|(p, q)| m[p][q] * m[p][q])
            .sum();
        if off < 1e-18 {
            break;
        }
        for p in 0..n - 1 {
            for q in (p + 1)..n {
                if m[p][q].abs() < 1e-30 {
                    continue;
                }
                let theta = (m[q][q] - m[p][p]) / (2.0 * m[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;

                // 回転後の p 列・q 列を作る（対称行列なので行 p・q も同じ値）。
                let (app, aqq, apq) = (m[p][p], m[q][q], m[p][q]);
                let mut new_p: Vec<f64> = m.iter().map(|row| c * row[p] - s * row[q]).collect();
                let mut new_q: Vec<f64> = m.iter().map(|row| s * row[p] + c * row[q]).collect();
                new_p[p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
                new_q[q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
                new_p[q] = 0.0;
                new_q[p] = 0.0;
                for (row, (&np, &nq)) in m.iter_mut().zip(new_p.iter().zip(new_q.iter())) {
                    row[p] = np;
                    row[q] = nq;
                }
                m[p] = new_p;
                m[q] = new_q;

                for row in v.iter_mut() {
                    let (vip, viq) = (row[p], row[q]);
                    row[p] = c * vip - s * viq;
                    row[q] = s * vip + c * viq;
                }
            }
        }
    }

    let eigvals: Vec<f64> = (0..n).map(|i| m[i][i]).collect();
    (eigvals, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jacobi_eigen_known_2x2() {
        // [[2,1],[1,2]] の固有値は 1 と 3。
        let a = vec![vec![2.0, 1.0], vec![1.0, 2.0]];
        let (mut vals, b) = jacobi_eigen(&a);
        vals.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert!((vals[0] - 1.0).abs() < 1e-9, "{vals:?}");
        assert!((vals[1] - 3.0).abs() < 1e-9, "{vals:?}");
        // B は直交行列のはず（BᵀB ≈ I）。
        for i in 0..2 {
            for j in 0..2 {
                let dot: f64 = (0..2).map(|k| b[k][i] * b[k][j]).sum();
                let expect = if i == j { 1.0 } else { 0.0 };
                assert!((dot - expect).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn jacobi_eigen_reconstructs_matrix() {
        // A = B diag(λ) Bᵀ を再構成して一致を確認。
        let a = vec![
            vec![4.0, 1.0, 0.5],
            vec![1.0, 3.0, 0.2],
            vec![0.5, 0.2, 2.0],
        ];
        let (vals, b) = jacobi_eigen(&a);
        for i in 0..3 {
            for j in 0..3 {
                let recon: f64 = (0..3).map(|k| b[i][k] * vals[k] * b[j][k]).sum();
                assert!((recon - a[i][j]).abs() < 1e-8, "({i},{j}): {recon}");
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
        // 条件数のある楕円関数でも収束することを確認（共分散適応の検証）。
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
