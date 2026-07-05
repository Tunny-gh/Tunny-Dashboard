//! Python (scipy) との階層クラスタリング(Ward法)クロスチェック用ハーネス。
//!
//! 入力データと計算結果を JSON で stdout に出力する。Python 側は同じ入力を
//! scipy.cluster.hierarchy.linkage(method='ward') で再計算して突き合わせる。
//!
//! 実行: `cargo run -p tunny-core --example verify_hierarchical`

use tunny_core::clustering::{cut_tree, ward_linkage};

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

fn main() {
    let mut rng = Rng(0x5EED_2234_ABCD_0002);
    let n_per_blob = 10;
    let centers = [(0.0, 0.0, 0.0), (8.0, 0.0, 0.0), (0.0, 8.0, 4.0)];

    // 3 個の明確に分離したブロブ (3 特徴)。標準化なしと標準化ありの両方を検証する。
    let mut data: Vec<Vec<f64>> = Vec::new();
    for &(cx, cy, cz) in &centers {
        for _ in 0..n_per_blob {
            data.push(vec![
                cx + (rng.next_f64() - 0.5) * 0.8,
                cy + (rng.next_f64() - 0.5) * 0.8,
                cz + (rng.next_f64() - 0.5) * 0.8,
            ]);
        }
    }
    // 第 3 列だけ桁違いに大きいスケールにして標準化の効果も見えるようにする。
    let mut data_scaled = data.clone();
    for row in &mut data_scaled {
        row[2] *= 1000.0;
    }

    let run = |data: &[Vec<f64>], standardize: bool| {
        let result = ward_linkage(data, standardize).unwrap();
        let labels = cut_tree(&result, 3);
        serde_json::json!({
            "merges": result.merges.iter().map(|m| serde_json::json!({
                "a": m.a, "b": m.b, "distance": m.distance, "size": m.size,
            })).collect::<Vec<_>>(),
            "leaf_order": result.leaf_order,
            "labels_k3": labels,
        })
    };

    let out = serde_json::json!({
        "data": data,
        "data_scaled": data_scaled,
        "n": data.len(),
        "raw": run(&data, false),
        "standardized_via_rust": run(&data_scaled, true),
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
