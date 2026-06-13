//! サロゲート曲面上の最適化手法。
//!
//! 正規化空間 [0,1]^d 内での最小化として実装する（maximize は符号反転）。
//! 新しい手法はここへバリアントを追加する。

mod cma_es;
mod nsga2;

use argmin::core::{CostFunction, Error, Gradient};

use super::models::FittedSurrogate;
use crate::math::rng::SeededRng;
use crate::optimization::LbfgsOptimizer;

/// サロゲート曲面上の最適化手法種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizerKind {
    /// 観測ベスト点＋乱数点からのマルチスタート L-BFGS（数値勾配）。
    MultiStartLbfgs,
    /// 固定シードのランダムサーチ（常に動くベースライン）。
    RandomSearch,
    /// NSGA-II（SBX 交叉・Polynomial Mutation・二項トーナメント選択）。
    Nsga2,
    /// CMA-ES（共分散行列適応進化戦略）。
    CmaEs,
}

/// マルチスタートのスタート点数（観測ベスト点 1 + 乱数 7）。
pub(crate) const N_RANDOM_STARTS: usize = 7;
/// ランダムサーチの評価点数。
const N_RANDOM_SEARCH: usize = 4096;
/// 数値勾配（中心差分）のステップ幅。
pub(crate) const FD_STEP: f64 = 1e-4;
/// 箱外ペナルティの重み。
pub(crate) const BOUND_PENALTY: f64 = 1e3;
/// 乱数シード（再現性のため固定）。
pub(crate) const SEED: u64 = 42;

/// 制約ペナルティの重み（z-score 単位）。
/// 制約違反量（正規化 z-score 超過分）に乗じるスカラー。
const CONSTRAINT_PENALTY: f64 = 100.0;

/// サロゲート曲面上で最適化し、正規化空間 [0,1]^d の最適点を返す。
/// `minimize=false`（最大化）は符号反転した曲面の最小化として扱う。
///
/// `constraint_models` が空でないとき、コスト関数に制約ペナルティを加える:
///
/// ```text
/// cost = sign * mu_y_norm(x) + CONSTRAINT_PENALTY * Σ max(0, mu_ci_norm(x) - z0_i)
/// ```
///
/// z0_i = (0 - c_mean_i) / c_std_i は実行可能境界（正規化 z-score 単位）。
pub(crate) fn minimize_on_surrogate(
    surrogate: &FittedSurrogate,
    minimize: bool,
    optimizer: OptimizerKind,
    start_norm: &[f64],
    constraint_models: &[FittedSurrogate],
) -> Vec<f64> {
    let sign = if minimize { 1.0 } else { -1.0 };

    if constraint_models.is_empty() {
        // 制約なし: 従来どおり surrogate のコストのみ最小化する。
        let t = match optimizer {
            OptimizerKind::MultiStartLbfgs => multi_start_lbfgs(surrogate, sign, start_norm),
            OptimizerKind::RandomSearch => random_search(surrogate, sign, start_norm),
            OptimizerKind::Nsga2 => run_nsga2(surrogate, sign, start_norm),
            OptimizerKind::CmaEs => run_cma_es(surrogate, sign, start_norm),
        };
        return t.iter().map(|v| v.clamp(0.0, 1.0)).collect();
    }

    // 制約あり: 汎用コスト関数 minimize_scalar_fn で最適化する。
    // z0_i = (0 - c_mean_i) / c_std_i
    let z0s: Vec<f64> = constraint_models
        .iter()
        .map(|cm| {
            if cm.y_std > 1e-12 {
                (0.0 - cm.y_mean) / cm.y_std
            } else if cm.y_mean <= 0.0 {
                f64::INFINITY // 常に実行可能
            } else {
                f64::NEG_INFINITY // 常に違反
            }
        })
        .collect();

    let constrained_cost = |t: &[f64]| -> f64 {
        let clamped: Vec<f64> = t.iter().map(|v| v.clamp(0.0, 1.0)).collect();
        let bound_pen: f64 = t
            .iter()
            .map(|&v| {
                let over = (v - 1.0).max(0.0);
                let under = (-v).max(0.0);
                over * over + under * under
            })
            .sum();
        let obj = sign * surrogate.predict_norm(&clamped);
        let con_pen: f64 = constraint_models
            .iter()
            .zip(z0s.iter())
            .map(|(cm, &z0)| CONSTRAINT_PENALTY * (cm.predict_norm(&clamped) - z0).max(0.0))
            .sum();
        obj + con_pen + BOUND_PENALTY * bound_pen
    };

    let n_dims = start_norm.len();
    let t = minimize_scalar_fn(&constrained_cost, n_dims, start_norm);
    t.iter().map(|v| v.clamp(0.0, 1.0)).collect()
}

