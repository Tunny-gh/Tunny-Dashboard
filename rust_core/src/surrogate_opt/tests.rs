use super::*;
use crate::math::rng::SeededRng;

// ────────────────────────────────────────────────────────────
// Helper functions
// ────────────────────────────────────────────────────────────

/// Generates data for a constrained quadratic function.
/// f = (x - 0.3)^2 + (y - 0.7)^2, c = 0.5 - x (c ≤ 0 ⟺ x ≥ 0.5).
fn constrained_quadratic_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<f64>) {
    let mut rng = SeededRng::from_seed(7);
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|r| (r[0] - 0.3).powi(2) + (r[1] - 0.7).powi(2))
        .collect();
    // c = 0.5 - x: feasible ⟺ x >= 0.5
    let c: Vec<f64> = x_matrix.iter().map(|r| 0.5 - r[0]).collect();
    (x_matrix, y, c)
}

/// Builds test data by evaluating the known function f(x, y) = (x − 0.3)² + (y − 0.7)²
/// at sample points within [0,1]² (minimum value 0 at (0.3, 0.7)).
fn quadratic_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    let mut rng = SeededRng::from_seed(7);
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|r| (r[0] - 0.3).powi(2) + (r[1] - 0.7).powi(2))
        .collect();
    (x_matrix, y)
}

/// Piecewise (discontinuous) test function. A case where MoE has an advantage
/// over a single GP. Uses different functional forms for x[0] < 0.5 and x[0] >= 0.5.
fn piecewise_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    // Uses the same LCG-based RNG as make_piecewise in gaussian_process.rs.
    let mut state: u64 = 13;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|_| vec![next(), next()]).collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|row| {
            if row[0] < 0.5 {
                (row[0] * 6.0).sin() + row[1]
            } else {
                5.0 + (row[0] * 3.0).cos() - 2.0 * row[1]
            }
        })
        .collect();
    (x_matrix, y)
}

fn base_request(x_matrix: Vec<Vec<f64>>, y: Vec<f64>) -> SurrogateOptRequest {
    SurrogateOptRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        minimize: true,
        model: SurrogateModelKind::GpFitc,
        optimizer: OptimizerKind::MultiStartLbfgs,
        slice_params: Some((0, 1)),
        n_grid: 10,
        constraints: vec![],
    }
}

// ────────────────────────────────────────────────────────────
// Strict verification of "surface-consuming" logic via an analytic mock surrogate
//
// Injects a known closed-form surface behind the same interface, in place of an
// actual GP fit. Since the surface is analytically known, we can strictly verify
// that the optimizer reaches the true optimum instead of relying on a loose
// tolerance, and because no GP fit (COBYLA hyperparameter optimization) runs, it
// executes instantly and deterministically. The fit quality of the GP backend
// itself is egobox's responsibility, and is covered by the minimal smoke tests
// gp_fitc_runs_and_finds_minimum_region / gp_vfe_* / gp_moe_*.
// ────────────────────────────────────────────────────────────

/// Known convex quadratic surface f(x, y) = (x − 0.3)² + (y − 0.7)².
/// The global minimum within [0,1]² is (0.3, 0.7) with value 0.
fn quad_surface(x: &[f64]) -> f64 {
    (x[0] - 0.3).powi(2) + (x[1] - 0.7).powi(2)
}

/// Known linear surface f(x, y) = 2x − y. The minimum within [0,1]² is at the
/// corner (0, 1) with value −1.
fn linear_surface(x: &[f64]) -> f64 {
    2.0 * x[0] - x[1]
}

/// Builds an analytic mock TrainedSurrogate for the given surface.
/// When `with_variance` is true, it behaves like a GP-family model with a constant
/// posterior variance of 0.01 (std 0.1); when false, it behaves like a Ridge-family
/// model with no posterior variance.
/// `x_matrix` is only a coarse sample used to compute the optimization start point
/// (the best observation).
fn analytic_trained(surface: fn(&[f64]) -> f64, with_variance: bool) -> TrainedSurrogate {
    let var: Option<models::AnalyticFn> = if with_variance {
        Some(Box::new(|_x: &[f64]| 0.01))
    } else {
        None
    };
    let surrogate = models::FittedSurrogate::analytic(2, surface, var);
    // Place observations in both the minimum and maximum basins so the start
    // point (best observation) doesn't get stuck in a local optimum.
    // Even single-start CMA-ES can reach the global optimum if the best
    // observation lies in its basin.
    let x_matrix = vec![
        vec![0.2, 0.8],  // Minimum basin of the quadratic surface (near (0.3,0.7))
        vec![0.9, 0.05], // Maximum basin of the quadratic surface (near the farthest corner (1,0))
        vec![0.5, 0.5],
        vec![0.1, 0.9],
    ];
    let y: Vec<f64> = x_matrix.iter().map(|r| surface(r)).collect();
    TrainedSurrogate::analytic_mock(x_matrix, y, surrogate)
}

/// Runs the given optimization method on the quadratic surface mock.
fn optimize_quad(optimizer: OptimizerKind, minimize: bool) -> SurrogateOptResult {
    let trained = analytic_trained(quad_surface, true);
    optimize_on_trained(
        &trained,
        &SurrogateOptimizeSpec {
            minimize,
            optimizer,
            slice_params: None,
            n_grid: 10,
        },
    )
}

#[test]
fn lbfgs_finds_exact_quadratic_minimum() {
    let result = optimize_quad(OptimizerKind::MultiStartLbfgs, true);
    // Since the surface is known, the gradient method reaches the true minimum
    // (0.3, 0.7) exactly.
    assert!(
        (result.best_params[0] - 0.3).abs() < 0.01,
        "x ≈ 0.3, got {}",
        result.best_params[0]
    );
    assert!(
        (result.best_params[1] - 0.7).abs() < 0.01,
        "y ≈ 0.7, got {}",
        result.best_params[1]
    );
    assert!(
        result.best_value < 1e-3,
        "predicted minimum ≈ 0, got {}",
        result.best_value
    );
    // Since the mock has a posterior variance, predicted_std is Some (std = sqrt(0.01) = 0.1).
    assert!(
        result.predicted_std.is_some(),
        "GP-like mock has posterior std"
    );
    assert!(
        (result.predicted_std.unwrap() - 0.1).abs() < 1e-9,
        "std should be exactly sqrt(0.01) = 0.1, got {}",
        result.predicted_std.unwrap()
    );

    // best_observed_value exactly matches the minimum observed value (0.02, from
    // the mock observation point (0.2,0.8)).
    let expected = analytic_trained(quad_surface, true)
        .y
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    assert_eq!(result.best_observed_value.to_bits(), expected.to_bits());
}

#[test]
fn random_search_finds_quadratic_minimum_loosely() {
    let result = optimize_quad(OptimizerKind::RandomSearch, true);
    // Random search (4096 points) allows for error on the order of the grid resolution.
    assert!((result.best_params[0] - 0.3).abs() < 0.1);
    assert!((result.best_params[1] - 0.7).abs() < 0.1);
}

#[test]
fn nsga2_finds_quadratic_minimum() {
    let result = optimize_quad(OptimizerKind::Nsga2, true);
    assert!(
        (result.best_params[0] - 0.3).abs() < 0.05,
        "x ≈ 0.3, got {}",
        result.best_params[0]
    );
    assert!(
        (result.best_params[1] - 0.7).abs() < 0.05,
        "y ≈ 0.7, got {}",
        result.best_params[1]
    );
}

