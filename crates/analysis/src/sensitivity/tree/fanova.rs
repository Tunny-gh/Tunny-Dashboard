//! fANOVA (functional ANOVA) — exact variance decomposition following Hutter et al. (2014).
//!
//! LightGBM does not provide an API that cleanly exposes leaf-node intervals (boxes),
//! so this module implements a pure-Rust CART regression tree / random forest
//! dedicated to this method.
//!
//! Algorithm overview:
//! 1. Train one CART regression tree per bootstrap sample (n-of-n sampling with replacement).
//! 2. Each tree's leaf nodes hold an axis-aligned interval (box) and the mean of y within the leaf.
//! 3. Treat the observed range over the full training data as a uniform prior, and decompose
//!    the objective's variance into per-leaf contributions using the leaf box's volume,
//!    normalized over this range, as the weight w_ℓ.
//! 4. For each dimension j, exactly compute the "main effect" variance V_j — obtained by
//!    marginalizing out all other dimensions — via piecewise quadrature, and use V_j / V as
//!    the per-tree importance.
//! 5. The forest-wide importance is the average of V_j/V across trees (only trees with
//!    positive total variance are included).
//!
//! Split candidates consider all features (no feature subsampling). Because fANOVA's main
//! effects rely on the tree's axis-aligned splits exactly covering the whole parameter space,
//! unlike Optuna's fanova (which defaults to a max_features cap), this implementation uses all
//! features so the main-effect decomposition is exact rather than approximate.

use super::common::normalize;
use crate::math::rng::SeededRng;
use crate::math::stats::value_range;
use rayon::prelude::*;

/// Leaf node: axis-aligned interval `[lo, hi]` per dimension, plus the mean of y within the leaf.
#[derive(Debug, Clone)]
pub(super) struct FanovaLeaf {
    pub(super) bounds: Vec<(f64, f64)>,
    pub(super) mean: f64,
}

/// A single CART regression tree. Only the leaf boxes and means are needed for the
/// fANOVA decomposition, so internal split nodes are discarded and only leaves are kept.
#[derive(Debug, Clone)]
pub(super) struct FanovaTree {
    pub(super) leaves: Vec<FanovaLeaf>,
}

/// Forest training parameters.
pub(super) struct FanovaConfig {
    pub(super) n_trees: usize,
    pub(super) max_depth: usize,
    pub(super) min_samples_leaf: usize,
    pub(super) seed: u64,
}

/// Read-only context shared across nodes while building a tree.
struct BuildCtx<'a> {
    x: &'a [Vec<f64>],
    y: &'a [f64],
    max_depth: usize,
    min_samples_leaf: usize,
    p: usize,
}

/// Observed range `[min, max]` per dimension over the full training data (before bootstrapping).
/// Each tree's root-node box is initialized to this range.
fn observed_ranges(x: &[Vec<f64>], p: usize) -> Vec<(f64, f64)> {
    (0..p)
        .map(|d| value_range(x.iter().map(|row| row[d])))
        .collect()
}

/// Finds the best `(feature, threshold)` to split the samples in a node.
/// Candidate split points are the midpoints between consecutive distinct sorted values
/// of each feature within the node.
/// Returns `None` if no split improves SSE (constant y, constant x, or too few samples).
fn best_split(
    indices: &[usize],
    x: &[Vec<f64>],
    y: &[f64],
    p: usize,
    min_samples_leaf: usize,
) -> Option<(usize, f64)> {
    let n = indices.len();
    if n < 2 * min_samples_leaf {
        return None;
    }
    let total_sum: f64 = indices.iter().map(|&i| y[i]).sum();
    let total_sum2: f64 = indices.iter().map(|&i| y[i] * y[i]).sum();
    let total_sse = total_sum2 - total_sum * total_sum / n as f64;
    if total_sse < 1e-12 {
        return None;
    }

    let mut best: Option<(usize, f64, f64)> = None;
    // For each of the p features we build the (x[i][d], y[i]) sequence of in-node samples,
    // so this cannot be rewritten as an iterator that enumerates x directly.
    #[allow(clippy::needless_range_loop)]
    for d in 0..p {
        let mut pairs: Vec<(f64, f64)> = indices.iter().map(|&i| (x[i][d], y[i])).collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if pairs[0].0 == pairs[n - 1].0 {
            continue; // constant feature: no split candidates
        }

        let mut left_sum = 0.0;
        let mut left_sum2 = 0.0;
        let mut left_n = 0usize;
        let mut idx = 0usize;
        while idx < n {
            let cur_x = pairs[idx].0;
            while idx < n && pairs[idx].0 == cur_x {
                left_sum += pairs[idx].1;
                left_sum2 += pairs[idx].1 * pairs[idx].1;
                left_n += 1;
                idx += 1;
            }
            if idx == n {
                break; // last value group: no right side can be formed beyond this
            }
            let right_n = n - left_n;
            if left_n < min_samples_leaf || right_n < min_samples_leaf {
                continue;
            }
            let right_sum = total_sum - left_sum;
            let right_sum2 = total_sum2 - left_sum2;
            let sse_left = (left_sum2 - left_sum * left_sum / left_n as f64).max(0.0);
            let sse_right = (right_sum2 - right_sum * right_sum / right_n as f64).max(0.0);
            let sse = sse_left + sse_right;
            let threshold = (cur_x + pairs[idx].0) / 2.0;
            // If cur_x and the next value are adjacent f64s, the midpoint rounds down to
            // cur_x, which would make the `x < threshold` split disagree with the
            // left_n/right_n counts above. Skip candidates for which threshold > cur_x
            // cannot be guaranteed.
            if threshold <= cur_x {
                continue;
            }
            if best.as_ref().is_none_or(|b| sse < b.2) {
                best = Some((d, threshold, sse));
            }
        }
    }

    best.and_then(|(feat, thr, sse)| (sse < total_sse - 1e-12).then_some((feat, thr)))
}

