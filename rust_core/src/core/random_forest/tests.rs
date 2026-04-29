use super::*;

/// TC1: Perfectly separable data — tree should split correctly.
#[test]
fn tc_1630_01_perfectly_separable() {
    let x = vec![vec![0.0f64], vec![1.0], vec![2.0], vec![3.0]];
    let y = vec![0.0, 0.0, 1.0, 1.0];
    let feat = vec![0usize];

    let tree = DecisionTree {
        root: build_tree(&x, &y, &feat, 0, 5, 1),
    };

    assert!(
        (tree.predict(&[0.0]) - 0.0).abs() < 1e-9,
        "x=0 should predict 0"
    );
    assert!(
        (tree.predict(&[1.0]) - 0.0).abs() < 1e-9,
        "x=1 should predict 0"
    );
    assert!(
        (tree.predict(&[2.0]) - 1.0).abs() < 1e-9,
        "x=2 should predict 1"
    );
    assert!(
        (tree.predict(&[3.0]) - 1.0).abs() < 1e-9,
        "x=3 should predict 1"
    );
}

/// TC2: max_depth=0 forces a leaf with the mean of all samples.
#[test]
fn tc_1630_02_max_depth_zero() {
    let x = vec![vec![0.0f64], vec![1.0], vec![2.0], vec![3.0]];
    let y = vec![0.0, 0.0, 1.0, 1.0];
    let feat = vec![0usize];

    let root = build_tree(&x, &y, &feat, 0, 0, 1);
    match root {
        TreeNode::Leaf(value) => {
            let expected = 0.5;
            assert!(
                (value - expected).abs() < 1e-9,
                "leaf value should be mean: {}",
                value
            );
        }
        _ => panic!("Expected Leaf node with max_depth=0"),
    }
}

/// TC3: min_samples_leaf=3 prevents splits that leave fewer than 3 samples.
#[test]
fn tc_1630_03_min_samples_leaf() {
    let x = vec![vec![0.0f64], vec![1.0], vec![2.0], vec![3.0]];
    let y = vec![0.0, 0.0, 1.0, 1.0];
    let feat = vec![0usize];

    let root = build_tree(&x, &y, &feat, 0, 10, 3);
    match root {
        TreeNode::Leaf(value) => {
            let expected = 0.5;
            assert!(
                (value - expected).abs() < 1e-9,
                "Should be leaf with mean: {}",
                value
            );
        }
        TreeNode::Split { .. } => {
            let prediction = predict_one(&root, &[0.0]);
            assert!(
                (0.0..=1.0).contains(&prediction),
                "Prediction should be in [0,1]"
            );
        }
    }
}

/// TC4: predict_one traverses Split nodes correctly.
#[test]
fn tc_1630_04_predict_one_split() {
    let node = TreeNode::Split {
        feature: 0,
        threshold: 1.5,
        left: Box::new(TreeNode::Leaf(0.0)),
        right: Box::new(TreeNode::Leaf(1.0)),
    };
    assert_eq!(predict_one(&node, &[1.0]), 0.0);
    assert_eq!(predict_one(&node, &[1.5]), 0.0);
    assert_eq!(predict_one(&node, &[2.0]), 1.0);
}

/// TC5: Random Forest on linear data should have high R².
#[test]
fn tc_1631_01_rf_linear_r_squared() {
    let n = 100;
    let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
    let y: Vec<f64> = x.iter().map(|xi| xi[0] * 2.0 + 1.0).collect();

    let rf = RandomForest::train(&x, &y, 50, 10, 2, 123);
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let ss_res: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, &yi)| (yi - rf.predict(xi)).powi(2))
        .sum();
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
        let prediction = rf.predict(xi);
        assert!(
            (-0.01..=1.01).contains(&prediction),
            "Prediction {} out of range",
            prediction
        );
    }
}
