use super::*;
use crate::math::rng::SeededRng;

/// 既知関数 f(x, y) = (x − 0.3)² + (y − 0.7)² を [0,1]² 内のサンプル点で評価した
/// テストデータを作る（最小値 0 at (0.3, 0.7)）。
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

fn base_request(x_matrix: Vec<Vec<f64>>, y: Vec<f64>) -> SurrogateOptRequest {
    SurrogateOptRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        minimize: true,
        model: SurrogateModelKind::Kriging,
        optimizer: OptimizerKind::MultiStartLbfgs,
        slice_params: Some((0, 1)),
        n_grid: 10,
    }
}

#[test]
fn kriging_lbfgs_finds_quadratic_minimum() {
    let (x_matrix, y) = quadratic_samples(50);
    let req = base_request(x_matrix.clone(), y.clone());
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

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
        "predicted minimum near 0, got {}",
        result.best_value
    );
    assert!(
        result.r_squared > 0.8,
        "GP should fit well: {}",
        result.r_squared
    );
    assert!(result.predicted_std.is_some(), "Kriging has posterior std");

    // 最小化時 best_observed_value == y.iter().cloned().fold(f64::INFINITY, f64::min)
    let expected_best_obs = y.iter().cloned().fold(f64::INFINITY, f64::min);
    assert_eq!(
        result.best_observed_value.to_bits(),
        expected_best_obs.to_bits(),
        "best_observed_value は観測最小値と等しい"
    );
}

#[test]
fn random_search_finds_quadratic_minimum_loosely() {
    let (x_matrix, y) = quadratic_samples(50);
    let mut req = base_request(x_matrix, y);
    req.optimizer = OptimizerKind::RandomSearch;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

    assert!((result.best_params[0] - 0.3).abs() < 0.15);
    assert!((result.best_params[1] - 0.7).abs() < 0.15);
}

#[test]
fn nsga2_finds_quadratic_minimum() {
    let (x_matrix, y) = quadratic_samples(50);
    let mut req = base_request(x_matrix, y);
    req.optimizer = OptimizerKind::Nsga2;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

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
}

#[test]
fn cma_es_finds_quadratic_minimum() {
    let (x_matrix, y) = quadratic_samples(50);
    let mut req = base_request(x_matrix, y);
    req.optimizer = OptimizerKind::CmaEs;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

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
}

#[test]
fn cma_es_maximize_finds_quadratic_maximum_corner() {
    // f は (0.3, 0.7) から最遠の角 (1, 0) で最大になる。
    let (x_matrix, y) = quadratic_samples(60);
    let mut req = base_request(x_matrix, y);
    req.minimize = false;
    req.optimizer = OptimizerKind::CmaEs;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

    assert!(
        result.best_params[0] > 0.7,
        "x should approach 1, got {}",
        result.best_params[0]
    );
    assert!(
        result.best_params[1] < 0.4,
        "y should approach 0, got {}",
        result.best_params[1]
    );
}

#[test]
fn maximize_direction_finds_quadratic_maximum_corner() {
    // f は (0.3, 0.7) から遠い角で最大になる。x=1,y=0 の角が最遠。
    let (x_matrix, y) = quadratic_samples(60);
    let mut req = base_request(x_matrix, y);
    req.minimize = false;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

    assert!(
        result.best_params[0] > 0.7,
        "x should approach 1, got {}",
        result.best_params[0]
    );
    assert!(
        result.best_params[1] < 0.4,
        "y should approach 0, got {}",
        result.best_params[1]
    );
}

#[test]
fn ridge_model_reaches_box_corner() {
    // 線形データ y = 2x − y2 は箱の角で最小。
    let mut rng = SeededRng::from_seed(11);
    let x_matrix: Vec<Vec<f64>> = (0..40)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix.iter().map(|r| 2.0 * r[0] - r[1]).collect();
    let mut req = base_request(x_matrix, y);
    req.model = SurrogateModelKind::Ridge;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

    assert!(
        result.best_params[0] < 0.05,
        "x → 0: {}",
        result.best_params[0]
    );
    assert!(
        result.best_params[1] > 0.95,
        "y → 1: {}",
        result.best_params[1]
    );
    assert!(result.predicted_std.is_none(), "Ridge has no posterior std");
    assert!(result.r_squared > 0.95);
}

