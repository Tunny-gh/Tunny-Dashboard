//! Random Forest module: CART decision tree + Bagging ensemble.

#[allow(dead_code)]
mod forest;
mod rng;
#[allow(dead_code)]
mod tree;
pub mod types;

pub(crate) use rng::Lcg;
pub use types::RandomForest;

#[cfg(test)]
use tree::{build_tree, predict_one};
#[cfg(test)]
use types::{DecisionTree, TreeNode};

#[cfg(test)]
mod tests;
