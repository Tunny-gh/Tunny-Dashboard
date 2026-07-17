//! Overall evaluation metrics (convergence indicators) for multi-objective optimization.
//!
//! In addition to Hypervolume, provides IGD+ / additive ε-indicator / R2 indicator.
//! These indicators are only meaningful as convergence measures for multi-objective
//! problems (number of objectives >= 2); they are undefined for single-objective problems.
//!
//! ## Sharing the reference set
//!
//! IGD+ / ε / R2 all require a "true Pareto front", but when analyzing the results of
//! a single Study the true front is unknown. This implementation instead fixes the
//! **non-dominated front of the union of observed points across all series (the
//! baseline Study + comparison Studies)** as the reference set, and measures convergence
//! toward it at each trial step (a self-referential convergence analysis). Sharing the
//! reference set and normalization scale across all series makes it possible to compare
//! multiple Studies on a unified metric.
//!
//! ## Space
//!
//! - Everything is computed in a normalized space unified to the minimization direction
//!   (maximization objectives have their sign flipped).
//! - IGD+ / ε / R2 scale each objective to [0, 1] using the union's ideal/nadir to make
//!   them scale-invariant.
//! - Hypervolume is computed in the normalized (sign-flip only) space to stay consistent
//!   with the existing implementation and preserve the units used for the reference-point
//!   argument; the reference point is derived from the nadir shared across all series.

use rayon::prelude::*;

use super::pareto::{add_to_pareto_front, compute_ref_point, hypervolume_nd, normalize_objectives};

/// Kinds of overall evaluation indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MoIndicator {
    /// Hypervolume (larger is better).
    Hypervolume,
    /// IGD+ (smaller is better).
    IgdPlus,
    /// additive ε-indicator (smaller is better).
    Epsilon,
    /// R2 indicator (weighted Tchebycheff, ideal-based; smaller is better).
    R2,
}

impl MoIndicator {
    /// All indicators (for enumerating the UI selector).
    pub fn all() -> [MoIndicator; 4] {
        [
            MoIndicator::Hypervolume,
            MoIndicator::IgdPlus,
            MoIndicator::Epsilon,
            MoIndicator::R2,
        ]
    }

    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            MoIndicator::Hypervolume => "Hypervolume",
            MoIndicator::IgdPlus => "IGD+",
            MoIndicator::Epsilon => "ε-indicator",
            MoIndicator::R2 => "R2",
        }
    }

    /// Whether a larger value is better (true only for Hypervolume).
    pub fn higher_is_better(self) -> bool {
        matches!(self, MoIndicator::Hypervolume)
    }
}

/// Input for one series (the baseline Study or a single comparison Study).
/// `objectives` is the trial-order vector of objective values (raw objective values, before sign flip).
pub struct SeriesInput<'a> {
    /// trial_id of each point (same order and length as `objectives`).
    pub trial_ids: &'a [u32],
    /// Trial-order vector of objective values.
    pub objectives: &'a [Vec<f64>],
}

/// Indicator trajectory for one series.
#[derive(Debug, Clone)]
pub struct IndicatorHistory {
    /// trial_id of each point.
    pub trial_ids: Vec<u32>,
    /// Indicator value trajectory (same length as `trial_ids`).
    pub values: Vec<f64>,
    /// Reference point used for the Hypervolume computation (normalized minimization space, shared across all series).
    /// Empty for indicators other than HV, or when computation is not possible.
    pub ref_point: Vec<f64>,
}