/// Recursively splits a node, appending to `leaves` once a leaf is reached.
fn build_node(
    ctx: &BuildCtx,
    indices: Vec<usize>,
    bounds: Vec<(f64, f64)>,
    depth: usize,
    leaves: &mut Vec<FanovaLeaf>,
) {
    let n = indices.len();
    let mean = indices.iter().map(|&i| ctx.y[i]).sum::<f64>() / n as f64;

    if depth >= ctx.max_depth {
        leaves.push(FanovaLeaf { bounds, mean });
        return;
    }

    match best_split(&indices, ctx.x, ctx.y, ctx.p, ctx.min_samples_leaf) {
        Some((feat, threshold)) => {
            let (left, right): (Vec<usize>, Vec<usize>) = indices
                .into_iter()
                .partition(|&i| ctx.x[i][feat] < threshold);
            // best_split only returns splits where both sides satisfy min_samples_leaf,
            // so neither side can be empty here.
            debug_assert!(!left.is_empty() && !right.is_empty());

            let mut left_bounds = bounds.clone();
            left_bounds[feat].1 = threshold;
            let mut right_bounds = bounds;
            right_bounds[feat].0 = threshold;

            build_node(ctx, left, left_bounds, depth + 1, leaves);
            build_node(ctx, right, right_bounds, depth + 1, leaves);
        }
        None => leaves.push(FanovaLeaf { bounds, mean }),
    }
}

/// Trains a single tree from a bootstrap sample. The RNG seed is varied deterministically
/// per tree via `config.seed + tree_index` (reusing the existing `SeededRng` (ChaCha8);
/// no new dependency is added).
fn train_tree(
    x: &[Vec<f64>],
    y: &[f64],
    p: usize,
    config: &FanovaConfig,
    tree_index: usize,
    ranges: &[(f64, f64)],
) -> FanovaTree {
    let n = x.len();
    let mut rng = SeededRng::from_seed(config.seed.wrapping_add(tree_index as u64));
    let boot_indices: Vec<usize> = (0..n).map(|_| rng.next_usize(n)).collect();

    let ctx = BuildCtx {
        x,
        y,
        max_depth: config.max_depth,
        min_samples_leaf: config.min_samples_leaf,
        p,
    };
    let mut leaves = Vec::new();
    build_node(&ctx, boot_indices, ranges.to_vec(), 0, &mut leaves);
    FanovaTree { leaves }
}

/// Trains a CART regression forest. Returns `(array of trees, observed range per dimension
/// over the full training data)`. Tree training is independent across trees, so it is
/// parallelized with rayon.
pub(super) fn train_forest(
    x: &[Vec<f64>],
    y: &[f64],
    config: &FanovaConfig,
) -> (Vec<FanovaTree>, Vec<(f64, f64)>) {
    let p = x.first().map_or(0, |row| row.len());
    if p == 0 {
        return (vec![], vec![]);
    }
    let ranges = observed_ranges(x, p);
    let trees: Vec<FanovaTree> = (0..config.n_trees)
        .into_par_iter()
        .map(|t| train_tree(x, y, p, config, t, &ranges))
        .collect();
    (trees, ranges)
}

