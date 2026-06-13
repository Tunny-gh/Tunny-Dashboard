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
    use crate::math::rng::SeededRng;
    use crate::surrogate_opt::models::{fit_surrogate, SurrogateModelKind};

    /// 制約値がすべて正（常に違反）→ P_feas は訓練点付近で 0 に近い。
    #[test]
    fn all_positive_constraints_p_feas_near_zero() {
        // c = 0.5 - x + 1  (c > 0 for all x in [0,1] since min = 0.5)
        let mut rng = SeededRng::from_seed(17);
        let n = 30usize;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|_| vec![rng.next_f64()]).collect();
        let c: Vec<f64> = x_matrix.iter().map(|r| 0.5 + r[0]).collect(); // always > 0
        let cm =
            fit_surrogate(SurrogateModelKind::GpFitc, &x_matrix, &c).expect("fit should succeed");

        // 訓練点での P_feas が 0.2 未満（違反側に引き寄せられる）。
        let mut count_low = 0usize;
        for row in &x_matrix {
            let x_norm = cm.to_norm_x(row);
            let p = feasibility_probability(std::slice::from_ref(&cm), &x_norm);
            if p < 0.2 {
                count_low += 1;
            }
        }
        assert!(
            count_low > n / 2,
            "Most training points should have P_feas < 0.2 when all constraints are positive: {}/{} were low",
            count_low, n
        );
    }

    /// 制約値がすべて負（常に実行可能）→ P_feas は訓練点付近で 0.8 以上。
    #[test]
    fn all_negative_constraints_p_feas_near_one() {
        let mut rng = SeededRng::from_seed(19);
        let n = 30usize;
        let x_matrix: Vec<Vec<f64>> = (0..n).map(|_| vec![rng.next_f64()]).collect();
        let c: Vec<f64> = x_matrix.iter().map(|r| r[0] - 1.5).collect(); // always < 0
        let cm =
            fit_surrogate(SurrogateModelKind::GpFitc, &x_matrix, &c).expect("fit should succeed");

        let mut count_high = 0usize;
        for row in &x_matrix {
            let x_norm = cm.to_norm_x(row);
            let p = feasibility_probability(std::slice::from_ref(&cm), &x_norm);
            if p > 0.8 {
                count_high += 1;
            }
        }
        assert!(
            count_high > n / 2,
            "Most training points should have P_feas > 0.8 when all constraints are negative: {}/{} were high",
            count_high, n
        );
    }

    /// ハード指標パス: Ridge モデルで違反側 → P_feas = 0.0。
    #[test]
    fn ridge_hard_indicator_infeasible() {
        let x_matrix: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64 / 20.0]).collect();
        // c = x + 0.5  → すべての訓練点で c > 0（常に違反）。
        let c: Vec<f64> = x_matrix.iter().map(|r| r[0] + 0.5).collect();
        let cm = fit_surrogate(SurrogateModelKind::Ridge, &x_matrix, &c)
            .expect("ridge fit should succeed");

        // x=1.0 （範囲外端）: 予測値 = 1.5 > 0 → 違反 → P=0。
        let x_norm = cm.to_norm_x(&[1.0]);
        let p = feasibility_probability(std::slice::from_ref(&cm), &x_norm);
        assert_eq!(
            p, 0.0,
            "Ridge hard indicator should return 0 for infeasible point"
        );
    }

    /// ハード指標パス: Ridge モデルで実行可能側 → P_feas = 1.0。
    #[test]
    fn ridge_hard_indicator_feasible() {
        let x_matrix: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64 / 20.0]).collect();
        // c = x - 2.0  → すべての訓練点で c < 0（常に実行可能）。
        let c: Vec<f64> = x_matrix.iter().map(|r| r[0] - 2.0).collect();
        let cm = fit_surrogate(SurrogateModelKind::Ridge, &x_matrix, &c)
            .expect("ridge fit should succeed");

        // x=0.0: 予測値 ≈ -2 < 0 → 実行可能 → P=1。
        let x_norm = cm.to_norm_x(&[0.0]);
        let p = feasibility_probability(std::slice::from_ref(&cm), &x_norm);
        assert_eq!(
            p, 1.0,
            "Ridge hard indicator should return 1 for feasible point"
        );
    }

    /// 制約なし（空 models）→ P_feas = 1.0。
    #[test]
    fn empty_models_returns_one() {
        let p = feasibility_probability(&[], &[0.5, 0.5]);
        assert_eq!(p, 1.0);
    }
}
