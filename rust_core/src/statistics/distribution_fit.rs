//! ヒストグラムへの分布あてはめ（最尤推定）。
//!
//! 正規・対数正規・ワイブルの 3 分布を MLE でフィットし、AIC で比較する。
//! 理論的背景は theory/{en,ja}/statistics/distribution-fitting.md。

use std::f64::consts::{PI, TAU};

/// あてはめ対象の分布族。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FitDistribution {
    /// 正規分布 N(μ, σ²)。
    Normal,
    /// 対数正規分布（ln x が正規）。正の値のみに適用可能。
    LogNormal,
    /// ワイブル分布（形状 k、尺度 λ）。正の値のみに適用可能。
    Weibull,
}

impl FitDistribution {
    pub fn label(self) -> &'static str {
        match self {
            FitDistribution::Normal => "Normal",
            FitDistribution::LogNormal => "Log-normal",
            FitDistribution::Weibull => "Weibull",
        }
    }
}

/// フィット結果。`params` の意味は分布ごとに異なる:
/// Normal = (μ, σ)、LogNormal = (μ_ln, σ_ln)、Weibull = (k 形状, λ 尺度)。
#[derive(Debug, Clone, PartialEq)]
pub struct FittedDistribution {
    pub dist: FitDistribution,
    pub params: (f64, f64),
    pub log_likelihood: f64,
    /// AIC = 2·2 − 2 ln L（全分布パラメータ数 2 のため比較は実質 ln L）。
    pub aic: f64,
}

impl FittedDistribution {
    /// 確率密度関数の値。ヒストグラム重ね描き時は呼び出し側が
    /// `n × ビン幅` を掛けてカウント尺度へ変換する。
    pub fn pdf(&self, x: f64) -> f64 {
        let (a, b) = self.params;
        match self.dist {
            FitDistribution::Normal => {
                let z = (x - a) / b;
                (-0.5 * z * z).exp() / (b * TAU.sqrt())
            }
            FitDistribution::LogNormal => {
                if x <= 0.0 {
                    return 0.0;
                }
                let z = (x.ln() - a) / b;
                (-0.5 * z * z).exp() / (x * b * TAU.sqrt())
            }
            FitDistribution::Weibull => {
                if x < 0.0 {
                    return 0.0;
                }
                let (k, lambda) = (a, b);
                let t = x / lambda;
                (k / lambda) * t.powf(k - 1.0) * (-t.powf(k)).exp()
            }
        }
    }

    /// 表示用のパラメータ文字列（例: "μ=1.23, σ=0.45"）。
    pub fn param_text(&self) -> String {
        let (a, b) = self.params;
        match self.dist {
            FitDistribution::Normal => format!("μ={a:.4}, σ={b:.4}"),
            FitDistribution::LogNormal => format!("μln={a:.4}, σln={b:.4}"),
            FitDistribution::Weibull => format!("k={a:.4}, λ={b:.4}"),
        }
    }
}

/// 指定した分布族を有限値サンプルへ MLE フィットする。
/// 適用不能（サンプル不足・非正値を含む・退化）な場合は `None`。
pub fn fit_distribution(values: &[f64], dist: FitDistribution) -> Option<FittedDistribution> {
    let xs: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let n = xs.len();
    if n < 3 {
        return None;
    }
    let nf = n as f64;

    let (params, log_likelihood) = match dist {
        FitDistribution::Normal => {
            let mean = xs.iter().sum::<f64>() / nf;
            let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nf;
            if var <= 0.0 {
                return None;
            }
            let sigma = var.sqrt();
            // MLE の σ² を代入した閉形式: ln L = -n/2 (ln 2πσ² + 1)
            let ll = -0.5 * nf * ((2.0 * PI * var).ln() + 1.0);
            ((mean, sigma), ll)
        }
        FitDistribution::LogNormal => {
            if xs.iter().any(|&x| x <= 0.0) {
                return None;
            }
            let logs: Vec<f64> = xs.iter().map(|x| x.ln()).collect();
            let mu = logs.iter().sum::<f64>() / nf;
            let var = logs.iter().map(|l| (l - mu).powi(2)).sum::<f64>() / nf;
            if var <= 0.0 {
                return None;
            }
            let sigma = var.sqrt();
            let sum_ln_x: f64 = logs.iter().sum();
            let ll = -0.5 * nf * ((2.0 * PI * var).ln() + 1.0) - sum_ln_x;
            ((mu, sigma), ll)
        }
        FitDistribution::Weibull => {
            if xs.iter().any(|&x| x <= 0.0) {
                return None;
            }
            // 形状方程式はスケール不変なので max で正規化し x^k のオーバーフローを防ぐ。
            let max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let ys: Vec<f64> = xs.iter().map(|x| x / max).collect();
            let k = solve_weibull_shape(&ys)?;
            let lambda_scaled = (ys.iter().map(|y| y.powf(k)).sum::<f64>() / nf).powf(1.0 / k);
            let lambda = max * lambda_scaled;
            if !(lambda.is_finite() && lambda > 0.0) {
                return None;
            }
            let ll = xs
                .iter()
                .map(|&x| {
                    let t = x / lambda;
                    (k / lambda).ln() + (k - 1.0) * t.ln() - t.powf(k)
                })
                .sum::<f64>();
            ((k, lambda), ll)
        }
    };

    if !log_likelihood.is_finite() {
        return None;
    }
    Some(FittedDistribution {
        dist,
        params,
        log_likelihood,
        aic: 4.0 - 2.0 * log_likelihood,
    })
}