/// Weight given by the leaf box's volume, normalized over the training data range.
/// If `exclude` specifies a dimension, this becomes the marginal weight with that dimension
/// excluded (used for fANOVA main-effect computation).
/// A dimension whose range is degenerate (length ≈ 0) is treated as having ratio 1.
fn leaf_weight(
    bounds: &[(f64, f64)],
    ranges: &[(f64, f64)],
    p: usize,
    exclude: Option<usize>,
) -> f64 {
    let mut w = 1.0;
    for d in 0..p {
        if Some(d) == exclude {
            continue;
        }
        let (range_lo, range_hi) = ranges[d];
        let range_len = range_hi - range_lo;
        if range_len < 1e-12 {
            continue;
        }
        let (lo, hi) = bounds[d];
        let inter_len = (hi.min(range_hi) - lo.max(range_lo)).max(0.0);
        w *= inter_len / range_len;
    }
    w
}

/// Result of the fANOVA variance decomposition for a single tree.
struct TreeDecomposition {
    // Allowed because it goes unread in non-test builds, where it has no validation use
    // (used by the t1 hand-computation check).
    #[allow(dead_code)]
    f0: f64,
    total_variance: f64,
    /// V_j (per-dimension main-effect variance)
    dim_variance: Vec<f64>,
}

/// Decomposes a single tree via fANOVA. If the total variance V is effectively zero
/// (near-constant output), returns `None` and the caller skips this tree.
fn decompose_tree(tree: &FanovaTree, ranges: &[(f64, f64)], p: usize) -> Option<TreeDecomposition> {
    if tree.leaves.is_empty() {
        return None;
    }
    let weights: Vec<f64> = tree
        .leaves
        .iter()
        .map(|l| leaf_weight(&l.bounds, ranges, p, None))
        .collect();

    let f0: f64 = weights
        .iter()
        .zip(&tree.leaves)
        .map(|(&w, l)| w * l.mean)
        .sum();
    let ey2: f64 = weights
        .iter()
        .zip(&tree.leaves)
        .map(|(&w, l)| w * l.mean * l.mean)
        .sum();
    let total_variance = ey2 - f0 * f0;
    if total_variance < 1e-12 {
        return None;
    }

    let mut dim_variance = vec![0.0; p];
    for (d, &(range_lo, range_hi)) in ranges.iter().enumerate() {
        let range_len = range_hi - range_lo;
        if range_len < 1e-12 {
            continue;
        }

        // The marginal weight excluding dimension d, times the leaf mean, does not depend
        // on the interval, so precompute it once per (leaf, d) pair (previously this was
        // recomputed on every interval-loop iteration, giving O(#leaves × #intervals)).
        let marginal_terms: Vec<f64> = tree
            .leaves
            .iter()
            .map(|leaf| leaf_weight(&leaf.bounds, ranges, p, Some(d)) * leaf.mean)
            .collect();

        // Collect the interval endpoints of all leaves for dimension d, and build the
        // "elementary intervals" for piecewise quadrature delimited by the distinct
        // endpoints. Within an elementary interval, the set of leaves containing it
        // does not change.
        let mut endpoints: Vec<f64> = Vec::with_capacity(tree.leaves.len() * 2 + 2);
        endpoints.push(range_lo);
        endpoints.push(range_hi);
        for leaf in &tree.leaves {
            let (lo, hi) = leaf.bounds[d];
            endpoints.push(lo.clamp(range_lo, range_hi));
            endpoints.push(hi.clamp(range_lo, range_hi));
        }
        endpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());
        endpoints.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

        for w in endpoints.windows(2) {
            let (i_lo, i_hi) = (w[0], w[1]);
            let i_len = i_hi - i_lo;
            if i_len < 1e-12 {
                continue;
            }
            // The midpoint of an elementary interval is never used as an endpoint, so it
            // never coincides with any leaf boundary.
            let mid = (i_lo + i_hi) / 2.0;

            let f_j: f64 = tree
                .leaves
                .iter()
                .zip(&marginal_terms)
                .filter(|(leaf, _)| {
                    let (lo, hi) = leaf.bounds[d];
                    mid >= lo && mid <= hi
                })
                .map(|(_, &term)| term)
                .sum();

            let diff = f_j - f0;
            dim_variance[d] += (i_len / range_len) * diff * diff;
        }
    }

    Some(TreeDecomposition {
        f0,
        total_variance,
        dim_variance,
    })
}

