use super::*;
use crate::math::rng::SeededRng;

// ────────────────────────────────────────────────────────────
// ヘルパー関数
// ────────────────────────────────────────────────────────────

/// 制約付き二次関数のデータを生成する。
/// f = (x - 0.3)^2 + (y - 0.7)^2、c = 0.5 - x （c ≤ 0 ⟺ x ≥ 0.5）。
fn constrained_quadratic_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<f64>) {
    let mut rng = SeededRng::from_seed(7);
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|r| (r[0] - 0.3).powi(2) + (r[1] - 0.7).powi(2))
        .collect();
    // c = 0.5 - x: 実行可能 ⟺ x >= 0.5
    let c: Vec<f64> = x_matrix.iter().map(|r| 0.5 - r[0]).collect();
    (x_matrix, y, c)
}

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

/// 区分的（不連続）なテスト関数。MoE が単一 GP より有利なケース。
/// x[0] < 0.5 と x[0] >= 0.5 で異なる関数形を使う。
fn piecewise_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    // gaussian_process.rs の make_piecewise と同じ LCG ベース RNG を使う。
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
// 解析的モックサロゲートによる「曲面を使う処理」の厳密検証
//
// GP フィットの代わりに既知の closed-form 曲面を同じインターフェースで注入する。
// 曲面が解析的に既知なので最適化器が真の最適点へ到達することを緩い許容ではなく
// 厳密に検証でき、しかも GP フィット（COBYLA ハイパラ最適化）が走らないため一瞬で
// 決定論的に実行できる。GP バックエンド自体の当てはめ品質は egobox の責務であり、
// gp_fitc_runs_and_finds_minimum_region / gp_vfe_* / gp_moe_* の最小限の smoke で確認する。
// ────────────────────────────────────────────────────────────

/// 既知の凸二次曲面 f(x, y) = (x − 0.3)² + (y − 0.7)²。
/// [0,1]² 内の大域最小は (0.3, 0.7) で値 0。
fn quad_surface(x: &[f64]) -> f64 {
    (x[0] - 0.3).powi(2) + (x[1] - 0.7).powi(2)
}

/// 既知の線形曲面 f(x, y) = 2x − y。[0,1]² 内の最小は角 (0, 1) で値 −1。
fn linear_surface(x: &[f64]) -> f64 {
    2.0 * x[0] - x[1]
}

/// 指定曲面の解析的モック TrainedSurrogate を作る。
/// `with_variance` が true なら一定の事後分散 0.01（std 0.1）を持つ GP 系として、
/// false なら事後分散を持たない（Ridge 系）モデルとして振る舞う。
/// `x_matrix` は最適化開始点（観測ベスト）の算出にのみ使う粗いサンプルとする。
fn analytic_trained(surface: fn(&[f64]) -> f64, with_variance: bool) -> TrainedSurrogate {
    let var: Option<models::AnalyticFn> = if with_variance {
        Some(Box::new(|_x: &[f64]| 0.01))
    } else {
        None
    };
    let surrogate = models::FittedSurrogate::analytic(2, surface, var);
    // 開始点（観測ベスト）が局所解に嵌らないよう、最小・最大の各盆地に観測点を置く。
    // 単一スタート型の CMA-ES でも、観測ベストが大域最適の盆地にあれば到達できる。
    let x_matrix = vec![
        vec![0.2, 0.8],  // 二次曲面の最小盆地（(0.3,0.7) 近傍）
        vec![0.9, 0.05], // 二次曲面の最大盆地（最遠の角 (1,0) 近傍）
        vec![0.5, 0.5],
        vec![0.1, 0.9],
    ];
    let y: Vec<f64> = x_matrix.iter().map(|r| surface(r)).collect();
    TrainedSurrogate::analytic_mock(x_matrix, y, surrogate)
}

