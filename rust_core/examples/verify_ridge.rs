//! Cross-check harness for Ridge regression against Python (sklearn).
//!
//! Targets `sensitivity::ridge::compute_ridge` (exposed as
//! `tunny_core::sensitivity::compute_ridge`). Internally it standardizes X to mean 0 /
//! variance 1 (denominator n, population std), then centers y, and solves the Ridge
//! normal equation (X'X + αI)β = X'y_c, which doesn't regularize the intercept. On the
//! sklearn side, set `fit_intercept=False` and pass in already-centered y to match the
//! same condition of not regularizing the intercept.
//!
//! Usage: `cargo run -p tunny-core --example verify_ridge`

use tunny_core::sensitivity::compute_ridge;

/// A deterministic pseudo-random generator (xorshift64*).
struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545F4914F6CDD1D);
        (v >> 11) as f64 / (1u64 << 53) as f64
    }
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_f64() * (hi - lo)
    }
}

fn run_case(label: &str, x_matrix: Vec<Vec<f64>>, y: Vec<f64>, alpha: f64) -> serde_json::Value {
    let n = x_matrix.len();
    let p = x_matrix[0].len();
    let faer_x = faer::Mat::from_fn(n, p, |i, j| x_matrix[i][j]);
    let result = compute_ridge(&faer_x, &y, alpha);
    serde_json::json!({
        "label": label,
        "x_matrix": x_matrix,
        "y": y,
        "alpha": alpha,
        "beta": result.beta,
        "r_squared": result.r_squared,
    })
}

fn main() {
    let mut rng = Rng(0x5EED_2B01_1D6E_0011);
    let alpha = 1.0; // same value as RIDGE_ALPHA

    // Case 1: 4 params, y is an exact linear combination of x1..x3 plus noise;
    // x4 is irrelevant (true coefficient 0) to check the surrogate assigns it ~0 weight.
    let n1 = 200;
    let x1: Vec<Vec<f64>> = (0..n1)
        .map(|_| {
            vec![
                rng.uniform(-5.0, 5.0),
                rng.uniform(-5.0, 5.0),
                rng.uniform(-5.0, 5.0),
                rng.uniform(-5.0, 5.0),
            ]
        })
        .collect();
    let y1: Vec<f64> = x1
        .iter()
        .map(|row| {
            2.0 * row[0] - 1.5 * row[1] + 0.5 * row[2] + 0.0 * row[3] + (rng.next_f64() - 0.5) * 1.0
        })
        .collect();

    // Case 2: noise-free exact linear function (no shrinkage-vs-noise ambiguity).
    let n2 = 150;
    let x2: Vec<Vec<f64>> = (0..n2)
        .map(|_| vec![rng.uniform(-3.0, 3.0), rng.uniform(-3.0, 3.0)])
        .collect();
    let y2: Vec<f64> = x2.iter().map(|row| 3.0 * row[0] + 0.7 * row[1]).collect();

    // Case 3: one constant column (zero variance) to exercise the std<EPSILON guard
    // (column_mean_std fixes std to 1.0 instead of dividing by zero).
    let n3 = 60;
    let x3: Vec<Vec<f64>> = (0..n3).map(|_| vec![rng.uniform(-2.0, 2.0), 7.0]).collect();
    let y3: Vec<f64> = x3
        .iter()
        .map(|row| 1.2 * row[0] + (rng.next_f64() - 0.5) * 0.2)
        .collect();

    let out = vec![
        run_case("linear_plus_irrelevant", x1, y1, alpha),
        run_case("noise_free_exact", x2, y2, alpha),
        run_case("constant_column_guard", x3, y3, alpha),
    ];

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
