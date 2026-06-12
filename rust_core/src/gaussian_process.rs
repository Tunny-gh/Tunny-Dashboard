//! ガウス過程回帰（egobox-gp バックエンド）。
//!
//! 全パスを FITC スパース近似（[`egobox_gp::SparseGaussianProcess`]、Matérn 5/2 ARD）で
//! 実装する。egobox の通常 GP（`GaussianProcess`）はノイズ分散を推定しない補間器であり、
//! 高次元データの一部の列だけで学習する PDP 用途（他次元の変動がノイズとして現れる）には
//! 適さないため使わない。
//!
//! - 「通常 GP」: 誘導点 M = min(N, max_inducing)。N ≤ max_inducing なら Z = X となり
//!   FITC は厳密 GP（＋ノイズ推定）と一致する。N が大きい場合も k-means 誘導点 M=100 で
//!   厳密解とほぼ同一の θ・ノイズ推定が得られることを検証済み。
//! - 「スパース GP」: 同じ実装で max_inducing を小さくしたもの。
//!
//! 学習は決定論的（k-means は固定シード、egobox の θ 多点スタートは固定格子、
//! SGP の乱数シードは明示指定）。

use egobox_gp::{
    correlation_models::Matern52Corr, Inducings, ParamTuning, SparseGaussianProcess, SparseMethod,
};
use linfa::prelude::*;
use ndarray::{Array1, Array2};

use crate::clustering::{run_kmeans, InitStrategy};

/// ノイズ分散の探索下限の候補（y は z-score 正規化済み＝分散 1 が前提）。
///
/// egobox のデフォルト下限（~1e-14）ではノイズゼロの滑らかな関数で共分散行列が
/// 正定値性を失い学習がパニックする。1e-6 は分散 1 に対して -120dB で予測バイアスは
/// 実質ゼロのまま行列の条件数を改善する。それでも失敗した場合は 1e-3 まで持ち上げて
/// 再試行する（わずかに平滑化されるが、学習失敗で何も表示できないよりよい）。
const NOISE_FLOORS: [f64; 2] = [1e-6, 1e-3];

/// 学習済みガウス過程モデル（FITC）。
pub(crate) struct GpModel {
    sgp: SparseGaussianProcess<f64, Matern52Corr>,
    n_dims: usize,
}