#[test]
fn cma_es_finds_quadratic_minimum() {
    let result = optimize_quad(OptimizerKind::CmaEs, true);
    assert!(
        (result.best_params[0] - 0.3).abs() < 0.02,
        "x ≈ 0.3, got {}",
        result.best_params[0]
    );
    assert!(
        (result.best_params[1] - 0.7).abs() < 0.02,
        "y ≈ 0.7, got {}",
        result.best_params[1]
    );
}

#[test]
fn cma_es_maximize_finds_quadratic_maximum_corner() {
    // f is maximized at the corner (1, 0), the farthest point from (0.3, 0.7).
    let result = optimize_quad(OptimizerKind::CmaEs, false);
    assert!(
        result.best_params[0] > 0.9,
        "x should approach 1, got {}",
        result.best_params[0]
    );
    assert!(
        result.best_params[1] < 0.1,
        "y should approach 0, got {}",
        result.best_params[1]
    );
}

#[test]
fn maximize_direction_finds_quadratic_maximum_corner() {
    // In the maximize direction, the gradient method reaches the farthest corner
    // (1, 0) exactly.
    let result = optimize_quad(OptimizerKind::MultiStartLbfgs, false);
    assert!(
        result.best_params[0] > 0.95,
        "x should approach 1, got {}",
        result.best_params[0]
    );
    assert!(
        result.best_params[1] < 0.05,
        "y should approach 0, got {}",
        result.best_params[1]
    );
}

#[test]
fn no_variance_model_reaches_box_corner() {
    // The linear surface f = 2x − y is minimized at the box corner (0, 1). Also
    // verifies that predicted_std is None for the no-variance mock (Ridge family).
    let trained = analytic_trained(linear_surface, false);
    let result = optimize_on_trained(
        &trained,
        &SurrogateOptimizeSpec {
            minimize: true,
            optimizer: OptimizerKind::MultiStartLbfgs,
            slice_params: None,
            n_grid: 10,
        },
    );

    assert!(
        result.best_params[0] < 0.01,
        "x → 0: {}",
        result.best_params[0]
    );
    assert!(
        result.best_params[1] > 0.99,
        "y → 1: {}",
        result.best_params[1]
    );
    assert!(
        result.predicted_std.is_none(),
        "variance-less mock has no posterior std"
    );
}

#[test]
fn slice_grid_passes_through_optimum_and_has_expected_shape() {
    // Builds a 2D slice grid from the known quadratic surface and strictly
    // verifies its shape and consistency with the minimum.
    let trained = analytic_trained(quad_surface, true);
    let result = optimize_on_trained(
        &trained,
        &SurrogateOptimizeSpec {
            minimize: true,
            optimizer: OptimizerKind::MultiStartLbfgs,
            slice_params: Some((0, 1)),
            n_grid: 12,
        },
    );
    let slice = result.slice.expect("slice requested");

    assert_eq!(slice.x_values.len(), 12);
    assert_eq!(slice.y_values.len(), 12);
    assert_eq!(slice.z_values.len(), 12);
    assert!(slice.z_values.iter().all(|row| row.len() == 12));
    // Since normalization is the identity, the grid evenly divides [0,1], and
    // z matches f(grid point) exactly.
    for (i, &vx) in slice.x_values.iter().enumerate() {
        for (j, &vy) in slice.y_values.iter().enumerate() {
            let expected = quad_surface(&[vx, vy]);
            assert!(
                (slice.z_values[i][j] - expected).abs() < 1e-9,
                "z[{i}][{j}] should equal f(grid): {} vs {}",
                slice.z_values[i][j],
                expected
            );
        }
    }
    // Constant posterior variance → z_std is 0.1 across the whole grid.
    let z_std = slice.z_std.expect("GP-like mock has z_std");
    assert!(z_std.iter().flatten().all(|&s| (s - 0.1).abs() < 1e-9));
    // The grid minimum lies close to the surrogate's optimal value.
    let grid_min = slice
        .z_values
        .iter()
        .flatten()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    assert!(
        grid_min <= result.best_value + 0.05,
        "grid_min {} vs best {}",
        grid_min,
        result.best_value
    );
}

#[test]
fn invalid_slice_params_are_ignored() {
    let trained = analytic_trained(quad_surface, true);
    let result = optimize_on_trained(
        &trained,
        &SurrogateOptimizeSpec {
            minimize: true,
            optimizer: OptimizerKind::MultiStartLbfgs,
            slice_params: Some((0, 0)), // same axis is invalid
            n_grid: 10,
        },
    );
    assert!(result.slice.is_none());
}

#[test]
fn gp_fitc_runs_and_finds_minimum_region() {
    let (x_matrix, y) = quadratic_samples(40);
    let mut req = base_request(x_matrix, y);
    req.model = SurrogateModelKind::GpFitc;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

    assert!((result.best_params[0] - 0.3).abs() < 0.2);
    assert!((result.best_params[1] - 0.7).abs() < 0.2);
    assert!(result.predicted_std.is_some(), "GP-FITC has posterior std");
}

#[test]
fn too_few_trials_returns_error() {
    let (x_matrix, y) = quadratic_samples(5);
    let req = base_request(x_matrix, y);
    let err = run_surrogate_optimization(&req).unwrap_err();
    assert!(err.contains("At least"), "unexpected error: {err}");
}

#[test]
fn non_finite_input_returns_error() {
    let (mut x_matrix, y) = quadratic_samples(20);
    x_matrix[3][0] = f64::NAN;
    let req = base_request(x_matrix, y);
    assert!(run_surrogate_optimization(&req).is_err());
}

// ============================================================================
// Send + Sync assertions for TrainedSurrogate
// ============================================================================

/// Verifies at compile time that TrainedSurrogate satisfies Send + Sync.
#[test]
fn trained_surrogate_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TrainedSurrogate>();
}

// ============================================================================
// Tests for validate_surrogate
// ============================================================================

// NOTE: We don't verify GP fit quality (e.g. high R² for smooth functions),
// since that's the responsibility of the egobox backend. The structure of the
// validation report (n_samples / cv_folds / oof_pairs length, all values finite)
// is checked by validate_surrogate_minimum_size_dataset and
// validate_surrogate_deterministic_with_same_seed.

#[test]
fn validate_surrogate_deterministic_with_same_seed() {
    // Verifies that results from calls with the same seed match exactly.
    let (x_matrix, y) = quadratic_samples(20);
    let r1 = validation::validate_surrogate(SurrogateModelKind::GpFitc, &x_matrix, &y, 42)
        .expect("first call should succeed");
    let r2 = validation::validate_surrogate(SurrogateModelKind::GpFitc, &x_matrix, &y, 42)
        .expect("second call should succeed");

    assert_eq!(r1.n_train, r2.n_train);
    assert_eq!(r1.n_test, r2.n_test);
    assert_eq!(
        r1.holdout_r2.to_bits(),
        r2.holdout_r2.to_bits(),
        "holdout_r2 must be identical"
    );
    assert_eq!(r1.holdout_rmse.to_bits(), r2.holdout_rmse.to_bits());
    assert_eq!(r1.cv_r2_mean.to_bits(), r2.cv_r2_mean.to_bits());
    assert_eq!(r1.oof_pairs.len(), r2.oof_pairs.len());
    for (a, b) in r1.oof_pairs.iter().zip(r2.oof_pairs.iter()) {
        assert_eq!(a.0.to_bits(), b.0.to_bits());
        assert_eq!(a.1.to_bits(), b.1.to_bits());
    }
}

