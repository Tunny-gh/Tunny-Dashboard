//! Python (pymcdm) とのエントロピー重みクロスチェック用ハーネス。
//!
//! pymcdm.weights.entropy_weights は内部で sum_normalization を使い、これは
//! 「全値が正 (>0)」でないと ValueError を送出する。そのため、このハーネスは
//! 正値のみの決定行列を生成する（負値/0を含む列の扱いは Rust 側の独自ロジック
//! であり、Rust 単体テスト tc_entropy_08/09/11 で別途検証済み）。
//!
//! 実行: `cargo run -p tunny-core --example verify_entropy`

use tunny_core::entropy::compute_entropy_weights;

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
}

fn main() {
    let mut rng = Rng(0x5EED_7095_1234_0004u64);
    let n_trials = 20usize;
    let n_objectives = 4usize;

    // All-positive matrix, different scale/variance per column so weights differ.
    let mut values = vec![0.0_f64; n_trials * n_objectives];
    for i in 0..n_trials {
        values[i * n_objectives] = 1.0 + rng.next_f64() * 100.0;
        values[i * n_objectives + 1] = 1.0 + rng.next_f64() * 5.0;
        values[i * n_objectives + 2] = 10.0 + rng.next_f64() * 1.0; // low variance
        values[i * n_objectives + 3] = 1.0 + rng.next_f64() * 10.0;
    }

    let result = compute_entropy_weights(&values, n_trials, n_objectives)
        .expect("entropy weights must succeed");

    let out = serde_json::json!({
        "n_trials": n_trials,
        "n_objectives": n_objectives,
        "values": values,
        "weights": result.weights,
        "entropies": result.entropies,
        "diversities": result.diversities,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
