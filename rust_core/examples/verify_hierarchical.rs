//! Cross-check harness against Python (scipy) for hierarchical clustering (Ward's method).
//!
//! Outputs the input data and the computed results to stdout as JSON. The Python
//! side recomputes the same input with scipy.cluster.hierarchy.linkage(method='ward')
//! and compares the results.
//!
//! Run: `cargo run -p tunny-core --example verify_hierarchical`

use tunny_core::clustering::{cut_tree, ward_linkage};

/// Deterministic pseudo-random generator (xorshift64*).
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

    // 3 clearly separated blobs (3 features). Verify both without and with standardization.
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
    // Scale only the 3rd column to an order of magnitude larger so the effect of
    // standardization is visible.
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