#[test]
fn validate_surrogate_minimum_size_dataset() {
    // Verifies that validation succeeds on the minimum-size dataset (n = 10) and
    // has the expected fields.
    let (x_matrix, y) = quadratic_samples(10);
    let report = validation::validate_surrogate(SurrogateModelKind::GpFitc, &x_matrix, &y, 42)
        .expect("minimum-size validate_surrogate should succeed");

    assert_eq!(report.n_samples, 10);
    assert!(report.n_test >= 1, "n_test >= 1");
    assert_eq!(report.cv_folds, 5, "k = min(5, 10) = 5");
    assert_eq!(report.oof_pairs.len(), 10);
    assert!(report.holdout_r2.is_finite());
    assert!(report.cv_rmse_mean.is_finite());
}

// ============================================================================
// End-to-end tests for fit_surrogate_with_validation + optimize_on_trained
// ============================================================================

#[test]
fn fit_and_optimize_on_trained_finds_quadratic_minimum() {
    let (x_matrix, y) = quadratic_samples(40);

    let fit_req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        model: SurrogateModelKind::GpFitc,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    };
    let trained = fit_surrogate_with_validation(&fit_req)
        .expect("fit_surrogate_with_validation should succeed");

    // Basic checks on the validation report.
    assert_eq!(trained.validation.n_samples, 40);
    assert!(
        trained.validation.train_r2 > 0.8,
        "train_r2 = {}",
        trained.validation.train_r2
    );
    assert!(trained.validation.holdout_r2.is_finite());
    assert!(trained.validation.cv_r2_mean.is_finite());

    let spec = SurrogateOptimizeSpec {
        minimize: true,
        optimizer: OptimizerKind::MultiStartLbfgs,
        slice_params: Some((0, 1)),
        n_grid: 10,
    };
    let result = optimize_on_trained(&trained, &spec);

    assert!(
        (result.best_params[0] - 0.3).abs() < 0.1,
        "x ≈ 0.3, got {}",
        result.best_params[0]
    );
    assert!(
        (result.best_params[1] - 0.7).abs() < 0.1,
        "y ≈ 0.7, got {}",
        result.best_params[1]
    );
    assert!(
        result.best_value < 0.05,
        "best_value = {}",
        result.best_value
    );
    assert!(result.r_squared > 0.8);
    assert!(result.slice.is_some(), "スライスが要求されている");
}

// ────────────────────────────────────────────────────────────
// ARD parameter importance (param_importance)
// ────────────────────────────────────────────────────────────

#[test]
fn param_importance_reflects_ard_for_gp_and_none_for_others() {
    // Function y = 3*x0 + 0.05*x1 (noiseless), strongly dependent on x0 and
    // nearly independent of x1.
    let mut rng = SeededRng::from_seed(7);
    let x_matrix: Vec<Vec<f64>> = (0..40)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix.iter().map(|r| 3.0 * r[0] + 0.05 * r[1]).collect();

    let make = |model: SurrogateModelKind| {
        let req = SurrogateFitRequest {
            x_matrix: x_matrix.clone(),
            y: y.clone(),
            param_names: vec!["x0".to_string(), "x1".to_string()],
            objective_name: "obj0".to_string(),
            model,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
        };
        fit_surrogate_with_validation(&req).expect("fit should succeed")
    };

    // GP-FITC: Some, length 2, sum ≈ 1.0, importance[0] > importance[1].
    let gp = make(SurrogateModelKind::GpFitc);
    let imp = gp
        .param_importance
        .as_ref()
        .expect("GP should expose param_importance");
    assert_eq!(imp.len(), 2);
    let sum: f64 = imp.iter().sum();
    assert!(
        (sum - 1.0).abs() < 1e-9,
        "importance should sum to 1: {sum}"
    );
    assert!(
        imp[0] > imp[1],
        "x0 should be more important than x1: {imp:?}"
    );

    // Ridge / LightGBM don't have ARD, so this is None.
    assert!(make(SurrogateModelKind::Ridge).param_importance.is_none());
    assert!(make(SurrogateModelKind::Lgbm).param_importance.is_none());

    // MoE is None because θ is split per expert and aggregation isn't unique.
    // Since MoE's CV training can degenerate on this purely linear, noiseless
    // data, we train the MoE model directly with models::fit_surrogate (which
    // doesn't go through CV) and check param_importance there.
    if let Ok(moe) = models::fit_surrogate(SurrogateModelKind::GpMoe, &x_matrix, &y) {
        assert!(moe.param_importance().is_none());
    }
}

// ────────────────────────────────────────────────────────────
// Additional coverage for GpVfe / GpMoe
// ────────────────────────────────────────────────────────────

#[test]
fn gp_vfe_trains_and_predicts_finite_with_std() {
    // Verifies that GP-VFE can train and predict on quadratic function data,
    // and that predicted_std is Some. Uses run_surrogate_optimization so we
    // don't depend on the small subsets of a CV fold.
    let (x_matrix, y) = quadratic_samples(40);
    let req = SurrogateOptRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        minimize: true,
        model: SurrogateModelKind::GpVfe,
        optimizer: OptimizerKind::MultiStartLbfgs,
        slice_params: None,
        n_grid: 10,
        constraints: vec![],
    };
    let result = run_surrogate_optimization(&req).expect("GP-VFE optimization should succeed");

    assert!(
        result.best_value.is_finite(),
        "GP-VFE best_value should be finite"
    );
    assert!(
        result.predicted_std.is_some(),
        "GP-VFE should return Some(predicted_std)"
    );
    assert!(result.predicted_std.unwrap().is_finite());
    assert!(result.r_squared.is_finite());
}

#[test]
fn gp_moe_trains_and_predicts_finite_with_std() {
    // Verifies that GP-MOE can train and predict on piecewise (discontinuous)
    // function data, and that predicted_std is Some. With a smooth quadratic
    // function (quadratic_samples), egobox-moe fails to pick a single cluster
    // and panics internally, so we use piecewise data where MoE actually has
    // an advantage.
    let (x_matrix, y) = piecewise_samples(60);
    let req = SurrogateOptRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        minimize: true,
        model: SurrogateModelKind::GpMoe,
        optimizer: OptimizerKind::MultiStartLbfgs,
        slice_params: None,
        n_grid: 10,
        constraints: vec![],
    };
    let result = run_surrogate_optimization(&req).expect("GP-MOE optimization should succeed");

    assert!(
        result.best_value.is_finite(),
        "GP-MOE best_value should be finite"
    );
    assert!(
        result.predicted_std.is_some(),
        "GP-MOE should return Some(predicted_std)"
    );
    assert!(result.predicted_std.unwrap().is_finite());
    assert!(result.r_squared.is_finite());
}

// ────────────────────────────────────────────────────────────
// Multi-objective surrogate optimization tests
// ────────────────────────────────────────────────────────────

/// Generates trade-off data equivalent to Schaffer N.1.
/// f1 = x0², f2 = (x0 − 1)², x0 ∈ [0,1], x1 is a dummy dimension.
fn schaffer_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<f64>) {
    let mut rng = SeededRng::from_seed(42);
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let f1: Vec<f64> = x_matrix.iter().map(|r| r[0].powi(2)).collect();
    let f2: Vec<f64> = x_matrix.iter().map(|r| (r[0] - 1.0).powi(2)).collect();
    (x_matrix, f1, f2)
}

