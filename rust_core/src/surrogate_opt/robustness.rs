//! ロバスト性解析: 学習済みサロゲート上での入力ノイズのモンテカルロ伝播。
//!
//! 候補設計点の周りにガウス入力ノイズ（σ = 相対ノイズレベル × 各パラメータの
//! 正規化箱レンジ）を与え、サロゲートの予測を通した出力分布と制約充足率を
//! 推定する。理論的背景は theory/{en,ja}/optimization/robustness-analysis.md。

use super::feasibility;
use super::TrainedSurrogate;
use crate::math::rng::SeededRng;

/// ロバスト性解析の入力仕様。
#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessSpec {
    /// 候補設計点（元単位、`TrainedSurrogate::param_names` と同順）。
    pub center: Vec<f64>,
    /// 入力ノイズの 1σ を各パラメータのレンジに対する割合で指定する（例: 0.02 = ±2%）。
    pub relative_sigma: f64,
    /// モンテカルロサンプル数。
    pub n_samples: usize,
    /// true なら GP 事後分散（認識論的不確かさ）もサンプリングに含める。
    /// 分散を提供しないモデル（Ridge 等）では無視される。
    pub include_epistemic: bool,
    /// 乱数シード（同一入力で再現可能な結果を得る）。
    pub seed: u64,
}

/// ロバスト性解析の結果（すべて元単位）。
#[derive(Debug, Clone)]
pub struct RobustnessResult {
    /// 候補点そのものでのサロゲート予測値。
    pub nominal: f64,
    /// 出力サンプルの経験平均。
    pub mean: f64,
    /// 出力サンプルの標準偏差（母集団分散の平方根）。
    pub std: f64,
    /// 5 パーセンタイル。
    pub p05: f64,
    /// 中央値。
    pub median: f64,
    /// 95 パーセンタイル。
    pub p95: f64,
    /// ヒストグラム描画用の出力サンプル全件。
    pub samples: Vec<f64>,
    /// 制約充足確率の推定値（制約サロゲートがない場合は `None`）。
    /// 各サンプル点での `P(全制約 ≤ 0)` をサンプル平均したもの。
    pub feasibility_rate: Option<f64>,
    /// いずれかの次元で宣言レンジ境界にクリップされたサンプルの割合。
    /// 大きい場合、報告された分布は「箱の内側に留まる」条件付き分布である。
    pub clipped_fraction: f64,
}

/// 学習済みサロゲート上で入力ノイズのモンテカルロ伝播を実行する。
///
/// `spec.center` の次元数はサロゲートのパラメータ数と一致しなければならない。
/// ノイズの 1σ は正規化箱（宣言レンジ、なければ観測レンジ）の幅に
/// `spec.relative_sigma` を掛けた値で、サンプルは箱内にクリップされる。
pub fn robustness_analysis(
    trained: &TrainedSurrogate,
    spec: &RobustnessSpec,
) -> Result<RobustnessResult, String> {
    let surrogate = &trained.surrogate;
    let n_dims = surrogate.col_stats.len();
    if spec.center.len() != n_dims {
        return Err(format!(
            "center has {} dims but surrogate expects {}",
            spec.center.len(),
            n_dims
        ));
    }
    if spec.n_samples == 0 {
        return Err("n_samples must be positive".to_string());
    }
    if !(spec.relative_sigma.is_finite() && spec.relative_sigma >= 0.0) {
        return Err("relative_sigma must be a non-negative finite value".to_string());
    }

    let mut rng = SeededRng::from_seed(spec.seed);

    let center_norm = surrogate.to_norm_x(&spec.center);
    let nominal = surrogate.to_original_y(surrogate.predict_norm(&center_norm));

    let has_constraints = !trained.constraint_models.is_empty();
    let mut samples = Vec::with_capacity(spec.n_samples);
    let mut feas_sum = 0.0;
    let mut clipped_count = 0usize;

    for _ in 0..spec.n_samples {
        // 元単位でノイズを与え、箱（col_stats の (min, range)）内にクリップする。
        let mut x = Vec::with_capacity(n_dims);
        let mut clipped = false;
        for (d, &(low, range)) in surrogate.col_stats.iter().enumerate() {
            let sigma = spec.relative_sigma * range;
            let raw = spec.center[d] + rng.next_gaussian() * sigma;
            let clamped = raw.clamp(low, low + range);
            if clamped != raw {
                clipped = true;
            }
            x.push(clamped);
        }
        if clipped {
            clipped_count += 1;
        }

        let x_norm = surrogate.to_norm_x(&x);
        let mut y_norm = surrogate.predict_norm(&x_norm);
        if spec.include_epistemic {
            if let Some(var) = surrogate.predict_var_norm(&x_norm) {
                y_norm += rng.next_gaussian() * var.max(0.0).sqrt();
            }
        }
        samples.push(surrogate.to_original_y(y_norm));

        if has_constraints {
            feas_sum += feasibility::feasibility_probability(&trained.constraint_models, &x_norm);
        }
    }

    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / n;

    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Ok(RobustnessResult {
        nominal,
        mean,
        std: var.sqrt(),
        p05: percentile(&sorted, 0.05),
        median: percentile(&sorted, 0.5),
        p95: percentile(&sorted, 0.95),
        samples,
        feasibility_rate: has_constraints.then(|| feas_sum / n),
        clipped_fraction: clipped_count as f64 / n,
    })
}

