//! ロバスト性解析: 学習済みサロゲート上での入力ノイズのモンテカルロ伝播。
//!
//! 候補設計点の周りに入力ノイズ（1σ = 相対ノイズレベル × 各パラメータの
//! 正規化箱レンジ、分布は正規 / 一様 / Weibull から選択）を与え、サロゲートの
//! 予測を通した出力分布・制約充足率・仕様限界に対する成功確率（σ レベル・
//! Cpk 換算付き）を推定する。理論的背景は
//! theory/{en,ja}/optimization/robustness-analysis.md。

use super::feasibility;
use super::TrainedSurrogate;
use crate::math::rng::SeededRng;
use crate::math::special::{ln_gamma, norm_ppf};

/// 入力ノイズの分布形。
///
/// いずれも **平均 0・分散 1 に標準化した変量**を `1σ = relative_sigma × レンジ`
/// でスケールして中心に加える。したがって分布を切り替えても入力ノイズの
/// 標準偏差は同一で、形状（裾・歪み）だけが変わり、比較が公平になる。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoiseDistribution {
    /// 正規分布 N(0, 1)。
    Normal,
    /// 一様分布 U(-√3, √3)（分散 1）。
    Uniform,
    /// Weibull 分布（形状 k）を平均 0・分散 1 に標準化したもの。
    /// k < 3.6 で右に歪み、k ≈ 3.6 でほぼ対称になる。素材強度や寿命など
    /// 歪んだ入力不確かさのモデル化に使う。
    Weibull {
        /// 形状パラメータ k（> 0）。
        shape: f64,
    },
}

impl NoiseDistribution {
    /// 平均 0・分散 1 の変量を 1 つサンプルする。
    fn sample_standardized(&self, rng: &mut SeededRng) -> f64 {
        match *self {
            NoiseDistribution::Normal => rng.next_gaussian(),
            NoiseDistribution::Uniform => (rng.next_f64() * 2.0 - 1.0) * 3.0_f64.sqrt(),
            NoiseDistribution::Weibull { shape } => {
                // 逆関数法: W = (-ln(1-u))^(1/k)（λ = 1）。
                // u = 1 は ln(0) になるため next_f64 の [0,1) をそのまま使う。
                let u = rng.next_f64();
                let w = (-(1.0 - u).ln()).powf(1.0 / shape);
                let (mean, std) = weibull_mean_std(shape);
                (w - mean) / std
            }
        }
    }

    /// 分布仕様の妥当性検証（Weibull の形状パラメータ範囲）。
    fn validate(&self) -> Result<(), String> {
        if let NoiseDistribution::Weibull { shape } = *self {
            // k が極端に小さいと分散が発散気味になり標準化が数値的に破綻する。
            if !(shape.is_finite() && (0.2..=20.0).contains(&shape)) {
                return Err(format!(
                    "Weibull shape must be finite and within [0.2, 20], got {shape}"
                ));
            }
        }
        Ok(())
    }
}

/// 標準 Weibull（λ = 1）の平均と標準偏差: μ = Γ(1+1/k)、σ² = Γ(1+2/k) − μ²。
fn weibull_mean_std(shape: f64) -> (f64, f64) {
    let mean = ln_gamma(1.0 + 1.0 / shape).exp();
    let m2 = ln_gamma(1.0 + 2.0 / shape).exp();
    let var = (m2 - mean * mean).max(f64::MIN_POSITIVE);
    (mean, var.sqrt())
}

