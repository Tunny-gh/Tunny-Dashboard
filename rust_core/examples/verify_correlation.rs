//! Python (scipy) との相関係数クロスチェック用ハーネス。
//!
//! 入力データと計算結果を JSON で stdout に出力する。Python 側は同じ入力を
//! scipy.stats で再計算して突き合わせる。
//!
//! 実行: `cargo run -p tunny-core --example verify_correlation`

use tunny_core::statistics::correlation::{compute_correlation_matrix, CorrelationMethod};

/// 決定的な擬似乱数 (xorshift64*)。Python 側へは値そのものを JSON で渡すため
/// 生成器を揃える必要はなく、決定性だけが必要。
struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545F4914F6CDD1D);
        (v >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn main() {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let n = 60;

    // 相関のある列・独立な列・タイを含む列・NaN 混入列を用意する
    let x: Vec<f64> = (0..n).map(|_| rng.next_f64() * 10.0).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&v| 2.0 * v + (rng.next_f64() - 0.5) * 4.0)
        .collect();
    let z: Vec<f64> = (0..n).map(|_| rng.next_f64() * 5.0 - 2.5).collect();
    let ties: Vec<f64> = (0..n).map(|_| (rng.next_f64() * 4.0).floor()).collect();
    let with_nan: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            if i % 7 == 0 {
                f64::NAN
            } else {
                v + rng.next_f64()
            }
        })
        .collect();

    let columns: Vec<(String, Vec<f64>)> = vec![
        ("x".into(), x),
        ("y".into(), y),
        ("z".into(), z),
        ("ties".into(), ties),
        ("with_nan".into(), with_nan),
    ];

    let pearson = compute_correlation_matrix(&columns, CorrelationMethod::Pearson).unwrap();
    let spearman = compute_correlation_matrix(&columns, CorrelationMethod::Spearman).unwrap();

    let out = serde_json::json!({
        "inputs": columns.iter().map(|(l, v)| serde_json::json!({"label": l, "values": v})).collect::<Vec<_>>(),
        "pearson": pearson.values,
        "spearman": spearman.values,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
