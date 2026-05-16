use crate::sensitivity::{
    compute_mdi_importances, compute_rf_anova_importances, compute_shap_importances,
};

fn make_xy(n: usize, dominant: usize, n_feats: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    let mut rng = crate::core::math::rng::SeededRng::from_seed(99);
    let x: Vec<Vec<f64>> = (0..n)
        .map(|_| {
            (0..n_feats)
                .map(|_| rng.next_usize(1000) as f64 / 1000.0)
                .collect()
        })
        .collect();
    let y: Vec<f64> = x.iter().map(|row| row[dominant] * 10.0).collect();
    (x, y)
}

#[test]
fn integration_shap_importances_sum_to_one() {
    let (x, y) = make_xy(60, 0, 3);
    let (importances, _r2) = compute_shap_importances(&x, &y);
    assert_eq!(importances.len(), 3);
    let sum: f64 = importances.iter().sum();
    assert!((sum - 1.0).abs() < 1e-9 || sum == 0.0, "SHAP sum={sum}");
}

#[test]
fn integration_mdi_importances_sum_to_one() {
    let (x, y) = make_xy(60, 1, 3);
    let (importances, _r2) = compute_mdi_importances(&x, &y);
    assert_eq!(importances.len(), 3);
    let sum: f64 = importances.iter().sum();
    assert!((sum - 1.0).abs() < 1e-9 || sum == 0.0, "MDI sum={sum}");
}

#[test]
fn integration_rf_anova_importances_sum_to_one() {
    let (x, y) = make_xy(60, 2, 3);
    let (importances, _r2) = compute_rf_anova_importances(&x, &y);
    assert_eq!(importances.len(), 3);
    let sum: f64 = importances.iter().sum();
    assert!((sum - 1.0).abs() < 1e-9 || sum == 0.0, "RF-ANOVA sum={sum}");
}

#[test]
fn integration_pdp_2d_returns_result() {
    let (x, y) = make_xy(40, 0, 2);
    let result = crate::core::lgbm::compute_pdp_2d_lgbm(&x, &y, 0, 1, 5);
    assert!(result.is_some(), "compute_pdp_2d_lgbm should return Some");
    let (x_vals, y_vals, z_vals, _r2) = result.unwrap();
    assert_eq!(x_vals.len(), 5);
    assert_eq!(y_vals.len(), 5);
    // z_vals has shape [n_grid][n_grid]
    assert_eq!(z_vals.len(), 5);
    assert_eq!(z_vals[0].len(), 5);
}
