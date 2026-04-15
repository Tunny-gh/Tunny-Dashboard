use super::*;

fn approx_eq(left: f64, right: f64, tolerance: f64) {
    assert!(
        (left - right).abs() <= tolerance,
        "left={} right={} tolerance={}",
        left,
        right,
        tolerance
    );
}

#[test]
fn cholesky_and_alpha_solve_spd_system() {
    let a = vec![vec![4.0, 2.0], vec![2.0, 3.0]];
    let y = vec![1.0, 2.0];

    let l = cholesky(&a).expect("matrix should be positive definite");
    let alpha = compute_alpha(&l, &y);

    approx_eq(alpha[0], -0.125, 1e-3);
    approx_eq(alpha[1], 0.75, 1e-3);
}

#[test]
fn matern52_diagonal_matches_signal_variance() {
    let x = [0.25, 0.75];
    let log_ls = [0.0, 0.0];
    let log_sf = std::f64::consts::LN_2;

    let kernel = matern52_ard(&x, &x, &log_ls, log_sf);

    approx_eq(kernel, 4.0, 1e-9);
}

#[test]
fn kernel_matrix_is_symmetric_with_noise() {
    let x = vec![vec![0.0], vec![1.0]];
    let matrix = build_kernel_matrix(&x, &[0.0], 0.0, (0.1f64).ln());

    assert_eq!(matrix.len(), 2);
    approx_eq(matrix[0][1], matrix[1][0], 1e-12);
    assert!(matrix[0][0] > 1.0);
    assert!(matrix[1][1] > 1.0);
}

#[test]
fn log_marginal_likelihood_and_gradient_are_finite() {
    let x = vec![vec![0.0], vec![0.5], vec![1.0]];
    let y = vec![0.0, 0.4, 1.0];
    let params = vec![0.0, 0.0, -2.0];

    let likelihood = log_marginal_likelihood(&x, &y, &[params[0]], params[1], params[2]);
    let (likelihood_with_grad, grad) = log_ml_with_gradient(&x, &y, &params);

    assert!(likelihood.is_finite());
    assert!(likelihood_with_grad.is_finite());
    assert_eq!(grad.len(), params.len());
    assert!(grad.iter().all(|value| value.is_finite()));
}

#[test]
fn unified_likelihood_matches_scalar_likelihood() {
    let x = vec![vec![0.0], vec![0.5], vec![1.0]];
    let y = vec![0.0, 0.4, 1.0];
    let params = vec![0.1, -0.2, -1.8];

    let scalar = log_marginal_likelihood(&x, &y, &[params[0]], params[1], params[2]);
    let (combined, _) = log_ml_with_gradient(&x, &y, &params);

    approx_eq(scalar, combined, 1e-10);
}

#[test]
fn likelihood_gradient_matches_finite_difference() {
    let x = vec![vec![0.0], vec![0.5], vec![1.0]];
    let y = vec![0.0, 0.4, 1.0];
    let params = vec![0.1, -0.2, -1.8];
    let epsilon = 1e-6;
    let (_, grad) = log_ml_with_gradient(&x, &y, &params);

    for i in 0..params.len() {
        let mut plus = params.clone();
        let mut minus = params.clone();
        plus[i] += epsilon;
        minus[i] -= epsilon;

        let plus_value = log_marginal_likelihood(&x, &y, &[plus[0]], plus[1], plus[2]);
        let minus_value = log_marginal_likelihood(&x, &y, &[minus[0]], minus[1], minus[2]);
        let finite_diff = (plus_value - minus_value) / (2.0 * epsilon);

        approx_eq(grad[i], finite_diff, 5e-4);
    }
}

#[test]
fn lbfgs_without_history_returns_negative_gradient() {
    let grad = vec![1.0, -2.0];

    let direction = lbfgs_direction(&grad, &[], &[]);

    approx_eq(direction[0], -1.0, 1e-12);
    approx_eq(direction[1], 2.0, 1e-12);
}

#[test]
fn armijo_line_search_reduces_quadratic_objective() {
    let x = vec![1.0, -1.0];
    let grad = vec![1.0, -1.0];
    let direction = vec![-1.0, 1.0];
    let objective = |values: &[f64]| 0.5 * values.iter().map(|value| value * value).sum::<f64>();
    let f_x = objective(&x);

    let alpha = armijo_line_search(f_x, &grad, &direction, objective, &x, 1e-4, 20);
    let x_new: Vec<f64> = x
        .iter()
        .zip(direction.iter())
        .map(|(value, step)| value + alpha * step)
        .collect();

    assert!(alpha > 0.0);
    assert!(objective(&x_new) < f_x);
}

#[test]
fn armijo_line_search_keeps_unit_step_when_sufficient() {
    let x = vec![1.0, 1.0];
    let grad = vec![1.0, 1.0];
    let direction = vec![-1.0, -1.0];
    let objective = |values: &[f64]| 0.5 * values.iter().map(|value| value * value).sum::<f64>();
    let f_x = objective(&x);

    let alpha = armijo_line_search(f_x, &grad, &direction, objective, &x, 1e-4, 20);

    approx_eq(alpha, 1.0, 1e-12);
}