/// 箱内にクランプした点でサロゲートを評価し、箱外には二次ペナルティを課す。
pub(crate) fn penalized_cost(surrogate: &FittedSurrogate, sign: f64, t: &[f64]) -> f64 {
    let clamped: Vec<f64> = t.iter().map(|v| v.clamp(0.0, 1.0)).collect();
    let penalty: f64 = t
        .iter()
        .map(|&v| {
            let over = (v - 1.0).max(0.0);
            let under = (-v).max(0.0);
            over * over + under * under
        })
        .sum();
    sign * surrogate.predict_norm(&clamped) + BOUND_PENALTY * penalty
}

/// 観測ベスト点＋乱数点からのマルチスタート L-BFGS。
/// 汎用化した `minimize_scalar_fn` を介してサロゲートを最小化する。
fn multi_start_lbfgs(surrogate: &FittedSurrogate, sign: f64, start_norm: &[f64]) -> Vec<f64> {
    let n_dims = start_norm.len();
    minimize_scalar_fn(&|t| penalized_cost(surrogate, sign, t), n_dims, start_norm)
}

/// 固定シードのランダムサーチ。観測ベスト点も候補に含める。
fn random_search(surrogate: &FittedSurrogate, sign: f64, start_norm: &[f64]) -> Vec<f64> {
    let n_dims = start_norm.len();
    let mut rng = SeededRng::from_seed(SEED);

    let mut best = start_norm.to_vec();
    let mut best_cost = penalized_cost(surrogate, sign, &best);

    for _ in 0..N_RANDOM_SEARCH {
        let t: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
        let cost = penalized_cost(surrogate, sign, &t);
        if cost < best_cost {
            best_cost = cost;
            best = t;
        }
    }
    best
}

