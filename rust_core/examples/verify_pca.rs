//! Python (numpy/sklearn) との PCA クロスチェック用ハーネス。
//!
//! `run_pca` / `run_pca_standardized` はアクティブ DataFrame に依存するため、
//! `tunny_core::dataframe` の公開 API (`store_dataframes` / `select_study`) で
//! 合成データを積んだ 1 study を用意してから呼び出す。
//!
//! 実行: `cargo run -p tunny-core --example verify_pca`

use std::collections::HashMap;
use tunny_core::clustering::{run_pca, run_pca_standardized, PcaSpace};
use tunny_core::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};

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
    let mut rng = Rng(0x5EED_3234_ABCD_0003);
    let n = 50;
    let param_names = vec!["p0".to_string(), "p1".to_string(), "p2".to_string()];
    let obj_names = vec!["obj0".to_string()];

    // 相関のある 3 パラメータ + 1 目的関数。p2 だけ桁違いのスケールにして
    // 標準化 (相関行列 PCA) の効果が見えるようにする。
    let mut data: Vec<Vec<f64>> = Vec::with_capacity(n);
    for _ in 0..n {
        let p0 = rng.next_f64() * 10.0;
        let p1 = 0.6 * p0 + (rng.next_f64() - 0.5) * 3.0;
        let p2 = (rng.next_f64() - 0.5) * 2000.0;
        data.push(vec![p0, p1, p2]);
    }
    let objective: Vec<f64> = data.iter().map(|r| r[0] + r[1] - 0.001 * r[2]).collect();

    let rows: Vec<TrialRow> = (0..n)
        .map(|i| {
            let mut param_display = HashMap::new();
            for (j, name) in param_names.iter().enumerate() {
                param_display.insert(name.clone(), data[i][j]);
            }
            TrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display,
                param_category_label: HashMap::new(),
                objective_values: vec![objective[i]],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            }
        })
        .collect();

    let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
    store_dataframes(vec![df]);
    select_study(0).unwrap();

    // Param 空間 (3 特徴) で中心化のみ / 標準化 (相関行列) 双方を検証する。
    let raw = run_pca(3, PcaSpace::Param).unwrap();
    let standardized = run_pca_standardized(3, PcaSpace::Param).unwrap();

    let out = serde_json::json!({
        "data": data,
        "n": n,
        "raw": {
            "explained_variance": raw.explained_variance,
            "explained_ratio": raw.explained_ratio,
            "loadings": raw.loadings,
            "projections": raw.projections,
        },
        "standardized": {
            "explained_variance": standardized.explained_variance,
            "explained_ratio": standardized.explained_ratio,
            "loadings": standardized.loadings,
            "projections": standardized.projections,
        },
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