fn base_multi_request(
    x_matrix: Vec<Vec<f64>>,
    f1: Vec<f64>,
    f2: Vec<f64>,
) -> SurrogateMultiOptRequest {
    SurrogateMultiOptRequest {
        x_matrix,
        ys: vec![f1, f2],
        param_names: vec!["x0".to_string(), "x1".to_string()],
        objective_names: vec!["f1".to_string(), "f2".to_string()],
        minimize: vec![true, true],
        model: SurrogateModelKind::GpFitc,
        slice_params: Some((0, 1)),
        n_grid: 10,
    }
}

// ── Strict verification of the multi-objective front mechanism via an analytic mock ──
// Schaffer N.1: f1 = x0², f2 = (x0 − 1)² (x1 is a dummy dimension). Injects a
// known surface and verifies NSGA-II's front generation, sorting, slicing, and
// maximize-direction wiring without an actual GP fit.

fn schaffer_f1(x: &[f64]) -> f64 {
    x[0].powi(2)
}
fn schaffer_f2(x: &[f64]) -> f64 {
    (x[0] - 1.0).powi(2)
}

/// A set of TrainedSurrogates for the two Schaffer N.1 objectives, pre-trained
/// via the analytic mock.
fn analytic_schaffer_trained() -> Vec<TrainedSurrogate> {
    let x_matrix = vec![
        vec![0.0, 0.5],
        vec![0.25, 0.5],
        vec![0.5, 0.5],
        vec![0.75, 0.5],
        vec![1.0, 0.5],
    ];
    let mk = |surface: fn(&[f64]) -> f64| {
        let s = models::FittedSurrogate::analytic(2, surface, Some(Box::new(|_x: &[f64]| 0.01)));
        let y: Vec<f64> = x_matrix.iter().map(|r| surface(r)).collect();
        TrainedSurrogate::analytic_mock(x_matrix.clone(), y, s)
    };
    vec![mk(schaffer_f1), mk(schaffer_f2)]
}

fn optimize_schaffer(
    minimize: Vec<bool>,
    slice_params: Option<(usize, usize)>,
) -> SurrogateMultiOptResult {
    let trained = analytic_schaffer_trained();
    let refs: Vec<&TrainedSurrogate> = trained.iter().collect();
    optimize_multi_on_trained(
        &refs,
        &SurrogateMultiOptimizeSpec {
            minimize,
            slice_params,
            n_grid: 10,
        },
    )
    .expect("staged multi optimize should succeed")
}

#[test]
fn multi_opt_front_spans_full_tradeoff() {
    // Since the surface is known, the front strictly spans the true Schaffer
    // front (the entire range).
    let result = optimize_schaffer(vec![true, true], None);

    assert!(
        result.front.len() >= 5,
        "front should have ≥5 points, got {}",
        result.front.len()
    );

    // f1 end: a point with x0 < 0.2 exists (small f1, large f2 side).
    let has_f1_side = result
        .front
        .iter()
        .any(|p| p.params[0] < 0.2 || p.values[0] < 0.05);
    assert!(has_f1_side, "front should reach f1-dominated region");

    // f2 end: a point with x0 > 0.8 exists (small f2, large f1 side).
    let has_f2_side = result
        .front
        .iter()
        .any(|p| p.params[0] > 0.8 || p.values[1] < 0.05);
    assert!(has_f2_side, "front should reach f2-dominated region");
}

#[test]
fn multi_opt_front_sorted_by_first_objective() {
    // The front should be sorted ascending by the first objective.
    let result = optimize_schaffer(vec![true, true], None);

    for w in result.front.windows(2) {
        assert!(
            w[0].values[0] <= w[1].values[0],
            "front not sorted: {} > {}",
            w[0].values[0],
            w[1].values[0]
        );
    }
}

#[test]
fn multi_opt_point_dimensions_match() {
    // Each ParetoFrontPoint's params/values length matches param_names/objective_names.
    let result = optimize_schaffer(vec![true, true], None);

    for (i, p) in result.front.iter().enumerate() {
        assert_eq!(
            p.params.len(),
            2,
            "point[{}] params len mismatch: {}",
            i,
            p.params.len()
        );
        assert_eq!(
            p.values.len(),
            2,
            "point[{}] values len mismatch: {}",
            i,
            p.values.len()
        );
        // Since the surface is known, each point's predicted value matches
        // f(params) exactly.
        assert!((p.values[0] - schaffer_f1(&p.params)).abs() < 1e-9);
        assert!((p.values[1] - schaffer_f2(&p.params)).abs() < 1e-9);
    }
    assert_eq!(result.r_squared.len(), 2, "r_squared should have 2 entries");
}

#[test]
fn multi_opt_maximize_objective_direction() {
    // When maximizing f2 (minimize=false), the resulting f2 values should be
    // distributed toward the positive direction.
    // f2 = (x0 − 1)² has its maximum value 1 at x0=0 and minimum value 0 at x0=1.
    let result = optimize_schaffer(vec![true, false], None);

    // A point with a value ≥ 0.5 for the maximized objective f2 should exist
    // on the front.
    let has_large_f2 = result.front.iter().any(|p| p.values[1] > 0.5);
    assert!(
        has_large_f2,
        "front should include high-f2 point when maximizing f2, max f2 = {}",
        result
            .front
            .iter()
            .map(|p| p.values[1])
            .fold(f64::NEG_INFINITY, f64::max)
    );
}

#[test]
fn multi_opt_slices_returned_for_each_objective() {
    // When slice_params is specified, a slice should be returned per objective.
    let result = optimize_schaffer(vec![true, true], Some((0, 1)));

    assert_eq!(
        result.slices.len(),
        2,
        "should return 2 slices (one per objective)"
    );
    for (k, slice) in result.slices.iter().enumerate() {
        assert_eq!(slice.x_values.len(), 10, "slice[{}] x_values len", k);
        assert_eq!(slice.y_values.len(), 10, "slice[{}] y_values len", k);
        assert_eq!(slice.z_values.len(), 10, "slice[{}] z_values rows", k);
    }
}

#[test]
fn multi_opt_no_slices_without_slice_params() {
    // slices should be empty when slice_params = None.
    let result = optimize_schaffer(vec![true, true], None);
    assert!(
        result.slices.is_empty(),
        "slices should be empty when slice_params is None"
    );
}

#[test]
fn multi_opt_error_on_single_objective() {
    // Having only 1 objective is an error.
    let (x_matrix, f1, _) = schaffer_samples(30);
    let req = SurrogateMultiOptRequest {
        x_matrix,
        ys: vec![f1],
        param_names: vec!["x0".to_string(), "x1".to_string()],
        objective_names: vec!["f1".to_string()],
        minimize: vec![true],
        model: SurrogateModelKind::GpFitc,
        slice_params: None,
        n_grid: 10,
    };
    let err = run_surrogate_multi_optimization(&req).unwrap_err();
    assert!(
        err.contains("At least 2 objectives"),
        "unexpected error: {err}"
    );
}

