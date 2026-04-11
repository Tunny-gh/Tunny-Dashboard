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

/// Random Forest regressor.
pub(crate) struct RandomForest {
    pub(super) trees: Vec<DecisionTree>,
}