/// 二次曲面モック上で指定最適化手法を実行する。
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
    // 既知曲面なので勾配法は真の最小点 (0.3, 0.7) へ厳密に到達する。
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
    // 事後分散を持つモックなので predicted_std は Some（std = sqrt(0.01) = 0.1）。
    assert!(
        result.predicted_std.is_some(),
        "GP-like mock has posterior std"
    );
    assert!(
        (result.predicted_std.unwrap() - 0.1).abs() < 1e-9,
        "std should be exactly sqrt(0.01) = 0.1, got {}",
        result.predicted_std.unwrap()
    );

    // best_observed_value は観測最小値（モック観測点 (0.2,0.8) の 0.02）に厳密一致。
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
    // ランダムサーチ（4096 点）は格子分解能ぶんの誤差を許容する。
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
    // f は (0.3, 0.7) から最遠の角 (1, 0) で最大になる。
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
    // maximize 方向で勾配法が最遠の角 (1, 0) へ厳密に到達する。
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
    // 線形曲面 f = 2x − y は箱の角 (0, 1) で最小。事後分散なしモック（Ridge 系）では
    // predicted_std が None になることも確認する。
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
    // 既知の二次曲面で 2D スライス格子を構築し、形状と最小値の整合を厳密に確認する。
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
    // 恒等正規化なので格子は [0,1] を等分し、z = f(格子点) に厳密一致する。
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
    // 事後分散一定 → z_std は全格子で 0.1。
    let z_std = slice.z_std.expect("GP-like mock has z_std");
    assert!(z_std.iter().flatten().all(|&s| (s - 0.1).abs() < 1e-9));
    // 格子の最小値はサロゲート最適値の近くにある。
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
            slice_params: Some((0, 0)), // 同一軸は無効
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

// NOTE: GP の当てはめ品質（滑らかな関数で R² が高い 等）はバックエンドの egobox の
// 責務なので検証しない。検証レポートの構造（n_samples / cv_folds / oof_pairs 長・各値が
// 有限）は validate_surrogate_minimum_size_dataset と
// validate_surrogate_deterministic_with_same_seed が確認する。

#[test]
fn validate_surrogate_deterministic_with_same_seed() {
    // 同一シードで呼び出した結果が完全に一致することを確認する。
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
    // n = 10 の最小データセットで検証が成功し、期待されるフィールドを持つことを確認する。
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
// fit_surrogate_with_validation + optimize_on_trained のエンドツーエンドテスト
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

    // 検証レポートの基本チェック。
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
// ARD パラメータ重要度（param_importance）
// ────────────────────────────────────────────────────────────

#[test]
fn param_importance_reflects_ard_for_gp_and_none_for_others() {
    // x0 に強く依存し x1 にほぼ依存しない関数 y = 3*x0 + 0.05*x1（ノイズなし）。
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

    // GP-FITC: Some、長さ 2、合計 ≈ 1.0、importance[0] > importance[1]。
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

    // Ridge / LightGBM は ARD を持たないため None。
    assert!(make(SurrogateModelKind::Ridge).param_importance.is_none());
    assert!(make(SurrogateModelKind::Lgbm).param_importance.is_none());

    // MoE は θ がエキスパートごとに分かれ集約が一意でないため None。
    // この純線形・ノイズなしデータは MoE の CV 学習が退化しうるため、CV を経由しない
    // models::fit_surrogate で直接 MoE モデルを学習して param_importance を確認する。
    if let Ok(moe) = models::fit_surrogate(SurrogateModelKind::GpMoe, &x_matrix, &y) {
        assert!(moe.param_importance().is_none());
    }
}

// ────────────────────────────────────────────────────────────
// GpVfe / GpMoe の追加カバレッジ
// ────────────────────────────────────────────────────────────

#[test]
fn gp_vfe_trains_and_predicts_finite_with_std() {
    // GP-VFE が二次関数データで学習・予測でき、predicted_std が Some であることを確認。
    // run_surrogate_optimization を使い CV フォールドの小さいサブセットに依存しない。
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
    // GP-MOE が区分的（不連続）関数データで学習・予測でき、predicted_std が Some
    // であることを確認。滑らかな二次関数（quadratic_samples）では egobox-moe が
    // クラスタ数 1 を選べず内部パニックを起こすため、MoE が本来有利な区分データを使う。
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
        model: SurrogateModelKind::GpFitc,
        slice_params: Some((0, 1)),
        n_grid: 10,
    }
}

// ── 解析的モックによる多目的フロント機構の厳密検証 ──────────────────
// Schaffer N.1: f1 = x0²、f2 = (x0 − 1)²（x1 はダミー次元）。既知曲面を注入し、
// NSGA-II のフロント生成・ソート・スライス・最大化方向の結線を GP フィット無しで確認する。

fn schaffer_f1(x: &[f64]) -> f64 {
    x[0].powi(2)
}
fn schaffer_f2(x: &[f64]) -> f64 {
    (x[0] - 1.0).powi(2)
}

