//! Python (scikit-learn) との k-means クロスチェック用ハーネス。
//!
//! k-means++ はシードが揃わないため厳密な数値一致は不可能。よく分離した
//! 合成ブロブに対し「クラスタ割当がラベル置換を除いて一致」「inertia (wcss)
//! がほぼ一致」を検証する方針。エルボー法の wcss 系列の単調減少性も出力する。
//!
//! 実行: `cargo run -p tunny-core --example verify_kmeans`

use tunny_core::clustering::{estimate_k_elbow, run_kmeans, InitStrategy};

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
    let mut rng = Rng(0x5EED_4234_ABCD_0004);
    let n_per_blob = 40;
    let centers = [(0.0, 0.0), (20.0, 0.0), (0.0, 20.0), (20.0, 20.0)];

    let mut data: Vec<Vec<f64>> = Vec::new();
    for &(cx, cy) in &centers {
        for _ in 0..n_per_blob {
            data.push(vec![
                cx + (rng.next_f64() - 0.5) * 2.0,
                cy + (rng.next_f64() - 0.5) * 2.0,
            ]);
        }
    }
    let n = data.len();
    let p = 2usize;
    let flat: Vec<f64> = data.iter().flatten().copied().collect();
    let k = 4;

    let result = run_kmeans(k, &flat, p, InitStrategy::KMeansPlusPlus);
    let elbow = estimate_k_elbow(&flat, p, 8);

    let out = serde_json::json!({
        "data": data,
        "n": n,
        "p": p,
        "k": k,
        "labels": result.labels,
        "centroids": result.centroids,
        "wcss": result.wcss,
        "elbow_wcss_per_k": elbow.wcss_per_k,
        "elbow_recommended_k": elbow.recommended_k,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
