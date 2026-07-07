//! NSGA-II（Deb et al., 2002）。
//!
//! 遺伝子は正規化空間 [0,1]^d の実数ベクトル。遺伝オペレータは
//! SBX 交叉（原論文どおり交叉ペアの全変数へ適用）・Polynomial Mutation・
//! 二項トーナメント選択（crowded comparison）。
//! 適応度は `Vec<f64>`（全目的を最小化）で扱う汎用実装のため、
//! 単一目的サロゲートでは長さ 1、多目的サロゲートでは目的数 n の長さで呼ぶ。
//! `optimizers::multi_objective_nsga2` が多目的サロゲートの呼び出しを担う。

use crate::math::rng::SeededRng;
use rayon::prelude::*;

/// 単目的最適化での SBX 分布指数 η_c（局所探索寄り）。
const SBX_ETA_SINGLE_OBJECTIVE: f64 = 20.0;
/// 多目的最適化での SBX 分布指数 η_c（フロント全域へ広く探索）。
const SBX_ETA_MULTI_OBJECTIVE: f64 = 2.0;

pub(crate) struct Nsga2Config {
    /// 個体数（偶数に切り上げて使う）。
    pub pop_size: usize,
    pub generations: usize,
    /// SBX を行うか（ペア単位の交叉確率）。行わない場合、子は親のコピーになる。
    pub crossover_prob: f64,
    /// SBX 分布指数 η_c（単目的 20 / 多目的 2 を推奨。`for_objectives` 参照）。
    pub crossover_eta: f64,
    /// Polynomial Mutation 分布指数 η_m（遺伝子ごとの変異確率は 1/d）。
    pub mutation_eta: f64,
    pub seed: u64,
}

impl Default for Nsga2Config {
    fn default() -> Self {
        Self {
            pop_size: 64,
            generations: 120,
            crossover_prob: 0.9,
            crossover_eta: SBX_ETA_SINGLE_OBJECTIVE,
            mutation_eta: 20.0,
            seed: 42,
        }
    }
}

impl Nsga2Config {
    /// 目的数に応じた推奨設定。
    /// η_c は単目的では 20（最良解近傍の局所改善を優先）、多目的では 2
    /// （親から離れた子を作りやすくしフロント全域をカバー）とする。
    pub fn for_objectives(n_obj: usize) -> Self {
        Self {
            crossover_eta: if n_obj <= 1 {
                SBX_ETA_SINGLE_OBJECTIVE
            } else {
                SBX_ETA_MULTI_OBJECTIVE
            },
            ..Default::default()
        }
    }
}