#[test]
fn multi_opt_error_on_ys_length_mismatch() {
    // It's an error when ys and objective_names have different lengths.
    let (x_matrix, f1, f2) = schaffer_samples(30);
    let req = SurrogateMultiOptRequest {
        x_matrix,
        ys: vec![f1, f2],
        param_names: vec!["x0".to_string(), "x1".to_string()],
        objective_names: vec!["f1".to_string()], // length mismatch
        minimize: vec![true, true],
        model: SurrogateModelKind::GpFitc,
        slice_params: None,
        n_grid: 10,
    };
    let err = run_surrogate_multi_optimization(&req).unwrap_err();
    assert!(
        err.contains("objective_names length mismatch"),
        "unexpected error: {err}"
    );
}

#[test]
fn multi_opt_error_on_too_few_trials() {
    // Too few trials is an error.
    let (x_matrix, f1, f2) = schaffer_samples(5);
    let req = base_multi_request(x_matrix, f1, f2);
    let err = run_surrogate_multi_optimization(&req).unwrap_err();
    assert!(err.contains("At least"), "unexpected error: {err}");
}

// ────────────────────────────────────────────────────────────
// Tests for the two-stage flow (fit → optimize_multi_on_trained)
// ────────────────────────────────────────────────────────────

/// Trains the two objectives from schaffer_samples with fit_surrogate_with_validation.
fn fit_schaffer_trained(n: usize) -> (TrainedSurrogate, TrainedSurrogate) {
    let (x_matrix, f1, f2) = schaffer_samples(n);
    let names = vec!["x0".to_string(), "x1".to_string()];
    let t1 = fit_surrogate_with_validation(&SurrogateFitRequest {
        x_matrix: x_matrix.clone(),
        y: f1,
        param_names: names.clone(),
        objective_name: "f1".to_string(),
        model: SurrogateModelKind::GpFitc,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    })
    .expect("fit f1 should succeed");
    let t2 = fit_surrogate_with_validation(&SurrogateFitRequest {
        x_matrix,
        y: f2,
        param_names: names,
        objective_name: "f2".to_string(),
        model: SurrogateModelKind::GpFitc,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    })
    .expect("fit f2 should succeed");
    (t1, t2)
}

// NOTE: That the front spans the entire trade-off and that a slice is returned
// per objective is strictly verified by the analytic mock versions
// multi_opt_front_spans_full_tradeoff / multi_opt_slices_returned_for_each_objective.
// staged_multi_opt_matches_one_shot_result guarantees the equivalence of the
// two-stage flow (fit → optimize_multi_on_trained) with a real fit.

#[test]
fn staged_multi_opt_error_on_single_trained() {
    // Only a single trained surrogate is an error.
    let (t1, _) = fit_schaffer_trained(30);
    let spec = SurrogateMultiOptimizeSpec {
        minimize: vec![true],
        slice_params: None,
        n_grid: 10,
    };
    let err = optimize_multi_on_trained(&[&t1], &spec).unwrap_err();
    assert!(
        err.contains("At least 2 objectives"),
        "unexpected error: {err}"
    );
}

#[test]
fn staged_multi_opt_error_on_minimize_length_mismatch() {
    // It's an error when minimize's length differs from trained's.
    let (t1, t2) = fit_schaffer_trained(30);
    let spec = SurrogateMultiOptimizeSpec {
        minimize: vec![true], // length mismatch
        slice_params: None,
        n_grid: 10,
    };
    let err = optimize_multi_on_trained(&[&t1, &t2], &spec).unwrap_err();
    assert!(
        err.contains("minimize length mismatch"),
        "unexpected error: {err}"
    );
}

#[test]
fn staged_multi_opt_matches_one_shot_result() {
    // With the same data and a deterministic seed, the one-shot and staged
    // versions should produce equivalent fronts.
    let (x_matrix, f1, f2) = schaffer_samples(40);
    let req = base_multi_request(x_matrix, f1.clone(), f2.clone());
    let one_shot = run_surrogate_multi_optimization(&req).expect("one-shot should succeed");

    let (t1, t2) = fit_schaffer_trained(40);
    let spec = SurrogateMultiOptimizeSpec {
        minimize: vec![true, true],
        slice_params: Some((0, 1)),
        n_grid: 10,
    };
    let staged = optimize_multi_on_trained(&[&t1, &t2], &spec).expect("staged should succeed");

    // The final models are trained on the same full data and NSGA-II uses the
    // same seed, so the fronts should match.
    assert_eq!(
        one_shot.front.len(),
        staged.front.len(),
        "front sizes should match"
    );
    for (a, b) in one_shot.front.iter().zip(staged.front.iter()) {
        for (va, vb) in a.values.iter().zip(b.values.iter()) {
            assert!(
                (va - vb).abs() < 1e-9,
                "front values should match: {va} vs {vb}"
            );
        }
        for (pa, pb) in a.params.iter().zip(b.params.iter()) {
            assert!(
                (pa - pb).abs() < 1e-9,
                "front params should match: {pa} vs {pb}"
            );
        }
    }
    // r_squared also matches (trained on the same full data).
    for (ra, rb) in one_shot.r_squared.iter().zip(staged.r_squared.iter()) {
        assert!((ra - rb).abs() < 1e-12);
    }
}

// ────────────────────────────────────────────────────────────
// fit_multi_surrogates (Pareto front focus)
// ────────────────────────────────────────────────────────────

#[test]
fn fit_multi_surrogates_runs_end_to_end() {
    // Verifies the fit→optimize pipeline wiring on a 2-objective trade-off problem.
    // The inducing-point path (N>100) is already covered by
    // select_inducing_points_* / fit_inducing_path in gaussian_process, so a
    // small N on the exact path suffices here. Determinism of the multi-objective
    // front is covered by staged_multi_opt_matches_one_shot_result.
    // Train 2 models with GpFitc → predictions are finite → NSGA-II front is non-empty.
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let param_names = vec!["x0".to_string(), "x1".to_string()];
    let objective_names = vec!["f1".to_string(), "f2".to_string()];
    let minimize = vec![true, true];
    let objective_values = vec![f1, f2];

    let trained = fit_multi_surrogates(
        &x_matrix,
        &objective_values,
        &param_names,
        &objective_names,
        SurrogateModelKind::GpFitc,
        &minimize,
    )
    .expect("fit_multi_surrogates should succeed");
    assert_eq!(trained.len(), 2, "should return 2 trained models");
    for t in &trained {
        assert!(t.validation.train_r2.is_finite());
        // Predictions are finite at the training data points.
        let pred: Vec<f64> = t
            .x_matrix
            .iter()
            .map(|row| {
                let norm = t.surrogate.to_norm_x(row);
                t.surrogate.to_original_y(t.surrogate.predict_norm(&norm))
            })
            .collect();
        assert!(pred.iter().all(|v| v.is_finite()));
    }

    // optimize_multi_on_trained yields a non-empty front.
    let refs: Vec<&TrainedSurrogate> = trained.iter().collect();
    let spec = SurrogateMultiOptimizeSpec {
        minimize: minimize.clone(),
        slice_params: None,
        n_grid: 10,
    };
    let result = optimize_multi_on_trained(&refs, &spec).expect("multi optimize should succeed");
    assert!(!result.front.is_empty(), "front should be non-empty");
    assert!(result
        .front
        .iter()
        .all(|p| p.values.iter().all(|v| v.is_finite())));
}