#[test]
fn lbfgs_with_history_still_points_downhill() {
    let grad = vec![1.0, -0.5];
    let s_hist = vec![vec![0.25, -0.1]];
    let y_hist = vec![vec![0.5, -0.2]];

    let direction = lbfgs_direction(&grad, &s_hist, &y_hist);
    let slope: f64 = grad
        .iter()
        .zip(direction.iter())
        .map(|(left, right)| left * right)
        .sum();

    assert!(slope < 0.0);
}

#[test]
fn optimize_hyperparams_returns_expected_shape() {
    let x = vec![vec![0.0], vec![0.5], vec![1.0]];
    let y = vec![0.0, 0.4, 1.0];

    let (params, iterations) = optimize_hyperparams(&x, &y, 3, 3);

    assert_eq!(params.len(), 3);
    assert!(iterations > 0);
    assert!(params.iter().all(|value| value.is_finite()));
}

#[test]
fn optimize_hyperparams_improves_likelihood() {
    let x = vec![vec![0.0], vec![0.5], vec![1.0]];
    let y = vec![0.0, 0.4, 1.0];
    let initial = [0.0, 0.0, -2.0];
    let initial_likelihood = log_marginal_likelihood(&x, &y, &[initial[0]], initial[1], initial[2]);

    let (params, _) = optimize_hyperparams(&x, &y, 10, 5);
    let final_likelihood = log_marginal_likelihood(&x, &y, &[params[0]], params[1], params[2]);

    assert!(final_likelihood >= initial_likelihood - 1e-8);
}

#[test]
fn optimize_hyperparams_handles_empty_input() {
    let (params, iterations) = optimize_hyperparams(&[], &[], 10, 5);

    assert!(params.is_empty());
    assert_eq!(iterations, 0);
}

#[test]
fn predict_mean_matches_single_point_model() {
    let model = GpModel {
        alpha: vec![2.0],
        x_train: vec![vec![1.0]],
        log_ls: vec![0.0],
        log_sf: 0.0,
        l: vec![vec![1.0]],
        log_sn: -2.0,
    };

    let prediction = predict_mean(&model, &[1.0]);

    approx_eq(prediction, 2.0, 1e-12);
}

#[test]
fn train_gp_returns_none_for_empty_input() {
    let model = train_gp(vec![], vec![], 10, 42);

    assert!(model.is_none());
}

#[test]
fn train_gp_subsamples_large_dataset() {
    let x: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64 / 20.0]).collect();
    let y: Vec<f64> = x.iter().map(|row| row[0] * 2.0).collect();

    let model = train_gp(x, y, 5, 42).expect("training should succeed");
    let prediction = predict_mean(&model, &[0.5]);

    assert_eq!(model.x_train.len(), 5);
    assert_eq!(model.log_ls.len(), 1);
    assert!(prediction.is_finite());
}

#[test]
fn train_gp_uses_full_dataset_when_below_limit() {
    let x: Vec<Vec<f64>> = (0..4).map(|i| vec![i as f64 / 4.0]).collect();
    let y: Vec<f64> = x.iter().map(|row| row[0] * 1.5).collect();

    let model = train_gp(x, y, 10, 7).expect("training should succeed");

    assert_eq!(model.x_train.len(), 4);
}

#[test]
fn trained_gp_predictions_track_training_signal() {
    let x: Vec<Vec<f64>> = (0..6).map(|i| vec![i as f64 / 5.0]).collect();
    let y: Vec<f64> = x.iter().map(|row| row[0] * 2.0 - 0.5).collect();

    let model = train_gp(x.clone(), y.clone(), 10, 42).expect("training should succeed");
    let mae: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(input, expected)| (predict_mean(&model, input) - expected).abs())
        .sum::<f64>()
        / x.len() as f64;

    assert!(mae < 0.35, "mae={}", mae);
}

#[test]
fn predict_variance_at_training_point_is_near_zero() {
    let x: Vec<Vec<f64>> = vec![vec![0.0], vec![0.5], vec![1.0]];
    let y: Vec<f64> = vec![0.0, 0.5, 1.0];

    let model = train_gp(x.clone(), y, 10, 42).expect("training should succeed");
    let var_at_train = predict_variance(&model, &[0.5]);

    // Variance at a training point should be very small (near 0)
    assert!(var_at_train >= 0.0, "variance must be non-negative");
    assert!(var_at_train < 0.1, "variance at training point should be small: {}", var_at_train);
}

#[test]
fn predict_variance_is_nonnegative() {
    let x: Vec<Vec<f64>> = (0..5).map(|i| vec![i as f64 / 4.0]).collect();
    let y: Vec<f64> = x.iter().map(|row| row[0] * 2.0).collect();

    let model = train_gp(x, y, 10, 42).expect("training should succeed");

    for xi in [0.1, 0.3, 0.7, 1.5] {
        let var = predict_variance(&model, &[xi]);
        assert!(var >= 0.0, "variance must be non-negative at x={}, got {}", xi, var);
    }
}
