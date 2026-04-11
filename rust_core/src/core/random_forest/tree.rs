use super::types::TreeNode;

/// Compute the mean of a slice. Returns 0.0 for empty input.
fn mean(y: &[f64]) -> f64 {
    if y.is_empty() {
        return 0.0;
    }
    y.iter().sum::<f64>() / y.len() as f64
}

/// Compute MSE of a slice.
fn mse(y: &[f64]) -> f64 {
    if y.is_empty() {
        return 0.0;
    }
    let mean_value = mean(y);
    y.iter()
        .map(|&value| (value - mean_value).powi(2))
        .sum::<f64>()
        / y.len() as f64
}

/// Find the best (feature, threshold) split that minimises weighted MSE.
///
/// Returns `None` if no valid split exists (all splits violate `min_samples_leaf`).
pub(crate) fn find_best_split(
    x: &[Vec<f64>],
    y: &[f64],
    feature_indices: &[usize],
    min_samples_leaf: usize,
) -> Option<(usize, f64)> {
    let n = y.len();
    if n < 2 * min_samples_leaf {
        return None;
    }

    let parent_mse = mse(y);
    let mut best_gain = 0.0;
    let mut best_feat: Option<usize> = None;
    let mut best_thresh = 0.0;

    for &feat in feature_indices {
        let mut pairs: Vec<(f64, f64)> = x
            .iter()
            .zip(y.iter())
            .map(|(xi, &yi)| (xi[feat], yi))
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        for i in (min_samples_leaf - 1)..(n - min_samples_leaf) {
            if (pairs[i].0 - pairs[i + 1].0).abs() < f64::EPSILON {
                continue;
            }

            let threshold = (pairs[i].0 + pairs[i + 1].0) / 2.0;

            let left_y: Vec<f64> = pairs[..=i].iter().map(|pair| pair.1).collect();
            let right_y: Vec<f64> = pairs[i + 1..].iter().map(|pair| pair.1).collect();

            if left_y.len() < min_samples_leaf || right_y.len() < min_samples_leaf {
                continue;
            }

            let n_left = left_y.len() as f64;
            let n_right = right_y.len() as f64;
            let n_total = n as f64;

            let weighted_mse = (n_left * mse(&left_y) + n_right * mse(&right_y)) / n_total;
            let gain = parent_mse - weighted_mse;

            if gain > best_gain {
                best_gain = gain;
                best_feat = Some(feat);
                best_thresh = threshold;
            }
        }
    }

    best_feat.map(|feature| (feature, best_thresh))
}

/// Recursively build a CART regression tree.
pub(crate) fn build_tree(
    x: &[Vec<f64>],
    y: &[f64],
    feature_indices: &[usize],
    depth: usize,
    max_depth: usize,
    min_samples_leaf: usize,
) -> TreeNode {
    if depth >= max_depth || y.len() <= min_samples_leaf {
        return TreeNode::Leaf(mean(y));
    }

    match find_best_split(x, y, feature_indices, min_samples_leaf) {
        None => TreeNode::Leaf(mean(y)),
        Some((feat, threshold)) => {
            let mut left_x: Vec<Vec<f64>> = Vec::new();
            let mut left_y: Vec<f64> = Vec::new();
            let mut right_x: Vec<Vec<f64>> = Vec::new();
            let mut right_y: Vec<f64> = Vec::new();

            for (xi, &yi) in x.iter().zip(y.iter()) {
                if xi[feat] <= threshold {
                    left_x.push(xi.clone());
                    left_y.push(yi);
                } else {
                    right_x.push(xi.clone());
                    right_y.push(yi);
                }
            }

            if left_y.is_empty() || right_y.is_empty() {
                return TreeNode::Leaf(mean(y));
            }

            TreeNode::Split {
                feature: feat,
                threshold,
                left: Box::new(build_tree(
                    &left_x,
                    &left_y,
                    feature_indices,
                    depth + 1,
                    max_depth,
                    min_samples_leaf,
                )),
                right: Box::new(build_tree(
                    &right_x,
                    &right_y,
                    feature_indices,
                    depth + 1,
                    max_depth,
                    min_samples_leaf,
                )),
            }
        }
    }
}

/// Predict a single sample by traversing the tree.
pub(crate) fn predict_one(node: &TreeNode, x: &[f64]) -> f64 {
    match node {
        TreeNode::Leaf(value) => *value,
        TreeNode::Split {
            feature,
            threshold,
            left,
            right,
        } => {
            if x[*feature] <= *threshold {
                predict_one(left, x)
            } else {
                predict_one(right, x)
            }
        }
    }
}
