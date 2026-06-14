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
        // 優先行なし（priority = &[]）で従来どおりの一様誘導点選択にデリゲートする。
        Self::fit_impl(x, y, method, max_inducing, seed, &[])
    }

    /// パレートフロント等の優先行に誘導点を集中させて学習する。
    ///
    /// `priority` は誘導点として優先する行 index（`x` への index）。N > max_inducing の
    /// ときのみ効果がある（N ≤ max_inducing では Z = X で全点を使うため変化しない）。
    pub(crate) fn fit_front_focused(
        x: &[Vec<f64>],
        y: &[f64],
        method: GpMethod,
        max_inducing: usize,
        seed: u64,
        priority: &[usize],
    ) -> Option<Self> {
        Self::fit_impl(x, y, method, max_inducing, seed, priority)
    }

    /// `fit` / `fit_front_focused` の共通実装。`priority` を誘導点選択に通す。
    fn fit_impl(
        x: &[Vec<f64>],
        y: &[f64],
        method: GpMethod,
        max_inducing: usize,
        seed: u64,
        priority: &[usize],
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

        // 誘導点: N ≤ M なら訓練点そのもの（Z = X）、それ以外は priority を考慮して選ぶ。
        let z = if n <= max_inducing {
            x_arr.clone()
        } else {
            select_inducing_points(x, n_dims, max_inducing, priority, seed)?
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

/// 2 点が全次元で一致するか（誘導点の重複判定用、厳密一致）。
fn rows_equal(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x == y)
}

/// N > max_inducing のときの誘導点を構築する（純粋・テスト可能）。
///
/// `priority` はパレートフロント等で優先する行 index。重複・範囲外は除去する。
/// 三つの場合に分かれる:
/// - P が空: 全行に対する k-means(max_inducing)（従来動作）。
/// - |P| ≥ max_inducing: 優先行のみに対する k-means(max_inducing)（フロントに完全集中）。
/// - 0 < |P| < max_inducing: 優先行を全て誘導点に採用し、残り枠を非優先行の
///   k-means で埋める（空間を粗くカバー）。非優先 centroid が優先点と一致する
///   場合は重複除去するため、最終個数が max_inducing をわずかに下回ることがある。
///
/// 戻り値は (誘導点数, n_dims) の column-major 互換 `Array2`。失敗時は `None`。
fn select_inducing_points(
    x: &[Vec<f64>],
    n_dims: usize,
    max_inducing: usize,
    priority: &[usize],
    seed: u64,
) -> Option<Array2<f64>> {
    let n = x.len();
    let _ = seed; // k-means は固定シード相当（決定論的）。署名統一のため受け取る。

    // 優先行を重複・範囲外除去して一意な点として取り出す。
    let mut priority_rows: Vec<usize> = Vec::new();
    let mut seen = vec![false; n];
    for &idx in priority {
        if idx < n && !seen[idx] {
            seen[idx] = true;
            priority_rows.push(idx);
        }
    }

    // 全行 k-means のヘルパ（従来動作）。
    let kmeans_over = |rows: &[Vec<f64>], k: usize| -> Option<Vec<Vec<f64>>> {
        if rows.is_empty() || k == 0 {
            return None;
        }
        let flat: Vec<f64> = rows.iter().flatten().copied().collect();
        let result = run_kmeans(k, &flat, n_dims, InitStrategy::KMeansPlusPlus);
        if result.centroids.is_empty() {
            None
        } else {
            Some(result.centroids)
        }
    };

    let centroids: Vec<Vec<f64>> = if priority_rows.is_empty() {
        // P が空: 従来動作（全行 k-means）。
        kmeans_over(x, max_inducing)?
    } else if priority_rows.len() >= max_inducing {
        // |P| ≥ M: 優先行のみに対する k-means でフロントに完全集中。
        let p_points: Vec<Vec<f64>> = priority_rows.iter().map(|&i| x[i].clone()).collect();
        kmeans_over(&p_points, max_inducing)?
    } else {
        // 0 < |P| < M: 優先行を全採用し、残りを非優先行の k-means で補う。
        let mut points: Vec<Vec<f64>> = priority_rows.iter().map(|&i| x[i].clone()).collect();
        let remaining = max_inducing - points.len();
        let non_priority: Vec<Vec<f64>> =
            (0..n).filter(|i| !seen[*i]).map(|i| x[i].clone()).collect();
        if remaining > 0 {
            if let Some(fill) = kmeans_over(&non_priority, remaining) {
                // 優先点と一致する centroid は重複除去する（二重計上を避ける）。
                for c in fill {
                    if !points.iter().any(|p| rows_equal(p, &c)) {
                        points.push(c);
                    }
                }
            }
        }
        points
    };

    if centroids.is_empty() {
        return None;
    }
    Some(Array2::from_shape_fn(
        (centroids.len(), n_dims),
        |(j, d)| centroids[j][d],
    ))
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
        // N <= max_inducing → Z = X（厳密 GP 相当）。
        // GP の当てはめ品質は egobox 側の責務なので最小限の確認に留める。
        // FITC / VFE 両方の SparseMethod 分岐を小さい N で一度だけ通す。
        let (x, y) = make_data(40, 3, 42);
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
        // N > max_inducing → k-means 誘導点。誘導点選択はメソッド非依存（dispatch 前に
        // 走る）ため FITC 一つで足りる。N は max_inducing=100 をわずかに超えれば
        // 誘導点経路を通る（巨大 N は不要）。
        let (x, y) = make_data(150, 2, 7);
        let model = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42).expect("fit should succeed");
        let pred = model.predict_mean_batch(&x);
        assert!(pred.iter().all(|v| v.is_finite()));
        assert!(r_squared(&y, &pred) > 0.8);
    }

    #[test]
    fn variance_positive_away_from_data() {
        let (x, y) = make_data(40, 2, 11);
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
        // 決定性は自前の責務（シード固定）なので 3 方式とも確認するが、N は小さくてよい。
        let (x, y) = make_data(60, 2, 3);
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
        // ノイズ推定つき SGP を使うという自前の設計選択を検証する（補間 GP を使うと
        // 過適合する）。挙動は SparseMethod 非依存なので代表として FITC のみ確認する。
        let (x_full, y) = make_data(120, 5, 7);
        let x_2col: Vec<Vec<f64>> = x_full.iter().map(|r| r[..2].to_vec()).collect();
        let model = GpModel::fit(&x_2col, &y, GpMethod::Fitc, 100, 42).expect("fit should succeed");
        let pred = model.predict_mean_batch(&x_2col);
        let r2 = r_squared(&y, &pred);
        assert!(
            r2 < 0.95,
            "noisy projection should not be interpolated: R²={r2}"
        );
        assert!(r2 > 0.0, "should still capture the signal: R²={r2}");
    }

    #[test]
    fn moe_fits_piecewise_function_well() {
        let (x, y) = make_piecewise(100, 11);
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
        let x: Vec<Vec<f64>> = (0..50).map(|_| vec![next(), next()]).collect();
        let y: Vec<f64> = x.iter().map(|r| 3.0 * r[0] + 0.05 * r[1]).collect();

        // SGP の θ 受け渡しは SparseMethod 非依存なので代表として FITC のみ確認する。
        let model = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42).expect("fit");
        let theta = model.ard_theta().expect("SGP should expose theta");
        assert_eq!(theta.len(), 2);
        assert!(theta.iter().all(|t| t.is_finite() && *t > 0.0));
        // x0 に敏感 ⇒ θ_0 が大きい（長さスケールが短い）
        assert!(theta[0] > theta[1], "theta={theta:?}");

        // MoE は None
        let moe = GpModel::fit(&x, &y, GpMethod::Moe, 100, 42).expect("MoE fit");
        assert!(moe.ard_theta().is_none());
    }

    #[test]
    fn model_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GpModel>();
    }

    // ────────────────────────────────────────────────────────────
    // 誘導点のフロント集中（select_inducing_points / fit_front_focused）
    // ────────────────────────────────────────────────────────────

    #[test]
    fn select_inducing_points_includes_priority_rows() {
        // N=200, M=50, 優先 10 行 → 優先 10 点を含み、総数 ≤ 50。
        let (x, _) = make_data(200, 2, 5);
        let priority: Vec<usize> = vec![0, 3, 7, 11, 20, 33, 55, 88, 120, 199];
        let z = select_inducing_points(&x, 2, 50, &priority, 42).expect("should select");
        assert!(z.nrows() <= 50, "count {} should be ≤ 50", z.nrows());
        // 優先各点が誘導点の行として（厳密一致で）存在する。
        for &p in &priority {
            let found = (0..z.nrows()).any(|r| (0..2).all(|d| z[[r, d]] == x[p][d]));
            assert!(found, "priority row {p} should be an inducing point");
        }
        assert!(z.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn select_inducing_points_priority_exceeds_budget() {
        // 優先 80 行 (> M=50) → 結果 ≤ 50 行、全て優先行集合の凸範囲内（k-means 制限）。
        let (x, _) = make_data(200, 2, 9);
        let priority: Vec<usize> = (0..80).collect();
        let z = select_inducing_points(&x, 2, 50, &priority, 42).expect("should select");
        assert!(z.nrows() <= 50, "count {} should be ≤ 50", z.nrows());
        assert!(z.iter().all(|v| v.is_finite()));
        // 各 centroid は優先行のみの k-means なので、各次元が優先行の範囲内にある。
        for d in 0..2 {
            let lo = priority
                .iter()
                .map(|&i| x[i][d])
                .fold(f64::INFINITY, f64::min);
            let hi = priority
                .iter()
                .map(|&i| x[i][d])
                .fold(f64::NEG_INFINITY, f64::max);
            for r in 0..z.nrows() {
                assert!(
                    z[[r, d]] >= lo - 1e-9 && z[[r, d]] <= hi + 1e-9,
                    "centroid out of priority range"
                );
            }
        }
    }

    #[test]
    fn select_inducing_points_empty_priority_behaves_as_before() {
        // 優先なし → 従来動作（全行 k-means、≤ M 行）。
        let (x, _) = make_data(200, 2, 13);
        let z = select_inducing_points(&x, 2, 50, &[], 42).expect("should select");
        assert!(z.nrows() <= 50);
        assert!(z.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn fit_front_focused_trains_and_is_deterministic() {
        // N=80, M=50, 少数の優先行で学習・予測が有限、2 回で決定論的。
        let (x, y) = make_data(80, 2, 21);
        let priority: Vec<usize> = vec![0, 5, 10, 17, 42];
        let m1 =
            GpModel::fit_front_focused(&x, &y, GpMethod::Fitc, 50, 42, &priority).expect("fit 1");
        let m2 =
            GpModel::fit_front_focused(&x, &y, GpMethod::Fitc, 50, 42, &priority).expect("fit 2");
        let probe = vec![vec![0.3, 0.7], vec![0.9, 0.1]];
        let p1 = m1.predict_mean_batch(&probe);
        let p2 = m2.predict_mean_batch(&probe);
        assert!(p1.iter().all(|v| v.is_finite()));
        assert_eq!(p1, p2, "front-focused fit should be deterministic");
    }

    #[test]
    fn fit_front_focused_equals_fit_when_n_le_max_inducing() {
        // N ≤ M では Z = X となり priority は無視される（fit と同一）。
        let (x, y) = make_data(40, 2, 3);
        let with_priority =
            GpModel::fit_front_focused(&x, &y, GpMethod::Fitc, 100, 42, &[0, 1, 2]).expect("fit");
        let plain = GpModel::fit(&x, &y, GpMethod::Fitc, 100, 42).expect("fit");
        let probe = vec![vec![0.4, 0.6]];
        assert_eq!(
            with_priority.predict_mean_batch(&probe),
            plain.predict_mean_batch(&probe),
            "priority must not change result when N ≤ max_inducing"
        );
    }
}
