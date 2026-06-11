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
// ────────────────────────────────────────────────────────────
// 多目的サロゲート最適化のテスト
// ────────────────────────────────────────────────────────────

/// Schaffer N.1 相当のトレードオフデータを生成する。
/// f1 = x0², f2 = (x0 − 1)²、x0 ∈ [0,1]、x1 はダミー次元。
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
        model: SurrogateModelKind::Kriging,
        slice_params: Some((0, 1)),
        n_grid: 10,
    }
}

#[test]
fn multi_opt_front_spans_full_tradeoff() {
    // 2 目的トレードオフ問題: フロントが 5 点以上、全域に広がること。
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let req = base_multi_request(x_matrix, f1, f2);
    let result = run_surrogate_multi_optimization(&req)
        .expect("multi-objective optimization should succeed");

    assert!(
        result.front.len() >= 5,
        "front should have ≥5 points, got {}",
        result.front.len()
    );

    // f1 側端: x0 < 0.2 の点（f1 が小さく f2 が大きい側）が存在する。
    let has_f1_side = result
        .front
        .iter()
        .any(|p| p.params[0] < 0.2 || p.values[0] < 0.05);
    assert!(has_f1_side, "front should reach f1-dominated region");

    // f2 側端: x0 > 0.8 の点（f2 が小さく f1 が大きい側）が存在する。
    let has_f2_side = result
        .front
        .iter()
        .any(|p| p.params[0] > 0.8 || p.values[1] < 0.05);
    assert!(has_f2_side, "front should reach f2-dominated region");
}

#[test]
fn multi_opt_front_sorted_by_first_objective() {
    // フロントが第 1 目的で昇順ソートされていること。
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let req = base_multi_request(x_matrix, f1, f2);
    let result = run_surrogate_multi_optimization(&req).expect("should succeed");

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
    // 各 ParetoFrontPoint の params/values 長が param_names/objective_names と一致。
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let req = base_multi_request(x_matrix, f1, f2);
    let result = run_surrogate_multi_optimization(&req).expect("should succeed");

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
    }
    assert_eq!(result.r_squared.len(), 2, "r_squared should have 2 entries");
}

#[test]
fn multi_opt_maximize_objective_direction() {
    // f2 を最大化（minimize=false）する場合、結果の f2 値が正の方向で分布すること。
    // f2 = (x0 − 1)² は x0=0 で最大値 1、x0=1 で最小値 0。
    // maximize 側のフロント端に f2 が大きい点が存在すること。
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let mut req = base_multi_request(x_matrix, f1, f2);
    req.minimize = vec![true, false]; // f2 を最大化

    let result = run_surrogate_multi_optimization(&req).expect("should succeed with maximize");

    // 最大化目的 f2 の最大値が 0.5 以上の点がフロントに存在すること。
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
    // slice_params 指定時に目的数ぶんのスライスが返ること。
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let req = base_multi_request(x_matrix, f1, f2);
    let result = run_surrogate_multi_optimization(&req).expect("should succeed");

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
    // slice_params = None のとき slices が空。
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let mut req = base_multi_request(x_matrix, f1, f2);
    req.slice_params = None;
    let result = run_surrogate_multi_optimization(&req).expect("should succeed");
    assert!(
        result.slices.is_empty(),
        "slices should be empty when slice_params is None"
    );
}

