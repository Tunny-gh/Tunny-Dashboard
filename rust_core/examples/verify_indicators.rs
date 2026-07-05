//! IGD+ / additive ε-indicator / R2 indicator のクロスチェック用ハーネス。
//!
//! IGD+ と ε-indicator は pymoo (moocore 経由) の実装と突き合わせる。
//! R2 indicator は pymoo に実装が無いため、重みベクトル生成だけ
//! `rust_core/src/multi_objective/indicators.rs` の `simplex_lattice_weights`
//! （private）と同じアルゴリズムをこのハーネス内に複製し、生成した重みを
//! 公開関数 `r2_indicator` にそのまま渡す。Python 側は同じ重みベクトルを使い
//! 標準定義（Hansen & Jaszkiewicz の重み付き Tchebycheff）で再計算する。
//!
//! すべて最小化前提・[0,1] に正規化済みの空間で計算する（indicators.rs の契約通り）。
//!
//! 実行: `cargo run -p tunny-core --example verify_indicators`

use tunny_core::indicators::{additive_epsilon, igd_plus, r2_indicator};

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

fn gen_points(rng: &mut Rng, n: usize, m: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|_| (0..m).map(|_| rng.next_f64()).collect())
        .collect()
}

// --- indicators.rs の simplex_lattice_weights を検証用に複製 ---
// (private のためこのハーネスからは呼べない。同一アルゴリズムをここに複製し、
//  生成した重みを公開関数 r2_indicator にそのまま渡すことで、
//  「重み生成が同じなら r2_indicator の計算が正しいか」を検証する。)

fn simplex_lattice_weights(m: usize) -> Vec<Vec<f64>> {
    const TARGET: usize = 100;
    const EPS: f64 = 1e-6;
    if m == 0 {
        return Vec::new();
    }
    if m == 1 {
        return vec![vec![1.0]];
    }
    let mut h = 1usize;
    loop {
        let next = h + 1;
        if lattice_count(next, m) > TARGET {
            break;
        }
        h = next;
        if h > 10_000 {
            break;
        }
    }
    let mut result = Vec::new();
    let mut current = vec![0usize; m];
    gen_lattice(&mut result, &mut current, 0, h, m);
    result
        .into_iter()
        .map(|counts| {
            let raw: Vec<f64> = counts
                .iter()
                .map(|&c| (c as f64 / h as f64).max(EPS))
                .collect();
            let s: f64 = raw.iter().sum();
            raw.into_iter().map(|v| v / s).collect()
        })
        .collect()
}

fn lattice_count(h: usize, m: usize) -> usize {
    let n = h + m - 1;
    let k = m - 1;
    let mut result: u128 = 1;
    for i in 0..k {
        result = result * (n - i) as u128 / (i as u128 + 1);
    }
    result.min(usize::MAX as u128) as usize
}

fn gen_lattice(
    out: &mut Vec<Vec<usize>>,
    current: &mut Vec<usize>,
    dim: usize,
    remaining: usize,
    m: usize,
) {
    if dim == m - 1 {
        current[dim] = remaining;
        out.push(current.clone());
        return;
    }
    for k in 0..=remaining {
        current[dim] = k;
        gen_lattice(out, current, dim + 1, remaining - k, m);
    }
}

fn igd_eps_case(label: &str, approx: Vec<Vec<f64>>, reference: Vec<Vec<f64>>) -> serde_json::Value {
    let igd = igd_plus(&approx, &reference);
    let eps = additive_epsilon(&approx, &reference);
    serde_json::json!({
        "label": label,
        "approx": approx,
        "reference": reference,
        "igd_plus": igd,
        "epsilon": eps,
    })
}

fn r2_case(label: &str, approx: Vec<Vec<f64>>, m: usize) -> serde_json::Value {
    let weights = simplex_lattice_weights(m);
    let r2 = r2_indicator(&approx, &weights);
    serde_json::json!({
        "label": label,
        "approx": approx,
        "weights": weights,
        "r2": r2,
    })
}

fn main() {
    let mut rng = Rng(0x5EED_1234_ABCD_0004);

    let igd_eps_cases = vec![
        // 2D: 手作り境界ケース（既存ユニットテストの拡張版）。
        igd_eps_case(
            "hand_2d_identical_sets",
            vec![vec![0.0, 1.0], vec![1.0, 0.0]],
            vec![vec![0.0, 1.0], vec![1.0, 0.0]],
        ),
        igd_eps_case(
            "hand_2d_approx_dominates_reference",
            vec![vec![0.2, 0.2]],
            vec![vec![0.5, 0.5], vec![0.8, 0.2]],
        ),
        // 2D 乱数ケース: approx n=20, reference n=15。
        igd_eps_case(
            "random_2d_n20_ref15",
            gen_points(&mut rng, 20, 2),
            gen_points(&mut rng, 15, 2),
        ),
        // 3D 乱数ケース: approx n=15, reference n=12。
        igd_eps_case(
            "random_3d_n15_ref12",
            gen_points(&mut rng, 15, 3),
            gen_points(&mut rng, 12, 3),
        ),
    ];

    let r2_cases = vec![
        r2_case("r2_hand_2d_near_ideal", vec![vec![0.0, 0.0]], 2),
        r2_case("r2_random_2d_n20", gen_points(&mut rng, 20, 2), 2),
        r2_case("r2_random_3d_n15", gen_points(&mut rng, 15, 3), 3),
    ];

    let out = serde_json::json!({
        "igd_eps_cases": igd_eps_cases,
        "r2_cases": r2_cases,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