/// Forest-wide main-effect importances. Averages `V_j / V` across trees (only trees with
/// positive total variance are included) and normalizes so the values sum to 1.
/// If no tree is valid, all values are 0.0.
pub(super) fn forest_importances(
    trees: &[FanovaTree],
    ranges: &[(f64, f64)],
    p: usize,
) -> Vec<f64> {
    let decomps: Vec<Option<TreeDecomposition>> = trees
        .par_iter()
        .map(|t| decompose_tree(t, ranges, p))
        .collect();

    let mut sum = vec![0.0; p];
    let mut count = 0usize;
    for d in decomps.into_iter().flatten() {
        for (s, dv) in sum.iter_mut().zip(d.dim_variance.iter()) {
            *s += dv / d.total_variance;
        }
        count += 1;
    }
    if count == 0 {
        return sum;
    }
    for v in sum.iter_mut() {
        *v /= count as f64;
    }
    normalize(&mut sum);
    sum
}

/// Computes the prediction (leaf mean) for a single tree. Out-of-range inputs are clamped
/// to the training data range before searching for the matching leaf box (a small tolerance
/// is applied at boundaries to guard against floating-point error).
fn predict_tree(tree: &FanovaTree, row: &[f64], ranges: &[(f64, f64)]) -> f64 {
    let clamped: Vec<f64> = row
        .iter()
        .zip(ranges)
        .map(|(&v, &(lo, hi))| v.clamp(lo, hi))
        .collect();

    for leaf in &tree.leaves {
        let contains = clamped
            .iter()
            .zip(leaf.bounds.iter())
            .all(|(&v, &(lo, hi))| v >= lo - 1e-9 && v <= hi + 1e-9);
        if contains {
            return leaf.mean;
        }
    }
    // Should be unreachable, but falls back here if floating-point error prevents a match.
    tree.leaves.first().map_or(0.0, |l| l.mean)
}

/// Forest prediction (average of per-tree leaf means).
fn predict_forest(trees: &[FanovaTree], row: &[f64], ranges: &[(f64, f64)]) -> f64 {
    if trees.is_empty() {
        return 0.0;
    }
    let sum: f64 = trees.iter().map(|t| predict_tree(t, row, ranges)).sum();
    sum / trees.len() as f64
}

