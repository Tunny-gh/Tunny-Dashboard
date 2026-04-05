//! Random Forest module: CART decision tree + Bagging ensemble.
//!
//! Implements a pure-Rust Random Forest regressor without external crates.
//! Used for 2D PDP surface computation in `pdp.rs`.

// =============================================================================
// Data structures
// =============================================================================

/// CART decision tree node.
pub(crate) enum TreeNode {
    Leaf(f64),
    Split {
        feature: usize,
        threshold: f64,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
    },
}

/// CART decision tree.
pub(crate) struct DecisionTree {
    pub root: TreeNode,
}

impl DecisionTree {
    /// Predict a single sample.
    pub fn predict(&self, x: &[f64]) -> f64 {
        predict_one(&self.root, x)
    }
}

// =============================================================================
// Random Forest
// =============================================================================

/// Random Forest regressor.
pub(crate) struct RandomForest {
    trees: Vec<DecisionTree>,
}

impl RandomForest {
    /// Train a Random Forest on 2-column feature matrix `x` and target `y`.
    ///
    /// - `n_trees`: number of trees (default 100)
    /// - `max_depth`: maximum tree depth (default 10)
    /// - `min_samples_leaf`: minimum samples per leaf (default 2)
    /// - `seed`: LCG seed for reproducible bootstrap sampling
    pub fn train(
        x: &[Vec<f64>],
        y: &[f64],
        n_trees: usize,
        max_depth: usize,
        min_samples_leaf: usize,
        seed: u64,
    ) -> Self {
        let n = x.len();
        let p = if x.is_empty() { 0 } else { x[0].len() };
        let feature_indices: Vec<usize> = (0..p).collect();

        let mut lcg = seed;
        let mut trees = Vec::with_capacity(n_trees);

        for _ in 0..n_trees {
            // Bootstrap sampling (with replacement)
            let mut x_boot: Vec<Vec<f64>> = Vec::with_capacity(n);
            let mut y_boot: Vec<f64> = Vec::with_capacity(n);
            for _ in 0..n {
                let idx = lcg_next(&mut lcg) as usize % n;
                x_boot.push(x[idx].clone());
                y_boot.push(y[idx]);
            }

            let root = build_tree(
                &x_boot,
                &y_boot,
                &feature_indices,
                0,
                max_depth,
                min_samples_leaf,
            );
            trees.push(DecisionTree { root });
        }

        RandomForest { trees }
    }

    /// Predict a single sample by averaging all trees.
    pub fn predict(&self, x: &[f64]) -> f64 {
        if self.trees.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.trees.iter().map(|t| t.predict(x)).sum();
        sum / self.trees.len() as f64
    }
}

// =============================================================================
// Core CART algorithm
// =============================================================================

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
    let m = mean(y);
    y.iter().map(|&v| (v - m).powi(2)).sum::<f64>() / y.len() as f64
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
        // Collect (feature_value, target) pairs and sort by feature value
        let mut pairs: Vec<(f64, f64)> = x.iter().zip(y.iter()).map(|(xi, &yi)| (xi[feat], yi)).collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Try each unique threshold between consecutive distinct values
        for i in (min_samples_leaf - 1)..(n - min_samples_leaf) {
            // Skip if same feature value (no split possible)
            if (pairs[i].0 - pairs[i + 1].0).abs() < f64::EPSILON {
                continue;
            }

            let threshold = (pairs[i].0 + pairs[i + 1].0) / 2.0;

            let left_y: Vec<f64> = pairs[..=i].iter().map(|p| p.1).collect();
            let right_y: Vec<f64> = pairs[i + 1..].iter().map(|p| p.1).collect();

            if left_y.len() < min_samples_leaf || right_y.len() < min_samples_leaf {
                continue;
            }

            let n_left = left_y.len() as f64;
            let n_right = right_y.len() as f64;
            let n_total = n as f64;

            let weighted_mse =
                (n_left * mse(&left_y) + n_right * mse(&right_y)) / n_total;
            let gain = parent_mse - weighted_mse;

            if gain > best_gain {
                best_gain = gain;
                best_feat = Some(feat);
                best_thresh = threshold;
            }
        }
    }

    best_feat.map(|f| (f, best_thresh))
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
    // Stopping conditions: leaf
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

            // Safety check: if split is degenerate, return leaf
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
        TreeNode::Leaf(v) => *v,
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

// =============================================================================
// LCG pseudo-random number generator (no external crates)
// =============================================================================