/// Computes the indicator trajectory for all series using a shared reference set and shared scale.
///
/// The return value has the same order and length as `series`. When the number of
/// objectives is fewer than 2, each series returns an empty trajectory (`values` is
/// empty), since the indicators are undefined for single-objective problems.
///
/// `hv_ref_point_override` is the HV-specific reference point (normalized minimization
/// space; maximization objectives already sign-flipped). It is used only when its
/// dimension matches and all elements are finite; otherwise it is derived automatically
/// from the shared nadir.
pub fn compute_indicator_histories(
    series: &[SeriesInput],
    is_minimize: &[bool],
    indicator: MoIndicator,
    hv_ref_point_override: Option<&[f64]>,
) -> Vec<IndicatorHistory> {
    let m = is_minimize.len();

    let empty_result = || -> Vec<IndicatorHistory> {
        series
            .iter()
            .map(|s| IndicatorHistory {
                trial_ids: s.trial_ids.to_vec(),
                values: Vec::new(),
                ref_point: Vec::new(),
            })
            .collect()
    };

    if m < 2 {
        return empty_result();
    }

    // Normalize each series to the minimization direction (sign flip only).
    let normalized: Vec<Vec<Vec<f64>>> = series
        .iter()
        .map(|s| normalize_objectives(s.objectives, is_minimize))
        .collect();

    // Gather the union of valid points (no NaN, matching dimension).
    let mut union_valid: Vec<Vec<f64>> = Vec::new();
    for norm in &normalized {
        for obj in norm {
            if obj.len() == m && !obj.iter().any(|v| v.is_nan() || v.is_infinite()) {
                union_valid.push(obj.clone());
            }
        }
    }
    if union_valid.is_empty() {
        return empty_result();
    }

    // Reference front shared across all series (non-dominated set of the union).
    let mut reference_front: Vec<Vec<f64>> = Vec::new();
    for p in &union_valid {
        add_to_pareto_front(&mut reference_front, p.clone());
    }

    // ideal / nadir shared across all series (for [0,1] scaling; computed from all points in the union).
    let mut ideal = vec![f64::INFINITY; m];
    let mut nadir = vec![f64::NEG_INFINITY; m];
    for p in &union_valid {
        for j in 0..m {
            if p[j] < ideal[j] {
                ideal[j] = p[j];
            }
            if p[j] > nadir[j] {
                nadir[j] = p[j];
            }
        }
    }
    let scale: Vec<f64> = (0..m)
        .map(|j| {
            let r = nadir[j] - ideal[j];
            if r > 0.0 {
                r
            } else {
                1.0
            }
        })
        .collect();
    let to_unit =
        |p: &[f64]| -> Vec<f64> { (0..m).map(|j| (p[j] - ideal[j]) / scale[j]).collect() };

    // Reference point for HV (shared nadir + 10% margin, or the overridden value).
    // The nadir is computed from the worst point among all valid points `union_valid`.
    // If instead the nadir of the reference front (the non-dominated set) were used, the
    // reference-point box would hug the boundary of the good solutions, and early
    // dominated trials would fail to satisfy `p[j] < ref[j]`, yielding an HV contribution
    // of 0 (making the trajectory jump abruptly near the end).
    let hv_ref_point: Vec<f64> = match hv_ref_point_override {
        Some(r) if r.len() == m && r.iter().all(|v| v.is_finite()) => r.to_vec(),
        _ => compute_ref_point(&union_valid, m),
    };

    // Scale the reference set to [0,1] (used by IGD+ / ε).
    let reference_unit: Vec<Vec<f64>> = reference_front.iter().map(|p| to_unit(p)).collect();

    // Weight vectors for R2 (generated only when the indicator is R2).
    let weights = if matches!(indicator, MoIndicator::R2) {
        simplex_lattice_weights(m)
    } else {
        Vec::new()
    };

    // For each series, accumulate the front in trial order and compute the indicator at each step.
    series
        .iter()
        .zip(normalized.iter())
        .map(|(s, norm)| {
            let n = norm.len();
            // HV incrementally updates the front in the normalized minimization space; the
            // others update the front in [0,1] space. Since the dominance relation is
            // preserved under a positive linear scale + translation (to_unit), the same
            // front set results without needing to re-map the whole front to [0,1] on every step.
            let mut current_front: Vec<Vec<f64>> = Vec::new();
            let mut values = Vec::with_capacity(n);

            for obj in norm.iter() {
                let invalid = obj.len() != m || obj.iter().any(|v| v.is_nan() || v.is_infinite());
                if invalid {
                    // Invalid points carry forward the previous value (same behavior as the HV history).
                    values.push(values.last().copied().unwrap_or(0.0));
                    continue;
                }

                let v = match indicator {
                    MoIndicator::Hypervolume => {
                        add_to_pareto_front(&mut current_front, obj.clone());
                        hypervolume_nd(&current_front, &hv_ref_point)
                    }
                    _ => {
                        add_to_pareto_front(&mut current_front, to_unit(obj));
                        match indicator {
                            MoIndicator::IgdPlus => igd_plus(&current_front, &reference_unit),
                            MoIndicator::Epsilon => {
                                additive_epsilon(&current_front, &reference_unit)
                            }
                            MoIndicator::R2 => r2_indicator(&current_front, &weights),
                            MoIndicator::Hypervolume => unreachable!(),
                        }
                    }
                };
                values.push(v);
            }

            IndicatorHistory {
                trial_ids: s.trial_ids.to_vec(),
                values,
                ref_point: if matches!(indicator, MoIndicator::Hypervolume) {
                    hv_ref_point.clone()
                } else {
                    Vec::new()
                },
            }
        })
        .collect()
}