/// NSGA-II をサロゲート単一目的の最小化として実行する。
/// 適応度は長さ 1 のベクトル（将来の多目的サロゲート対応に備えた汎用実装を使う）。
fn run_nsga2(surrogate: &FittedSurrogate, sign: f64, start_norm: &[f64]) -> Vec<f64> {
    // 現状は単一目的サロゲートのため η_c = 20 の設定を使う。
    let cfg = nsga2::Nsga2Config::for_objectives(1);
    let front = nsga2::nsga2_minimize(
        |t| vec![penalized_cost(surrogate, sign, t)],
        start_norm.len(),
        std::slice::from_ref(&start_norm.to_vec()),
        &cfg,
    );
    front
        .into_iter()
        .min_by(|a, b| {
            a.1[0]
                .partial_cmp(&b.1[0])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(genome, _)| genome)
        .unwrap_or_else(|| start_norm.to_vec())
}

/// CMA-ES を観測ベスト点を初期平均として実行する。
fn run_cma_es(surrogate: &FittedSurrogate, sign: f64, start_norm: &[f64]) -> Vec<f64> {
    let cfg = cma_es::CmaEsConfig::default();
    cma_es::cma_es_minimize(|t| penalized_cost(surrogate, sign, t), start_norm, &cfg)
}

/// 任意のスカラー関数 `f: [0,1]^d → f64` をマルチスタート L-BFGS で最小化する。
///
/// - `start_norm`: 提供済みの初期点（[0,1]^d 内の正規化座標）。
/// - `n_dims`: 次元数。
/// - 戻り値: `[0,1]^d` にクランプした最良点。
///
/// 内部的に `start_norm` + `N_RANDOM_STARTS` 個の固定シード乱数点からマルチスタートする。
pub(crate) fn minimize_scalar_fn(
    f: &(dyn Fn(&[f64]) -> f64 + Sync),
    n_dims: usize,
    start_norm: &[f64],
) -> Vec<f64> {
    let mut rng = SeededRng::from_seed(SEED);

    let mut starts: Vec<Vec<f64>> = vec![start_norm.to_vec()];
    for _ in 0..N_RANDOM_STARTS {
        starts.push((0..n_dims).map(|_| rng.next_f64()).collect());
    }

    /// argmin 用コスト関数（任意のクロージャをラップする）。
    struct ScalarProblem<'a> {
        f: &'a (dyn Fn(&[f64]) -> f64 + Sync),
    }

    impl CostFunction for ScalarProblem<'_> {
        type Param = Vec<f64>;
        type Output = f64;
        fn cost(&self, p: &Vec<f64>) -> Result<f64, Error> {
            Ok(penalized_fn(self.f, p))
        }
    }

    impl Gradient for ScalarProblem<'_> {
        type Param = Vec<f64>;
        type Gradient = Vec<f64>;
        fn gradient(&self, p: &Vec<f64>) -> Result<Vec<f64>, Error> {
            let mut grad = vec![0.0; p.len()];
            let mut pt = p.clone();
            for d in 0..p.len() {
                pt[d] = p[d] + FD_STEP;
                let plus = penalized_fn(self.f, &pt);
                pt[d] = p[d] - FD_STEP;
                let minus = penalized_fn(self.f, &pt);
                pt[d] = p[d];
                grad[d] = (plus - minus) / (2.0 * FD_STEP);
            }
            Ok(grad)
        }
    }

    let lbfgs = LbfgsOptimizer::new(100, 5);
    let mut best = start_norm.to_vec();
    let mut best_cost = penalized_fn(f, &best);

    for start in starts {
        let start_cost = penalized_fn(f, &start);
        if start_cost < best_cost {
            best_cost = start_cost;
            best = start.clone();
        }
        let problem = ScalarProblem { f };
        let candidate = lbfgs.optimize(start, problem);
        if candidate.iter().all(|v| v.is_finite()) {
            let cost = penalized_fn(f, &candidate);
            if cost < best_cost {
                best_cost = cost;
                best = candidate;
            }
        }
    }

    best.iter().map(|v| v.clamp(0.0, 1.0)).collect()
}

/// 箱内にクランプした点で任意関数を評価し、箱外に二次ペナルティを課す。
fn penalized_fn(f: &(dyn Fn(&[f64]) -> f64 + Sync), t: &[f64]) -> f64 {
    let clamped: Vec<f64> = t.iter().map(|v| v.clamp(0.0, 1.0)).collect();
    let penalty: f64 = t
        .iter()
        .map(|&v| {
            let over = (v - 1.0).max(0.0);
            let under = (-v).max(0.0);
            over * over + under * under
        })
        .sum();
    f(&clamped) + BOUND_PENALTY * penalty
}

/// 多目的サロゲート曲面上で NSGA-II を実行し、第一パレートフロントを返す。
///
/// - `surrogates`: 目的ごとの学習済みサロゲート（`signs[k]` が 1.0 なら最小化、-1.0 なら最大化）。
/// - `signs`: 目的ごとの符号（最小化 = 1.0、最大化 = −1.0）。
/// - `initial_seeds`: 初期集団にシードする正規化空間の点。
///
/// 戻り値は `(遺伝子, 適応度ベクトル)` のリスト（第一フロントのみ）。
pub(crate) fn multi_objective_nsga2(
    surrogates: &[&super::models::FittedSurrogate],
    signs: &[f64],
    initial_seeds: &[Vec<f64>],
) -> Vec<(Vec<f64>, Vec<f64>)> {
    let n_obj = surrogates.len();
    let cfg = nsga2::Nsga2Config::for_objectives(n_obj);
    nsga2::nsga2_minimize(
        |t| {
            signs
                .iter()
                .zip(surrogates.iter())
                .map(|(&sign, &surrogate)| penalized_cost(surrogate, sign, t))
                .collect()
        },
        surrogates[0].col_stats.len(),
        initial_seeds,
        &cfg,
    )
}