#[test]
fn fit_multi_surrogates_validates_lengths() {
    let (x_matrix, f1, f2) = schaffer_samples(20);
    // objective_names length mismatch.
    // TrainedSurrogate doesn't implement Debug, so unwrap_err can't be used; extract via match.
    let err = match fit_multi_surrogates(
        &x_matrix,
        &[f1.clone(), f2.clone()],
        &["x0".to_string(), "x1".to_string()],
        &["f1".to_string()], // mismatch
        SurrogateModelKind::GpFitc,
        &[true, true],
    ) {
        Ok(_) => panic!("expected length-mismatch error"),
        Err(e) => e,
    };
    assert!(err.contains("equal length"), "unexpected error: {err}");

    // Objective column length doesn't match the x_matrix row count.
    let err2 = match fit_multi_surrogates(
        &x_matrix,
        &[f1, vec![0.0; 5]],
        &["x0".to_string(), "x1".to_string()],
        &["f1".to_string(), "f2".to_string()],
        SurrogateModelKind::GpFitc,
        &[true, true],
    ) {
        Ok(_) => panic!("expected length-mismatch error"),
        Err(e) => e,
    };
    assert!(err2.contains("does not match"), "unexpected error: {err2}");
}

// ────────────────────────────────────────────────────────────
// LightGBM surrogate tests
// ────────────────────────────────────────────────────────────

#[test]
fn lgbm_fit_validate_and_optimize_finds_minimum_region() {
    // L-BFGS doesn't work because LGBM predicts a piecewise-constant function.
    // Uses RandomSearch instead, and verifies it reaches the minimum's vicinity
    // (with a loose tolerance).
    let (x_matrix, y) = quadratic_samples(50);
    let trained = fit_surrogate_with_validation(&SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        model: SurrogateModelKind::Lgbm,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    })
    .expect("LGBM fit & validation should succeed");

    assert!(
        trained.validation.train_r2 > 0.5,
        "LGBM train R² should be reasonable: {}",
        trained.validation.train_r2
    );

    let result = optimize_on_trained(
        &trained,
        &SurrogateOptimizeSpec {
            minimize: true,
            optimizer: OptimizerKind::RandomSearch,
            slice_params: Some((0, 1)),
            n_grid: 10,
        },
    );

    assert!(
        (result.best_params[0] - 0.3).abs() < 0.2,
        "x ≈ 0.3, got {}",
        result.best_params[0]
    );
    assert!(
        (result.best_params[1] - 0.7).abs() < 0.2,
        "y ≈ 0.7, got {}",
        result.best_params[1]
    );
    assert!(result.predicted_std.is_none(), "LGBM has no posterior std");
    assert!(result.slice.is_some(), "スライスが要求されている");
}

#[test]
fn lgbm_multi_opt_returns_front() {
    // Verifies (loosely) that multi-objective surrogate optimization works
    // with LGBM and returns a front.
    let (x_matrix, f1, f2) = schaffer_samples(40);
    let mut req = base_multi_request(x_matrix, f1, f2);
    req.model = SurrogateModelKind::Lgbm;
    let result =
        run_surrogate_multi_optimization(&req).expect("LGBM multi-objective should succeed");

    assert!(
        result.front.len() >= 3,
        "front should have ≥3 points, got {}",
        result.front.len()
    );
    assert_eq!(result.r_squared.len(), 2);
    for p in &result.front {
        assert_eq!(p.params.len(), 2);
        assert_eq!(p.values.len(), 2);
        assert!(p.values.iter().all(|v| v.is_finite()));
    }
}

// ============================================================================
// Constrained surrogate optimization tests
// ============================================================================

/// Helper that builds a constrained SurrogateFitRequest.
fn constrained_fit_req(x_matrix: Vec<Vec<f64>>, y: Vec<f64>, c: Vec<f64>) -> SurrogateFitRequest {
    SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        model: SurrogateModelKind::GpFitc,
        auto_select: false,
        constraints: vec![ConstraintData {
            name: "c1".to_string(),
            values: c,
        }],
        priority_rows: vec![],
        param_bounds: None,
    }
}

/// Constraint surface c(x) = 0.5 − x0. Feasible when c ≤ 0 ⟺ x0 ≥ 0.5.
fn constraint_surface(x: &[f64]) -> f64 {
    0.5 - x[0]
}

/// Builds an analytic mock TrainedSurrogate with objective = quadratic surface
/// and constraint c = 0.5 − x0.
fn analytic_constrained_trained() -> TrainedSurrogate {
    let x_matrix = vec![
        vec![0.0, 0.0],
        vec![1.0, 1.0],
        vec![0.5, 0.5],
        vec![0.2, 0.8],
    ];
    let y: Vec<f64> = x_matrix.iter().map(|r| quad_surface(r)).collect();
    let c_values: Vec<f64> = x_matrix.iter().map(|r| constraint_surface(r)).collect();
    let obj = models::FittedSurrogate::analytic(2, quad_surface, Some(Box::new(|_x: &[f64]| 0.01)));
    let con =
        models::FittedSurrogate::analytic(2, constraint_surface, Some(Box::new(|_x: &[f64]| 0.01)));
    TrainedSurrogate::analytic_mock(x_matrix, y, obj).with_analytic_constraint("c1", c_values, con)
}

#[test]
fn constrained_opt_pushes_x_toward_feasible_region() {
    // c = 0.5 − x0 (feasible for x0 ≥ 0.5). With the known surface, the
    // constraint-penalized minimum converges exactly to x0 = 0.5 (the boundary),
    // x1 = 0.7.
    let trained = analytic_constrained_trained();
    let result = optimize_on_trained(
        &trained,
        &SurrogateOptimizeSpec {
            minimize: true,
            optimizer: OptimizerKind::MultiStartLbfgs,
            slice_params: None,
            n_grid: 10,
        },
    );

    assert!(
        (result.best_params[0] - 0.5).abs() < 0.02,
        "制約境界 x0 = 0.5 へ収束するはず, got x = {}",
        result.best_params[0]
    );
    assert!(
        (result.best_params[1] - 0.7).abs() < 0.02,
        "x1 = 0.7 へ収束するはず, got y = {}",
        result.best_params[1]
    );
    assert_eq!(result.predicted_constraints.len(), 1);
    // At the boundary, c(x) = 0.5 − 0.5 = 0 matches exactly.
    assert!(
        result.predicted_constraints[0].abs() < 0.02,
        "predicted constraint ≈ 0 at boundary, got {}",
        result.predicted_constraints[0]
    );
    // P(c ≤ 0) = Φ((0 − mu)/σ) = Φ(0) = 0.5 (at the boundary, σ=0.1).
    let p_feas = result
        .feasibility_probability
        .expect("制約ありのとき feasibility_probability は Some");
    assert!(
        (p_feas - 0.5).abs() < 0.1,
        "境界では P_feas ≈ 0.5 を期待, got {}",
        p_feas
    );
}

#[test]
fn unconstrained_opt_finds_true_minimum_near_0_3() {
    // No constraint: exactly finds the true minimum (0.3, 0.7) of the known surface.
    let trained = analytic_trained(quad_surface, true);
    let result = optimize_on_trained(
        &trained,
        &SurrogateOptimizeSpec {
            minimize: true,
            optimizer: OptimizerKind::MultiStartLbfgs,
            slice_params: None,
            n_grid: 10,
        },
    );

    assert!(
        (result.best_params[0] - 0.3).abs() < 0.01,
        "x ≈ 0.3 (unconstrained), got {}",
        result.best_params[0]
    );
    assert!(
        result.feasibility_probability.is_none(),
        "制約なしのとき feasibility_probability は None"
    );
    assert!(
        result.predicted_constraints.is_empty(),
        "制約なしのとき predicted_constraints は空"
    );
}