/// ロバスト性解析の入力仕様。
#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessSpec {
    /// 候補設計点（元単位、`TrainedSurrogate::param_names` と同順）。
    pub center: Vec<f64>,
    /// 入力ノイズの 1σ を各パラメータのレンジに対する割合で指定する（例: 0.02 = ±2%）。
    pub relative_sigma: f64,
    /// 入力ノイズの分布形（全パラメータ共通）。
    pub distribution: NoiseDistribution,
    /// モンテカルロサンプル数。
    pub n_samples: usize,
    /// true なら GP 事後分散（認識論的不確かさ）もサンプリングに含める。
    /// 分散を提供しないモデル（Ridge 等）では無視される。
    pub include_epistemic: bool,
    /// 乱数シード（同一入力で再現可能な結果を得る）。
    pub seed: u64,
    /// 仕様下限（LSL、目的の元単位）。出力がこの値未満のサンプルは不良と数える。
    pub lower_spec: Option<f64>,
    /// 仕様上限（USL、目的の元単位）。出力がこの値超のサンプルは不良と数える。
    pub upper_spec: Option<f64>,
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
    /// 仕様限界（LSL/USL）内に収まったサンプルの割合。限界未指定なら `None`。
    pub success_rate: Option<f64>,
    /// 成功確率の片側正規換算 z = Φ⁻¹(success_rate)。
    /// 経験確率は [1/(2n), 1−1/(2n)] にクランプするため有限値になる
    /// （全数成功でも「n サンプルで観測できた範囲」以上は主張しない）。
    pub sigma_level: Option<f64>,
    /// 工程能力指数 Cpk = min((USL−μ)/3σ, (μ−LSL)/3σ)（指定側のみ）。
    /// 出力分布の標準偏差が 0 の場合は `None`。
    pub cpk: Option<f64>,
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
    spec.distribution.validate()?;
    if let (Some(lsl), Some(usl)) = (spec.lower_spec, spec.upper_spec) {
        if lsl >= usl {
            return Err(format!(
                "lower_spec ({lsl}) must be less than upper_spec ({usl})"
            ));
        }
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
            let raw = spec.center[d] + spec.distribution.sample_standardized(&mut rng) * sigma;
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

    // フェイルクローズ: 非有限の予測が混入すると mean/std/percentile と
    // 成功率がすべて静かに歪む（NaN は仕様判定の両側で false になり
    // 分母にだけ残る）ため、明示的にエラーにする。
    if samples.iter().any(|y| !y.is_finite()) {
        return Err(
            "surrogate produced non-finite predictions around this center; \
             the model may be degenerate here"
                .to_string(),
        );
    }

    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();

    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let (success_rate, sigma_level, cpk) =
        spec_metrics(&samples, mean, std, spec.lower_spec, spec.upper_spec);

    Ok(RobustnessResult {
        nominal,
        mean,
        std,
        p05: crate::statistics::quantile(&sorted, 0.05),
        median: crate::statistics::quantile(&sorted, 0.5),
        p95: crate::statistics::quantile(&sorted, 0.95),
        samples,
        feasibility_rate: has_constraints.then(|| feas_sum / n),
        clipped_fraction: clipped_count as f64 / n,
        success_rate,
        sigma_level,
        cpk,
    })
}

