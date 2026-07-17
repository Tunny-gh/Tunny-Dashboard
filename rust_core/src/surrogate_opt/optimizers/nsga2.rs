//! NSGA-II (Deb et al., 2002).
//!
//! A genome is a real-valued vector in normalized space [0,1]^d. Genetic
//! operators are SBX crossover (applied to all variables of a crossed pair,
//! as in the original paper), polynomial mutation, and binary tournament
//! selection (crowded comparison).
//! This is a generic implementation that treats fitness as `Vec<f64>`
//! (minimizing all objectives), so it's called with length 1 for
//! single-objective surrogates and length n (the number of objectives) for
//! multi-objective surrogates.
//! `optimizers::multi_objective_nsga2` handles calls for multi-objective
//! surrogates.

use crate::math::rng::SeededRng;
use crate::multi_objective::pareto::dominates_minimized;
use rayon::prelude::*;

/// SBX distribution index η_c for single-objective optimization (favors local search).
const SBX_ETA_SINGLE_OBJECTIVE: f64 = 20.0;
/// SBX distribution index η_c for multi-objective optimization (broad exploration
/// across the whole front).
const SBX_ETA_MULTI_OBJECTIVE: f64 = 2.0;

pub(crate) struct Nsga2Config {
    /// Population size (rounded up to an even number).
    pub pop_size: usize,
    pub generations: usize,
    /// Whether to perform SBX (per-pair crossover probability). If not, the
    /// offspring are copies of the parents.
    pub crossover_prob: f64,
    /// SBX distribution index η_c (recommended: 20 for single-objective, 2 for
    /// multi-objective; see `for_objectives`).
    pub crossover_eta: f64,
    /// Polynomial mutation distribution index η_m (per-gene mutation
    /// probability is 1/d).
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
    /// Recommended settings based on the number of objectives.
    /// η_c is 20 for single-objective (favors local improvement near the best
    /// solution) and 2 for multi-objective (makes it easier to produce
    /// offspring further from their parents, covering the whole front).
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

/// Minimizes all objectives returned by `eval` over [0,1]^d, and returns the
/// final generation's first front as `(genome, fitness)` pairs. Individuals in
/// `initial` are seeded into the initial population.
pub(crate) fn nsga2_minimize<F>(
    eval: F,
    n_dims: usize,
    initial: &[Vec<f64>],
    cfg: &Nsga2Config,
) -> Vec<(Vec<f64>, Vec<f64>)>
where
    F: Fn(&[f64]) -> Vec<f64> + Sync,
{
    let n = (cfg.pop_size.max(4) + 1) & !1; // Round up to even (minimum 4)
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
    // Population evaluation is pure surrogate prediction with no RNG use, so
    // we parallelize it with rayon (par_iter preserves input order, so
    // determinism is maintained).
    let mut fit: Vec<Vec<f64>> = pop.par_iter().map(|g| eval(g)).collect();

    for _ in 0..cfg.generations {
        // Parent population's ranks and crowding distance (for tournament selection).
        let fronts = fast_non_dominated_sort(&fit);
        let (ranks, crowd) = ranks_and_crowding(&fit, &fronts);

        // ── Generate offspring population ───────────────────────────
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

        // ── Environmental selection (keep the elite n out of the combined 2n parents+offspring) ──
        pop.extend(offspring);
        fit.extend(offspring_fit);
        let fronts = fast_non_dominated_sort(&fit);
        let mut survivors: Vec<usize> = Vec::with_capacity(n);
        for front in &fronts {
            if survivors.len() + front.len() <= n {
                survivors.extend(front.iter().copied());
            } else {
                // Truncate the last front by descending crowding distance.
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

/// Fast non-dominated sort. Returns a list of individual indices per front
/// (the first element is the first front).
///
/// NOTE: this is the same algorithm (Fast Non-dominated Sort) as
/// `multi_objective::pareto::ranking::nd_sort`, but is intentionally kept
/// as a separate implementation. This one is a hot path called every
/// generation on a small population (default `pop_size = 64`, at most 128
/// individuals including parents and offspring) inside NSGA-II's
/// generation loop, so the parallelization and NaN-row masking machinery
/// `nd_sort` has for large DataFrames is unnecessary overhead here; this
/// stays a simple sequential O(n^2) implementation.
fn fast_non_dominated_sort(fit: &[Vec<f64>]) -> Vec<Vec<usize>> {
    let n = fit.len();
    let mut dominated_by: Vec<Vec<usize>> = vec![Vec::new(); n]; // individuals that i dominates
    let mut domination_count = vec![0usize; n]; // number of individuals that dominate i
    for i in 0..n {
        for j in (i + 1)..n {
            if dominates_minimized(&fit[i], &fit[j]) {
                dominated_by[i].push(j);
                domination_count[j] += 1;
            } else if dominates_minimized(&fit[j], &fit[i]) {
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

/// Crowding distance within the front (returned in the same order as
/// `front`). Boundary individuals get +∞.
fn crowding_distance(fit: &[Vec<f64>], front: &[usize]) -> Vec<f64> {
    let len = front.len();
    let mut dist = vec![0.0f64; len];
    if len <= 2 {
        return vec![f64::INFINITY; len];
    }
    let n_obj = fit[front[0]].len();
    // For each objective, extract the within-front values as a column
    // before accumulating the distance.
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

/// Returns the rank (front number) and crowding distance of every individual.
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

/// Binary tournament selection (crowded comparison: rank first; on a tie,
/// pick the individual with the larger crowding distance).
fn tournament(rng: &mut SeededRng, n: usize, ranks: &[usize], crowd: &[f64]) -> usize {
    let a = rng.next_usize(n);
    let b = rng.next_usize(n);
    if ranks[a] < ranks[b] || (ranks[a] == ranks[b] && crowd[a] > crowd[b]) {
        a
    } else {
        b
    }
}

/// SBX (Simulated Binary Crossover, Deb & Agrawal 1995).
/// For a pair satisfying the crossover probability `prob`, β blending is
/// applied to all variables (β is sampled independently per variable).
/// Offspring are clamped to [0,1].
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
        // next_f64 returns [0,1), so 1-u > 0 is guaranteed.
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

/// Polynomial Mutation (mutation probability is 1/d per gene). Clamped to [0,1].
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
        assert!(dominates_minimized(&[1.0, 1.0], &[2.0, 1.0]));
        assert!(!dominates_minimized(&[1.0, 1.0], &[1.0, 1.0]));
        assert!(!dominates_minimized(&[1.0, 2.0], &[2.0, 1.0])); // non-dominated relation
    }

    #[test]
    fn non_dominated_sort_splits_fronts() {
        // f0: (1,1) dominates (2,2). (0,3) and (1,1) are non-dominated with respect to each other.
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
        // Equivalent to Schaffer N.1: f1 = x^2, f2 = (x-1)^2 (x in [0,1]).
        // The first front should span the entire tradeoff (multi-objective setting eta_c = 2).
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
