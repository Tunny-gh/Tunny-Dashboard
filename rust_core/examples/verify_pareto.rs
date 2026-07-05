//! pymoo (NonDominatedSorting) とのパレートランク付けクロスチェック用ハーネス。
//!
//! 入力データ（目的値・最小化/最大化方向）とランク付け結果を JSON で stdout に出力する。
//! Python 側は同じ入力を pymoo.util.nds.non_dominated_sorting で再計算して突き合わせる。
//!
//! 実行: `cargo run -p tunny-core --example verify_pareto`

use tunny_core::pareto::nd_sort;

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

    // 2目的 n=50、両方最小化。
    let objs_2d_min = gen_objectives(&mut rng, 50, 2);
    let case_2d_min = case("2obj_n50_all_minimize", objs_2d_min, vec![true, true]);

    // 2目的 n=50、片方最大化（符号反転の扱いを検証）。
    let objs_2d_mixed = gen_objectives(&mut rng, 50, 2);
    let case_2d_mixed = case("2obj_n50_mixed_direction", objs_2d_mixed, vec![true, false]);

    // 3目的 n=30、全て最小化。
    let objs_3d_min = gen_objectives(&mut rng, 30, 3);
    let case_3d_min = case("3obj_n30_all_minimize", objs_3d_min, vec![true, true, true]);

    // 3目的 n=30、1つ最大化。
    let objs_3d_mixed = gen_objectives(&mut rng, 30, 3);
    let case_3d_mixed = case(
        "3obj_n30_mixed_direction",
        objs_3d_mixed,
        vec![true, true, false],
    );

    // 支配関係が自明な手作りケース（重複点・境界ケースを含む）。
    let hand_objs = vec![
        vec![0.0, 0.0],  // 支配的
        vec![1.0, 1.0],  // 支配される
        vec![0.0, 0.0],  // 重複点（同ランクになるべき）
        vec![0.5, 0.5],  // 中間
        vec![-1.0, 2.0], // 非支配（トレードオフ）
        vec![2.0, -1.0], // 非支配（トレードオフ）
    ];
    let hand_case = case("hand_crafted_2obj", hand_objs, vec![true, true]);

    let out = serde_json::json!({
        "cases": [case_2d_min, case_2d_mixed, case_3d_min, case_3d_mixed, hand_case],
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
