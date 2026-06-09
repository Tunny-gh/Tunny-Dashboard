//! サロゲート曲面上の最適化手法。
//!
//! 正規化空間 [0,1]^d 内での最小化として実装する（maximize は符号反転）。
//! 新しい手法（例: Nelder-Mead / DE）はここへバリアントを追加する。

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
}

/// マルチスタートのスタート点数（観測ベスト点 1 + 乱数 7）。
const N_RANDOM_STARTS: usize = 7;
/// ランダムサーチの評価点数。
const N_RANDOM_SEARCH: usize = 4096;
/// 数値勾配（中心差分）のステップ幅。
const FD_STEP: f64 = 1e-4;
/// 箱外ペナルティの重み。
const BOUND_PENALTY: f64 = 1e3;
/// 乱数シード（再現性のため固定）。
const SEED: u64 = 42;

/// サロゲート曲面上で最適化し、正規化空間 [0,1]^d の最適点を返す。
/// `minimize=false`（最大化）は符号反転した曲面の最小化として扱う。
pub(crate) fn minimize_on_surrogate(
    surrogate: &FittedSurrogate,
    minimize: bool,
    optimizer: OptimizerKind,
    start_norm: &[f64],
) -> Vec<f64> {
    let sign = if minimize { 1.0 } else { -1.0 };
    let t = match optimizer {
        OptimizerKind::MultiStartLbfgs => multi_start_lbfgs(surrogate, sign, start_norm),
        OptimizerKind::RandomSearch => random_search(surrogate, sign, start_norm),
    };
    t.iter().map(|v| v.clamp(0.0, 1.0)).collect()
}

/// 箱内にクランプした点でサロゲートを評価し、箱外には二次ペナルティを課す。
fn penalized_cost(surrogate: &FittedSurrogate, sign: f64, t: &[f64]) -> f64 {
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

/// argmin 用のコスト関数（中心差分による数値勾配付き）。
struct SurrogateProblem<'a> {
    surrogate: &'a FittedSurrogate,
    sign: f64,
}

impl CostFunction for SurrogateProblem<'_> {
    type Param = Vec<f64>;
    type Output = f64;
    fn cost(&self, p: &Vec<f64>) -> Result<f64, Error> {
        Ok(penalized_cost(self.surrogate, self.sign, p))
    }
}

impl Gradient for SurrogateProblem<'_> {
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;
    fn gradient(&self, p: &Vec<f64>) -> Result<Vec<f64>, Error> {
        let mut grad = vec![0.0; p.len()];
        let mut pt = p.clone();
        for d in 0..p.len() {
            pt[d] = p[d] + FD_STEP;
            let plus = penalized_cost(self.surrogate, self.sign, &pt);
            pt[d] = p[d] - FD_STEP;
            let minus = penalized_cost(self.surrogate, self.sign, &pt);
            pt[d] = p[d];
            grad[d] = (plus - minus) / (2.0 * FD_STEP);
        }
        Ok(grad)
    }
}

/// 観測ベスト点＋乱数点からのマルチスタート L-BFGS。
/// 各スタートの収束点と、スタート点自体も候補に含めて最良を返す
/// （線探索が失敗してもスタート点より悪化しないことを保証する）。
fn multi_start_lbfgs(surrogate: &FittedSurrogate, sign: f64, start_norm: &[f64]) -> Vec<f64> {
    let n_dims = start_norm.len();
    let mut rng = SeededRng::from_seed(SEED);

    let mut starts: Vec<Vec<f64>> = vec![start_norm.to_vec()];
    for _ in 0..N_RANDOM_STARTS {
        starts.push((0..n_dims).map(|_| rng.next_f64()).collect());
    }

    let lbfgs = LbfgsOptimizer::new(100, 5);
    let mut best = start_norm.to_vec();
    let mut best_cost = penalized_cost(surrogate, sign, &best);

    for start in starts {
        let start_cost = penalized_cost(surrogate, sign, &start);
        if start_cost < best_cost {
            best_cost = start_cost;
            best = start.clone();
        }
        let problem = SurrogateProblem { surrogate, sign };
        let candidate = lbfgs.optimize(start, problem);
        if candidate.iter().all(|v| v.is_finite()) {
            let cost = penalized_cost(surrogate, sign, &candidate);
            if cost < best_cost {
                best_cost = cost;
                best = candidate;
            }
        }
    }
    best
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