/// IGD+ (inverted generational distance plus).
///
/// For each point z in the reference set `reference`, takes the minimum modified
/// distance d+(a, z) = sqrt(Σ max(a_j - z_j, 0)^2) over points a in the approximation
/// set `approx`, then averages. Computed in the [0,1] space assuming minimization.
/// Smaller is better.
pub fn igd_plus(approx: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    if reference.is_empty() {
        return 0.0;
    }
    if approx.is_empty() {
        return f64::INFINITY;
    }
    // Parallelize the nearest-neighbor search per reference point. To preserve
    // determinism, collect into a Vec in the reference set's original order, then
    // sum sequentially (avoids the lowest-bit jitter caused by varying addition
    // order under a parallel reduction).
    let mins: Vec<f64> = reference
        .par_iter()
        .map(|z| {
            approx
                .iter()
                .map(|a| dist_plus(a, z))
                .fold(f64::INFINITY, f64::min)
        })
        .collect();
    mins.iter().sum::<f64>() / reference.len() as f64
}

/// Modified distance d+(a, z) (only objectives where a is worse than z contribute; assumes minimization).
fn dist_plus(a: &[f64], z: &[f64]) -> f64 {
    let s: f64 = a
        .iter()
        .zip(z.iter())
        .map(|(&ai, &zi)| {
            let d = ai - zi;
            if d > 0.0 {
                d * d
            } else {
                0.0
            }
        })
        .sum();
    s.sqrt()
}