#[test]
fn sparse_kriging_runs_and_finds_minimum_region() {
    let (x_matrix, y) = quadratic_samples(80);
    let mut req = base_request(x_matrix, y);
    req.model = SurrogateModelKind::SparseKriging;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");

    assert!((result.best_params[0] - 0.3).abs() < 0.2);
    assert!((result.best_params[1] - 0.7).abs() < 0.2);
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

#[test]
fn slice_grid_passes_through_optimum_and_has_expected_shape() {
    let (x_matrix, y) = quadratic_samples(50);
    let mut req = base_request(x_matrix, y);
    req.n_grid = 12;
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");
    let slice = result.slice.expect("slice requested");

    assert_eq!(slice.x_values.len(), 12);
    assert_eq!(slice.y_values.len(), 12);
    assert_eq!(slice.z_values.len(), 12);
    assert!(slice.z_values.iter().all(|row| row.len() == 12));
    // 格子の最小値はサロゲート最適値の近くにあるはず。
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
    let (x_matrix, y) = quadratic_samples(30);
    let mut req = base_request(x_matrix, y);
    req.slice_params = Some((0, 0)); // 同一軸は無効
    let result = run_surrogate_optimization(&req).expect("optimization should succeed");
    assert!(result.slice.is_none());
}

// ============================================================================
// TrainedSurrogate の Send + Sync アサーション
// ============================================================================

/// TrainedSurrogate が Send + Sync を満たすことをコンパイル時に検証する。
#[test]
fn trained_surrogate_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TrainedSurrogate>();
}

// ============================================================================
// validate_surrogate のテスト
// ============================================================================

#[test]
fn validate_surrogate_kriging_high_r2_on_smooth_function() {
    // 決定論的な滑らかな関数で学習・検証し、CV R² とホールドアウト R² が高いことを確認する。
    let (x_matrix, y) = quadratic_samples(50);
    let report = validation::validate_surrogate(SurrogateModelKind::Kriging, &x_matrix, &y, 42)
        .expect("validate_surrogate should succeed");

    assert_eq!(report.n_samples, 50);
    assert!(report.n_test >= 1, "n_test >= 1");
    assert_eq!(report.n_train + report.n_test, 50);
    assert!(
        report.holdout_r2 > 0.7,
        "holdout_r2 = {}",
        report.holdout_r2
    );
    assert!(
        report.cv_r2_mean > 0.7,
        "cv_r2_mean = {}",
        report.cv_r2_mean
    );
    assert_eq!(
        report.oof_pairs.len(),
        50,
        "oof_pairs の長さはサンプル数と一致する"
    );
    assert!(report.holdout_r2.is_finite());
    assert!(report.holdout_rmse.is_finite());
    assert!(report.cv_r2_mean.is_finite());
    assert!(report.cv_r2_std.is_finite());
    assert!(report.cv_rmse_mean.is_finite());
    assert!(report.cv_rmse_std.is_finite());
}

#[test]
fn validate_surrogate_deterministic_with_same_seed() {
    // 同一シードで呼び出した結果が完全に一致することを確認する。
    let (x_matrix, y) = quadratic_samples(30);
    let r1 = validation::validate_surrogate(SurrogateModelKind::Kriging, &x_matrix, &y, 42)
        .expect("first call should succeed");
    let r2 = validation::validate_surrogate(SurrogateModelKind::Kriging, &x_matrix, &y, 42)
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
    // n = 10 の最小データセットで検証が成功し、期待されるフィールドを持つことを確認する。
    let (x_matrix, y) = quadratic_samples(10);
    let report = validation::validate_surrogate(SurrogateModelKind::Kriging, &x_matrix, &y, 42)
        .expect("minimum-size validate_surrogate should succeed");

    assert_eq!(report.n_samples, 10);
    assert!(report.n_test >= 1, "n_test >= 1");
    assert_eq!(report.cv_folds, 5, "k = min(5, 10) = 5");
    assert_eq!(report.oof_pairs.len(), 10);
    assert!(report.holdout_r2.is_finite());
    assert!(report.cv_rmse_mean.is_finite());
}

// ============================================================================
// fit_surrogate_with_validation + optimize_on_trained のエンドツーエンドテスト
// ============================================================================

#[test]
fn fit_and_optimize_on_trained_finds_quadratic_minimum() {
    let (x_matrix, y) = quadratic_samples(50);

    let fit_req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        model: SurrogateModelKind::Kriging,
    };
    let trained = fit_surrogate_with_validation(&fit_req)
        .expect("fit_surrogate_with_validation should succeed");

    // 検証レポートの基本チェック。
    assert_eq!(trained.validation.n_samples, 50);
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