/// 適用可能な全分布をフィットし AIC 昇順（良い順）で返す。
pub fn fit_all(values: &[f64]) -> Vec<FittedDistribution> {
    let mut fits: Vec<FittedDistribution> = [
        FitDistribution::Normal,
        FitDistribution::LogNormal,
        FitDistribution::Weibull,
    ]
    .into_iter()
    .filter_map(|d| fit_distribution(values, d))
    .collect();
    fits.sort_by(|a, b| {
        a.aic
            .partial_cmp(&b.aic)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    fits
}

/// ワイブル形状パラメータ k の MLE 方程式
/// `Σ x^k ln x / Σ x^k − 1/k − mean(ln x) = 0` を二分法で解く。
/// 単調性が保証される範囲 [1e-2, 1e3] で符号変化を探し、見つからなければ `None`。
fn solve_weibull_shape(xs: &[f64]) -> Option<f64> {
    let nf = xs.len() as f64;
    let mean_ln = xs.iter().map(|x| x.ln()).sum::<f64>() / nf;
    let g = |k: f64| -> f64 {
        let mut sum_pow = 0.0;
        let mut sum_pow_ln = 0.0;
        for &x in xs {
            let p = x.powf(k);
            sum_pow += p;
            sum_pow_ln += p * x.ln();
        }
        if sum_pow <= 0.0 || !sum_pow.is_finite() || !sum_pow_ln.is_finite() {
            return f64::NAN;
        }
        sum_pow_ln / sum_pow - 1.0 / k - mean_ln
    };

    let (mut lo, mut hi) = (1e-2, 1e3);
    let (glo, ghi) = (g(lo), g(hi));
    if !(glo.is_finite() && ghi.is_finite()) || glo * ghi > 0.0 {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        let gm = g(mid);
        if !gm.is_finite() {
            return None;
        }
        if gm.abs() < 1e-12 {
            return Some(mid);
        }
        if glo * gm <= 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 決定論的な擬似正規サンプル（Box-Muller 相当の固定列は避け、
    /// 対称な格子点で代用。数値品質の再検証はしない — 結線のみ確認）。
    fn symmetric_sample() -> Vec<f64> {
        // 平均 10、対称分布
        vec![7.0, 8.0, 9.0, 9.5, 10.0, 10.0, 10.5, 11.0, 12.0, 13.0]
    }

    #[test]
    fn normal_fit_recovers_sample_mean() {
        let f = fit_distribution(&symmetric_sample(), FitDistribution::Normal).unwrap();
        assert!((f.params.0 - 10.0).abs() < 1e-9, "μ = sample mean");
        assert!(f.params.1 > 0.0);
        assert!(f.aic.is_finite());
    }

    #[test]
    fn lognormal_rejects_non_positive() {
        let vals = vec![-1.0, 2.0, 3.0, 4.0];
        assert!(fit_distribution(&vals, FitDistribution::LogNormal).is_none());
        assert!(fit_distribution(&vals, FitDistribution::Weibull).is_none());
        // Normal は符号を問わない
        assert!(fit_distribution(&vals, FitDistribution::Normal).is_some());
    }

    #[test]
    fn weibull_fit_produces_positive_params() {
        let vals: Vec<f64> = (1..=30).map(|i| i as f64 * 0.3).collect();
        let f = fit_distribution(&vals, FitDistribution::Weibull).unwrap();
        assert!(f.params.0 > 0.0 && f.params.1 > 0.0);
        assert!(f.log_likelihood.is_finite());
    }

    #[test]
    fn pdf_integrates_roughly_to_one() {
        // 結線確認: 正規 PDF を粗く数値積分して 1 に近いこと（数値品質の検証ではない）
        let f = fit_distribution(&symmetric_sample(), FitDistribution::Normal).unwrap();
        let (lo, hi, n) = (-20.0, 40.0, 6000);
        let dx = (hi - lo) / n as f64;
        let integral: f64 = (0..n).map(|i| f.pdf(lo + (i as f64 + 0.5) * dx) * dx).sum();
        assert!((integral - 1.0).abs() < 1e-3, "integral = {integral}");
    }

    #[test]
    fn fit_all_sorted_by_aic() {
        let vals: Vec<f64> = (1..=40)
            .map(|i| 5.0 + (i as f64 * 0.37).sin() + i as f64 * 0.05)
            .collect();
        let fits = fit_all(&vals);
        assert!(!fits.is_empty());
        for w in fits.windows(2) {
            assert!(w[0].aic <= w[1].aic);
        }
    }

    #[test]
    fn too_few_or_degenerate_samples_rejected() {
        assert!(fit_distribution(&[1.0, 2.0], FitDistribution::Normal).is_none());
        assert!(fit_distribution(&[3.0, 3.0, 3.0, 3.0], FitDistribution::Normal).is_none());
    }
}