/// Unary additive ε-indicator I_ε+(A, Z).
///
/// The minimum amount ε by which the points of A must be translated so that every
/// point z in the reference set Z is weakly dominated.
/// I_ε+ = max_{z in Z} min_{a in A} max_j (a_j - z_j). Assumes minimization. Smaller is better.
pub fn additive_epsilon(approx: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    if reference.is_empty() {
        return 0.0;
    }
    if approx.is_empty() {
        return f64::INFINITY;
    }
    // Parallelize the min-max computation per reference point. max is associative
    // and introduces no rounding error, but as with igd_plus we still use an
    // order-preserving collect + sequential fold to explicitly guarantee determinism.
    let per_ref: Vec<f64> = reference
        .par_iter()
        .map(|z| {
            approx
                .iter()
                .map(|a| {
                    a.iter()
                        .zip(z.iter())
                        .map(|(&ai, &zi)| ai - zi)
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .fold(f64::INFINITY, f64::min)
        })
        .collect();
    per_ref.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// R2 indicator (weighted Tchebycheff scalarization, ideal-based).
///
/// For each weight w, takes min_{a in A} max_j w_j * a_j, then averages over all weights.
/// The ideal is taken as the origin (= 0) of the [0,1] space. Smaller is better.
pub fn r2_indicator(approx: &[Vec<f64>], weights: &[Vec<f64>]) -> f64 {
    if weights.is_empty() {
        return 0.0;
    }
    if approx.is_empty() {
        return f64::INFINITY;
    }
    let sum: f64 = weights
        .iter()
        .map(|w| {
            approx
                .iter()
                .map(|a| {
                    a.iter()
                        .zip(w.iter())
                        .map(|(&ai, &wi)| wi * ai)
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .fold(f64::INFINITY, f64::min)
        })
        .sum();
    sum / weights.len() as f64
}

/// Generates an m-dimensional simplex-lattice (Das-Dennis) set of weight vectors.
///
/// Each component is k/h (Σ = 1). Uses a small epsilon as the lower bound so that a
/// weight of 0 never makes an objective ignored. Chooses the largest h for which the
/// count C(h+m-1, m-1) stays at or below about 100 (m=2 → h=99, m=3 → h≈13).
fn simplex_lattice_weights(m: usize) -> Vec<Vec<f64>> {
    const TARGET: usize = 100;
    const EPS: f64 = 1e-6;
    if m == 0 {
        return Vec::new();
    }
    if m == 1 {
        return vec![vec![1.0]];
    }

    // Choose the largest h whose count stays within TARGET (at least 1).
    let mut h = 1usize;
    loop {
        let next = h + 1;
        if lattice_count(next, m) > TARGET {
            break;
        }
        h = next;
        if h > 10_000 {
            break;
        }
    }

    let mut result = Vec::new();
    let mut current = vec![0usize; m];
    gen_lattice(&mut result, &mut current, 0, h, m);
    // Convert k/h into weights in [eps,1] and normalize.
    result
        .into_iter()
        .map(|counts| {
            let raw: Vec<f64> = counts
                .iter()
                .map(|&c| (c as f64 / h as f64).max(EPS))
                .collect();
            let s: f64 = raw.iter().sum();
            raw.into_iter().map(|v| v / s).collect()
        })
        .collect()
}

/// Number of simplex-lattice points for h divisions in m dimensions = C(h+m-1, m-1).
fn lattice_count(h: usize, m: usize) -> usize {
    // Computes C(h+m-1, m-1) while avoiding overflow.
    let n = h + m - 1;
    let k = m - 1;
    let mut result: u128 = 1;
    for i in 0..k {
        result = result * (n - i) as u128 / (i as u128 + 1);
    }
    result.min(usize::MAX as u128) as usize
}

/// Recursively generates simplex-lattice points (non-negative integer vectors whose components sum to `total`).
fn gen_lattice(
    out: &mut Vec<Vec<usize>>,
    current: &mut Vec<usize>,
    dim: usize,
    remaining: usize,
    m: usize,
) {
    if dim == m - 1 {
        current[dim] = remaining;
        out.push(current.clone());
        return;
    }
    for k in 0..=remaining {
        current[dim] = k;
        gen_lattice(out, current, dim + 1, remaining - k, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "expected {b}, got {a}");
    }

    #[test]
    fn igd_plus_zero_when_approx_covers_reference() {
        // IGD+ = 0 when the approximation set contains the reference set (identical points).
        let reference = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let approx = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        approx_eq(igd_plus(&approx, &reference), 0.0);
    }

    #[test]
    fn igd_plus_only_counts_worse_objectives() {
        // d+ = 0 when a is better (smaller) than z in every objective.
        let reference = vec![vec![1.0, 1.0]];
        let approx = vec![vec![0.0, 0.0]];
        approx_eq(igd_plus(&approx, &reference), 0.0);
        // Only contributes when a is worse than z. z=(0,0), a=(0,1) → d+ = 1.
        let reference = vec![vec![0.0, 0.0]];
        let approx = vec![vec![0.0, 1.0]];
        approx_eq(igd_plus(&approx, &reference), 1.0);
    }

    #[test]
    fn additive_epsilon_zero_when_identical() {
        let reference = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let approx = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        approx_eq(additive_epsilon(&approx, &reference), 0.0);
    }

    #[test]
    fn additive_epsilon_translation_amount() {
        // ε=0.5 is required for a=(0.5,0.5) to weakly dominate z=(0,0).
        let reference = vec![vec![0.0, 0.0]];
        let approx = vec![vec![0.5, 0.5]];
        approx_eq(additive_epsilon(&approx, &reference), 0.5);
    }

    #[test]
    fn additive_epsilon_can_be_negative_when_dominating() {
        // a=(−0.3,−0.3) strongly weakly-dominates z=(0,0), so ε=−0.3.
        let reference = vec![vec![0.0, 0.0]];
        let approx = vec![vec![-0.3, -0.3]];
        approx_eq(additive_epsilon(&approx, &reference), -0.3);
    }

    #[test]
    fn r2_zero_at_ideal() {
        // With a solution at the ideal (= origin), the Tchebycheff value is 0 for every weight.
        let weights = simplex_lattice_weights(2);
        let approx = vec![vec![0.0, 0.0]];
        approx_eq(r2_indicator(&approx, &weights), 0.0);
    }

    #[test]
    fn r2_decreases_as_set_approaches_ideal() {
        let weights = simplex_lattice_weights(2);
        let far = vec![vec![1.0, 1.0]];
        let near = vec![vec![0.2, 0.2]];
        assert!(r2_indicator(&near, &weights) < r2_indicator(&far, &weights));
    }

    #[test]
    fn simplex_lattice_sums_to_one() {
        for m in [2usize, 3, 4] {
            let ws = simplex_lattice_weights(m);
            assert!(!ws.is_empty());
            assert!(ws.len() <= 120, "m={m} produced {} weights", ws.len());
            for w in &ws {
                assert_eq!(w.len(), m);
                let s: f64 = w.iter().sum();
                approx_eq(s, 1.0);
            }
        }
    }

    #[test]
    fn histories_shared_reference_make_series_comparable() {
        // 2 series, 2 minimization objectives. Both series are evaluated against the same reference set and scale.
        let s0_objs = vec![vec![2.0, 2.0], vec![1.0, 1.0]];
        let s0_ids = vec![0u32, 1];
        let s1_objs = vec![vec![3.0, 3.0], vec![0.0, 0.0]];
        let s1_ids = vec![0u32, 1];
        let series = vec![
            SeriesInput {
                trial_ids: &s0_ids,
                objectives: &s0_objs,
            },
            SeriesInput {
                trial_ids: &s1_ids,
                objectives: &s1_objs,
            },
        ];
        let is_min = vec![true, true];
        let hist = compute_indicator_histories(&series, &is_min, MoIndicator::IgdPlus, None);
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].values.len(), 2);
        // Series 1 eventually reaches the union's ideal (0,0), so its final IGD+ is smaller than series 0's.
        let last0 = *hist[0].values.last().unwrap();
        let last1 = *hist[1].values.last().unwrap();
        assert!(last1 <= last0);
    }

    #[test]
    fn single_objective_returns_empty_values() {
        let objs = vec![vec![1.0], vec![0.5]];
        let ids = vec![0u32, 1];
        let series = vec![SeriesInput {
            trial_ids: &ids,
            objectives: &objs,
        }];
        let hist = compute_indicator_histories(&series, &[true], MoIndicator::Hypervolume, None);
        assert_eq!(hist.len(), 1);
        assert!(hist[0].values.is_empty());
    }

    #[test]
    fn hypervolume_ref_point_bounds_all_observed_points() {
        // Regression guard: even early dominated trials should stay inside the reference-point
        // box and yield HV > 0. If the reference point were derived from the non-dominated
        // set's nadir, the dominated point ([10,10]) would fall outside the box and early HV
        // would collapse to 0, causing the trajectory to jump abruptly at the end. The
        // reference point should instead be based on the worst point among all observed points
        // ([10,10]) plus a margin.
        let objs = vec![vec![10.0, 10.0], vec![1.0, 1.0]];
        let ids = vec![0u32, 1];
        let series = vec![SeriesInput {
            trial_ids: &ids,
            objectives: &objs,
        }];
        let hist =
            compute_indicator_histories(&series, &[true, true], MoIndicator::Hypervolume, None);
        let v = &hist[0].values;
        assert_eq!(v.len(), 2);
        // Even at the 1st point (dominated only), it is contained by the reference point and HV > 0.
        assert!(
            v[0] > 0.0,
            "early dominated point should yield HV > 0, got {}",
            v[0]
        );
        assert!(v[1] > v[0]);
    }

    #[test]
    fn hypervolume_history_is_nondecreasing() {
        // HV is monotonically non-decreasing as trials progress.
        let objs = vec![vec![2.0, 2.0], vec![1.0, 2.0], vec![1.0, 1.0]];
        let ids = vec![0u32, 1, 2];
        let series = vec![SeriesInput {
            trial_ids: &ids,
            objectives: &objs,
        }];
        let hist =
            compute_indicator_histories(&series, &[true, true], MoIndicator::Hypervolume, None);
        let v = &hist[0].values;
        assert_eq!(v.len(), 3);
        assert!(v[1] >= v[0]);
        assert!(v[2] >= v[1]);
    }
}
