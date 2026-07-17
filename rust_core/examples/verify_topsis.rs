//! Cross-check harness against Python (pymcdm) for TOPSIS.
//!
//! Outputs the input data (decision matrix, weights, directions) and the computed
//! results to stdout as JSON. The Python side recomputes the same input with
//! pymcdm.methods.TOPSIS and compares the results.
//!
//! Run: `cargo run -p tunny-core --example verify_topsis`

use tunny_core::topsis::compute_topsis;

/// Deterministic pseudo-random generator (xorshift64*). Since the raw values are
/// passed to the Python side as JSON, the generators don't need to match — only
/// determinism is required.
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
    // obj2: minimize, scale -50..50 (includes negative values) / obj3: maximize, scale 0..10
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