/// Trains an fANOVA forest from preprocessed train/eval data and returns `(importances, R²)`.
/// R² is computed by predicting the held-out eval data using the forest prediction
/// (average of per-tree leaf means).
pub(super) fn compute_fanova(
    x_train: &[Vec<f64>],
    y_train: &[f64],
    x_eval: &[Vec<f64>],
    y_eval: &[f64],
    config: &FanovaConfig,
) -> Option<(Vec<f64>, f64)> {
    let p = x_train.first()?.len();
    if p == 0 {
        return None;
    }
    let (trees, ranges) = train_forest(x_train, y_train, config);
    if trees.is_empty() {
        return None;
    }

    let importances = forest_importances(&trees, &ranges, p);

    let mse = if y_eval.is_empty() {
        0.0
    } else {
        x_eval
            .iter()
            .zip(y_eval)
            .map(|(row, &y)| (predict_forest(&trees, row, &ranges) - y).powi(2))
            .sum::<f64>()
            / y_eval.len() as f64
    };
    let r_squared = crate::lgbm::mse_to_r_squared(mse, y_eval).max(0.0);

    Some((importances, r_squared))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(b0: (f64, f64), b1: (f64, f64), mean: f64) -> FanovaLeaf {
        FanovaLeaf {
            bounds: vec![b0, b1],
            mean,
        }
    }

    /// t1: tree structure verified by hand computation (see
    /// theory/ja/sensitivity-analysis/rfanova.md). Root x0<0.5 → left leaf y=0. The right
    /// side (x0>=0.5) is further split by x1<0.25 into y=1 / y=3.
    #[test]
    fn analytic_two_split_tree_matches_hand_computation() {
        let tree = FanovaTree {
            leaves: vec![
                leaf((0.0, 0.5), (0.0, 1.0), 0.0),
                leaf((0.5, 1.0), (0.0, 0.25), 1.0),
                leaf((0.5, 1.0), (0.25, 1.0), 3.0),
            ],
        };
        let ranges = vec![(0.0, 1.0), (0.0, 1.0)];

        // Leaf weights: w = ([0,0.5]x[0,1], 0.5), ([0.5,1]x[0,0.25], 0.125), ([0.5,1]x[0.25,1], 0.375)
        let weights: Vec<f64> = tree
            .leaves
            .iter()
            .map(|l| leaf_weight(&l.bounds, &ranges, 2, None))
            .collect();
        assert!((weights[0] - 0.5).abs() < 1e-12);
        assert!((weights[1] - 0.125).abs() < 1e-12);
        assert!((weights[2] - 0.375).abs() < 1e-12);

        let decomp = decompose_tree(&tree, &ranges, 2).expect("全分散は正のはず");

        assert!((decomp.f0 - 1.25).abs() < 1e-12, "f0={}", decomp.f0);
        assert!(
            (decomp.total_variance - 1.9375).abs() < 1e-12,
            "V={}",
            decomp.total_variance
        );
        assert!(
            (decomp.dim_variance[0] - 1.5625).abs() < 1e-12,
            "V0={}",
            decomp.dim_variance[0]
        );
        assert!(
            (decomp.dim_variance[1] - 0.1875).abs() < 1e-12,
            "V1={}",
            decomp.dim_variance[1]
        );

        let frac0 = decomp.dim_variance[0] / decomp.total_variance;
        let frac1 = decomp.dim_variance[1] / decomp.total_variance;
        assert!(
            (frac0 - 0.806_451_612_903_225_8).abs() < 1e-9,
            "frac0={frac0}"
        );
        assert!(
            (frac1 - 0.096_774_193_548_387_1).abs() < 1e-9,
            "frac1={frac1}"
        );
    }

    /// Verifies that caching the marginal weight per (leaf, d) (hoisting it out of the
    /// interval loop) does not change the decomposition result for a forest trained on
    /// real data at all, by cross-checking against a naive implementation that
    /// recomputes `leaf_weight` on every interval-loop iteration.
    #[test]
    fn cached_marginal_weights_match_naive_recomputation() {
        // Naive implementation (no caching): follows decompose_tree's old logic.
        fn decompose_naive(tree: &FanovaTree, ranges: &[(f64, f64)], p: usize) -> Vec<f64> {
            let weights: Vec<f64> = tree
                .leaves
                .iter()
                .map(|l| leaf_weight(&l.bounds, ranges, p, None))
                .collect();
            let f0: f64 = weights
                .iter()
                .zip(&tree.leaves)
                .map(|(&w, l)| w * l.mean)
                .sum();
            let mut dim_variance = vec![0.0; p];
            for (d, &(range_lo, range_hi)) in ranges.iter().enumerate() {
                let range_len = range_hi - range_lo;
                if range_len < 1e-12 {
                    continue;
                }
                let mut endpoints: Vec<f64> = vec![range_lo, range_hi];
                for leaf in &tree.leaves {
                    let (lo, hi) = leaf.bounds[d];
                    endpoints.push(lo.clamp(range_lo, range_hi));
                    endpoints.push(hi.clamp(range_lo, range_hi));
                }
                endpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());
                endpoints.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
                for w in endpoints.windows(2) {
                    let (i_lo, i_hi) = (w[0], w[1]);
                    let i_len = i_hi - i_lo;
                    if i_len < 1e-12 {
                        continue;
                    }
                    let mid = (i_lo + i_hi) / 2.0;
                    let f_j: f64 = tree
                        .leaves
                        .iter()
                        .filter(|leaf| {
                            let (lo, hi) = leaf.bounds[d];
                            mid >= lo && mid <= hi
                        })
                        .map(|leaf| leaf_weight(&leaf.bounds, ranges, p, Some(d)) * leaf.mean)
                        .sum();
                    let diff = f_j - f0;
                    dim_variance[d] += (i_len / range_len) * diff * diff;
                }
            }
            dim_variance
        }

        // Train a forest on real data and cross-check both implementations.
        let mut rng = SeededRng::from_seed(12345);
        let x: Vec<Vec<f64>> = (0..50)
            .map(|_| {
                (0..3)
                    .map(|_| rng.next_usize(1000) as f64 / 1000.0)
                    .collect()
            })
            .collect();
        let y: Vec<f64> = x.iter().map(|r| 3.0 * r[0] + r[1] * r[1]).collect();
        let config = FanovaConfig {
            n_trees: 8,
            max_depth: 6,
            min_samples_leaf: 2,
            seed: 7,
        };
        let (trees, ranges) = train_forest(&x, &y, &config);
        assert!(!trees.is_empty());

        for tree in &trees {
            let Some(decomp) = decompose_tree(tree, &ranges, 3) else {
                continue;
            };
            let naive = decompose_naive(tree, &ranges, 3);
            for (a, b) in decomp.dim_variance.iter().zip(&naive) {
                assert_eq!(a, b, "cached vs naive dim_variance must be bit-identical");
            }
        }
    }
}
