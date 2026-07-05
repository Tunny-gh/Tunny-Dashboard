//! Python (numpy/matplotlib) とのボックスプロット統計クロスチェック用ハーネス。
//!
//! 入力データと計算結果 (five-number summary, Tukey フェンス, 外れ値) を JSON で
//! stdout に出力する。Python 側は同じ入力を numpy.percentile / matplotlib で
//! 再計算して突き合わせる。
//!
//! 実行: `cargo run -p tunny-core --example verify_boxplot`

use tunny_core::statistics::boxplot::{compute_boxplot, quantile};

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

fn boxplot_json(values: &[f64]) -> serde_json::Value {
    match compute_boxplot(values) {
        Some(s) => serde_json::json!({
            "n": s.n,
            "mean": s.mean,
            "min": s.min,
            "q1": s.q1,
            "median": s.median,
            "q3": s.q3,
            "max": s.max,
            "whisker_low": s.whisker_low,
            "whisker_high": s.whisker_high,
            "outliers": s.outliers,
        }),
        None => serde_json::Value::Null,
    }
}

fn main() {
    let mut rng = Rng(0xABCDEF01_23456789);

    // A: n=99 の一様分布 (奇数nで補間確認)
    let uniform: Vec<f64> = (0..99).map(|_| rng.next_f64() * 50.0).collect();

    // B: n=60、正規に近い分布 + 明確な外れ値を数点混入
    let mut normal_with_outliers: Vec<f64> =
        (0..55).map(|_| rng.next_normal() * 3.0 + 20.0).collect();
    normal_with_outliers.extend_from_slice(&[100.0, -50.0, 80.0]);

    // C: n=50、多数のタイ (離散データ)
    let ties: Vec<f64> = (0..50).map(|_| (rng.next_f64() * 5.0).floor()).collect();

    // D: n=51、NaN/Inf混入
    let with_nonfinite: Vec<f64> = (0..51)
        .enumerate()
        .map(|(i, _)| {
            if i % 8 == 0 {
                f64::NAN
            } else if i % 13 == 0 {
                f64::INFINITY
            } else {
                rng.next_f64() * 30.0 - 15.0
            }
        })
        .collect();

    // E: 単一要素
    let single = vec![42.0];

    // F: 偶数 n=8 の既知データ (linear interpolation の手計算確認用)
    let known: Vec<f64> = (1..=9).map(|v| v as f64).collect();

    let datasets: Vec<(String, Vec<f64>)> = vec![
        ("uniform_n99".into(), uniform),
        ("normal_with_outliers_n58".into(), normal_with_outliers),
        ("ties_n50".into(), ties),
        ("with_nonfinite_n51".into(), with_nonfinite),
        ("single_n1".into(), single),
        ("known_1to9".into(), known),
    ];

    let quantile_checks: Vec<(String, Vec<f64>, Vec<f64>)> = datasets
        .iter()
        .map(|(label, values)| {
            let mut finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
            finite.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let qs = vec![0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0];
            let vals: Vec<f64> = qs.iter().map(|&q| quantile(&finite, q)).collect();
            (label.clone(), qs, vals)
        })
        .collect();

    let out = serde_json::json!({
        "datasets": datasets.iter().map(|(label, values)| {
            serde_json::json!({
                "label": label,
                "values": values,
                "boxplot": boxplot_json(values),
            })
        }).collect::<Vec<_>>(),
        "quantile_checks": quantile_checks.iter().map(|(label, qs, vals)| {
            serde_json::json!({ "label": label, "qs": qs, "values": vals })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