#[test]
fn multi_opt_error_on_single_objective() {
    // 目的数 1 はエラー。
    let (x_matrix, f1, _) = schaffer_samples(30);
    let req = SurrogateMultiOptRequest {
        x_matrix,
        ys: vec![f1],
        param_names: vec!["x0".to_string(), "x1".to_string()],
        objective_names: vec!["f1".to_string()],
        minimize: vec![true],
        model: SurrogateModelKind::Kriging,
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
    // ys と objective_names の長さが異なる場合はエラー。
    let (x_matrix, f1, f2) = schaffer_samples(30);
    let req = SurrogateMultiOptRequest {
        x_matrix,
        ys: vec![f1, f2],
        param_names: vec!["x0".to_string(), "x1".to_string()],
        objective_names: vec!["f1".to_string()], // 長さ不一致
        minimize: vec![true, true],
        model: SurrogateModelKind::Kriging,
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
    // trial 不足はエラー。
    let (x_matrix, f1, f2) = schaffer_samples(5);
    let req = base_multi_request(x_matrix, f1, f2);
    let err = run_surrogate_multi_optimization(&req).unwrap_err();
    assert!(err.contains("At least"), "unexpected error: {err}");
}

// ────────────────────────────────────────────────────────────
// 2 段階フロー（fit → optimize_multi_on_trained）のテスト
// ────────────────────────────────────────────────────────────

/// schaffer_samples の 2 目的を fit_surrogate_with_validation で学習する。
fn fit_schaffer_trained(n: usize) -> (TrainedSurrogate, TrainedSurrogate) {
    let (x_matrix, f1, f2) = schaffer_samples(n);
    let names = vec!["x0".to_string(), "x1".to_string()];
    let t1 = fit_surrogate_with_validation(&SurrogateFitRequest {
        x_matrix: x_matrix.clone(),
        y: f1,
        param_names: names.clone(),
        objective_name: "f1".to_string(),
        model: SurrogateModelKind::Kriging,
    })
    .expect("fit f1 should succeed");
    let t2 = fit_surrogate_with_validation(&SurrogateFitRequest {
        x_matrix,
        y: f2,
        param_names: names,
        objective_name: "f2".to_string(),
        model: SurrogateModelKind::Kriging,
    })
    .expect("fit f2 should succeed");
    (t1, t2)
}

#[test]
fn staged_multi_opt_front_spans_full_tradeoff() {
    // fit & validate → optimize の 2 段階でフロントがトレードオフ全域に広がること。
    let (t1, t2) = fit_schaffer_trained(50);
    let spec = SurrogateMultiOptimizeSpec {
        minimize: vec![true, true],
        slice_params: Some((0, 1)),
        n_grid: 10,
    };
    let result = optimize_multi_on_trained(&[&t1, &t2], &spec)
        .expect("staged multi-objective optimization should succeed");

    assert!(
        result.front.len() >= 5,
        "front should have ≥5 points, got {}",
        result.front.len()
    );
    let has_f1_side = result
        .front
        .iter()
        .any(|p| p.params[0] < 0.2 || p.values[0] < 0.05);
    assert!(has_f1_side, "front should reach f1-dominated region");
    let has_f2_side = result
        .front
        .iter()
        .any(|p| p.params[0] > 0.8 || p.values[1] < 0.05);
    assert!(has_f2_side, "front should reach f2-dominated region");

    // スライスは目的数ぶん返る。r_squared も目的数ぶん。
    assert_eq!(result.slices.len(), 2);
    assert_eq!(result.r_squared.len(), 2);
}

#[test]
fn staged_multi_opt_error_on_single_trained() {
    // trained 1 件のみはエラー。
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
    // minimize の長さが trained と異なる場合はエラー。
    let (t1, t2) = fit_schaffer_trained(30);
    let spec = SurrogateMultiOptimizeSpec {
        minimize: vec![true], // 長さ不一致
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
    // 同一データ・決定的シードなら one-shot 版と staged 版で同等のフロントが得られること。
    let (x_matrix, f1, f2) = schaffer_samples(50);
    let req = base_multi_request(x_matrix, f1.clone(), f2.clone());
    let one_shot = run_surrogate_multi_optimization(&req).expect("one-shot should succeed");

    let (t1, t2) = fit_schaffer_trained(50);
    let spec = SurrogateMultiOptimizeSpec {
        minimize: vec![true, true],
        slice_params: Some((0, 1)),
        n_grid: 10,
    };
    let staged = optimize_multi_on_trained(&[&t1, &t2], &spec).expect("staged should succeed");

    // 最終モデルは同じ全データ学習・NSGA-II は同一シードなので、フロントは一致するはず。
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
    // r_squared も一致する（同じ全データ学習）。
    for (ra, rb) in one_shot.r_squared.iter().zip(staged.r_squared.iter()) {
        assert!((ra - rb).abs() < 1e-12);
    }
}

// ────────────────────────────────────────────────────────────
// LightGBM サロゲートのテスト
// ────────────────────────────────────────────────────────────

#[test]
fn lgbm_fit_validate_and_optimize_finds_minimum_region() {
    // LGBM は区分定数の予測のため L-BFGS が機能しない。RandomSearch を使い、
    // 最小値近傍（緩い許容）に到達することを確認する。
    let (x_matrix, y) = quadratic_samples(50);
    let trained = fit_surrogate_with_validation(&SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "obj0".to_string(),
        model: SurrogateModelKind::Lgbm,
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
    // LGBM で多目的サロゲート最適化が動き、フロントが返ること（緩い検証）。
    let (x_matrix, f1, f2) = schaffer_samples(50);
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
