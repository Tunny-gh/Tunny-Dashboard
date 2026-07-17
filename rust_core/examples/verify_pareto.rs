//! Cross-check harness against pymoo (NonDominatedSorting) for Pareto ranking.
//!
//! Outputs the input data (objective values, minimize/maximize directions) and
//! the ranking result to stdout as JSON. On the Python side, the same input is
//! recomputed via pymoo.util.nds.non_dominated_sorting and the results are compared.
//!
//! Run: `cargo run -p tunny-core --example verify_pareto`

use tunny_core::pareto::nd_sort;

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

fn case(label: &str, objectives: Vec<Vec<f64>>, is_minimize: Vec<bool>) -> serde_json::Value {
    let ranks = nd_sort(&objectives, &is_minimize);
    serde_json::json!({
        "label": label,
        "is_minimize": is_minimize,
        "objectives": objectives,
        "ranks": ranks,
    })
}

fn main() {
    let mut rng = Rng(0x5EED_1234_ABCD_0002);

    // 2 objectives, n=50, both minimize.
    let objs_2d_min = gen_objectives(&mut rng, 50, 2);
    let case_2d_min = case("2obj_n50_all_minimize", objs_2d_min, vec![true, true]);

    // 2 objectives, n=50, one maximize (verifies sign-flip handling).
    let objs_2d_mixed = gen_objectives(&mut rng, 50, 2);
    let case_2d_mixed = case("2obj_n50_mixed_direction", objs_2d_mixed, vec![true, false]);

    // 3 objectives, n=30, all minimize.
    let objs_3d_min = gen_objectives(&mut rng, 30, 3);
    let case_3d_min = case("3obj_n30_all_minimize", objs_3d_min, vec![true, true, true]);

    // 3 objectives, n=30, one maximize.
    let objs_3d_mixed = gen_objectives(&mut rng, 30, 3);
    let case_3d_mixed = case(
        "3obj_n30_mixed_direction",
        objs_3d_mixed,
        vec![true, true, false],
    );

    // Hand-crafted case with obvious dominance relations (includes duplicate
    // points and edge cases).
    let hand_objs = vec![
        vec![0.0, 0.0],  // dominant
        vec![1.0, 1.0],  // dominated
        vec![0.0, 0.0],  // duplicate point (should get the same rank)
        vec![0.5, 0.5],  // intermediate
        vec![-1.0, 2.0], // non-dominated (trade-off)
        vec![2.0, -1.0], // non-dominated (trade-off)
    ];
    let hand_case = case("hand_crafted_2obj", hand_objs, vec![true, true]);

    let out = serde_json::json!({
        "cases": [case_2d_min, case_2d_mixed, case_3d_min, case_3d_mixed, hand_case],
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
