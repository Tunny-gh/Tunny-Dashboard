//! Python (pymcdm) との TOPSIS クロスチェック用ハーネス。
//!
//! 入力データ（決定行列・重み・方向）と計算結果を JSON で stdout に出力する。
//! Python 側は pymcdm.methods.TOPSIS で同じ入力を再計算して突き合わせる。
//!
//! 実行: `cargo run -p tunny-core --example verify_topsis`

use tunny_core::topsis::compute_topsis;

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
    let mut rng = Rng(0x5EED_7095_1234_0001u64);
    let n_trials = 20usize;
    let n_objectives = 4usize;

    // obj0: minimize, scale 0..100 / obj1: maximize, scale 0..1
    // obj2: minimize, scale -50..50 (負値を含む) / obj3: maximize, scale 0..10
    let mut values = vec![0.0_f64; n_trials * n_objectives];
    for i in 0..n_trials {
        values[i * n_objectives] = rng.next_f64() * 100.0;
        values[i * n_objectives + 1] = rng.next_f64();
        values[i * n_objectives + 2] = rng.next_f64() * 100.0 - 50.0;
        values[i * n_objectives + 3] = rng.next_f64() * 10.0;
    }

    let is_minimize = [true, false, true, false];
    // Unnormalized weights (sum=10) to also verify internal normalize_weights.
    let weights = [4.0_f64, 1.0, 3.0, 2.0];

    let result = compute_topsis(&values, n_trials, n_objectives, &weights, &is_minimize)
        .expect("TOPSIS must succeed");

    let out = serde_json::json!({
        "n_trials": n_trials,
        "n_objectives": n_objectives,
        "values": values,
        "weights": weights,
        "is_minimize": is_minimize,
        "scores": result.scores,
        "ranked_indices": result.ranked_indices,
        "positive_ideal": result.positive_ideal,
        "negative_ideal": result.negative_ideal,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
