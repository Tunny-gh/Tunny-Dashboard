//! ガウス過程回帰（egobox-gp / egobox-moe バックエンド）。
//!
//! 単一 GP は FITC / VFE スパース近似（[`egobox_gp::SparseGaussianProcess`]、
//! Matérn 5/2 ARD、ノイズ分散推定つき）、混合エキスパート（MoE）は
//! [`egobox_moe::GpMixture`]（SparseGp エキスパート）で実装する。
//!
//! egobox の通常 GP（`GaussianProcess`）はノイズ分散を推定しない補間器であり、
//! 高次元データの一部の列だけで学習する PDP 用途（他次元の変動がノイズとして
//! 現れる）には適さないため使わない。N ≤ max_inducing では誘導点 Z = X となり、
//! FITC / VFE はノイズ推定つきの厳密 GP と数学的に等価になるため、機能としての
//! Full GP はこの経路でカバーされる。
//!
//! - FITC / VFE: 誘導点 M = min(N, max_inducing)。N が大きい場合は k-means
//!   誘導点（M=100 で厳密解とほぼ同一の θ・ノイズ推定が得られることを検証済み）。
//! - MoE: 入力空間を GMM でクラスタリングし、クラスタごとに FITC エキスパートを
//!   学習して滑らかに再結合する。クラスタ数は最大 500 点のサブサンプルに対する
//!   交差検証（最大 3）で決める。エキスパートの誘導点には明示的な `Located` を
//!   渡す（egobox-moe 0.35 はエキスパートへのシード伝播に `Option<u64>` の乱数を
//!   使っており、`Randomized` だと非決定論的になるため）。
//!
//! 学習は決定論的（k-means は固定シード、egobox の θ 多点スタートは固定格子、
//! SGP / MoE の乱数シードは明示指定）。

use egobox_gp::{
    correlation_models::Matern52Corr, Inducings, ParamTuning, SparseGaussianProcess, SparseMethod,
};
use egobox_moe::{
    find_best_number_of_clusters, CorrelationSpec, GpMixture, GpMixtureParams, GpType, NbClusters,
    Recombination, RegressionSpec,
};
use linfa::prelude::*;
use ndarray::{Array1, Array2, Axis};
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;

use crate::clustering::{run_kmeans, InitStrategy};

/// ガウス過程の学習方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GpMethod {
    /// FITC（Fully Independent Training Conditional）近似。
    Fitc,
    /// VFE（Variational Free Energy）近似。FITC よりノイズを保守的に見積もる傾向。
    Vfe,
    /// 混合エキスパート（クラスタごとの FITC GP を滑らかに再結合）。
    Moe,
}

/// ノイズ分散の探索下限の候補（y は z-score 正規化済み＝分散 1 が前提）。
///
/// egobox のデフォルト下限（~1e-14）ではノイズゼロの滑らかな関数で共分散行列が
/// 正定値性を失い学習がパニックする。1e-6 は分散 1 に対して -120dB で予測バイアスは
/// 実質ゼロのまま行列の条件数を改善する。それでも失敗した場合は 1e-3 まで持ち上げて
/// 再試行する（わずかに平滑化されるが、学習失敗で何も表示できないよりよい）。
const NOISE_FLOORS: [f64; 2] = [1e-6, 1e-3];

/// MoE のクラスタ数探索に使うサブサンプルの上限。
/// 探索は k-fold 交差検証で O(N) の繰り返しが重く、N=1000 で約 10 秒かかるため
/// 500 点に制限する（N=500 までなら約 1.5 秒）。
const MOE_CLUSTER_SEARCH_MAX_N: usize = 500;

/// MoE の最大クラスタ数。
const MOE_MAX_CLUSTERS: usize = 3;

/// 学習済みガウス過程モデル。
pub(crate) struct GpModel {
    inner: GpInner,
    n_dims: usize,
}

enum GpInner {
    Sgp(Box<SparseGaussianProcess<f64, Matern52Corr>>),
    Moe(Box<GpMixture>),
}