#[test]
fn constrained_fit_validation_succeeds() {
    // fit_surrogate_with_validation succeeds with constraints, and constraint_names is set.
    let (x_matrix, y, c) = constrained_quadratic_samples(40);
    let req = constrained_fit_req(x_matrix, y, c);
    let trained = fit_surrogate_with_validation(&req).expect("constrained fit should succeed");

    assert_eq!(trained.constraint_names, vec!["c1".to_string()]);
    assert_eq!(trained.constraint_models.len(), 1);
    assert_eq!(trained.constraint_values.len(), 40);
    assert!(trained.constraint_values.iter().all(|row| row.len() == 1));
}

// NOTE: That optimize_on_trained returns predicted_constraints /
// feasibility_probability when constrained is strictly verified at the value
// level by the analytic mock version constrained_opt_pushes_x_toward_feasible_region.

#[test]
fn suggest_candidates_constrained_p_feas_present() {
    // Constrained suggest_candidates: all candidates have Some feasibility_probability.
    // mean P_feas > 0.3.
    let (x_matrix, y, c) = constrained_quadratic_samples(40);
    let req = constrained_fit_req(x_matrix, y, c);
    let trained = fit_surrogate_with_validation(&req).expect("fit should succeed");

    let candidates = suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
        .expect("constrained suggest should succeed");

    assert_eq!(candidates.len(), 3);
    for c in &candidates {
        assert!(
            c.feasibility_probability.is_some(),
            "constrained candidate must have feasibility_probability"
        );
        assert_eq!(c.predicted_constraints.len(), 1);
        let p = c.feasibility_probability.unwrap();
        assert!((0.0..=1.0).contains(&p), "P_feas must be [0,1], got {p}");
    }

    let mean_p: f64 = candidates
        .iter()
        .map(|c| c.feasibility_probability.unwrap())
        .sum::<f64>()
        / 3.0;
    assert!(
        mean_p > 0.3,
        "制約付きサジェストで mean P_feas > 0.3 を期待, got {mean_p}"
    );

    // Determinism: calling twice with the same trained surrogate gives the same result.
    let candidates2 = suggest_candidates(&trained, 3, AcquisitionKind::ExpectedImprovement, true)
        .expect("second run");
    for (a, b) in candidates.iter().zip(candidates2.iter()) {
        for (pa, pb) in a.params.iter().zip(b.params.iter()) {
            assert!(
                (pa - pb).abs() < 1e-9,
                "constrained suggest must be deterministic: {pa} vs {pb}"
            );
        }
    }
}

#[test]
fn suggest_candidates_unconstrained_p_feas_none() {
    // Unconstrained suggest_candidates (n=1, no refit): verifies that
    // feasibility_probability is None and predicted_constraints is empty, using
    // the known GP-family mock.
    let trained = analytic_trained(quad_surface, true);
    let candidates = suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, true)
        .expect("unconstrained suggest should succeed");

    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].feasibility_probability.is_none());
    assert!(candidates[0].predicted_constraints.is_empty());
    // The proposed point lies within the original [0,1] unit range, and EI is non-negative.
    assert!(candidates[0]
        .params
        .iter()
        .all(|&v| (0.0..=1.0).contains(&v)));
    assert!(candidates[0].acq_score >= 0.0);
    assert!(
        candidates[0].predicted_std.is_some(),
        "GP-like mock has std"
    );
}

// ────────────────────────────────────────────────────────────
// Automatic model selection (Auto)
// ────────────────────────────────────────────────────────────

/// Samples from a clearly nonlinear, smooth function y = sin(3·x0) + x1².
/// GP is expected to beat Ridge on CV R².
fn nonlinear_smooth_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    let mut rng = SeededRng::from_seed(11);
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|r| (3.0 * r[0]).sin() + r[1].powi(2))
        .collect();
    (x_matrix, y)
}

/// Samples from a clearly linear function y = 2·x0 − x1 + small noise.
/// Ridge is expected to win, or to be chosen by tie-break in case of a tie.
fn linear_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    let mut rng = SeededRng::from_seed(17);
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        // Adds small noise ([-0.005, 0.005]).
        .map(|r| 2.0 * r[0] - r[1] + (rng.next_f64() - 0.5) * 0.01)
        .collect();
    (x_matrix, y)
}

/// Helper that extracts a given model's score from `scores`.
fn score_of(report: &ModelSelectionReport, kind: SurrogateModelKind) -> f64 {
    report
        .scores
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, s)| *s)
        .expect("candidate must be present in scores")
}

#[test]
fn select_best_model_picks_gp_on_nonlinear_smooth() {
    let (x_matrix, y) = nonlinear_smooth_samples(50);
    let report = select_best_model(&x_matrix, &y, 42).expect("selection should succeed");

    // The candidates are the 4 in AUTO_CANDIDATES.
    assert_eq!(report.scores.len(), 4);

    // For a nonlinear, smooth function, GP is chosen (not Ridge).
    assert!(
        matches!(
            report.chosen,
            SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe
        ),
        "expected a GP, got {:?}",
        report.chosen
    );

    // GP's CV R² exceeds Ridge's.
    let ridge = score_of(&report, SurrogateModelKind::Ridge);
    let gp = score_of(&report, SurrogateModelKind::GpFitc)
        .max(score_of(&report, SurrogateModelKind::GpVfe));
    assert!(gp > ridge, "GP cv_r2 {gp} should exceed Ridge {ridge}");
}

#[test]
fn select_best_model_picks_ridge_on_linear() {
    // The Ridge-selection (tie-break) judgment on linear data is highly
    // sensitive to quality: lowering N causes GP's CV R² to exceed Ridge's by
    // more than 1e-3, breaking the tie-break condition, so we keep N at 80.
    let (x_matrix, y) = linear_samples(80);
    let report = select_best_model(&x_matrix, &y, 42).expect("selection should succeed");

    assert_eq!(report.scores.len(), 4);

    // Ridge is chosen for a linear function. On a tie, Ridge (first in AUTO_CANDIDATES) wins.
    let ridge = score_of(&report, SurrogateModelKind::Ridge);
    let best = report
        .scores
        .iter()
        .map(|(_, s)| *s)
        .fold(f64::NEG_INFINITY, f64::max);
    // Ridge is within 1e-3 of the best score (the condition for Ridge to win the tie-break).
    assert!(
        best - ridge < 1e-3,
        "Ridge cv_r2 {ridge} should be within 1e-3 of best {best}"
    );
    assert_eq!(
        report.chosen,
        SurrogateModelKind::Ridge,
        "linear data should choose Ridge (tie-break to simpler), got {:?}",
        report.chosen
    );
}

#[test]
fn fit_surrogate_with_validation_auto_select_end_to_end() {
    let (x_matrix, y) = nonlinear_smooth_samples(50);
    let req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x0".to_string(), "x1".to_string()],
        objective_name: "obj0".to_string(),
        // Auto: the model field is ignored (placeholder).
        model: SurrogateModelKind::Ridge,
        auto_select: true,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    };
    let trained = fit_surrogate_with_validation(&req).expect("auto fit should succeed");

    // The selection history is attached and holds scores for 4 candidates.
    let selection = trained
        .model_selection
        .as_ref()
        .expect("auto fit must attach a model_selection report");
    assert_eq!(selection.scores.len(), 4);

    // model_kind is the concretely selected model kind (not the Ridge placeholder).
    assert_eq!(trained.model_kind, selection.chosen);
    assert!(
        matches!(
            trained.model_kind,
            SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe
        ),
        "expected a concrete GP kind, got {:?}",
        trained.model_kind
    );
}