impl GpModel {
    /// ガウス過程を学習する。
    ///
    /// - `x`: 訓練入力（行 = サンプル）。正規化済みであること（[0,1]^d を想定）。
    /// - `y`: 訓練目的値（z-score 正規化済みを想定）。
    /// - `max_inducing`: 誘導点数の上限 M。N ≤ M なら Z = X（厳密 GP 相当）。
    /// - `seed`: SGP 内部乱数のシード（再現性のため固定する）。
    ///
    /// 学習失敗（数値的破綻・入力不正）時は `None`。
    pub(crate) fn fit(x: &[Vec<f64>], y: &[f64], max_inducing: usize, seed: u64) -> Option<Self> {
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

        let inducings = if n <= max_inducing {
            Inducings::Located(x_arr.clone())
        } else {
            let flat: Vec<f64> = x.iter().flatten().copied().collect();
            let result = run_kmeans(max_inducing, &flat, n_dims, InitStrategy::KMeansPlusPlus);
            if result.centroids.is_empty() {
                return None;
            }
            let m = result.centroids.len();
            let z = Array2::from_shape_fn((m, n_dims), |(j, d)| result.centroids[j][d]);
            Inducings::Located(z)
        };

        let dataset = Dataset::new(x_arr, y_arr);
        // egobox-gp may panic (e.g. NotPositiveDefinite in COBYLA loop) instead of
        // returning Err for ill-conditioned data.  Catch such panics and treat them
        // as a training failure, then retry with a higher noise floor before
        // giving up (→ None), so callers can fall back gracefully.
        let sgp = NOISE_FLOORS.iter().find_map(|&floor| {
            let inducings = inducings.clone();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                SparseGaussianProcess::<f64, Matern52Corr>::params(
                    Matern52Corr::default(),
                    inducings,
                )
                .sparse_method(SparseMethod::Fitc)
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

        // 数値的破綻の検出: 訓練点での予測が有限であること
        let model = GpModel { sgp, n_dims };
        let check = model.predict_mean_batch(&x[..1.min(x.len())]);
        if check.iter().any(|v| !v.is_finite()) {
            return None;
        }
        Some(model)
    }

    /// 複数点の事後平均を一括予測する。
    pub(crate) fn predict_mean_batch(&self, rows: &[Vec<f64>]) -> Vec<f64> {
        let x = Array2::from_shape_fn((rows.len(), self.n_dims), |(i, d)| rows[i][d]);
        match self.sgp.predict(&x) {
            Ok(mean) => mean.to_vec(),
            Err(_) => vec![f64::NAN; rows.len()],
        }
    }

    /// 複数点の事後分散を一括予測する（負値は 0 にクランプ）。
    pub(crate) fn predict_variance_batch(&self, rows: &[Vec<f64>]) -> Vec<f64> {
        let x = Array2::from_shape_fn((rows.len(), self.n_dims), |(i, d)| rows[i][d]);
        match self.sgp.predict_var(&x) {
            Ok(var) => var.iter().map(|v| v.max(0.0)).collect(),
            Err(_) => vec![f64::NAN; rows.len()],
        }
    }

    /// 1 点の事後平均を予測する。
    pub(crate) fn predict_mean(&self, x: &[f64]) -> f64 {
        let arr = Array2::from_shape_fn((1, self.n_dims), |(_, d)| x[d]);
        self.sgp.predict(&arr).map(|m| m[0]).unwrap_or(f64::NAN)
    }

    /// 1 点の事後分散を予測する（負値は 0 にクランプ）。
    pub(crate) fn predict_variance(&self, x: &[f64]) -> f64 {
        let arr = Array2::from_shape_fn((1, self.n_dims), |(_, d)| x[d]);
        self.sgp
            .predict_var(&arr)
            .map(|v| v[0].max(0.0))
            .unwrap_or(f64::NAN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 5 次元の滑らかなテスト関数でデータを作る（決定論的な擬似乱数）。
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
        let model = GpModel::fit(&x, &y, 100, 42).expect("fit should succeed");
        let pred = model.predict_mean_batch(&x);
        assert!(
            r_squared(&y, &pred) > 0.8,
            "exact GP should fit smooth function well: R²={}",
            r_squared(&y, &pred)
        );
    }

    #[test]
    fn fit_inducing_path_handles_large_n() {
        // N > max_inducing → k-means 誘導点
        let (x, y) = make_data(500, 2, 7);
        let model = GpModel::fit(&x, &y, 100, 42).expect("fit should succeed");
        let pred = model.predict_mean_batch(&x);
        assert!(pred.iter().all(|v| v.is_finite()));
        assert!(r_squared(&y, &pred) > 0.8);
    }

    #[test]
    fn variance_positive_away_from_data() {
        let (x, y) = make_data(50, 2, 11);
        let model = GpModel::fit(&x, &y, 100, 42).expect("fit should succeed");
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
        let m1 = GpModel::fit(&x, &y, 50, 42).expect("fit 1");
        let m2 = GpModel::fit(&x, &y, 50, 42).expect("fit 2");
        let p = vec![vec![0.3, 0.7], vec![0.9, 0.1]];
        assert_eq!(m1.predict_mean_batch(&p), m2.predict_mean_batch(&p));
        assert_eq!(m1.predict_variance_batch(&p), m2.predict_variance_batch(&p));
    }

    #[test]
    fn noisy_projection_is_smoothed_not_interpolated() {
        // 5 次元関数の 2 列だけで学習 → 残り 3 次元の変動はノイズ。
        // ノイズ推定が機能していれば R² は 1.0 に張り付かない（過適合しない）。
        let (x_full, y) = make_data(300, 5, 7);
        let x_2col: Vec<Vec<f64>> = x_full.iter().map(|r| r[..2].to_vec()).collect();
        let model = GpModel::fit(&x_2col, &y, 100, 42).expect("fit should succeed");
        let pred = model.predict_mean_batch(&x_2col);
        let r2 = r_squared(&y, &pred);
        assert!(
            r2 < 0.95,
            "noisy projection should not be interpolated: R²={r2}"
        );
        assert!(r2 > 0.0, "should still capture the signal: R²={r2}");
    }

    #[test]
    fn degenerate_inputs_return_none() {
        // n < 3
        assert!(GpModel::fit(&[vec![0.0], vec![1.0]], &[0.0, 1.0], 10, 42).is_none());
        // 空入力
        assert!(GpModel::fit(&[], &[], 10, 42).is_none());
        // 列数不一致
        let x = vec![vec![0.0, 1.0], vec![0.5], vec![1.0, 0.0]];
        assert!(GpModel::fit(&x, &[0.0, 0.5, 1.0], 10, 42).is_none());
        // max_inducing = 0
        let (x, y) = make_data(10, 2, 1);
        assert!(GpModel::fit(&x, &y, 0, 42).is_none());
    }

    #[test]
    fn duplicate_rows_do_not_break_fit() {
        // 重複点があっても（K_ZZ が特異に近くても）ノイズ推定＋nugget で学習できる
        let (mut x, mut y) = make_data(40, 2, 5);
        for i in 0..10 {
            x.push(x[i].clone());
            y.push(y[i]);
        }
        let model = GpModel::fit(&x, &y, 100, 42);
        if let Some(m) = model {
            let pred = m.predict_mean_batch(&x);
            assert!(pred.iter().all(|v| v.is_finite()));
        }
        // None でもパニックしないことが要件
    }

    #[test]
    fn model_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GpModel>();
    }
}
