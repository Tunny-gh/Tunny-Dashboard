//! Harness for cross-checking hypervolume (HV) against pymoo (indicators.hv.HV).
//!
//! Verifies both direct calls to `hypervolume_nd` (hand-crafted cases, 2D/3D) and automatic
//! reference-point computation via `compute_hv_history_from_data` (random cases, 2D/3D).
//! A past audit (A1) flagged an issue where "3+ objectives incorrectly fell back to
//! hypervolume_2d," so 3-objective cases are given particular emphasis here.
//!
//! Run with: `cargo run -p tunny-core --example verify_hypervolume`

use tunny_core::pareto::{compute_hv_history_from_data, hypervolume_nd};

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

fn gen_objectives(rng: &mut Rng, n: usize, m: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|_| (0..m).map(|_| rng.next_f64() * 10.0).collect())
        .collect()
}

/// Hand-crafted case calling `hypervolume_nd` directly (a normalized space assuming all dimensions are minimized).
fn direct_case(label: &str, points: Vec<Vec<f64>>, ref_point: Vec<f64>) -> serde_json::Value {
    let hv = hypervolume_nd(&points, &ref_point);
    serde_json::json!({
        "label": label,
        "kind": "direct",
        "points": points,
        "ref_point": ref_point,
        "hv": hv,
    })
}

/// Automatic reference-point computation case via `compute_hv_history_from_data` (raw objective values + directions).
fn auto_case(label: &str, objectives: Vec<Vec<f64>>, is_minimize: Vec<bool>) -> serde_json::Value {
    let n = objectives.len();
    let trial_ids: Vec<u32> = (0..n as u32).collect();
    let result = compute_hv_history_from_data(&trial_ids, &objectives, &is_minimize);
    let final_hv = result.hv_values.last().copied().unwrap_or(0.0);
    serde_json::json!({
        "label": label,
        "kind": "auto",
        "objectives": objectives,
        "is_minimize": is_minimize,
        "ref_point": result.ref_point,
        "hv": final_hv,
    })
}

fn main() {
    let mut rng = Rng(0x5EED_1234_ABCD_0003);

    let cases = vec![
        // 2D: a simple staircase front.
        direct_case(
            "hand_2d_staircase",
            vec![vec![0.2, 0.8], vec![0.5, 0.5], vec![0.8, 0.2]],
            vec![1.0, 1.0],
        ),
        // 2D: checks the result is unchanged when dominated/duplicate points are mixed in.
        direct_case(
            "hand_2d_with_dominated_and_duplicate",
            vec![
                vec![0.2, 0.8],
                vec![0.8, 0.2],
                vec![0.2, 0.8], // duplicate
                vec![0.9, 0.9], // dominated
            ],
            vec![1.0, 1.0],
        ),
        // 3D: a hand-computed case (same as the unit test; comes out to 0.131 via inclusion-exclusion).
        direct_case(
            "hand_3d_two_points",
            vec![vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 0.0]],
            vec![1.1, 1.1, 1.1],
        ),
        // 3D: a single point (should match the box volume).
        direct_case(
            "hand_3d_single_point",
            vec![vec![0.25, 0.5, 0.75]],
            vec![1.0, 1.0, 1.0],
        ),
        // 3D: a non-trivial front (5 points, with a dominated point mixed in).
        direct_case(
            "hand_3d_five_points_with_dominated",
            vec![
                vec![0.1, 0.6, 0.7],
                vec![0.4, 0.4, 0.5],
                vec![0.7, 0.2, 0.3],
                vec![0.9, 0.1, 0.1],
                vec![0.8, 0.8, 0.8], // dominated
            ],
            vec![1.0, 1.0, 1.0],
        ),
        // 2D n=50, all minimize.
        auto_case(
            "auto_2obj_n50_all_minimize",
            gen_objectives(&mut rng, 50, 2),
            vec![true, true],
        ),
        // 2D n=50, one maximize.
        auto_case(
            "auto_2obj_n50_mixed_direction",
            gen_objectives(&mut rng, 50, 2),
            vec![true, false],
        ),
        // 3D n=30, all minimize (WFG path; the focus case for past audit A1).
        auto_case(
            "auto_3obj_n30_all_minimize",
            gen_objectives(&mut rng, 30, 3),
            vec![true, true, true],
        ),
        // 3D n=30, one maximize.
        auto_case(
            "auto_3obj_n30_mixed_direction",
            gen_objectives(&mut rng, 30, 3),
            vec![true, true, false],
        ),
    ];

    let out = serde_json::json!({ "cases": cases });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
