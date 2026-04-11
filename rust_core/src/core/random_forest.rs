//! Random Forest module: CART decision tree + Bagging ensemble.
//!
//! Implements a pure-Rust Random Forest regressor without external crates.
//! Used for 2D PDP surface computation in `pdp.rs`.

mod forest;
mod pdp;
mod rng;
mod tree;
pub(crate) mod types;

pub(crate) use forest::{extract_columns, mse_on_dataset, train_rf_on_columns};
pub(crate) use pdp::compute_pdp_2d_rf;
pub(crate) use rng::Lcg;

#[cfg(test)]
use tree::{build_tree, predict_one};
#[cfg(test)]
use types::{DecisionTree, RandomForest, TreeNode};

#[cfg(test)]
mod tests;