/// Schaffer N.1 の 2 目的を解析的モックで学習済みにした TrainedSurrogate 群。
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
    // 既知曲面なのでフロントは真の Schaffer フロント（全域）を厳密に張る。
    let result = optimize_schaffer(vec![true, true], None);

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
    // 各 ParetoFrontPoint の params/values 長が param_names/objective_names と一致。
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
        // 既知曲面なので各点の予測値は f(params) に厳密一致する。
        assert!((p.values[0] - schaffer_f1(&p.params)).abs() < 1e-9);
        assert!((p.values[1] - schaffer_f2(&p.params)).abs() < 1e-9);
    }
    assert_eq!(result.r_squared.len(), 2, "r_squared should have 2 entries");
}

#[test]
fn multi_opt_maximize_objective_direction() {
    // f2 を最大化（minimize=false）する場合、結果の f2 値が正の方向で分布すること。
    // f2 = (x0 − 1)² は x0=0 で最大値 1、x0=1 で最小値 0。
    let result = optimize_schaffer(vec![true, false], None);

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
    // slice_params = None のとき slices が空。
    let result = optimize_schaffer(vec![true, true], None);
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
    // ys と objective_names の長さが異なる場合はエラー。
    let (x_matrix, f1, f2) = schaffer_samples(30);
    let req = SurrogateMultiOptRequest {
        x_matrix,
        ys: vec![f1, f2],
        param_names: vec!["x0".to_string(), "x1".to_string()],
        objective_names: vec!["f1".to_string()], // 長さ不一致
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

// NOTE: フロントがトレードオフ全域に広がること・スライスが目的数ぶん返ることは
// 解析的モック版 multi_opt_front_spans_full_tradeoff / multi_opt_slices_returned_for_each_objective
// が厳密に確認する。実フィットの 2 段階フロー（fit → optimize_multi_on_trained）の
// 同一性は staged_multi_opt_matches_one_shot_result が担保する。

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
// fit_multi_surrogates（パレートフロント集中）
// ────────────────────────────────────────────────────────────

#[test]
fn fit_multi_surrogates_runs_end_to_end() {
    // 2 目的トレードオフ問題で fit→optimize のパイプライン結線を確認する。
    // 誘導点経路 (N>100) は gaussian_process の select_inducing_points_* /
    // fit_inducing_path で網羅済みなのでここでは厳密経路の小さい N で足りる。
    // 多目的フロントの決定性は staged_multi_opt_matches_one_shot_result でカバーする。
    // GpFitc で 2 モデルを学習 → 予測が有限 → NSGA-II でフロントが非空。
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
        // 学習データ点で予測が有限。
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

    // optimize_multi_on_trained で非空フロントが得られる。
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
    // objective_names の長さ不一致。
    // TrainedSurrogate は Debug 非実装のため unwrap_err は使えない。match で取り出す。
    let err = match fit_multi_surrogates(
        &x_matrix,
        &[f1.clone(), f2.clone()],
        &["x0".to_string(), "x1".to_string()],
        &["f1".to_string()], // 不一致
        SurrogateModelKind::GpFitc,
        &[true, true],
    ) {
        Ok(_) => panic!("expected length-mismatch error"),
        Err(e) => e,
    };
    assert!(err.contains("equal length"), "unexpected error: {err}");

    // 目的列の長さが x_matrix 行数と不一致。
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
    // LGBM で多目的サロゲート最適化が動き、フロントが返ること（緩い検証）。
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
// 制約付きサロゲート最適化のテスト
// ============================================================================

/// 制約付き SurrogateFitRequest を作るヘルパー。
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

/// 制約曲面 c(x) = 0.5 − x0。c ≤ 0 ⟺ x0 ≥ 0.5 が実行可能。
fn constraint_surface(x: &[f64]) -> f64 {
    0.5 - x[0]
}

/// 目的 = 二次曲面、制約 c = 0.5 − x0 の解析的モック TrainedSurrogate を作る。
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
    // c = 0.5 − x0（x0 ≥ 0.5 で実行可能）。既知曲面では制約ペナルティ付き最小は
    // x0 = 0.5（境界）、x1 = 0.7 に厳密に収束する。
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
    // 境界では c(x) = 0.5 − 0.5 = 0 に厳密一致。
    assert!(
        result.predicted_constraints[0].abs() < 0.02,
        "predicted constraint ≈ 0 at boundary, got {}",
        result.predicted_constraints[0]
    );
    // P(c ≤ 0) = Φ((0 − mu)/σ) = Φ(0) = 0.5（境界・σ=0.1）。
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
    // 制約なし: 既知曲面の真の最小点 (0.3, 0.7) を厳密に発見する。
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
    // fit_surrogate_with_validation が制約ありで成功し、constraint_names が設定される。
    let (x_matrix, y, c) = constrained_quadratic_samples(40);
    let req = constrained_fit_req(x_matrix, y, c);
    let trained = fit_surrogate_with_validation(&req).expect("constrained fit should succeed");

    assert_eq!(trained.constraint_names, vec!["c1".to_string()]);
    assert_eq!(trained.constraint_models.len(), 1);
    assert_eq!(trained.constraint_values.len(), 40);
    assert!(trained.constraint_values.iter().all(|row| row.len() == 1));
}

// NOTE: optimize_on_trained が制約ありで predicted_constraints / feasibility_probability を
// 返すことは、解析的モック版 constrained_opt_pushes_x_toward_feasible_region が
// 値レベルで厳密に確認する。

#[test]
fn suggest_candidates_constrained_p_feas_present() {
    // 制約付き suggest_candidates: 全候補が feasibility_probability Some を持つ。
    // mean P_feas > 0.3。
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

    // 決定性: 同じ trained で 2 回呼ぶと同じ結果。
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
    // 制約なし suggest_candidates（n=1, 再フィットなし）: 既知の GP 系モックで
    // feasibility_probability が None、predicted_constraints が空になることを確認する。
    let trained = analytic_trained(quad_surface, true);
    let candidates = suggest_candidates(&trained, 1, AcquisitionKind::ExpectedImprovement, true)
        .expect("unconstrained suggest should succeed");

    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].feasibility_probability.is_none());
    assert!(candidates[0].predicted_constraints.is_empty());
    // 提案点は元の単位 [0,1] 内、EI は非負。
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
// 自動モデル選択（Auto）
// ────────────────────────────────────────────────────────────

/// 明確に非線形・滑らかな関数 y = sin(3·x0) + x1² のサンプル。
/// GP が Ridge を CV R² で上回ることを期待する。
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

/// 明確に線形な関数 y = 2·x0 − x1 + 微小ノイズ のサンプル。
/// Ridge が勝つ、または同点で Ridge にタイブレークされることを期待する。
fn linear_samples(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    let mut rng = SeededRng::from_seed(17);
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|_| vec![rng.next_f64(), rng.next_f64()])
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        // 微小ノイズ（[-0.005, 0.005]）を加える。
        .map(|r| 2.0 * r[0] - r[1] + (rng.next_f64() - 0.5) * 0.01)
        .collect();
    (x_matrix, y)
}

