//! Cross-check harness against Python (pymcdm) for PROMETHEE I/II.
//!
//! The Rust implementation (rust_core/src/mcdm/promethee.rs) uses a linear
//! preference function (V-shape, q=0), and automatically sets each threshold p_j
//! to 0.2 times the objective's range_j (max-min over valid rows).
//! On the Python side, PROMETHEE_II('vshape', p=p_thresholds, q=None) is passed
//! the same thresholds. p_j is also included in the output, so there is no need
//! to recompute it on the Python side.
//!
//! Run: `cargo run -p tunny-core --example verify_promethee`

use tunny_core::promethee::compute_promethee;

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
    let mut rng = Rng(0x5EED_7095_1234_0003u64);
    let n_trials = 20usize;
    let n_objectives = 4usize;

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

    // p_j = 0.2 * range_j, computed the same way compute_thresholds() does
    // internally, so the Python side can reproduce it without re-deriving it.
    let mut p_thresholds = vec![0.0_f64; n_objectives];
    for j in 0..n_objectives {
        let mut min_j = f64::INFINITY;
        let mut max_j = f64::NEG_INFINITY;
        for i in 0..n_trials {
            let v = values[i * n_objectives + j];
            if v < min_j {
                min_j = v;
            }
            if v > max_j {
                max_j = v;
            }
        }
        p_thresholds[j] = 0.2 * (max_j - min_j);
    }

    let result = compute_promethee(&values, n_trials, n_objectives, &weights, &is_minimize)
        .expect("PROMETHEE must succeed");

    let out = serde_json::json!({
        "n_trials": n_trials,
        "n_objectives": n_objectives,
        "values": values,
        "weights": weights,
        "is_minimize": is_minimize,
        "p_thresholds": p_thresholds,
        "phi_plus": result.phi_plus,
        "phi_minus": result.phi_minus,
        "phi_net": result.phi_net,
        "ranked_indices_i": result.ranked_indices_i,
        "ranked_indices_ii": result.ranked_indices_ii,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
