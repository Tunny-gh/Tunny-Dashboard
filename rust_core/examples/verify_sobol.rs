//! Python (SALib) との Sobol感度分析クロスチェック用ハーネス。
//!
//! `sensitivity::sobol::compute_sobol_from_df` は生の目的関数値を直接
//! Saltelli型サンプリングするのではなく、学習データに2次特徴量（各パラメータの
//! 1次・2乗・全ペアの交互作用項）の Ridge 回帰サロゲートを内部で fit し、その
//! サロゲートを A/B/AB 行列で評価して Sobol 指数を Monte Carlo 推定する。
//! SALib と同一のサンプル・同一の目的関数を直接比較することは pub API の制約上
//! できないため、本ハーネスは 2 ケースで検証する。
//!
//! - Case 1（`quadratic_exact`）: 2次特徴量の空間で厳密に表現できる関数
//!   （線形3項 + 1交互作用項）。学習データを十分多く取ればサロゲートはほぼ
//!   厳密にフィットする（r_squared ≈ 1）ため、Sobol 推定量そのものの正しさを
//!   サロゲート近似誤差と切り離して検証できる。ANOVA分解から導いた解析解と比較する。
//! - Case 2（`ishigami`）: 4次項を含む標準テスト関数（Ishigami, a=7, b=0.1）。
//!   2次までしか表現できないサロゲートには本質的に近似誤差が生じるため、
//!   SALib・解析解との数値一致は期待しない。サロゲートの r_squared と併記し、
//!   「推定量は正しいがサロゲート経由である」という実装上の制約を記録する目的で実行する。
//!
//! 実行: `cargo run -p tunny-core --example verify_sobol`

use std::collections::HashMap;
use tunny_core::dataframe::{DataFrame, TrialRow};
use tunny_core::sensitivity::compute_sobol_from_df;

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
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_f64() * (hi - lo)
    }
}

fn build_df(param_names: &[&str], obj_name: &str, x: &[Vec<f64>], y: &[f64]) -> DataFrame {
    let rows: Vec<TrialRow> = x
        .iter()
        .zip(y.iter())
        .enumerate()
        .map(|(i, (xi, &yi))| TrialRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: param_names
                .iter()
                .zip(xi.iter())
                .map(|(&name, &v)| (name.to_string(), v))
                .collect(),
            param_category_label: HashMap::new(),
            objective_values: vec![yi],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();

    DataFrame::from_trials(
        &rows,
        &param_names
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        &[obj_name.to_string()],
        &[],
        &[],
        0,
    )
}

fn main() {
    let mut rng = Rng(0x5EED_50B0_1234_5678);

    // ---- Case 1: quadratic-exact function ----
    // f(x1,x2,x3) = c1*x1 + c2*x2 + c3*x3 + c12*x1*x2 (no noise).
    // This lies exactly in the surrogate's feature space (x_i, x_i^2, x_i*x_j),
    // so with enough training rows the ridge surrogate fits it almost exactly.
    let (c1, c2, c3, c12) = (3.0, 2.0, -1.0, 1.5);
    let n1 = 3000;
    let x1_matrix: Vec<Vec<f64>> = (0..n1)
        .map(|_| {
            vec![
                rng.uniform(-2.0, 3.0),
                rng.uniform(0.0, 4.0),
                rng.uniform(-1.0, 1.0),
            ]
        })
        .collect();
    let y1: Vec<f64> = x1_matrix
        .iter()
        .map(|r| c1 * r[0] + c2 * r[1] + c3 * r[2] + c12 * r[0] * r[1])
        .collect();
    let df1 = build_df(&["x1", "x2", "x3"], "y", &x1_matrix, &y1);
    let n_samples_1 = 200_000;
    let sobol1 = compute_sobol_from_df(&df1, n_samples_1).expect("case1 sobol");

    // ---- Case 2: Ishigami function (a=7, b=0.1), domain [-pi, pi]^3 ----
    let (a, b) = (7.0, 0.1);
    let pi = std::f64::consts::PI;
    let n2 = 3000;
    let x2_matrix: Vec<Vec<f64>> = (0..n2)
        .map(|_| {
            vec![
                rng.uniform(-pi, pi),
                rng.uniform(-pi, pi),
                rng.uniform(-pi, pi),
            ]
        })
        .collect();
    let y2: Vec<f64> = x2_matrix
        .iter()
        .map(|r| r[0].sin() + a * r[1].sin().powi(2) + b * r[2].powi(4) * r[0].sin())
        .collect();
    let df2 = build_df(&["x1", "x2", "x3"], "y", &x2_matrix, &y2);
    let n_samples_2 = 200_000;
    let sobol2 = compute_sobol_from_df(&df2, n_samples_2).expect("case2 sobol");

    let out = serde_json::json!({
        "case1_quadratic_exact": {
            "true_coeffs": {"c1": c1, "c2": c2, "c3": c3, "c12": c12},
            "x_matrix": x1_matrix,
            "y": y1,
            "n_samples": n_samples_1,
            "param_names": sobol1.param_names,
            "first_order": sobol1.first_order,
            "total_effect": sobol1.total_effect,
            "r_squared": sobol1.r_squared,
        },
        "case2_ishigami": {
            "a": a,
            "b": b,
            "x_matrix": x2_matrix,
            "y": y2,
            "n_samples": n_samples_2,
            "param_names": sobol2.param_names,
            "first_order": sobol2.first_order,
            "total_effect": sobol2.total_effect,
            "r_squared": sobol2.r_squared,
        },
    });

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
