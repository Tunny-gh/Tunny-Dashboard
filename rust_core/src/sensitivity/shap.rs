use crate::core::random_forest::{mse_on_dataset, train_rf_on_columns, Lcg};

const RF_TREES: usize = 64;
const RF_MAX_DEPTH: usize = 10;
const RF_MIN_SAMPLES_LEAF: usize = 2;
const RF_SEED: u64 = 42;
const SHAP_MAX_ROWS: usize = 1_000;
/// Path array size: max_depth + 2 (phantom root + max_depth splits + buffer)
const PATH_SIZE: usize = RF_MAX_DEPTH + 2;

// ---------------------------------------------------------------------------
// Tree structure for TreeSHAP
// ---------------------------------------------------------------------------

struct ShapNode {
    feature: Option<usize>,
    threshold: f64,
    /// Mean y value of training samples at this node (leaf prediction)
    value: f64,
    /// Count of (bootstrap) samples at this node (used for zero_fraction)
    n_samples: usize,
    left: Option<Box<ShapNode>>,
    right: Option<Box<ShapNode>>,
}

// ---------------------------------------------------------------------------
// PathElement: tracks polynomial weights along the decision path
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct PathElement {
    /// Splitting feature index; -1 for the phantom root element
    feature: i32,
    /// Fraction of training samples reaching this node when feature is absent
    zero_fraction: f64,
    /// Fraction of training samples reaching this node when feature is present
    one_fraction: f64,
    /// Polynomial coefficient for Shapley weight computation
    pweight: f64,
}

// ---------------------------------------------------------------------------
// Tree building
// ---------------------------------------------------------------------------

fn node_mean(y: &[f64], indices: &[usize]) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }
    indices.iter().map(|&i| y[i]).sum::<f64>() / indices.len() as f64
}

fn find_best_split(
    x: &[Vec<f64>],
    y: &[f64],
    indices: &[usize],
    feature_indices: &[usize],
    min_samples_leaf: usize,
    pairs_buf: &mut Vec<(f64, f64)>,
) -> Option<(usize, f64)> {
    let n = indices.len();
    if n < 2 * min_samples_leaf {
        return None;
    }

    let total_sum: f64 = indices.iter().map(|&i| y[i]).sum();
    let total_sum_sq: f64 = indices.iter().map(|&i| y[i] * y[i]).sum();
    let n_f = n as f64;
    let parent_mse = total_sum_sq / n_f - (total_sum / n_f).powi(2);

    let mut best_gain = 0.0_f64;
    let mut best_feat: Option<usize> = None;
    let mut best_thresh = 0.0_f64;

    for &feat in feature_indices {
        pairs_buf.clear();
        pairs_buf.extend(indices.iter().map(|&i| (x[i][feat], y[i])));
        pairs_buf.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut left_sum = 0.0_f64;
        let mut left_sum_sq = 0.0_f64;

        for i in 0..(n - min_samples_leaf) {
            let yi = pairs_buf[i].1;
            left_sum += yi;
            left_sum_sq += yi * yi;

            if i + 1 < min_samples_leaf {
                continue;
            }
            if (pairs_buf[i].0 - pairs_buf[i + 1].0).abs() < f64::EPSILON {
                continue;
            }

            let n_left = (i + 1) as f64;
            let n_right = n_f - n_left;
            if n_right < min_samples_leaf as f64 {
                break;
            }

            let left_mse = (left_sum_sq / n_left - (left_sum / n_left).powi(2)).max(0.0);
            let right_sum = total_sum - left_sum;
            let right_sum_sq = total_sum_sq - left_sum_sq;
            let right_mse = (right_sum_sq / n_right - (right_sum / n_right).powi(2)).max(0.0);

            let weighted_mse = (n_left * left_mse + n_right * right_mse) / n_f;
            let gain = parent_mse - weighted_mse;
            if gain > best_gain {
                best_gain = gain;
                best_feat = Some(feat);
                best_thresh = (pairs_buf[i].0 + pairs_buf[i + 1].0) / 2.0;
            }
        }
    }
    best_feat.map(|f| (f, best_thresh))
}