/// ソート済み配列の分位点（順序統計量間の線形補間、NumPy "linear" と同方式）。
fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let h = (sorted.len() - 1) as f64 * q;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    sorted[lo] + (h - h.floor()) * (sorted[hi] - sorted[lo])
}

#[cfg(test)]
mod tests {
    use super::super::{fit_surrogate_with_validation, SurrogateFitRequest};
    use super::*;
    use crate::surrogate_opt::{ConstraintData, SurrogateModelKind};

    /// 2 変数の単純な訓練データでサロゲートを学習する（結線検証用の最小構成）。
    fn train_simple(with_constraint: bool) -> TrainedSurrogate {
        let n = 20;
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                vec![t * 10.0, 5.0 - t * 10.0]
            })
            .collect();
        let y: Vec<f64> = x_matrix.iter().map(|r| r[0] * 2.0 + r[1]).collect();
        let constraints = if with_constraint {
            // x0 - 5 <= 0 相当の線形制約値
            vec![ConstraintData {
                name: "c0".to_string(),
                values: x_matrix.iter().map(|r| r[0] - 5.0).collect(),
            }]
        } else {
            vec![]
        };
        let req = SurrogateFitRequest {
            x_matrix,
            y,
            param_names: vec!["x0".to_string(), "x1".to_string()],
            objective_name: "obj".to_string(),
            model: SurrogateModelKind::Ridge,
            auto_select: false,
            constraints,
            priority_rows: vec![],
            param_bounds: Some(vec![Some((0.0, 10.0)), Some((-5.0, 5.0))]),
        };
        fit_surrogate_with_validation(&req).expect("fit")
    }

    fn spec(center: Vec<f64>) -> RobustnessSpec {
        RobustnessSpec {
            center,
            relative_sigma: 0.05,
            n_samples: 256,
            include_epistemic: false,
            seed: 42,
        }
    }

    #[test]
    fn same_seed_is_reproducible() {
        let trained = train_simple(false);
        let s = spec(vec![5.0, 0.0]);
        let a = robustness_analysis(&trained, &s).unwrap();
        let b = robustness_analysis(&trained, &s).unwrap();
        assert_eq!(a.samples, b.samples);
        assert_eq!(a.mean, b.mean);
    }

    #[test]
    fn zero_noise_collapses_to_nominal() {
        let trained = train_simple(false);
        let mut s = spec(vec![5.0, 0.0]);
        s.relative_sigma = 0.0;
        let r = robustness_analysis(&trained, &s).unwrap();
        assert!(r.samples.iter().all(|&y| (y - r.nominal).abs() < 1e-9));
        assert!(r.std < 1e-9);
        assert_eq!(r.clipped_fraction, 0.0);
    }

    #[test]
    fn result_shape_and_ordering() {
        let trained = train_simple(false);
        let r = robustness_analysis(&trained, &spec(vec![5.0, 0.0])).unwrap();
        assert_eq!(r.samples.len(), 256);
        assert!(r.p05 <= r.median && r.median <= r.p95);
        assert!(r.feasibility_rate.is_none(), "no constraints -> None");
    }

    #[test]
    fn feasibility_rate_present_with_constraints() {
        let trained = train_simple(true);
        // 制約 x0 - 5 <= 0: x0=1 は余裕で充足、x0=9 はほぼ違反
        let feas = robustness_analysis(&trained, &spec(vec![1.0, 0.0])).unwrap();
        let infeas = robustness_analysis(&trained, &spec(vec![9.0, 0.0])).unwrap();
        let f = feas.feasibility_rate.expect("Some with constraints");
        let i = infeas.feasibility_rate.expect("Some with constraints");
        assert!((0.0..=1.0).contains(&f) && (0.0..=1.0).contains(&i));
        assert!(f > i, "feasible center must score higher: {f} vs {i}");
    }

    #[test]
    fn center_near_bound_reports_clipping() {
        let trained = train_simple(false);
        let mut s = spec(vec![0.0, 0.0]); // x0 が下限ちょうど
        s.relative_sigma = 0.10;
        let r = robustness_analysis(&trained, &s).unwrap();
        assert!(r.clipped_fraction > 0.0, "half the noise should clip");
    }

    #[test]
    fn dimension_mismatch_errors() {
        let trained = train_simple(false);
        let err = robustness_analysis(&trained, &spec(vec![5.0])).unwrap_err();
        assert!(err.contains("dims"));
    }

    #[test]
    fn surface_slice_at_returns_expected_grid() {
        // train_simple を再利用するためここに置く（surface_slice_at の結線確認）。
        let trained = train_simple(false);
        let slice =
            crate::surrogate_opt::surface_slice_at(&trained, &[5.0, 0.0], 0, 1, 10).unwrap();
        assert_eq!(slice.x_values.len(), 10);
        assert_eq!(slice.y_values.len(), 10);
        assert_eq!(slice.z_values.len(), 10);
        assert!(slice.z_values.iter().all(|row| row.len() == 10));
        // 次元不一致・同一軸は None
        assert!(crate::surrogate_opt::surface_slice_at(&trained, &[5.0], 0, 1, 10).is_none());
        assert!(crate::surrogate_opt::surface_slice_at(&trained, &[5.0, 0.0], 0, 0, 10).is_none());
    }

    #[test]
    fn percentile_linear_interpolation() {
        let sorted = vec![0.0, 1.0, 2.0, 3.0];
        assert_eq!(percentile(&sorted, 0.5), 1.5);
        assert_eq!(percentile(&sorted, 0.0), 0.0);
        assert_eq!(percentile(&sorted, 1.0), 3.0);
    }
}