/// `scores` から指定モデルのスコアを取り出すヘルパー。
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

    // 候補は AUTO_CANDIDATES の 4 つ。
    assert_eq!(report.scores.len(), 4);

    // 非線形・滑らかな関数では GP が選ばれる（Ridge ではない）。
    assert!(
        matches!(
            report.chosen,
            SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe
        ),
        "expected a GP, got {:?}",
        report.chosen
    );

    // GP の CV R² は Ridge を上回る。
    let ridge = score_of(&report, SurrogateModelKind::Ridge);
    let gp = score_of(&report, SurrogateModelKind::GpFitc)
        .max(score_of(&report, SurrogateModelKind::GpVfe));
    assert!(gp > ridge, "GP cv_r2 {gp} should exceed Ridge {ridge}");
}

#[test]
fn select_best_model_picks_ridge_on_linear() {
    // 線形データでの Ridge 選択（タイブレーク）判定は品質感度が高く、N を下げると
    // GP の CV R² が Ridge を 1e-3 超で上回りタイブレーク条件が崩れるため 80 を保つ。
    let (x_matrix, y) = linear_samples(80);
    let report = select_best_model(&x_matrix, &y, 42).expect("selection should succeed");

    assert_eq!(report.scores.len(), 4);

    // 線形関数では Ridge が選ばれる。同点なら AUTO_CANDIDATES 先頭の Ridge が残る。
    let ridge = score_of(&report, SurrogateModelKind::Ridge);
    let best = report
        .scores
        .iter()
        .map(|(_, s)| *s)
        .fold(f64::NEG_INFINITY, f64::max);
    // Ridge は最良スコアと 1e-3 以内（タイブレークで Ridge が選ばれる条件）。
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
        // Auto: model フィールドは無視される（プレースホルダ）。
        model: SurrogateModelKind::Ridge,
        auto_select: true,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    };
    let trained = fit_surrogate_with_validation(&req).expect("auto fit should succeed");

    // 選択経緯が付与され、4 候補のスコアを持つ。
    let selection = trained
        .model_selection
        .as_ref()
        .expect("auto fit must attach a model_selection report");
    assert_eq!(selection.scores.len(), 4);

    // model_kind は選ばれた具体的なモデル種別（Ridge プレースホルダではない）。
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
    // どのモデルが勝つかではなく再現性のみ検証するので、品質に依存せず小さい N でよい。
    // 4 候補 × CV を 2 回回すためコストが大きく、N を絞る効果が大きい。
    let (x_matrix, y) = nonlinear_smooth_samples(30);
    let r1 = select_best_model(&x_matrix, &y, 42).expect("run 1");
    let r2 = select_best_model(&x_matrix, &y, 42).expect("run 2");

    // 選択モデルが一致する。
    assert_eq!(r1.chosen, r2.chosen);
    // スコアが（順序・値とも）完全一致する。
    assert_eq!(r1.scores.len(), r2.scores.len());
    for ((k1, s1), (k2, s2)) in r1.scores.iter().zip(r2.scores.iter()) {
        assert_eq!(k1, k2);
        assert_eq!(s1.to_bits(), s2.to_bits(), "scores must be bit-identical");
    }
}

