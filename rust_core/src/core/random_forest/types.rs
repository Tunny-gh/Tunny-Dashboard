#[allow(dead_code)]
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

#[allow(dead_code)]
/// CART decision tree.
pub(crate) struct DecisionTree {
    pub root: TreeNode,
}

#[allow(dead_code)]
/// Random Forest regressor.
pub(crate) struct RandomForest {
    pub(super) trees: Vec<DecisionTree>,
}
