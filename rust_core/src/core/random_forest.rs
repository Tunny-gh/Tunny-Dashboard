//! Random Forest module: CART decision tree + Bagging ensemble.

#[allow(dead_code)]
mod forest;
mod rng;
#[allow(dead_code)]
mod tree;
pub(crate) mod types;

pub(crate) use rng::Lcg;

#[cfg(test)]
use tree::{build_tree, predict_one};
#[cfg(test)]
use types::{DecisionTree, RandomForest, TreeNode};

#[cfg(test)]
mod tests;