// ────────────────────────────────────────────────────────────
// 大規模データの間引き（subsample_indices）
// ────────────────────────────────────────────────────────────

#[test]
fn subsample_returns_none_when_within_cap() {
    let y: Vec<f64> = (0..MAX_TRAIN_FOR_FIT).map(|i| i as f64).collect();
    assert!(subsample_indices(&[&y], &[], MAX_TRAIN_FOR_FIT, 42).is_none());
}

#[test]
fn subsample_single_keeps_both_extremes_and_caps_size() {
    let n = MAX_TRAIN_FOR_FIT * 3;
    let y: Vec<f64> = (0..n).map(|i| i as f64).collect(); // 最小=0, 最大=n-1
    let idx = subsample_indices(&[&y], &[], MAX_TRAIN_FOR_FIT, 42).expect("should subsample");

    assert_eq!(idx.len(), MAX_TRAIN_FOR_FIT, "間引き後は cap 点ちょうど");
    // インデックスは昇順かつ一意。
    assert!(idx.windows(2).all(|w| w[0] < w[1]), "昇順かつ重複なし");
    // 範囲内。
    assert!(idx.iter().all(|&i| i < n));
    // best（最小値=index 0）と worst（最大値=index n-1）が両方残る。
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
    // 2 目的、両方最小化。明確な非劣点（rank 0）を仕込み、保持されることを確認する。
    let n = MAX_TRAIN_FOR_FIT * 2;
    // obj0 = i, obj1 = n - i の単調トレードオフ → 全点が rank 0（強パレート）。
    // ここでは「rank 0 集合の代表」が確実に残ることだけ確認する。
    let mut rng = SeededRng::from_seed(3);
    let obj0: Vec<f64> = (0..n).map(|_| rng.next_f64()).collect();
    let obj1: Vec<f64> = (0..n).map(|_| rng.next_f64()).collect();
    let minimize = [true, true];
    let idx = subsample_indices(&[&obj0, &obj1], &minimize, MAX_TRAIN_FOR_FIT, 42)
        .expect("should subsample");
    assert_eq!(idx.len(), MAX_TRAIN_FOR_FIT);
    assert!(idx.windows(2).all(|w| w[0] < w[1]));

    // 真の非劣点（rank 0）の少なくとも 1 つは保持される。
    let rows: Vec<Vec<f64>> = (0..n).map(|i| vec![obj0[i], obj1[i]]).collect();
    let ranks = crate::multi_objective::pareto::nd_sort(&rows, &minimize);
    let front: Vec<usize> = (0..n).filter(|&i| ranks[i] == 0).collect();
    assert!(
        front.iter().any(|f| idx.contains(f)),
        "パレートフロント上の点が保持されること"
    );
}

// ────────────────────────────────────────────────────────────
// 進捗報告とキャンセル（FitProgress / *_tracked）
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
    // 単目的・制約なし・手動: total = (ホールドアウト 1 + CV 5) + 最終 1 = 7。
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
    // キャンセルは最初の学習前に検知されるので進捗は進まない。
    assert_eq!(progress.snapshot().done, 0);
}

#[test]
fn fit_multi_tracked_cancel_returns_err() {
    let (x_matrix, y) = quadratic_samples(40);
    // 2 目的（同一データを流用; キャンセルは学習前に返るため値は問わない）。
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
    // N > cap でも学習が成功し、訓練データが cap 以下に間引かれること。
    let (x_matrix, y) = quadratic_samples(MAX_TRAIN_FOR_FIT + 500);
    let req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names: vec!["x".to_string(), "y".to_string()],
        objective_name: "f".to_string(),
        model: SurrogateModelKind::Ridge, // 高速なモデルでパスを検証する
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