impl GpModel {
    /// ガウス過程を学習する。
    ///
    /// - `x`: 訓練入力（行 = サンプル）。正規化済みであること（[0,1]^d を想定）。
    /// - `y`: 訓練目的値（z-score 正規化済みを想定）。
    /// - `method`: 学習方式（FITC / VFE / MoE）。
    /// - `max_inducing`: 誘導点数の上限 M。N ≤ M なら Z = X（厳密 GP 相当）。
    /// - `seed`: 内部乱数のシード（再現性のため固定する）。
    ///
    /// 学習失敗（数値的破綻・入力不正）時は `None`。MoE はフォールバックしない
    /// （呼び出し側が明示的に別方式へフォールバックする）。
    pub(crate) fn fit(
        x: &[Vec<f64>],
        y: &[f64],
        method: GpMethod,
        max_inducing: usize,
        seed: u64,
    ) -> Option<Self> {
        let n = y.len();
        let n_dims = x.first()?.len();
        if n < 3 || x.len() != n || n_dims == 0 || max_inducing == 0 {
            return None;
        }
        if x.iter().any(|row| row.len() != n_dims) {
            return None;
        }

        let x_arr = Array2::from_shape_fn((n, n_dims), |(i, d)| x[i][d]);
        let y_arr = Array1::from_iter(y.iter().copied());

        // 誘導点: N ≤ M なら訓練点そのもの（Z = X）、それ以外は k-means 中心。
        let z = if n <= max_inducing {
            x_arr.clone()
        } else {
            let flat: Vec<f64> = x.iter().flatten().copied().collect();
            let result = run_kmeans(max_inducing, &flat, n_dims, InitStrategy::KMeansPlusPlus);
            if result.centroids.is_empty() {
                return None;
            }
            Array2::from_shape_fn((result.centroids.len(), n_dims), |(j, d)| {
                result.centroids[j][d]
            })
        };

        let inner = match method {
            GpMethod::Fitc => Self::fit_sgp(&x_arr, &y_arr, &z, SparseMethod::Fitc, seed)?,
            GpMethod::Vfe => Self::fit_sgp(&x_arr, &y_arr, &z, SparseMethod::Vfe, seed)?,
            GpMethod::Moe => Self::fit_moe(&x_arr, &y_arr, &z, seed)?,
        };

        // 数値的破綻の検出: 訓練点での予測が有限であること
        let model = GpModel { inner, n_dims };
        let check = model.predict_mean_batch(&x[..1]);
        if check.iter().any(|v| !v.is_finite()) {
            return None;
        }
        Some(model)
    }

