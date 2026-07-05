//! Python (numpy) とのクラスタ統計クロスチェック用ハーネス。
//!
//! `compute_global_stats` / `compute_cluster_centroid_std` / `compute_significant_features`
//! はいずれも flat_data を直接受け取る公開 API なので、DataFrame を経由せず検証できる。
//! 有意差判定は固定閾値 3.0 の独自ロジックであり、scipy の t 検定 (p 値) とは
//! 定義が異なる点に注意 (SE の式は Welch 検定に似るが「クラスタ vs 全体母集団」であり
//! 「クラスタ vs 残り」の 2 標本検定ではない)。
//!
//! 実行: `cargo run -p tunny-core --example verify_cluster_stats`

use tunny_core::clustering::{
    compute_cluster_centroid_std, compute_global_stats, compute_significant_features,
};

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
    let mut rng = Rng(0x5EED_5234_ABCD_0005);
    let n_per_cluster = [15usize, 25, 10];
    let means = [(0.0, 5.0), (10.0, 5.2), (-3.0, 40.0)]; // cluster 2 の第2特徴だけ大きくずらす
    let p = 3usize; // 第3特徴はクラスタ間で意図的にシフトなし(非有意ケースの確認用)

    let mut rows: Vec<Vec<f64>> = Vec::new();
    let mut labels: Vec<usize> = Vec::new();
    for (cid, &nc) in n_per_cluster.iter().enumerate() {
        for _ in 0..nc {
            rows.push(vec![
                means[cid].0 + (rng.next_f64() - 0.5) * 4.0,
                means[cid].1 + (rng.next_f64() - 0.5) * 4.0,
                (rng.next_f64() - 0.5) * 4.0,
            ]);
            labels.push(cid);
        }
    }
    let n = rows.len();
    let flat: Vec<f64> = rows.iter().flatten().copied().collect();
    let k = n_per_cluster.len();

    let (global_mean, global_std) = compute_global_stats(&flat, n, p);
    let stats = compute_cluster_centroid_std(&flat, &labels, n, p, k, &global_mean);
    let stats = compute_significant_features(stats, &global_mean, &global_std, n);

    let out = serde_json::json!({
        "data": rows,
        "labels": labels,
        "n": n,
        "p": p,
        "k": k,
        "global_mean": global_mean,
        "global_std": global_std,
        "cluster_stats": stats.iter().map(|s| serde_json::json!({
            "cluster_id": s.cluster_id,
            "size": s.size,
            "centroid": s.centroid,
            "std_dev": s.std_dev,
            "significant_features": s.significant_features,
        })).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
