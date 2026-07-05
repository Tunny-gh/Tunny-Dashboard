//! pymoo (indicators.hv.HV) とのハイパーボリューム (HV) クロスチェック用ハーネス。
//!
//! `hypervolume_nd` への直接呼び出し（手作りケース・2D/3D）と、
//! `compute_hv_history_from_data` を通した自動参照点算出（乱数ケース・2D/3D）の
//! 両方を検証する。過去監査(A1)で「3目的以上で hypervolume_2d に誤って
//! フォールバックする」指摘があったため、3目的ケースは特に重点的に含める。
//!
//! 実行: `cargo run -p tunny-core --example verify_hypervolume`

use tunny_core::pareto::{compute_hv_history_from_data, hypervolume_nd};

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

/// 直接 `hypervolume_nd` を呼ぶ手作りケース（正規化済み・全次元最小化前提の空間）。
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

/// `compute_hv_history_from_data` を通した自動参照点算出ケース（生の目的値 + 方向）。
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
        // 2D: 単純な階段状フロント。
        direct_case(
            "hand_2d_staircase",
            vec![vec![0.2, 0.8], vec![0.5, 0.5], vec![0.8, 0.2]],
            vec![1.0, 1.0],
        ),
        // 2D: 支配点・重複点を混ぜても結果が変わらないことの確認。
        direct_case(
            "hand_2d_with_dominated_and_duplicate",
            vec![
                vec![0.2, 0.8],
                vec![0.8, 0.2],
                vec![0.2, 0.8], // 重複
                vec![0.9, 0.9], // 支配される
            ],
            vec![1.0, 1.0],
        ),
        // 3D: 手計算済みケース（単体テストと同一。包除原理で 0.131 になる）。
        direct_case(
            "hand_3d_two_points",
            vec![vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 0.0]],
            vec![1.1, 1.1, 1.1],
        ),
        // 3D: 単一点（box 体積に一致するはず）。
        direct_case(
            "hand_3d_single_point",
            vec![vec![0.25, 0.5, 0.75]],
            vec![1.0, 1.0, 1.0],
        ),
        // 3D: 非自明なフロント（5点、支配点混入）。
        direct_case(
            "hand_3d_five_points_with_dominated",
            vec![
                vec![0.1, 0.6, 0.7],
                vec![0.4, 0.4, 0.5],
                vec![0.7, 0.2, 0.3],
                vec![0.9, 0.1, 0.1],
                vec![0.8, 0.8, 0.8], // 支配される
            ],
            vec![1.0, 1.0, 1.0],
        ),
        // 2D n=50、全て最小化。
        auto_case(
            "auto_2obj_n50_all_minimize",
            gen_objectives(&mut rng, 50, 2),
            vec![true, true],
        ),
        // 2D n=50、片方最大化。
        auto_case(
            "auto_2obj_n50_mixed_direction",
            gen_objectives(&mut rng, 50, 2),
            vec![true, false],
        ),
        // 3D n=30、全て最小化（WFG 経路・過去監査 A1 の重点確認ケース）。
        auto_case(
            "auto_3obj_n30_all_minimize",
            gen_objectives(&mut rng, 30, 3),
            vec![true, true, true],
        ),
        // 3D n=30、1つ最大化。
        auto_case(
            "auto_3obj_n30_mixed_direction",
            gen_objectives(&mut rng, 30, 3),
            vec![true, true, false],
        ),
    ];

    let out = serde_json::json!({ "cases": cases });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
