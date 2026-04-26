use crate::core::random_forest::{mse_on_dataset, train_rf_on_columns, Lcg};

const RF_TREES: usize = 64;
const RF_MAX_DEPTH: usize = 64;
const RF_MIN_SAMPLES_LEAF: usize = 2;
const RF_SEED: u64 = 42;
const MDI_MAX_ROWS: usize = 1_000;

/// Local tree node that stores the weighted impurity gain at each split.
/// Self-contained — does not modify the shared `TreeNode` in core.
enum MdiNode {
    Leaf,
    Split {
        feature: usize,
        weighted_gain: f64,
        left: Box<MdiNode>,
        right: Box<MdiNode>,
    },
}

fn find_best_split_with_gain_idx(
    x: &[Vec<f64>],
    y: &[f64],
    indices: &[usize],
    feature_indices: &[usize],
    min_samples_leaf: usize,
    pairs_buf: &mut Vec<(f64, f64)>,
) -> Option<(usize, f64, f64)> {
    let n = indices.len();
    if n < 2 * min_samples_leaf {
        return None;
    }

    let total_sum: f64 = indices.iter().map(|&i| y[i]).sum();
    let total_sum_sq: f64 = indices.iter().map(|&i| y[i] * y[i]).sum();
    let n_f = n as f64;
    let parent_mse = total_sum_sq / n_f - (total_sum / n_f).powi(2);

    let mut best_gain = 0.0;
    let mut best_feat: Option<usize> = None;
    let mut best_thresh = 0.0;

    for &feat in feature_indices {
        pairs_buf.clear();
        pairs_buf.extend(indices.iter().map(|&i| (x[i][feat], y[i])));
        pairs_buf.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut left_sum = 0.0f64;
        let mut left_sum_sq = 0.0f64;

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
    best_feat.map(|f| (f, best_thresh, best_gain))
}

#[allow(clippy::too_many_arguments)]
fn build_mdi_tree_idx(
    x: &[Vec<f64>],
    y: &[f64],
    indices: &[usize],
    feature_indices: &[usize],
    depth: usize,
    max_depth: usize,
    min_samples_leaf: usize,
    n_root: usize,
    pairs_buf: &mut Vec<(f64, f64)>,
    idx_buf: &mut Vec<usize>,
) -> MdiNode {
    if depth >= max_depth || indices.len() <= min_samples_leaf {
        return MdiNode::Leaf;
    }

    match find_best_split_with_gain_idx(x, y, indices, feature_indices, min_samples_leaf, pairs_buf)
    {
        None => MdiNode::Leaf,
        Some((feat, threshold, raw_gain)) => {
            // Partition indices in-place using a temporary buffer
            idx_buf.clear();
            idx_buf.extend_from_slice(indices);
            let pivot = partition_in_place(idx_buf, |&i| x[i][feat] <= threshold);

            let (left_idx, right_idx) = idx_buf.split_at(pivot);
            if left_idx.is_empty() || right_idx.is_empty() {
                return MdiNode::Leaf;
            }

            let weighted_gain = (indices.len() as f64 / n_root as f64) * raw_gain;

            // Clone the index slices before recursing (idx_buf is mutably borrowed)
            let left_indices: Vec<usize> = left_idx.to_vec();
            let right_indices: Vec<usize> = right_idx.to_vec();

            let left = Box::new(build_mdi_tree_idx(
                x,
                y,
                &left_indices,
                feature_indices,
                depth + 1,
                max_depth,
                min_samples_leaf,
                n_root,
                pairs_buf,
                idx_buf,
            ));
            let right = Box::new(build_mdi_tree_idx(
                x,
                y,
                &right_indices,
                feature_indices,
                depth + 1,
                max_depth,
                min_samples_leaf,
                n_root,
                pairs_buf,
                idx_buf,
            ));

            MdiNode::Split {
                feature: feat,
                weighted_gain,
                left,
                right,
            }
        }
    }
}

fn partition_in_place<T, F: Fn(&T) -> bool>(v: &mut [T], pred: F) -> usize {
    let mut pivot = 0;
    for i in 0..v.len() {
        if pred(&v[i]) {
            v.swap(i, pivot);
            pivot += 1;
        }
    }
    pivot
}

/// Walk a built MDI tree and accumulate per-feature weighted gains.
fn accumulate_gains(node: &MdiNode, gains: &mut [f64]) {
    match node {
        MdiNode::Leaf => {}
        MdiNode::Split {
            feature,
            weighted_gain,
            left,
            right,
        } => {
            if *feature < gains.len() {
                gains[*feature] += weighted_gain;
            }
            accumulate_gains(left, gains);
            accumulate_gains(right, gains);
        }
    }
}

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

/// Compute MDI importances for each feature. Returns `(importances, r_squared)`.
///
/// MDI = Mean Decrease Impurity. For each split node using feature j across all
/// trees, accumulates `(n_node / n_root) * ΔI`, averaged over all trees and
/// normalized to sum=1.
///
/// R² is computed on a held-out 20% split using a standard `RandomForest`
/// (identical to the RF-ANOVA evaluation methodology).
pub fn compute_mdi_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }

    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }

    // Filter non-finite rows
    let valid_indices: Vec<usize> = (0..n)
        .filter(|&i| y[i].is_finite() && x_matrix[i].iter().all(|v| v.is_finite()))
        .collect();

    let (x_clean, y_clean): (Vec<Vec<f64>>, Vec<f64>) = if valid_indices.len() < n {
        (
            valid_indices.iter().map(|&i| x_matrix[i].clone()).collect(),
            valid_indices.iter().map(|&i| y[i]).collect(),
        )
    } else {
        (x_matrix.to_vec(), y.to_vec())
    };

    let n = y_clean.len();
    if n < 2 {
        return (vec![0.0; p], 0.0);
    }

    // Downsample large datasets
    let (x_data, y_data) = if n > MDI_MAX_ROWS {
        sample_rows(&x_clean, &y_clean, MDI_MAX_ROWS, RF_SEED)
    } else {
        (x_clean, y_clean)
    };

    let n = y_data.len();

    // 80/20 holdout split for R² (identical to rf_anova.rs)
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
    let x_shuffled: Vec<Vec<f64>> = shuffle_idx.iter().map(|&i| x_data[i].clone()).collect();
    let y_shuffled: Vec<f64> = shuffle_idx.iter().map(|&i| y_data[i]).collect();

    let (x_train, x_eval, y_train, y_eval) = if use_holdout {
        (
            &x_shuffled[..split_idx],
            &x_shuffled[split_idx..],
            &y_shuffled[..split_idx],
            &y_shuffled[split_idx..],
        )
    } else {
        (
            x_shuffled.as_slice(),
            x_shuffled.as_slice(),
            y_shuffled.as_slice(),
            y_shuffled.as_slice(),
        )
    };

    // ---- MDI accumulation across RF_TREES bootstrap trees ----
    let feature_indices: Vec<usize> = (0..p).collect();
    let n_train = y_train.len();
    let mut total_gains = vec![0.0f64; p];
    let mut rng = Lcg::new(RF_SEED);

    let mut pairs_buf: Vec<(f64, f64)> = Vec::with_capacity(n_train);
    let mut idx_buf: Vec<usize> = Vec::with_capacity(n_train);

    for _ in 0..RF_TREES {
        let boot_indices: Vec<usize> = (0..n_train).map(|_| rng.next_usize(n_train)).collect();
        let n_root = boot_indices.len();
        let tree = build_mdi_tree_idx(
            x_train,
            y_train,
            &boot_indices,
            &feature_indices,
            0,
            RF_MAX_DEPTH,
            RF_MIN_SAMPLES_LEAF,
            n_root,
            &mut pairs_buf,
            &mut idx_buf,
        );
        accumulate_gains(&tree, &mut total_gains);
    }

    // Average over trees then normalize to sum=1
    for g in total_gains.iter_mut() {
        *g /= RF_TREES as f64;
    }
    normalize(&mut total_gains);

    // ---- R² on holdout using standard RandomForest ----
    let all_columns: Vec<usize> = (0..p).collect();
    let r_squared = match train_rf_on_columns(
        x_train,
        y_train,
        &all_columns,
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

    (total_gains, r_squared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_xy(n: usize, dominant_feat: usize, n_feats: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut rng = Lcg::new(99);
        let x: Vec<Vec<f64>> = (0..n)
            .map(|_| {
                (0..n_feats)
                    .map(|_| rng.next_usize(1000) as f64 / 1000.0)
                    .collect()
            })
            .collect();
        let y: Vec<f64> = x.iter().map(|row| row[dominant_feat] * 10.0).collect();
        (x, y)
    }

    #[test]
    fn importances_sum_to_one() {
        let (x, y) = make_xy(60, 0, 3);
        let (importances, _) = compute_mdi_importances(&x, &y);
        assert_eq!(importances.len(), 3);
        let sum: f64 = importances.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "sum={sum}");
    }

    #[test]
    fn dominant_feature_ranks_first() {
        let (x, y) = make_xy(80, 1, 3);
        let (importances, _) = compute_mdi_importances(&x, &y);
        assert_eq!(importances.len(), 3);
        let max_idx = importances
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(max_idx, 1, "importances={importances:?}");
    }

    #[test]
    fn empty_input_returns_empty() {
        let (imp, r2) = compute_mdi_importances(&[], &[]);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }

    #[test]
    fn single_sample_returns_empty() {
        let x = vec![vec![1.0, 2.0]];
        let y = vec![3.0];
        let (imp, r2) = compute_mdi_importances(&x, &y);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }
}