#[test]
fn select_best_model_is_deterministic() {
    // This only verifies reproducibility, not which model wins, so a small N that
    // doesn't depend on quality is fine.
    // Running 4 candidates × CV twice is expensive, so keeping N small has a big payoff.
    let (x_matrix, y) = nonlinear_smooth_samples(30);
    let r1 = select_best_model(&x_matrix, &y, 42).expect("run 1");
    let r2 = select_best_model(&x_matrix, &y, 42).expect("run 2");

    // The chosen model matches.
    assert_eq!(r1.chosen, r2.chosen);
    // The scores match exactly (both order and values).
    assert_eq!(r1.scores.len(), r2.scores.len());
    for ((k1, s1), (k2, s2)) in r1.scores.iter().zip(r2.scores.iter()) {
        assert_eq!(k1, k2);
        assert_eq!(s1.to_bits(), s2.to_bits(), "scores must be bit-identical");
    }
}

// ────────────────────────────────────────────────────────────
// Subsampling large datasets (subsample_indices)
// ────────────────────────────────────────────────────────────

#[test]
fn subsample_returns_none_when_within_cap() {
    let y: Vec<f64> = (0..MAX_TRAIN_FOR_FIT).map(|i| i as f64).collect();
    assert!(subsample_indices(&[&y], &[], MAX_TRAIN_FOR_FIT, 42).is_none());
}

#[test]
fn subsample_single_keeps_both_extremes_and_caps_size() {
    let n = MAX_TRAIN_FOR_FIT * 3;
    let y: Vec<f64> = (0..n).map(|i| i as f64).collect(); // min=0, max=n-1
    let idx = subsample_indices(&[&y], &[], MAX_TRAIN_FOR_FIT, 42).expect("should subsample");

    assert_eq!(idx.len(), MAX_TRAIN_FOR_FIT, "間引き後は cap 点ちょうど");
    // Indices are ascending and unique.
    assert!(idx.windows(2).all(|w| w[0] < w[1]), "昇順かつ重複なし");
    // Within range.
    assert!(idx.iter().all(|&i| i < n));
    // Both the best (minimum = index 0) and worst (maximum = index n-1) remain.
    assert!(idx.contains(&0), "best 点が保持されること");
    assert!(idx.contains(&(n - 1)), "worst 点が保持されること");
}

#[test]
fn subsample_is_deterministic() {
    let n = MAX_TRAIN_FOR_FIT * 2;
    let y: Vec<f64> = (0..n).map(|i| ((i * 7) % 13) as f64).collect();
    let a = subsample_indices(&[&y], &[], MAX_TRAIN_FOR_FIT, 42).expect("a");
    let b = subsample_indices(&[&y], &[], MAX_TRAIN_FOR_FIT, 42).expect("b");
    assert_eq!(a, b, "同一シードで同一の部分集合");
}

#[test]
fn subsample_multi_keeps_pareto_front() {
    // 2 objectives, both minimized. Plants clear non-dominated points (rank 0) and
    // verifies they are retained.
    let n = MAX_TRAIN_FOR_FIT * 2;
    // obj0 = i, obj1 = n - i is a monotonic trade-off → every point is rank 0 (a strong Pareto set).
    // Here we only verify that a "representative of the rank-0 set" is definitely retained.
    let mut rng = SeededRng::from_seed(3);
    let obj0: Vec<f64> = (0..n).map(|_| rng.next_f64()).collect();
    let obj1: Vec<f64> = (0..n).map(|_| rng.next_f64()).collect();
    let minimize = [true, true];
    let idx = subsample_indices(&[&obj0, &obj1], &minimize, MAX_TRAIN_FOR_FIT, 42)
        .expect("should subsample");
    assert_eq!(idx.len(), MAX_TRAIN_FOR_FIT);
    assert!(idx.windows(2).all(|w| w[0] < w[1]));

    // At least one of the true non-dominated points (rank 0) is retained.
    let rows: Vec<Vec<f64>> = (0..n).map(|i| vec![obj0[i], obj1[i]]).collect();
    let ranks = crate::multi_objective::pareto::nd_sort(&rows, &minimize);
    let front: Vec<usize> = (0..n).filter(|&i| ranks[i] == 0).collect();
    assert!(
        front.iter().any(|f| idx.contains(f)),
        "パレートフロント上の点が保持されること"
    );
}

// ────────────────────────────────────────────────────────────
// Progress reporting and cancellation (FitProgress / *_tracked)
// ────────────────────────────────────────────────────────────

#[test]
fn fit_tracked_reports_progress_to_completion() {
    let (x_matrix, y) = quadratic_samples(60);
    let req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "f".to_string(),
        model: SurrogateModelKind::Ridge,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    };
    let progress = FitProgress::new();
    let _ = fit_surrogate_with_validation_tracked(&req, &progress).expect("fit should succeed");
    let s = progress.snapshot();
    // Single objective, unconstrained, manual: total = (holdout 1 + CV 5) + final 1 = 7.
    assert_eq!(s.total, 7, "total fit count");
    assert_eq!(s.done, s.total, "progress should reach 100%");
    assert!(!s.stage.is_empty(), "stage label should be set");
}

#[test]
fn fit_tracked_cancel_before_start_returns_err() {
    let (x_matrix, y) = quadratic_samples(60);
    let req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "f".to_string(),
        model: SurrogateModelKind::Ridge,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    };
    let progress = FitProgress::new();
    progress.request_cancel();
    let res = fit_surrogate_with_validation_tracked(&req, &progress);
    assert!(res.is_err(), "cancelled fit should return Err");
    assert!(progress.is_cancelled());
    // Cancellation is detected before the first fit, so progress doesn't advance.
    assert_eq!(progress.snapshot().done, 0);
}

#[test]
fn fit_multi_tracked_cancel_returns_err() {
    let (x_matrix, y) = quadratic_samples(40);
    // 2 objectives (reusing the same data; since cancellation returns before fitting, the values don't matter).
    let objective_values = vec![y.clone(), y];
    let names = vec!["x".to_string(), "y".to_string()];
    let obj_names = vec!["f0".to_string(), "f1".to_string()];
    let progress = FitProgress::new();
    progress.request_cancel();
    let res = fit_multi_surrogates_tracked(
        &x_matrix,
        &objective_values,
        &names,
        &obj_names,
        SurrogateModelKind::Ridge,
        &[true, true],
        None,
        &progress,
    );
    assert!(res.is_err(), "cancelled multi fit should return Err");
    assert!(progress.is_cancelled());
}

#[test]
fn fit_with_validation_subsamples_large_data() {
    // Fitting succeeds even when N > cap, and the training data is subsampled down to at most cap.
    let (x_matrix, y) = quadratic_samples(MAX_TRAIN_FOR_FIT + 500);
    let req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "f".to_string(),
        model: SurrogateModelKind::Ridge, // verify the path with a fast model
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    };
    let trained = fit_surrogate_with_validation(&req).expect("fit should succeed");
    assert!(
        trained.x_matrix.len() <= MAX_TRAIN_FOR_FIT,
        "学習データが cap 以下に間引かれること: {}",
        trained.x_matrix.len()
    );
    assert_eq!(trained.x_matrix.len(), trained.y.len());
}