/// [0,1]^d 上で `eval` の返す全目的を最小化し、最終世代の第一フロント
/// `(遺伝子, 適応度)` を返す。`initial` の個体は初期集団へシードされる。
pub(crate) fn nsga2_minimize<F>(
    eval: F,
    n_dims: usize,
    initial: &[Vec<f64>],
    cfg: &Nsga2Config,
) -> Vec<(Vec<f64>, Vec<f64>)>
where
    F: Fn(&[f64]) -> Vec<f64> + Sync,
{
    let n = (cfg.pop_size.max(4) + 1) & !1; // 偶数化（最低 4）
    let mut rng = SeededRng::from_seed(cfg.seed);

    let mut pop: Vec<Vec<f64>> = initial
        .iter()
        .filter(|g| g.len() == n_dims)
        .take(n)
        .cloned()
        .collect();
    while pop.len() < n {
        pop.push((0..n_dims).map(|_| rng.next_f64()).collect());
    }
    // 集団評価は RNG を使わない純粋なサロゲート予測なので rayon で並列化する
    // （par_iter は入力順を保つため決定性は保たれる）。
    let mut fit: Vec<Vec<f64>> = pop.par_iter().map(|g| eval(g)).collect();

    for _ in 0..cfg.generations {
        // 親集団のランクと混雑度（トーナメント選択用）。
        let fronts = fast_non_dominated_sort(&fit);
        let (ranks, crowd) = ranks_and_crowding(&fit, &fronts);

        // ── 子集団の生成 ────────────────────────────────────────────
        let mut offspring: Vec<Vec<f64>> = Vec::with_capacity(n);
        while offspring.len() < n {
            let p1 = tournament(&mut rng, n, &ranks, &crowd);
            let p2 = tournament(&mut rng, n, &ranks, &crowd);
            let (mut c1, mut c2) = sbx_crossover(
                &mut rng,
                &pop[p1],
                &pop[p2],
                cfg.crossover_prob,
                cfg.crossover_eta,
            );
            polynomial_mutation(&mut rng, &mut c1, cfg.mutation_eta);
            polynomial_mutation(&mut rng, &mut c2, cfg.mutation_eta);
            offspring.push(c1);
            if offspring.len() < n {
                offspring.push(c2);
            }
        }
        let offspring_fit: Vec<Vec<f64>> = offspring.par_iter().map(|g| eval(g)).collect();

        // ── 環境選択（親子 2n からエリート n を残す） ────────────────
        pop.extend(offspring);
        fit.extend(offspring_fit);
        let fronts = fast_non_dominated_sort(&fit);
        let mut survivors: Vec<usize> = Vec::with_capacity(n);
        for front in &fronts {
            if survivors.len() + front.len() <= n {
                survivors.extend(front.iter().copied());
            } else {
                // 最後のフロントは混雑度の大きい順に切り詰める。
                let cd = crowding_distance(&fit, front);
                let mut order: Vec<usize> = (0..front.len()).collect();
                order.sort_by(|&a, &b| {
                    cd[b]
                        .partial_cmp(&cd[a])
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                for &k in &order {
                    if survivors.len() < n {
                        survivors.push(front[k]);
                    }
                }
                break;
            }
        }
        pop = survivors.iter().map(|&i| pop[i].clone()).collect();
        fit = survivors.iter().map(|&i| fit[i].clone()).collect();
    }

    let fronts = fast_non_dominated_sort(&fit);
    fronts
        .first()
        .map(|front| {
            front
                .iter()
                .map(|&i| (pop[i].clone(), fit[i].clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// a が b をパレート支配するか（全目的最小化）。
fn dominates(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y) && a.iter().zip(b).any(|(x, y)| x < y)
}

/// 高速非劣ソート。フロントごとの個体 index リストを返す（先頭が第一フロント）。
fn fast_non_dominated_sort(fit: &[Vec<f64>]) -> Vec<Vec<usize>> {
    let n = fit.len();
    let mut dominated_by: Vec<Vec<usize>> = vec![Vec::new(); n]; // i が支配する個体
    let mut domination_count = vec![0usize; n]; // i を支配する個体数
    for i in 0..n {
        for j in (i + 1)..n {
            if dominates(&fit[i], &fit[j]) {
                dominated_by[i].push(j);
                domination_count[j] += 1;
            } else if dominates(&fit[j], &fit[i]) {
                dominated_by[j].push(i);
                domination_count[i] += 1;
            }
        }
    }
    let mut fronts: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = (0..n).filter(|&i| domination_count[i] == 0).collect();
    while !current.is_empty() {
        let mut next: Vec<usize> = Vec::new();
        for &i in &current {
            for &j in &dominated_by[i] {
                domination_count[j] -= 1;
                if domination_count[j] == 0 {
                    next.push(j);
                }
            }
        }
        fronts.push(std::mem::replace(&mut current, next));
    }
    fronts
}

/// フロント内の混雑距離（front と同じ並びで返す）。境界個体は +∞。
fn crowding_distance(fit: &[Vec<f64>], front: &[usize]) -> Vec<f64> {
    let len = front.len();
    let mut dist = vec![0.0f64; len];
    if len <= 2 {
        return vec![f64::INFINITY; len];
    }
    let n_obj = fit[front[0]].len();
    // 目的ごとにフロント内の値を列として取り出してから距離を累積する。
    let columns: Vec<Vec<f64>> = (0..n_obj)
        .map(|m| front.iter().map(|&i| fit[i][m]).collect())
        .collect();
    for values in &columns {
        let mut order: Vec<usize> = (0..len).collect();
        order.sort_by(|&a, &b| {
            values[a]
                .partial_cmp(&values[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let f_min = values[order[0]];
        let f_max = values[order[len - 1]];
        dist[order[0]] = f64::INFINITY;
        dist[order[len - 1]] = f64::INFINITY;
        let range = f_max - f_min;
        if range <= f64::EPSILON {
            continue;
        }
        for w in 1..len - 1 {
            dist[order[w]] += (values[order[w + 1]] - values[order[w - 1]]) / range;
        }
    }
    dist
}

/// 全個体のランク（フロント番号）と混雑距離を返す。
fn ranks_and_crowding(fit: &[Vec<f64>], fronts: &[Vec<usize>]) -> (Vec<usize>, Vec<f64>) {
    let n = fit.len();
    let mut ranks = vec![0usize; n];
    let mut crowd = vec![0.0f64; n];
    for (rank, front) in fronts.iter().enumerate() {
        let cd = crowding_distance(fit, front);
        for (k, &i) in front.iter().enumerate() {
            ranks[i] = rank;
            crowd[i] = cd[k];
        }
    }
    (ranks, crowd)
}

/// 二項トーナメント選択（crowded comparison: ランク優先、同ランクは混雑度大を選ぶ）。
fn tournament(rng: &mut SeededRng, n: usize, ranks: &[usize], crowd: &[f64]) -> usize {
    let a = rng.next_usize(n);
    let b = rng.next_usize(n);
    if ranks[a] < ranks[b] || (ranks[a] == ranks[b] && crowd[a] > crowd[b]) {
        a
    } else {
        b
    }
}

/// SBX（Simulated Binary Crossover、Deb & Agrawal 1995）。
/// 交叉確率 `prob` を満たしたペアでは全変数に β 混合を適用する
/// （β は変数ごとに独立にサンプリング）。子は [0,1] にクランプする。
fn sbx_crossover(
    rng: &mut SeededRng,
    p1: &[f64],
    p2: &[f64],
    prob: f64,
    eta: f64,
) -> (Vec<f64>, Vec<f64>) {
    let mut c1 = p1.to_vec();
    let mut c2 = p2.to_vec();
    if rng.next_f64() > prob {
        return (c1, c2);
    }
    for d in 0..p1.len() {
        let (x1, x2) = (p1[d], p2[d]);
        if (x1 - x2).abs() <= 1e-12 {
            continue;
        }
        // next_f64 は [0,1) を返すため 1−u > 0 が保証される。
        let u = rng.next_f64();
        let beta = if u <= 0.5 {
            (2.0 * u).powf(1.0 / (eta + 1.0))
        } else {
            (1.0 / (2.0 * (1.0 - u))).powf(1.0 / (eta + 1.0))
        };
        c1[d] = (0.5 * ((1.0 + beta) * x1 + (1.0 - beta) * x2)).clamp(0.0, 1.0);
        c2[d] = (0.5 * ((1.0 - beta) * x1 + (1.0 + beta) * x2)).clamp(0.0, 1.0);
    }
    (c1, c2)
}

/// Polynomial Mutation（変異確率は遺伝子ごとに 1/d）。[0,1] にクランプする。
fn polynomial_mutation(rng: &mut SeededRng, genome: &mut [f64], eta: f64) {
    if genome.is_empty() {
        return;
    }
    let pm = 1.0 / genome.len() as f64;
    for g in genome.iter_mut() {
        if rng.next_f64() > pm {
            continue;
        }
        let u = rng.next_f64();
        let delta = if u < 0.5 {
            (2.0 * u).powf(1.0 / (eta + 1.0)) - 1.0
        } else {
            1.0 - (2.0 * (1.0 - u)).powf(1.0 / (eta + 1.0))
        };
        *g = (*g + delta).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominates_requires_strict_improvement() {
        assert!(dominates(&[1.0, 1.0], &[2.0, 1.0]));
        assert!(!dominates(&[1.0, 1.0], &[1.0, 1.0]));
        assert!(!dominates(&[1.0, 2.0], &[2.0, 1.0])); // 非劣関係
    }

    #[test]
    fn non_dominated_sort_splits_fronts() {
        // f0: (1,1) が (2,2) を支配。 (0,3) と (1,1) は非劣。
        let fit = vec![vec![2.0, 2.0], vec![1.0, 1.0], vec![0.0, 3.0]];
        let fronts = fast_non_dominated_sort(&fit);
        assert_eq!(fronts.len(), 2);
        let mut first = fronts[0].clone();
        first.sort_unstable();
        assert_eq!(first, vec![1, 2]);
        assert_eq!(fronts[1], vec![0]);
    }

    #[test]
    fn crowding_distance_boundaries_are_infinite() {
        let fit = vec![vec![0.0], vec![0.5], vec![1.0]];
        let front = vec![0, 1, 2];
        let cd = crowding_distance(&fit, &front);
        assert!(cd[0].is_infinite());
        assert!(cd[2].is_infinite());
        assert!(cd[1].is_finite());
    }

    #[test]
    fn sbx_and_mutation_stay_in_unit_box() {
        let mut rng = SeededRng::from_seed(3);
        for _ in 0..200 {
            let p1: Vec<f64> = (0..4).map(|_| rng.next_f64()).collect();
            let p2: Vec<f64> = (0..4).map(|_| rng.next_f64()).collect();
            let (mut c1, c2) = sbx_crossover(&mut rng, &p1, &p2, 0.9, 15.0);
            polynomial_mutation(&mut rng, &mut c1, 20.0);
            for v in c1.iter().chain(c2.iter()) {
                assert!((0.0..=1.0).contains(v), "out of box: {v}");
            }
        }
    }

    #[test]
    fn nsga2_minimizes_sphere_single_objective() {
        let cfg = Nsga2Config {
            pop_size: 32,
            generations: 80,
            ..Default::default()
        };
        let front = nsga2_minimize(
            |x| vec![(x[0] - 0.25).powi(2) + (x[1] - 0.75).powi(2)],
            2,
            &[],
            &cfg,
        );
        let best = front
            .iter()
            .min_by(|a, b| a.1[0].partial_cmp(&b.1[0]).unwrap())
            .unwrap();
        assert!(
            (best.0[0] - 0.25).abs() < 0.05,
            "x ≈ 0.25, got {}",
            best.0[0]
        );
        assert!(
            (best.0[1] - 0.75).abs() < 0.05,
            "y ≈ 0.75, got {}",
            best.0[1]
        );
    }

    #[test]
    fn config_selects_eta_by_objective_count() {
        assert_eq!(Nsga2Config::for_objectives(1).crossover_eta, 20.0);
        assert_eq!(Nsga2Config::for_objectives(2).crossover_eta, 2.0);
        assert_eq!(Nsga2Config::for_objectives(3).crossover_eta, 2.0);
    }

    #[test]
    fn nsga2_two_objective_front_spans_tradeoff() {
        // Schaffer N.1 相当: f1 = x², f2 = (x−1)²（x ∈ [0,1]）。
        // 第一フロントはトレードオフ全域に広がるはず（多目的設定 η_c = 2）。
        let cfg = Nsga2Config {
            pop_size: 32,
            generations: 60,
            ..Nsga2Config::for_objectives(2)
        };
        let front = nsga2_minimize(|x| vec![x[0].powi(2), (x[0] - 1.0).powi(2)], 1, &[], &cfg);
        assert!(front.len() > 5, "front too small: {}", front.len());
        let min_x = front
            .iter()
            .map(|(g, _)| g[0])
            .fold(f64::INFINITY, f64::min);
        let max_x = front
            .iter()
            .map(|(g, _)| g[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(min_x < 0.2, "front should reach x≈0: {min_x}");
        assert!(max_x > 0.8, "front should reach x≈1: {max_x}");
    }
}