    /// 単一 SGP（FITC / VFE）を学習する。
    fn fit_sgp(
        x: &Array2<f64>,
        y: &Array1<f64>,
        z: &Array2<f64>,
        sparse_method: SparseMethod,
        seed: u64,
    ) -> Option<GpInner> {
        let dataset = Dataset::new(x.clone(), y.clone());
        // egobox-gp may panic (e.g. NotPositiveDefinite in COBYLA loop) instead of
        // returning Err for ill-conditioned data.  Catch such panics and treat them
        // as a training failure, then retry with a higher noise floor before
        // giving up (→ None), so callers can fall back gracefully.
        let sgp = NOISE_FLOORS.iter().find_map(|&floor| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                SparseGaussianProcess::<f64, Matern52Corr>::params(
                    Matern52Corr::default(),
                    Inducings::Located(z.clone()),
                )
                .sparse_method(sparse_method)
                .noise_variance(ParamTuning::Optimized {
                    init: 1e-2_f64.max(floor),
                    bounds: (floor, 1e2),
                })
                .seed(Some(seed))
                .fit(&dataset)
            }))
            .ok()
            .and_then(|r| r.ok())
        })?;
        Some(GpInner::Sgp(Box::new(sgp)))
    }

    /// 混合エキスパート GP を学習する。
    ///
    /// クラスタ数は最大 [`MOE_CLUSTER_SEARCH_MAX_N`] 点の等間隔サブサンプルに
    /// 対する交差検証で選ぶ（上限 [`MOE_MAX_CLUSTERS`]）。エキスパートは
    /// FITC SGP（誘導点は全データの k-means / Z=X を共有）。
    ///
    /// MoE エキスパートのノイズ分散下限は egobox-moe が外部公開していないため
    /// 設定できない。ノイズゼロのデータでは学習がパニックし得るが、
    /// `catch_unwind` で捕捉して `None` を返す。
    fn fit_moe(x: &Array2<f64>, y: &Array1<f64>, z: &Array2<f64>, seed: u64) -> Option<GpInner> {
        let n = x.nrows();

        // クラスタ数の探索（等間隔サブサンプル、決定論的）
        let n_clusters = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (x_sub, y_sub) = if n > MOE_CLUSTER_SEARCH_MAX_N {
                let idx: Vec<usize> = (0..MOE_CLUSTER_SEARCH_MAX_N)
                    .map(|j| j * n / MOE_CLUSTER_SEARCH_MAX_N)
                    .collect();
                (x.select(Axis(0), &idx), y.select(Axis(0), &idx))
            } else {
                (x.clone(), y.clone())
            };
            let (k, _recombination) = find_best_number_of_clusters(
                &x_sub,
                &y_sub,
                MOE_MAX_CLUSTERS,
                None,
                RegressionSpec::CONSTANT,
                CorrelationSpec::MATERN52,
                Xoshiro256Plus::seed_from_u64(seed),
            );
            k.max(1)
        }))
        .unwrap_or(1);

        let dataset = Dataset::new(x.clone(), y.clone());
        let moe = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            GpMixtureParams::<f64>::new_with_rng(
                GpType::SparseGp {
                    sparse_method: SparseMethod::Fitc,
                    inducings: Inducings::Located(z.clone()),
                },
                Xoshiro256Plus::seed_from_u64(seed),
            )
            .n_clusters(NbClusters::fixed(n_clusters))
            .recombination(Recombination::Smooth(None))
            .correlation_spec(CorrelationSpec::MATERN52)
            .fit(&dataset)
        }))
        .ok()
        .and_then(|r| r.ok())?;
        Some(GpInner::Moe(Box::new(moe)))
    }

    /// 事後平均の生予測（エラー時 None）。SGP と MoE のエラー型を吸収する。
    fn predict_raw(&self, x: &Array2<f64>) -> Option<Array1<f64>> {
        match &self.inner {
            GpInner::Sgp(sgp) => sgp.predict(x).ok(),
            GpInner::Moe(moe) => moe.predict(x).ok(),
        }
    }

    /// 事後分散の生予測（エラー時 None）。
    fn predict_var_raw(&self, x: &Array2<f64>) -> Option<Array1<f64>> {
        match &self.inner {
            GpInner::Sgp(sgp) => sgp.predict_var(x).ok(),
            GpInner::Moe(moe) => moe.predict_var(x).ok(),
        }
    }

    /// 複数点の事後平均を一括予測する。
    pub(crate) fn predict_mean_batch(&self, rows: &[Vec<f64>]) -> Vec<f64> {
        let x = Array2::from_shape_fn((rows.len(), self.n_dims), |(i, d)| rows[i][d]);
        match self.predict_raw(&x) {
            Some(mean) => mean.to_vec(),
            None => vec![f64::NAN; rows.len()],
        }
    }

    /// 複数点の事後分散を一括予測する（負値は 0 にクランプ）。
    pub(crate) fn predict_variance_batch(&self, rows: &[Vec<f64>]) -> Vec<f64> {
        let x = Array2::from_shape_fn((rows.len(), self.n_dims), |(i, d)| rows[i][d]);
        match self.predict_var_raw(&x) {
            Some(var) => var.iter().map(|v| v.max(0.0)).collect(),
            None => vec![f64::NAN; rows.len()],
        }
    }

    /// 1 点の事後平均を予測する。
    pub(crate) fn predict_mean(&self, x: &[f64]) -> f64 {
        let arr = Array2::from_shape_fn((1, self.n_dims), |(_, d)| x[d]);
        self.predict_raw(&arr).map(|m| m[0]).unwrap_or(f64::NAN)
    }

    /// 1 点の事後分散を予測する（負値は 0 にクランプ）。
    pub(crate) fn predict_variance(&self, x: &[f64]) -> f64 {
        let arr = Array2::from_shape_fn((1, self.n_dims), |(_, d)| x[d]);
        self.predict_var_raw(&arr)
            .map(|v| v[0].max(0.0))
            .unwrap_or(f64::NAN)
    }

    /// ARD 相関パラメータ θ（正規化 [0,1] 入力上、入力次元ごとに 1 個）を返す。
    ///
    /// egobox / SMT の規約では θ_d が大きいほど次元 d の長さスケールが短く、
    /// サロゲートはその次元に敏感になる。単一 SGP（FITC / VFE）のみ Some を返す。
    /// MoE はエキスパートごとに θ を持ち、集約が一意でないため None を返す。
    pub(crate) fn ard_theta(&self) -> Option<Vec<f64>> {
        match &self.inner {
            GpInner::Sgp(sgp) => Some(sgp.theta().to_vec()),
            GpInner::Moe(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 滑らかなテスト関数でデータを作る（決定論的な擬似乱数）。
    fn make_data(n: usize, d: usize, seed: u64) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut state = seed;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let x: Vec<Vec<f64>> = (0..n).map(|_| (0..d).map(|_| next()).collect()).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|row| {
                row.iter().map(|v| (v - 0.5) * (v - 0.5)).sum::<f64>()
                    + 0.01 * (row[0] * 20.0).sin()
            })
            .collect();
        (x, y)
    }

    /// 不連続（区分的）なテスト関数。MoE が単一 GP より有利なケース。
    fn make_piecewise(n: usize, seed: u64) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut state = seed;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let x: Vec<Vec<f64>> = (0..n).map(|_| vec![next(), next()]).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|row| {
                if row[0] < 0.5 {
                    (row[0] * 6.0).sin() + row[1]
                } else {
                    5.0 + (row[0] * 3.0).cos() - 2.0 * row[1]
                }
            })
            .collect();
        (x, y)
    }

    fn r_squared(y: &[f64], pred: &[f64]) -> f64 {
        let mean = y.iter().sum::<f64>() / y.len() as f64;
        let ss_res: f64 = y.iter().zip(pred).map(|(a, b)| (a - b) * (a - b)).sum();
        let ss_tot: f64 = y.iter().map(|v| (v - mean) * (v - mean)).sum();
        1.0 - ss_res / ss_tot
    }

    #[test]
    fn fit_exact_path_recovers_smooth_function() {
        // N <= max_inducing → Z = X（厳密 GP 相当）
        let (x, y) = make_data(80, 3, 42);
        for method in [GpMethod::Fitc, GpMethod::Vfe] {
            let model = GpModel::fit(&x, &y, method, 100, 42).expect("fit should succeed");
            let pred = model.predict_mean_batch(&x);
            assert!(
                r_squared(&y, &pred) > 0.7,
                "{method:?}: exact GP should fit smooth function well: R²={}",
                r_squared(&y, &pred)
            );
        }
    }

    #[test]
    fn fit_inducing_path_handles_large_n() {
        // N > max_inducing → k-means 誘導点
        let (x, y) = make_data(500, 2, 7);
        for method in [GpMethod::Fitc, GpMethod::Vfe] {
            let model = GpModel::fit(&x, &y, method, 100, 42).expect("fit should succeed");
            let pred = model.predict_mean_batch(&x);
            assert!(pred.iter().all(|v| v.is_finite()));
            assert!(r_squared(&y, &pred) > 0.8, "{method:?}");
        }
    }

    #[test]
    fn variance_positive_away_from_data() {
        let (x, y) = make_data(50, 2, 11);
        let model = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42).expect("fit should succeed");
        // 訓練域の外側では分散が訓練点近傍より大きい
        let far = vec![5.0, 5.0];
        let near = x[0].clone();
        let var_far = model.predict_variance(&far);
        let var_near = model.predict_variance(&near);
        assert!(var_far.is_finite() && var_near.is_finite());
        assert!(var_far > var_near, "far={var_far}, near={var_near}");
        assert!(var_near >= 0.0);
    }

    #[test]
    fn fit_is_deterministic() {
        let (x, y) = make_data(150, 2, 3);
        for method in [GpMethod::Fitc, GpMethod::Vfe, GpMethod::Moe] {
            let m1 = GpModel::fit(&x, &y, method, 50, 42).expect("fit 1");
            let m2 = GpModel::fit(&x, &y, method, 50, 42).expect("fit 2");
            let p = vec![vec![0.3, 0.7], vec![0.9, 0.1]];
            assert_eq!(
                m1.predict_mean_batch(&p),
                m2.predict_mean_batch(&p),
                "{method:?}"
            );
            assert_eq!(
                m1.predict_variance_batch(&p),
                m2.predict_variance_batch(&p),
                "{method:?}"
            );
        }
    }

    #[test]
    fn noisy_projection_is_smoothed_not_interpolated() {
        // 5 次元関数の 2 列だけで学習 → 残り 3 次元の変動はノイズ。
        // ノイズ推定が機能していれば R² は 1.0 に張り付かない（過適合しない）。
        let (x_full, y) = make_data(300, 5, 7);
        let x_2col: Vec<Vec<f64>> = x_full.iter().map(|r| r[..2].to_vec()).collect();
        for method in [GpMethod::Fitc, GpMethod::Vfe, GpMethod::Moe] {
            let model = GpModel::fit(&x_2col, &y, method, 100, 42).expect("fit should succeed");
            let pred = model.predict_mean_batch(&x_2col);
            let r2 = r_squared(&y, &pred);
            assert!(
                r2 < 0.95,
                "{method:?}: noisy projection should not be interpolated: R²={r2}"
            );
            assert!(
                r2 > 0.0,
                "{method:?}: should still capture the signal: R²={r2}"
            );
        }
    }

    #[test]
    fn moe_fits_piecewise_function_well() {
        let (x, y) = make_piecewise(200, 11);
        let model = GpModel::fit(&x, &y, GpMethod::Moe, 100, 42).expect("MoE fit");
        let pred = model.predict_mean_batch(&x);
        let r2 = r_squared(&y, &pred);
        assert!(r2 > 0.8, "MoE should fit piecewise function: R²={r2}");
        // 分散も有限・非負で返ること
        let var = model.predict_variance_batch(&x);
        assert!(var.iter().all(|v| v.is_finite() && *v >= 0.0));
    }

    #[test]
    fn moe_handles_small_n() {
        let (x, y) = make_data(12, 2, 5);
        // 小さい N でもパニックせず Some/None を返すこと
        if let Some(model) = GpModel::fit(&x, &y, GpMethod::Moe, 100, 42) {
            let pred = model.predict_mean_batch(&x);
            assert!(pred.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn degenerate_inputs_return_none() {
        for method in [GpMethod::Fitc, GpMethod::Vfe, GpMethod::Moe] {
            // n < 3
            assert!(GpModel::fit(&[vec![0.0], vec![1.0]], &[0.0, 1.0], method, 10, 42).is_none());
            // 空入力
            assert!(GpModel::fit(&[], &[], method, 10, 42).is_none());
            // 列数不一致
            let x = vec![vec![0.0, 1.0], vec![0.5], vec![1.0, 0.0]];
            assert!(GpModel::fit(&x, &[0.0, 0.5, 1.0], method, 10, 42).is_none());
            // max_inducing = 0
            let (x, y) = make_data(10, 2, 1);
            assert!(GpModel::fit(&x, &y, method, 0, 42).is_none());
        }
    }

    #[test]
    fn duplicate_rows_do_not_break_fit() {
        // 重複点があっても（K_ZZ が特異に近くても）ノイズ推定＋nugget で学習できる
        let (mut x, mut y) = make_data(40, 2, 5);
        for i in 0..10 {
            x.push(x[i].clone());
            y.push(y[i]);
        }
        let model = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42);
        if let Some(m) = model {
            let pred = m.predict_mean_batch(&x);
            assert!(pred.iter().all(|v| v.is_finite()));
        }
        // None でもパニックしないことが要件
    }

    #[test]
    fn ard_theta_is_some_for_sgp_none_for_moe() {
        // x0 に強く依存し x1 にほぼ依存しない関数 → θ_0 > θ_1 を期待する。
        let mut state = 12345u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let x: Vec<Vec<f64>> = (0..60).map(|_| vec![next(), next()]).collect();
        let y: Vec<f64> = x.iter().map(|r| 3.0 * r[0] + 0.05 * r[1]).collect();

        for method in [GpMethod::Fitc, GpMethod::Vfe] {
            let model = GpModel::fit(&x, &y, method, 100, 42).expect("fit");
            let theta = model.ard_theta().expect("SGP should expose theta");
            assert_eq!(theta.len(), 2);
            assert!(theta.iter().all(|t| t.is_finite() && *t > 0.0));
            // x0 に敏感 ⇒ θ_0 が大きい（長さスケールが短い）
            assert!(theta[0] > theta[1], "{method:?}: theta={theta:?}");
        }

        // MoE は None
        let moe = GpModel::fit(&x, &y, GpMethod::Moe, 100, 42).expect("MoE fit");
        assert!(moe.ard_theta().is_none());
    }

    #[test]
    fn model_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GpModel>();
    }
}
