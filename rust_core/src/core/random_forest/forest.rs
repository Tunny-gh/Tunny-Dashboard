use rayon::prelude::*;
use super::rng::Lcg;
use super::tree::{build_tree, predict_one};
use super::types::{DecisionTree, RandomForest};

impl DecisionTree {
    /// Predict a single sample.
    pub fn predict(&self, x: &[f64]) -> f64 {
        predict_one(&self.root, x)
    }
}

impl RandomForest {
    /// Train a Random Forest on feature matrix `x` and target `y`.
    pub fn train(
        x: &[Vec<f64>],
        y: &[f64],
        n_trees: usize,
        max_depth: usize,
        min_samples_leaf: usize,
        seed: u64,
    ) -> Self {
        let n = x.len();
        let p = x.first().map(|row| row.len()).unwrap_or(0);
        let feature_indices: Vec<usize> = (0..p).collect();

        let trees: Vec<DecisionTree> = (0..n_trees)
            .into_par_iter()
            .map(|tree_idx| {
                let mut local_rng = Lcg::new(seed.wrapping_add(tree_idx as u64));
                let mut x_boot: Vec<Vec<f64>> = Vec::with_capacity(n);
                let mut y_boot: Vec<f64> = Vec::with_capacity(n);

                for _ in 0..n {
                    let idx = local_rng.next_usize(n);
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
                DecisionTree { root }
            })
            .collect();

        RandomForest { trees }
    }

    /// Predict a single sample by averaging all trees.
    pub fn predict(&self, x: &[f64]) -> f64 {
        if self.trees.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.trees.iter().map(|tree| tree.predict(x)).sum();
        sum / self.trees.len() as f64
    }
}

pub(crate) fn extract_columns(
    x_matrix: &[Vec<f64>],
    column_indices: &[usize],
) -> Option<Vec<Vec<f64>>> {
    if x_matrix.is_empty() || column_indices.is_empty() {
        return None;
    }
    let n_features = x_matrix[0].len();
    if column_indices.iter().any(|&i| i >= n_features) {
        return None;
    }

    Some(
        x_matrix
            .iter()
            .map(|row| column_indices.iter().map(|&i| row[i]).collect())
            .collect(),
    )
}
