//! Cross-check harness against Python (pymcdm) for VIKOR.
//!
//! Outputs the input data (decision matrix, weights, directions, v) and the
//! computation result to stdout as JSON. On the Python side, pymcdm.methods.VIKOR
//! recomputes the same input and the results are compared.
//! pymcdm's VIKOR raises ValueError when a column has the same value across all
//! alternatives (fstar==fminus), so the generated data here avoids constant columns.
//!
//! Run: `cargo run -p tunny-core --example verify_vikor`

use tunny_core::vikor::compute_vikor;

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
    let mut rng = Rng(0x5EED_7095_1234_0002u64);
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
    let v = 0.5_f64; // matches the app's default (egui-app mcdm_chart.rs)

    let result = compute_vikor(&values, n_trials, n_objectives, &weights, &is_minimize, v)
        .expect("VIKOR must succeed");

    // Second scenario: same matrix, but every objective treated as minimize.
    // compute_vikor's best/worst-value accumulators are initialized to
    // (+INFINITY, -INFINITY) regardless of direction, which is only correct
    // for the minimize branch (see minimal_maximize_bug_repro below for the
    // maximize-direction defect this causes). This all-minimize scenario
    // avoids that defect entirely, giving a clean basis for cross-checking
    // the core S/R/Q formula against pymcdm.
    let is_minimize_all = [true, true, true, true];
    let result_all_min = compute_vikor(
        &values,
        n_trials,
        n_objectives,
        &weights,
        &is_minimize_all,
        v,
    )
    .expect("VIKOR (all-minimize) must succeed");

    // Minimal, hand-checkable reproduction of the maximize-direction defect:
    // 2 alternatives x 1 objective, maximize. best/worst accumulators start
    // at (+INFINITY, -INFINITY) unconditionally regardless of direction, so
    // for a maximize objective `f64::max(best, val)` can never move `best`
    // away from +INFINITY, and `f64::min(worst, val)` can never move `worst`
    // away from -INFINITY.
    let bug_values = [1.0_f64, 5.0];
    let bug_result = compute_vikor(&bug_values, 2, 1, &[1.0], &[false], 0.5)
        .expect("VIKOR minimal repro must succeed");

    let out = serde_json::json!({
        "n_trials": n_trials,
        "n_objectives": n_objectives,
        "values": values,
        "weights": weights,
        "is_minimize": is_minimize,
        "v": v,
        "s_values": result.s_values,
        "r_values": result.r_values,
        "q_values": result.q_values,
        "display_scores": result.display_scores,
        "ranked_indices": result.ranked_indices,
        "best_values": result.best_values,
        "worst_values": result.worst_values,
        "compromise_indices": result.compromise_indices,
        "all_minimize_scenario": {
            "is_minimize": is_minimize_all,
            "s_values": result_all_min.s_values,
            "r_values": result_all_min.r_values,
            "q_values": result_all_min.q_values,
            "best_values": result_all_min.best_values,
            "worst_values": result_all_min.worst_values,
        },
        "minimal_maximize_bug_repro": {
            "values": bug_values,
            "is_minimize": [false],
            "weights": [1.0],
            "v": 0.5,
            "s_values": bug_result.s_values,
            "r_values": bug_result.r_values,
            "q_values": bug_result.q_values,
            "best_values": bug_result.best_values,
            "worst_values": bug_result.worst_values,
        },
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
