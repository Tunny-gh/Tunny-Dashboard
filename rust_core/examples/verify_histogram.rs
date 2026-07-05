//! Python (numpy) とのヒストグラムビン計算クロスチェック用ハーネス。
//!
//! 入力データと各ビン規則(Sturges/Scott/FreedmanDiaconis/Manual)の計算結果を
//! JSON で stdout に出力する。Python 側は同じ入力を numpy で再計算して突き合わせる。
//!
//! 実行: `cargo run -p tunny-core --example verify_histogram`

use tunny_core::statistics::histogram::{compute_histogram, BinRule};

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

    /// Box-Muller で標準正規に近い値を生成する。
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

fn hist_json(values: &[f64], rule: BinRule) -> serde_json::Value {
    match compute_histogram(values, rule) {
        Some(h) => serde_json::json!({
            "bin_edges": h.bin_edges,
            "counts": h.counts,
            "n": h.n,
        }),
        None => serde_json::Value::Null,
    }
}

fn all_rules_json(values: &[f64]) -> serde_json::Value {
    serde_json::json!({
        "sturges": hist_json(values, BinRule::Sturges),
        "scott": hist_json(values, BinRule::Scott),
        "fd": hist_json(values, BinRule::FreedmanDiaconis),
        "manual_5": hist_json(values, BinRule::Manual(5)),
        "manual_20": hist_json(values, BinRule::Manual(20)),
        "manual_0": hist_json(values, BinRule::Manual(0)),
        "manual_10000": hist_json(values, BinRule::Manual(10_000)),
    })
}

fn main() {
    let mut rng = Rng(0x1357_9BDF_2468_ACE0);

    // A: n=80 の一様分布
    let uniform: Vec<f64> = (0..80).map(|_| rng.next_f64() * 100.0).collect();

    // B: n=60、整数丸めで多数のタイを持つ (FD の IQR=0 フォールバックを誘発しうる)
    let ties: Vec<f64> = (0..60).map(|_| (rng.next_f64() * 3.0).floor()).collect();

    // C: n=50、NaN/Inf混入
    let with_nonfinite: Vec<f64> = (0..50)
        .enumerate()
        .map(|(i, _)| {
            if i % 9 == 0 {
                f64::NAN
            } else if i % 11 == 0 {
                f64::INFINITY
            } else {
                rng.next_f64() * 20.0 - 10.0
            }
        })
        .collect();

    // D: 定数データ
    let constant = vec![7.5; 10];

    // E: n=70、対数正規に近い歪んだ分布 (Box-Muller -> exp)
    let skewed: Vec<f64> = (0..70)
        .map(|_| (rng.next_normal() * 0.5 + 1.0).exp())
        .collect();

    // F: n=8 の小サンプル (Sturges の丸め境界を確認)
    let small: Vec<f64> = (0..8).map(|v| v as f64).collect();

    let datasets: Vec<(String, Vec<f64>)> = vec![
        ("uniform_n80".into(), uniform),
        ("ties_n60".into(), ties),
        ("with_nonfinite_n50".into(), with_nonfinite),
        ("constant_n10".into(), constant),
        ("skewed_lognormal_n70".into(), skewed),
        ("small_n8".into(), small),
    ];

    let out: serde_json::Value = datasets
        .iter()
        .map(|(label, values)| {
            serde_json::json!({
                "label": label,
                "values": values,
                "results": all_rules_json(values),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