fn partition_in_place(v: &mut [usize], pred: impl Fn(usize) -> bool) -> usize {
    let mut pivot = 0;
    for i in 0..v.len() {
        if pred(v[i]) {
            v.swap(i, pivot);
            pivot += 1;
        }
    }
    pivot
}

#[allow(clippy::too_many_arguments)]
fn build_shap_tree(
    x: &[Vec<f64>],
    y: &[f64],
    indices: &[usize],
    feature_indices: &[usize],
    depth: usize,
    max_depth: usize,
    min_samples_leaf: usize,
    pairs_buf: &mut Vec<(f64, f64)>,
    idx_buf: &mut Vec<usize>,
) -> ShapNode {
    let value = node_mean(y, indices);
    let n_samples = indices.len();

    if depth >= max_depth || n_samples <= min_samples_leaf {
        return ShapNode {
            feature: None,
            threshold: 0.0,
            value,
            n_samples,
            left: None,
            right: None,
        };
    }

    match find_best_split(x, y, indices, feature_indices, min_samples_leaf, pairs_buf) {
        None => ShapNode {
            feature: None,
            threshold: 0.0,
            value,
            n_samples,
            left: None,
            right: None,
        },
        Some((feat, threshold)) => {
            idx_buf.clear();
            idx_buf.extend_from_slice(indices);
            let pivot = partition_in_place(idx_buf, |i| x[i][feat] <= threshold);

            if pivot == 0 || pivot == idx_buf.len() {
                return ShapNode {
                    feature: None,
                    threshold: 0.0,
                    value,
                    n_samples,
                    left: None,
                    right: None,
                };
            }

            let left_indices: Vec<usize> = idx_buf[..pivot].to_vec();
            let right_indices: Vec<usize> = idx_buf[pivot..].to_vec();

            let left = Box::new(build_shap_tree(
                x,
                y,
                &left_indices,
                feature_indices,
                depth + 1,
                max_depth,
                min_samples_leaf,
                pairs_buf,
                idx_buf,
            ));
            let right = Box::new(build_shap_tree(
                x,
                y,
                &right_indices,
                feature_indices,
                depth + 1,
                max_depth,
                min_samples_leaf,
                pairs_buf,
                idx_buf,
            ));

            ShapNode {
                feature: Some(feat),
                threshold,
                value,
                n_samples,
                left: Some(left),
                right: Some(right),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TreeSHAP path polynomial operations (Lundberg & Lee 2018, Algorithm 1)
// ---------------------------------------------------------------------------

/// Extend the path polynomial by adding a new feature at position `unique_depth`.
fn extend_path(
    path: &mut [PathElement; PATH_SIZE],
    unique_depth: usize,
    z: f64,
    o: f64,
    feature: i32,
) {
    path[unique_depth] = PathElement {
        feature,
        zero_fraction: z,
        one_fraction: o,
        pweight: if unique_depth == 0 { 1.0 } else { 0.0 },
    };
    for i in (0..unique_depth).rev() {
        path[i + 1].pweight += o * path[i].pweight * (i + 1) as f64 / (unique_depth + 1) as f64;
        path[i].pweight *= z * (unique_depth - i) as f64 / (unique_depth + 1) as f64;
    }
}

/// Undo `extend_path` for the element at `path_index`, shifting the array left.
fn unwind_path(path: &mut [PathElement; PATH_SIZE], unique_depth: usize, path_index: usize) {
    let o = path[path_index].one_fraction;
    let z = path[path_index].zero_fraction;
    let mut next_one = path[unique_depth].pweight;

    for i in (0..unique_depth).rev() {
        if o != 0.0 {
            let tmp = path[i].pweight;
            path[i].pweight = next_one * (unique_depth + 1) as f64 / ((i + 1) as f64 * o);
            next_one =
                tmp - path[i].pweight * z * (unique_depth - i) as f64 / (unique_depth + 1) as f64;
        } else {
            path[i].pweight *= (unique_depth + 1) as f64 / (z * (unique_depth - i) as f64);
        }
    }
    for i in path_index..unique_depth {
        path[i] = path[i + 1];
    }
}

/// Compute the Shapley weight for the element at `path_index` without modifying
/// the path (read-only equivalent of unwind + sum).
fn unwound_sum(path: &[PathElement; PATH_SIZE], unique_depth: usize, path_index: usize) -> f64 {
    let o = path[path_index].one_fraction;
    let z = path[path_index].zero_fraction;
    let mut next_one = path[unique_depth].pweight;
    let mut total = 0.0_f64;

    for i in (0..unique_depth).rev() {
        if o != 0.0 {
            let tmp = path[i].pweight;
            let w = next_one * (unique_depth + 1) as f64 / ((i + 1) as f64 * o);
            next_one = tmp - w * z * (unique_depth - i) as f64 / (unique_depth + 1) as f64;
            total += w;
        } else if z != 0.0 {
            total += path[i].pweight / z / (unique_depth - i) as f64;
        }
    }
    total / (unique_depth + 1) as f64
}

// ---------------------------------------------------------------------------
// Recursive TreeSHAP traversal
// ---------------------------------------------------------------------------

/// Recurse through the tree computing exact Shapley values for sample `x`.
///
/// `path` is passed by value (Copy) so each branch receives an independent copy.
/// `unique_depth` is the position at which the current call will call extend_path.
#[allow(clippy::too_many_arguments)]
fn tree_shap_recurse(
    node: &ShapNode,
    x: &[f64],
    unique_depth: usize,
    path: [PathElement; PATH_SIZE],
    parent_z: f64,
    parent_o: f64,
    parent_feature: i32,
    phi: &mut Vec<f64>,
) {
    let mut path = path;
    extend_path(&mut path, unique_depth, parent_z, parent_o, parent_feature);

    if node.feature.is_none() {
        // Leaf: accumulate Shapley contributions for each feature on the path
        for k in 1..=unique_depth {
            let w = unwound_sum(&path, unique_depth, k);
            let f = path[k].feature;
            if f >= 0 && (f as usize) < phi.len() {
                phi[f as usize] += w * (path[k].one_fraction - path[k].zero_fraction) * node.value;
            }
        }
        return;
    }

    let feat = node.feature.unwrap();
    let left = node.left.as_deref().unwrap();
    let right = node.right.as_deref().unwrap();

    let (hot, cold) = if x.get(feat).copied().unwrap_or(0.0) <= node.threshold {
        (left, right)
    } else {
        (right, left)
    };

    let hot_z = hot.n_samples as f64 / node.n_samples as f64;
    let cold_z = cold.n_samples as f64 / node.n_samples as f64;

    // Check whether this feature already appears on the path (may happen with
    // repeated splits on the same feature axis).
    let path_index_opt = (1..=unique_depth).find(|&i| path[i].feature == feat as i32);

    if let Some(idx) = path_index_opt {
        let incoming_z = path[idx].zero_fraction;
        let incoming_o = path[idx].one_fraction;
        // Unwind the earlier occurrence so the feature can be re-added cleanly.
        let mut unwound = path;
        unwind_path(&mut unwound, unique_depth, idx);
        let new_depth = unique_depth - 1;
        tree_shap_recurse(
            hot,
            x,
            new_depth + 1,
            unwound,
            hot_z * incoming_z,
            incoming_o,
            feat as i32,
            phi,
        );
        tree_shap_recurse(
            cold,
            x,
            new_depth + 1,
            unwound,
            cold_z * incoming_z,
            0.0,
            feat as i32,
            phi,
        );
    } else {
        // PathElement is Copy — each branch gets an independent copy of path.
        tree_shap_recurse(hot, x, unique_depth + 1, path, hot_z, 1.0, feat as i32, phi);
        tree_shap_recurse(
            cold,
            x,
            unique_depth + 1,
            path,
            cold_z,
            0.0,
            feat as i32,
            phi,
        );
    }
}

// ---------------------------------------------------------------------------
// Utilities shared with mdi.rs
// ---------------------------------------------------------------------------

fn sample_rows(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    max_rows: usize,
    seed: u64,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = y.len();
    let mut indices: Vec<usize> = (0..n).collect();
    let mut rng = Lcg::new(seed);
    for i in (1..n).rev() {
        let j = rng.next_usize(i + 1);
        indices.swap(i, j);
    }
    indices.truncate(max_rows);
    (
        indices.iter().map(|&i| x_matrix[i].clone()).collect(),
        indices.iter().map(|&i| y[i]).collect(),
    )
}

fn normalize(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum < f64::EPSILON {
        return;
    }
    for v in values.iter_mut() {
        *v /= sum;
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute TreeSHAP global feature importance.
///
/// For each training sample and each tree, exact Shapley values φ_j(x) are
/// computed (Lundberg & Lee 2018). The global importance is the mean |φ_j|
/// across all samples and trees, normalised to sum = 1.
///
/// Returns `(importances, r_squared)`.
pub fn compute_shap_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }

    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }

    // Filter non-finite rows
    let valid: Vec<usize> = (0..n)
        .filter(|&i| y[i].is_finite() && x_matrix[i].iter().all(|v| v.is_finite()))
        .collect();

    let (x_clean, y_clean): (Vec<Vec<f64>>, Vec<f64>) = if valid.len() < n {
        (
            valid.iter().map(|&i| x_matrix[i].clone()).collect(),
            valid.iter().map(|&i| y[i]).collect(),
        )
    } else {
        (x_matrix.to_vec(), y.to_vec())
    };

    let n = y_clean.len();
    if n < 2 {
        return (vec![0.0; p], 0.0);
    }

    // Downsample
    let (x_data, y_data) = if n > SHAP_MAX_ROWS {
        sample_rows(&x_clean, &y_clean, SHAP_MAX_ROWS, RF_SEED)
    } else {
        (x_clean, y_clean)
    };

    let n = y_data.len();

    // 80/20 holdout split for R²
    const MIN_EVAL: usize = 2;
    const MIN_TRAIN: usize = 2;
    let use_holdout = n >= MIN_TRAIN + MIN_EVAL;
    let split_idx = if use_holdout {
        ((n * 4) / 5).max(MIN_TRAIN)
    } else {
        n
    };

    let mut shuffle_idx: Vec<usize> = (0..n).collect();
    let mut rng_split = Lcg::new(RF_SEED.wrapping_add(1));
    for i in (1..n).rev() {
        let j = rng_split.next_usize(i + 1);
        shuffle_idx.swap(i, j);
    }
    let x_sh: Vec<Vec<f64>> = shuffle_idx.iter().map(|&i| x_data[i].clone()).collect();
    let y_sh: Vec<f64> = shuffle_idx.iter().map(|&i| y_data[i]).collect();

    let (x_train, x_eval, y_train, y_eval) = if use_holdout {
        (
            &x_sh[..split_idx],
            &x_sh[split_idx..],
            &y_sh[..split_idx],
            &y_sh[split_idx..],
        )
    } else {
        (
            x_sh.as_slice(),
            x_sh.as_slice(),
            y_sh.as_slice(),
            y_sh.as_slice(),
        )
    };

    // ---- TreeSHAP: accumulate |phi_j| over all samples and trees ----
    let feature_indices: Vec<usize> = (0..p).collect();
    let n_train = y_train.len();
    let mut phi_sum = vec![0.0_f64; p];
    let mut rng = Lcg::new(RF_SEED);

    let mut pairs_buf: Vec<(f64, f64)> = Vec::with_capacity(n_train);
    let mut idx_buf: Vec<usize> = Vec::with_capacity(n_train);

    for _ in 0..RF_TREES {
        // Bootstrap sample
        let boot_indices: Vec<usize> = (0..n_train).map(|_| rng.next_usize(n_train)).collect();

        let tree = build_shap_tree(
            x_train,
            y_train,
            &boot_indices,
            &feature_indices,
            0,
            RF_MAX_DEPTH,
            RF_MIN_SAMPLES_LEAF,
            &mut pairs_buf,
            &mut idx_buf,
        );

        // Compute |phi_j(x)| for every training sample
        let mut phi = vec![0.0_f64; p];
        let empty_path = [PathElement::default(); PATH_SIZE];

        for row in x_train.iter().take(n_train) {
            phi.fill(0.0);
            tree_shap_recurse(&tree, row, 0, empty_path, 1.0, 1.0, -1, &mut phi);
            for j in 0..p {
                phi_sum[j] += phi[j].abs();
            }
        }
    }

    // Average over samples and trees, then normalise to sum = 1
    let denom = (n_train * RF_TREES) as f64;
    for v in phi_sum.iter_mut() {
        *v /= denom;
    }
    normalize(&mut phi_sum);

    // ---- R² on holdout using standard RandomForest ----
    let all_cols: Vec<usize> = (0..p).collect();
    let r_squared = match train_rf_on_columns(
        x_train,
        y_train,
        &all_cols,
        RF_TREES,
        RF_MAX_DEPTH,
        RF_MIN_SAMPLES_LEAF,
        RF_SEED,
    ) {
        None => 0.0,
        Some(rf) => {
            let baseline_mse = mse_on_dataset(&rf, x_eval, y_eval)
                .unwrap_or(0.0)
                .max(f64::EPSILON);
            let n_eval = y_eval.len();
            let y_mean = y_eval.iter().sum::<f64>() / n_eval as f64;
            let ss_tot: f64 = y_eval.iter().map(|&v| (v - y_mean).powi(2)).sum();
            if ss_tot < f64::EPSILON {
                0.0
            } else {
                (1.0 - baseline_mse * n_eval as f64 / ss_tot).max(0.0)
            }
        }
    };

    (phi_sum, r_squared)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_xy(n: usize, dominant: usize, n_feats: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut rng = Lcg::new(77);
        let x: Vec<Vec<f64>> = (0..n)
            .map(|_| {
                (0..n_feats)
                    .map(|_| rng.next_usize(1000) as f64 / 1000.0)
                    .collect()
            })
            .collect();
        let y: Vec<f64> = x.iter().map(|row| row[dominant] * 10.0).collect();
        (x, y)
    }

    #[test]
    fn importances_sum_to_one() {
        let (x, y) = make_xy(60, 0, 3);
        let (imp, _) = compute_shap_importances(&x, &y);
        assert_eq!(imp.len(), 3);
        let sum: f64 = imp.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "sum={sum}");
    }

    #[test]
    fn dominant_feature_ranks_first() {
        let (x, y) = make_xy(100, 1, 3);
        let (imp, _) = compute_shap_importances(&x, &y);
        let max_idx = imp
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(max_idx, 1, "importances={imp:?}");
    }

    #[test]
    fn empty_input_returns_empty() {
        let (imp, r2) = compute_shap_importances(&[], &[]);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }

    #[test]
    fn single_sample_returns_empty() {
        let x = vec![vec![1.0, 2.0]];
        let y = vec![3.0];
        let (imp, r2) = compute_shap_importances(&x, &y);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }

    #[test]
    fn extend_then_unwound_sum_is_positive() {
        let mut path = [PathElement::default(); PATH_SIZE];
        // phantom root
        extend_path(&mut path, 0, 1.0, 1.0, -1);
        // feature 0, hot_z=0.6, cold_z=0.4
        extend_path(&mut path, 1, 0.6, 1.0, 0);
        let w = unwound_sum(&path, 1, 1);
        assert!(w > 0.0, "weight should be positive, got {w}");
    }
}