/// 仕様限界に対する成功確率・σ レベル・Cpk を計算する。
///
/// - `success_rate`: LSL ≤ y ≤ USL のサンプル割合（指定側のみ判定）。
/// - `sigma_level`: Φ⁻¹(成功確率)。経験確率は [1/(2n), 1−1/(2n)] にクランプ
///   するため、全数成功でも n に応じた有限値（例 n=1024 → 約 3.3σ）に留まる。
/// - `cpk`: min((USL−μ)/3σ, (μ−LSL)/3σ)。指定された側のみで min を取る。
///
/// 限界が両方 `None` なら 3 値とも `None`。
fn spec_metrics(
    samples: &[f64],
    mean: f64,
    std: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    if lower_spec.is_none() && upper_spec.is_none() {
        return (None, None, None);
    }

    let n = samples.len() as f64;
    let ok = samples
        .iter()
        .filter(|&&y| lower_spec.is_none_or(|l| y >= l) && upper_spec.is_none_or(|u| y <= u))
        .count() as f64;
    let rate = ok / n;

    // 経験確率のクランプ（0/1 で z が発散するのを防ぎ、n の情報量を保つ）。
    let half = 1.0 / (2.0 * n);
    let clamped = rate.clamp(half, 1.0 - half);
    let sigma_level = norm_ppf(clamped);

    let cpk = if std > 0.0 {
        let upper_side = upper_spec.map(|u| (u - mean) / (3.0 * std));
        let lower_side = lower_spec.map(|l| (mean - l) / (3.0 * std));
        Some(match (lower_side, upper_side) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => unreachable!("guarded above"),
        })
    } else {
        None
    };

    (Some(rate), Some(sigma_level), cpk)
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
            distribution: NoiseDistribution::Normal,
            n_samples: 256,
            include_epistemic: false,
            seed: 42,
            lower_spec: None,
            upper_spec: None,
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
    fn spec_metrics_none_without_limits() {
        let trained = train_simple(false);
        let r = robustness_analysis(&trained, &spec(vec![5.0, 0.0])).unwrap();
        assert!(r.success_rate.is_none() && r.sigma_level.is_none() && r.cpk.is_none());
    }

    #[test]
    fn success_rate_and_sigma_level_with_limits() {
        let trained = train_simple(false);
        // y = 2*x0 + x1、center (5,0) → nominal ≈ 10。
        // 上限を分布のはるか上に置けば全数成功、中央値に置けば約半数成功。
        let mut s = spec(vec![5.0, 0.0]);
        s.upper_spec = Some(1e6);
        let all_ok = robustness_analysis(&trained, &s).unwrap();
        assert_eq!(all_ok.success_rate, Some(1.0));
        // 全数成功でもクランプにより有限（n=256 → Φ⁻¹(1−1/512) ≈ 2.88）。
        let z = all_ok.sigma_level.unwrap();
        assert!(z > 2.5 && z < 3.2, "z = {z}");
        assert!(all_ok.cpk.unwrap() > 10.0, "limit far away -> huge Cpk");

        let mut s2 = spec(vec![5.0, 0.0]);
        s2.upper_spec = Some(all_ok.median);
        let half = robustness_analysis(&trained, &s2).unwrap();
        let rate = half.success_rate.unwrap();
        assert!((0.4..=0.6).contains(&rate), "rate = {rate}");
        assert!(half.sigma_level.unwrap().abs() < 0.3);
    }

    #[test]
    fn two_sided_limits_and_cpk_direction() {
        let trained = train_simple(false);
        let base = robustness_analysis(&trained, &spec(vec![5.0, 0.0])).unwrap();
        let mut s = spec(vec![5.0, 0.0]);
        // 平均を挟む2側限界。上側の方が近い → Cpk は上側で決まる。
        s.lower_spec = Some(base.mean - 10.0 * base.std);
        s.upper_spec = Some(base.mean + 2.0 * base.std);
        let r = robustness_analysis(&trained, &s).unwrap();
        let cpk = r.cpk.unwrap();
        assert!((cpk - 2.0 / 3.0).abs() < 0.05, "cpk = {cpk} (≈ 2σ/3σ)");
        // LSL >= USL は入力エラー。
        let mut bad = spec(vec![5.0, 0.0]);
        bad.lower_spec = Some(1.0);
        bad.upper_spec = Some(0.0);
        assert!(robustness_analysis(&trained, &bad).is_err());
    }

    #[test]
    fn distributions_share_scale_but_differ_in_shape() {
        let trained = train_simple(false);
        let mk = |d: NoiseDistribution| {
            let mut s = spec(vec![5.0, 0.0]);
            s.distribution = d;
            s.n_samples = 4096;
            robustness_analysis(&trained, &s).unwrap()
        };
        let normal = mk(NoiseDistribution::Normal);
        let uniform = mk(NoiseDistribution::Uniform);
        let weibull = mk(NoiseDistribution::Weibull { shape: 1.5 });

        // 標準化により出力分布の std はどの分布でもほぼ同じ
        // （y は線形 → 出力 std = |係数| ノルム × 入力 std）。
        let ratio_u = uniform.std / normal.std;
        let ratio_w = weibull.std / normal.std;
        assert!((0.9..=1.1).contains(&ratio_u), "uniform ratio = {ratio_u}");
        assert!(
            (0.85..=1.15).contains(&ratio_w),
            "weibull ratio = {ratio_w}"
        );

        // 一様分布は裾が有限: 正規より極値が小さい。
        let max_dev_n = normal
            .samples
            .iter()
            .map(|y| (y - normal.mean).abs())
            .fold(0.0, f64::max);
        let max_dev_u = uniform
            .samples
            .iter()
            .map(|y| (y - uniform.mean).abs())
            .fold(0.0, f64::max);
        assert!(max_dev_u < max_dev_n, "{max_dev_u} vs {max_dev_n}");
    }

    #[test]
    fn weibull_shape_out_of_range_errors() {
        let trained = train_simple(false);
        let mut s = spec(vec![5.0, 0.0]);
        s.distribution = NoiseDistribution::Weibull { shape: 0.0 };
        assert!(robustness_analysis(&trained, &s).is_err());
        s.distribution = NoiseDistribution::Weibull { shape: f64::NAN };
        assert!(robustness_analysis(&trained, &s).is_err());
    }

    #[test]
    fn weibull_standardization_is_zero_mean_unit_std() {
        // sample_standardized の標準化検証（サロゲート不要の純サンプリング）。
        for shape in [0.8, 1.5, 3.6, 8.0] {
            let dist = NoiseDistribution::Weibull { shape };
            let mut rng = SeededRng::from_seed(7);
            let n = 200_000;
            let mut sum = 0.0;
            let mut sq = 0.0;
            for _ in 0..n {
                let v = dist.sample_standardized(&mut rng);
                sum += v;
                sq += v * v;
            }
            let mean = sum / n as f64;
            let std = (sq / n as f64 - mean * mean).sqrt();
            assert!(mean.abs() < 0.02, "shape {shape}: mean = {mean}");
            assert!((std - 1.0).abs() < 0.03, "shape {shape}: std = {std}");
        }
    }

    #[test]
    fn percentile_linear_interpolation() {
        // 分位点は共通の statistics::quantile（NumPy type-7）へ委譲する。
        let sorted = vec![0.0, 1.0, 2.0, 3.0];
        assert_eq!(crate::statistics::quantile(&sorted, 0.5), 1.5);
        assert_eq!(crate::statistics::quantile(&sorted, 0.0), 0.0);
        assert_eq!(crate::statistics::quantile(&sorted, 1.0), 3.0);
    }
}
