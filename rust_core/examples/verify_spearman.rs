//! Harness for cross-checking Spearman rank correlation against Python (scipy).
//!
//! Targets `sensitivity::spearman::compute_spearman` (exposed as the pub function
//! `tunny_core::sensitivity::compute_spearman` in the `sensitivity` module).
//! The ranking and Pearson correlation used internally (`math::stats::rank` /
//! `spearman_correlation`) have already been cross-checked against scipy in a separate report
//! (correlation.md), so this harness focuses on whether `compute_spearman`'s own preprocessing
//! (pairwise removal of NaN/Inf) works correctly.
//!
//! Run with: `cargo run -p tunny-core --example verify_spearman`

use tunny_core::sensitivity::compute_spearman;

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
    let mut rng = Rng(0x5EED_9E58_0001_ABCD);
    let n = 30;

    // Case 1: clean data, no ties, no missing values.
    let x_clean: Vec<f64> = (0..n).map(|_| rng.next_f64() * 10.0).collect();
    let y_clean: Vec<f64> = x_clean
        .iter()
        .map(|&v| 2.0 * v + (rng.next_f64() - 0.5) * 4.0)
        .collect();

    // Case 2: ties in both x and y (discrete-looking values).
    let x_ties: Vec<f64> = (0..n).map(|_| (rng.next_f64() * 4.0).floor()).collect();
    let y_ties: Vec<f64> = (0..n).map(|_| (rng.next_f64() * 5.0).floor()).collect();

    // Case 3: NaN scattered in x only.
    let mut x_nan = x_clean.clone();
    for i in (0..n).step_by(6) {
        x_nan[i] = f64::NAN;
    }
    let y_nan = y_clean.clone();

    // Case 4: Inf scattered in y only (Inf is_finite()==false but not NaN;
    // scipy's nan_policy='omit' does NOT drop these, so the Python side must
    // apply the same manual finite-mask as Rust, not rely on nan_policy).
    let x_inf = x_clean.clone();
    let mut y_inf = y_clean.clone();
    for i in (1..n).step_by(7) {
        y_inf[i] = f64::INFINITY;
    }

    // Case 5: mixed NaN (x) + Inf (y) + a negative correlation.
    let x_mixed: Vec<f64> = (0..n).map(|_| rng.next_f64() * 6.0 - 3.0).collect();
    let mut y_mixed: Vec<f64> = x_mixed.iter().map(|&v| -1.5 * v + 1.0).collect();
    let mut x_mixed_c = x_mixed.clone();
    x_mixed_c[2] = f64::NAN;
    y_mixed[9] = f64::NEG_INFINITY;
    y_mixed[20] = f64::NAN;

    // Case 6: fewer than 2 finite pairs remain after filtering -> expect NaN.
    let x_sparse = vec![1.0, f64::NAN, f64::INFINITY, 4.0, f64::NAN];
    let y_sparse = vec![f64::NAN, 2.0, 3.0, f64::NAN, f64::NAN];

    let cases = [
        ("clean", x_clean, y_clean),
        ("ties", x_ties, y_ties),
        ("nan_in_x", x_nan, y_nan),
        ("inf_in_y", x_inf, y_inf),
        ("mixed_nan_inf_negative_corr", x_mixed_c, y_mixed),
        ("sparse_below_min_pairs", x_sparse, y_sparse),
    ];

    let out: Vec<serde_json::Value> = cases
        .iter()
        .map(|(label, x, y)| {
            let rho = compute_spearman(x, y);
            serde_json::json!({
                "label": label,
                "x": x,
                "y": y,
                "rho": if rho.is_finite() { serde_json::json!(rho) } else { serde_json::Value::Null },
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
