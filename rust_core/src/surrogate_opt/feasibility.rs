//! 制約サロゲートに基づく実行可能性確率の計算。
//!
//! Optuna の制約規約: 値 ≤ 0 が実行可能（feasible）。
//!
//! ## 数式
//!
//! 制約サロゲート `cm` は正規化 x → z-score 空間で予測する。
//! 元の単位での制約は `c_orig(x) = mu_norm(x) * c_std + c_mean`。
//! 実行可能条件 `c_orig(x) ≤ 0` を正規化空間で書き直すと:
//!
//! ```text
//! mu_norm(x) ≤ z0    ただし z0 = (0 - c_mean) / c_std
//! ```
//!
//! ### GP モデル（事後分散あり）
//!
//! ```text
//! P(c ≤ 0 | x) = Φ(z)    where z = (z0 - mu_norm(x)) / sigma_norm(x)
//! ```
//!
//! sigma_norm(x) = sqrt(max(predict_var_norm(x), 0))
//!
//! ### 非 GP モデル（事後分散なし）
//!
//! ハード指標: `mu_orig(x) ≤ 0` なら 1.0、さもなくば 0.0。
//!
//! ### 複数制約
//!
//! 制約が独立と仮定し、積で計算する:
//!
//! ```text
//! P_feas(x) = ∏_i P(c_i ≤ 0 | x)
//! ```

use super::acquisition::normal_cdf;
use super::models::FittedSurrogate;

/// 正規化空間の点 `x_norm` における実行可能性確率を計算する。
///
/// `models` が空のとき 1.0 を返す（制約なし = 常に実行可能）。
pub(crate) fn feasibility_probability(models: &[FittedSurrogate], x_norm: &[f64]) -> f64 {
    models
        .iter()
        .fold(1.0, |acc, cm| acc * single_prob(cm, x_norm))
}

/// 単一制約モデルについて P(c ≤ 0 | x) を計算する。
fn single_prob(cm: &FittedSurrogate, x_norm: &[f64]) -> f64 {
    let mu_norm = cm.predict_norm(x_norm);

    // 実行可能境界を正規化空間に変換する: z0 = (0 - c_mean) / c_std
    // c_std が 0 に近い場合（制約値が定数）は退化処理する。
    let z0 = if cm.y_std > 1e-12 {
        (0.0 - cm.y_mean) / cm.y_std
    } else {
        // 定数制約: c_mean ≤ 0 なら常に実行可能、さもなくば常に違反。
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
                // 分散なし → ハード指標で判定する。
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
            // 非 GP モデル: ハード指標。
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

    // 実行可能性確率の計算は GP の当てはめ品質に依存しない純粋な数式なので、既知の
    // 制約曲面 c(x) と一定分散 σ² を持つ解析的モックを注入して厳密に検証する
    // （恒等正規化のため z0 = 0、すなわち実行可能境界 c(x) ≤ 0 がそのまま使われる）。
    // GP / Ridge 制約サロゲートの「フィット」自体は surrogate_opt::tests
    // （constrained_fit_validation_succeeds 等）が確認する。

    /// 既知の制約曲面 c とその一定分散 σ²（None なら事後分散なし=ハード指標）から
    /// 解析的モック制約サロゲートを作る。
    fn analytic_constraint(c: fn(&[f64]) -> f64, var: Option<f64>) -> FittedSurrogate {
        let v: Option<crate::surrogate_opt::models::AnalyticFn> = var
            .map(|s2| Box::new(move |_x: &[f64]| s2) as crate::surrogate_opt::models::AnalyticFn);
        FittedSurrogate::analytic(1, c, v)
    }

    /// GP（事後分散あり）の P(c ≤ 0 | x) が Φ((0 − c)/σ) に厳密一致すること。
    #[test]
    fn gp_feasibility_matches_normal_cdf_exactly() {
        // c(x) = x0 − 0.5、σ = 0.1。
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

    /// すべて違反（c > 0）の領域では P_feas が 0 に、すべて実行可能（c < 0）では
    /// 1 に厳密に近づくこと。
    #[test]
    fn p_feas_saturates_at_extremes() {
        // c = 5（強く違反）→ Φ(−50) ≈ 0。
        let infeasible = analytic_constraint(|_x| 5.0, Some(0.01));
        let p_low = feasibility_probability(std::slice::from_ref(&infeasible), &[0.5]);
        assert!(
            p_low < 1e-9,
            "strongly infeasible should give P≈0, got {p_low}"
        );

        // c = −5（強く実行可能）→ Φ(50) ≈ 1。
        let feasible = analytic_constraint(|_x| -5.0, Some(0.01));
        let p_high = feasibility_probability(std::slice::from_ref(&feasible), &[0.5]);
        assert!(
            p_high > 1.0 - 1e-9,
            "strongly feasible should give P≈1, got {p_high}"
        );
    }

    /// 複数制約は独立と仮定して積になること。
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

    /// ハード指標パス（事後分散なし）: 違反側 → 0.0、実行可能側 → 1.0。
    #[test]
    fn no_variance_uses_hard_indicator() {
        let cm = analytic_constraint(|x| x[0] - 0.5, None);
        // x0=0.4 → c = −0.1 ≤ 0 → 実行可能 → 1.0。
        assert_eq!(
            feasibility_probability(std::slice::from_ref(&cm), &[0.4]),
            1.0,
            "feasible point → 1.0"
        );
        // x0=0.7 → c = 0.2 > 0 → 違反 → 0.0。
        assert_eq!(
            feasibility_probability(std::slice::from_ref(&cm), &[0.7]),
            0.0,
            "infeasible point → 0.0"
        );
    }

    /// 制約なし（空 models）→ P_feas = 1.0。
    #[test]
    fn empty_models_returns_one() {
        let p = feasibility_probability(&[], &[0.5, 0.5]);
        assert_eq!(p, 1.0);
    }
}
