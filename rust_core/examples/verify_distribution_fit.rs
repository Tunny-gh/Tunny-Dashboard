//! Python (scipy.stats) との分布あてはめ (MLE) クロスチェック用ハーネス。
//!
//! 入力データと Normal/LogNormal/Weibull 各分布の MLE フィット結果 (パラメータ,
//! 対数尤度, AIC) を JSON で stdout に出力する。Python 側は同じ入力を
//! scipy.stats.*.fit で再計算して突き合わせる。
//!
//! 実行: `cargo run -p tunny-core --example verify_distribution_fit`

use tunny_core::statistics::distribution_fit::{fit_distribution, FitDistribution};

/// 決定的な擬似乱数 (xorshift64*)。
struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545F4914F6CDD1D);
        (v >> 11) as f64 / (1u64 << 53) as f64
    }

    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

fn fit_json(values: &[f64], dist: FitDistribution) -> serde_json::Value {
    match fit_distribution(values, dist) {
        Some(f) => serde_json::json!({
            "params": [f.params.0, f.params.1],
            "log_likelihood": f.log_likelihood,
            "aic": f.aic,
        }),
        None => serde_json::Value::Null,
    }
}

fn main() {
    let mut rng = Rng(0x0F1E2D3C_4B5A6978);

    // A: 正規分布に近い n=100 サンプル (Box-Muller, mu=10, sigma=2)
    let normal_like: Vec<f64> = (0..100).map(|_| rng.next_normal() * 2.0 + 10.0).collect();

    // B: 対数正規に近い n=90 サンプル (exp(normal(mu=0.5, sigma=0.8)))
    let lognormal_like: Vec<f64> = (0..90)
        .map(|_| (rng.next_normal() * 0.8 + 0.5).exp())
        .collect();

    // C: ワイブルに近い n=80 サンプル (逆変換法, k=2.5, lambda=5.0)
    let weibull_like: Vec<f64> = (0..80)
        .map(|_| {
            let u = rng.next_f64().max(1e-12);
            5.0 * (-u.ln()).powf(1.0 / 2.5)
        })
        .collect();

    // D: 歪んだ小サンプル n=40 (正規からはずれた非対称分布)
    let skewed_small: Vec<f64> = (0..40)
        .map(|i| 3.0 + (i as f64 * 0.31).sin().abs() * 5.0 + rng.next_f64() * 0.5)
        .collect();

    let datasets: Vec<(String, Vec<f64>)> = vec![
        ("normal_like_n100".into(), normal_like),
        ("lognormal_like_n90".into(), lognormal_like),
        ("weibull_like_n80".into(), weibull_like),
        ("skewed_small_n40".into(), skewed_small),
    ];

    let out: serde_json::Value = datasets
        .iter()
        .map(|(label, values)| {
            serde_json::json!({
                "label": label,
                "values": values,
                "normal": fit_json(values, FitDistribution::Normal),
                "lognormal": fit_json(values, FitDistribution::LogNormal),
                "weibull": fit_json(values, FitDistribution::Weibull),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
