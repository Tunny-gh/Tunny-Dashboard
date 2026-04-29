//! Random Forest module: CART decision tree + Bagging ensemble.

mod forest;
mod rng;
mod tree;
pub(crate) mod types;

pub(crate) use forest::extract_columns;
pub(crate) use rng::Lcg;

#[cfg(test)]
use tree::{build_tree, predict_one};
#[cfg(test)]
use types::{DecisionTree, RandomForest, TreeNode};

#[cfg(test)]
mod tests;