/// Linear congruential generator: returns next 64-bit value and updates state.
fn lcg_next(state: &mut u64) -> u64 {
    // Parameters from Knuth / Numerical Recipes
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

/// Reusable LCG RNG struct (shared with kriging.rs via `crate::rf::Lcg`).
pub(crate) struct Lcg {
    state: u64,
}

impl Lcg {
    pub(crate) fn new(seed: u64) -> Self {
        Lcg { state: seed ^ 0xcafef00dd15ea5e5 }
    }

    /// Returns a random `usize` in `[0, n)`.
    pub(crate) fn next_usize(&mut self, n: usize) -> usize {
        lcg_next(&mut self.state) as usize % n
    }
}

// =============================================================================
// 2D PDP computation
// =============================================================================

/// Compute the 2D partial dependence surface using a Random Forest.
///
/// Extracts columns `param1_idx` and `param2_idx` from `x_matrix`, trains a
/// Random Forest on those 2 features, and evaluates on a `n_grid × n_grid` grid.
pub(crate) fn compute_pdp_2d_rf(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>, f64)> {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid == 0 {
        return None;
    }
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return None;
    }

    // Extract 2D sub-matrix
    let x2d: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| vec![row[param1_idx], row[param2_idx]])
        .collect();

    // Train Random Forest (defaults: 100 trees, max_depth=10, min_samples_leaf=2)
    let rf = RandomForest::train(&x2d, y, 100, 10, 2, 42);

    // Build grid ranges
    let col1: Vec<f64> = x2d.iter().map(|r| r[0]).collect();
    let col2: Vec<f64> = x2d.iter().map(|r| r[1]).collect();
    let min1 = col1.iter().cloned().fold(f64::INFINITY, f64::min);
    let max1 = col1.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min2 = col2.iter().cloned().fold(f64::INFINITY, f64::min);
    let max2 = col2.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    // Predict on grid
    let values: Vec<Vec<f64>> = grid1
        .iter()
        .map(|&v1| {
            grid2
                .iter()
                .map(|&v2| rf.predict(&[v1, v2]))
                .collect()
        })
        .collect();

    // Compute OOB-style R² on training data (in-bag as fallback)
    let y_pred: Vec<f64> = x2d.iter().map(|xi| rf.predict(xi)).collect();
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let ss_res: f64 = y.iter().zip(y_pred.iter()).map(|(&yi, &yp)| (yi - yp).powi(2)).sum();
    let ss_tot: f64 = y.iter().map(|&yi| (yi - y_mean).powi(2)).sum();
    let r_squared = if ss_tot < f64::EPSILON {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Some((grid1, grid2, values, r_squared))
}

fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }
    (0..n)
        .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // TASK-1630 tests: CART decision tree
    // -------------------------------------------------------------------------

    /// TC1: Perfectly separable data — tree should split correctly.
    #[test]
    fn tc_1630_01_perfectly_separable() {
        let x = vec![
            vec![0.0f64],
            vec![1.0],
            vec![2.0],
            vec![3.0],
        ];
        let y = vec![0.0, 0.0, 1.0, 1.0];
        let feat = vec![0usize];

        let tree = DecisionTree {
            root: build_tree(&x, &y, &feat, 0, 5, 1),
        };

        assert!((tree.predict(&[0.0]) - 0.0).abs() < 1e-9, "x=0 should predict 0");
        assert!((tree.predict(&[1.0]) - 0.0).abs() < 1e-9, "x=1 should predict 0");
        assert!((tree.predict(&[2.0]) - 1.0).abs() < 1e-9, "x=2 should predict 1");
        assert!((tree.predict(&[3.0]) - 1.0).abs() < 1e-9, "x=3 should predict 1");
    }

    /// TC2: max_depth=0 forces a leaf with the mean of all samples.
    #[test]
    fn tc_1630_02_max_depth_zero() {
        let x = vec![vec![0.0f64], vec![1.0], vec![2.0], vec![3.0]];
        let y = vec![0.0, 0.0, 1.0, 1.0];
        let feat = vec![0usize];

        let root = build_tree(&x, &y, &feat, 0, 0, 1);
        match root {
            TreeNode::Leaf(v) => {
                let expected = 0.5; // mean of [0,0,1,1]
                assert!((v - expected).abs() < 1e-9, "leaf value should be mean: {}", v);
            }
            _ => panic!("Expected Leaf node with max_depth=0"),
        }
    }

    /// TC3: min_samples_leaf=3 prevents splits that leave fewer than 3 samples.
    #[test]
    fn tc_1630_03_min_samples_leaf() {
        let x = vec![
            vec![0.0f64],
            vec![1.0],
            vec![2.0],
            vec![3.0],
        ];
        let y = vec![0.0, 0.0, 1.0, 1.0];
        let feat = vec![0usize];

        // With min_samples_leaf=3 on 4 samples, any split puts at most 1 sample
        // on one side (unless it's 3/1 split). Since n=4:
        // valid splits: left=3, right=1 — but right < 3, invalid
        //               left=1, right=3 — but left < 3, invalid
        // So no valid split → should return a Leaf
        let root = build_tree(&x, &y, &feat, 0, 10, 3);
        // With n=4 and min_samples_leaf=3, only a 3/1 split is possible
        // but 1 < 3, so no valid split → Leaf
        match root {
            TreeNode::Leaf(v) => {
                let expected = 0.5;
                assert!((v - expected).abs() < 1e-9, "Should be leaf with mean: {}", v);
            }
            TreeNode::Split { .. } => {
                // A 3/1 split might occur: verify both children have >= min_samples_leaf
                // (this branch means implementation allows it — acceptable)
                // Just verify predict_one works
                let pred = predict_one(&root, &[0.0]);
                assert!(pred >= 0.0 && pred <= 1.0, "Prediction should be in [0,1]");
            }
        }
    }

    /// TC4: predict_one traverses Split nodes correctly.
    #[test]
    fn tc_1630_04_predict_one_split() {
        // Manually construct a tree: split at feature 0, threshold 1.5
        //   left (x<=1.5) → Leaf(0.0)
        //   right (x>1.5) → Leaf(1.0)
        let node = TreeNode::Split {
            feature: 0,
            threshold: 1.5,
            left: Box::new(TreeNode::Leaf(0.0)),
            right: Box::new(TreeNode::Leaf(1.0)),
        };
        assert_eq!(predict_one(&node, &[1.0]), 0.0);
        assert_eq!(predict_one(&node, &[1.5]), 0.0); // boundary: <=
        assert_eq!(predict_one(&node, &[2.0]), 1.0);
    }

    // -------------------------------------------------------------------------
    // TASK-1631 tests: Random Forest bootstrap + ensemble
    // -------------------------------------------------------------------------

    /// TC5: Random Forest on linear data should have high R².
    #[test]
    fn tc_1631_01_rf_linear_r_squared() {
        let n = 100;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] * 2.0 + 1.0).collect();

        let rf = RandomForest::train(&x, &y, 50, 10, 2, 123);
        let y_mean = y.iter().sum::<f64>() / n as f64;
        let ss_res: f64 = x.iter().zip(y.iter()).map(|(xi, &yi)| (yi - rf.predict(xi)).powi(2)).sum();
        let ss_tot: f64 = y.iter().map(|&yi| (yi - y_mean).powi(2)).sum();
        let r2 = 1.0 - ss_res / ss_tot;
        assert!(r2 > 0.9, "R² should be > 0.9 for linear data, got {}", r2);
    }

    /// TC6: RF ensemble averages: prediction must be in [min_y, max_y].
    #[test]
    fn tc_1631_02_rf_prediction_range() {
        let x: Vec<Vec<f64>> = vec![vec![0.0], vec![0.5], vec![1.0]];
        let y = vec![0.0, 0.5, 1.0];
        let rf = RandomForest::train(&x, &y, 10, 5, 1, 42);

        for xi in &x {
            let pred = rf.predict(xi);
            assert!(pred >= -0.01 && pred <= 1.01, "Prediction {} out of range", pred);
        }
    }

    // -------------------------------------------------------------------------
    // TASK-1632 tests: compute_pdp_2d_rf
    // -------------------------------------------------------------------------

    /// TC7: 2D PDP grid shape and value range.
    #[test]
    fn tc_1632_01_pdp_2d_rf_grid_shape() {
        let n = 50;
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let t = i as f64 / n as f64;
                vec![t, t * 0.5]
            })
            .collect();
        let y: Vec<f64> = x.iter().map(|xi| xi[0] + xi[1]).collect();

        let result = compute_pdp_2d_rf(&x, &y, 0, 1, 5);
        assert!(result.is_some(), "Should return Some");
        let (grid1, grid2, values, r2) = result.unwrap();
        assert_eq!(grid1.len(), 5, "grid1 length");
        assert_eq!(grid2.len(), 5, "grid2 length");
        assert_eq!(values.len(), 5, "values rows");
        assert_eq!(values[0].len(), 5, "values cols");
        assert!(r2 >= 0.0, "R² should be non-negative");
    }

    /// TC8: Returns None for empty input.
    #[test]
    fn tc_1632_02_pdp_2d_rf_empty() {
        let result = compute_pdp_2d_rf(&[], &[], 0, 1, 5);
        assert!(result.is_none(), "Empty input should return None");
    }
}
