//! Harness for cross-checking entropy weights against Python (pymcdm).
//!
//! pymcdm.weights.entropy_weights uses sum_normalization internally, which raises a
//! ValueError unless "all values are positive (>0)". This harness therefore generates a
//! decision matrix with only positive values (handling of columns containing negative
//! values/0 is Rust-side custom logic, separately verified by the Rust unit tests
//! tc_entropy_08/09/11).
//!
//! Run with: `cargo run -p tunny-core --example verify_entropy`

use tunny_core::entropy::compute_entropy_weights;

/// Deterministic pseudo-random number generator (xorshift64*).
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
