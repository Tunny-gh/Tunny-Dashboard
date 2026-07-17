use super::*;

/// Build `n` samples with 2 params (`x1`, `x2`) where the objective depends
/// only on `x1` (`y = x1`), linearly and monotonically increasing.
///
/// `x2` is an unrelated oscillating feature (`sin(i * 0.3)`), used to check
/// that the PDP for `x1` stays monotone regardless of `x2`.
fn make_linear_data_1d(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<String>) {
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let x1 = i as f64 / n as f64;
            let x2 = (i as f64 * 0.3).sin();
            vec![x1, x2]
        })
        .collect();
    let y: Vec<f64> = x_matrix.iter().map(|row| row[0]).collect();
    let names = vec!["x1".to_string(), "x2".to_string()];
    (x_matrix, y, names)
}

/// Build `n` samples with 3 correlated params (`x1`, `x2`, `x3`, all derived
/// from a shared `t = i / n`) and an objective that is an exact linear
/// combination of them: `y = 2*x1 - 0.5*x2 + 0.3*x3`.
///
/// Used to check that PDP/surrogate fits recover a known linear relationship
/// across multiple parameters.
fn make_linear_data_multi(n: usize) -> (Vec<Vec<f64>>, Vec<f64>, Vec<String>) {
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            vec![t, t * 1.3 + 0.1, 1.0 - t * 0.7]
        })
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|row| 2.0 * row[0] - 0.5 * row[1] + 0.3 * row[2])
        .collect();
    let names = vec!["x1".to_string(), "x2".to_string(), "x3".to_string()];
    (x_matrix, y, names)
}

#[test]
fn tc_803_01_pdp_monotone_positive() {
    let n = 200;
    let (x_matrix, y, names) = make_linear_data_1d(n);

    let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

    assert_eq!(result.grid.len(), 10, "translated 10 translated");
    assert_eq!(result.values.len(), 10, "PDPtranslated 10 translated");

    for i in 0..result.values.len() - 1 {
        assert!(
            result.values[i] < result.values[i + 1],
            "PDP[{}]={} translated PDP[{}]={} translated（translated）",
            i,
            result.values[i],
            i + 1,
            result.values[i + 1]
        );
    }
}

#[test]
fn tc_803_02_pdp_monotone_negative() {
    let n = 100;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
    let y: Vec<f64> = x_matrix.iter().map(|row| -row[0]).collect();
    let names = vec!["x1".to_string()];

    let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 8);

    for i in 0..result.values.len() - 1 {
        assert!(
            result.values[i] > result.values[i + 1],
            "PDP[{}]={} translated PDP[{}]={} translated（translated）",
            i,
            result.values[i],
            i + 1,
            result.values[i + 1]
        );
    }
}

#[test]
fn tc_803_03_pdp_midpoint_equals_ymean() {
    let n = 200;
    let (x_matrix, y, names) = make_linear_data_1d(n);
    let y_mean = y.iter().sum::<f64>() / n as f64;

    let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 11);

    let mid_idx = result.values.len() / 2;
    let mid_val = result.values[mid_idx];

    let tolerance = (y_mean.abs() + 0.01) * 0.05;
    assert!(
        (mid_val - y_mean).abs() < tolerance,
        "translatedPDPtranslated {} translated y_mean {} translated ±5% translated",
        mid_val,
        y_mean
    );
}

#[test]
fn tc_803_04_pdp_r_squared_high_for_linear() {
    let n = 200;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
    let y: Vec<f64> = x_matrix.iter().map(|row| row[0] * 3.0 + 1.0).collect();
    let names = vec!["x1".to_string()];

    let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

    assert!(
        result.r_squared > 0.99,
        "translated R² {} translated 0.99 translated",
        result.r_squared
    );
}

#[test]
fn tc_803_05_empty_data_returns_empty() {
    let x_matrix: Vec<Vec<f64>> = vec![vec![1.0]];
    let y = vec![1.0f64];
    let names = vec!["x1".to_string()];

    let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 10);

    assert!(result.grid.is_empty(), "n<2 translated");
    assert!(result.values.is_empty(), "n<2 translated");
    assert_eq!(
        result.r_squared, 0.0,
        "n<2 translated R² translated 0.0 translated"
    );
}

#[test]
fn tc_803_06_pdp_2d_grid_shape() {
    let n = 100;
    let (x_matrix, y, names) = make_linear_data_multi(n);

    let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 8);

    assert_eq!(result.x_values.len(), 8, "x_values translated 8 translated");
    assert_eq!(result.y_values.len(), 8, "y_values translated 8 translated");
    assert_eq!(result.z_values.len(), 8, "z_values translated 8 translated");
    for row in &result.z_values {
        assert_eq!(row.len(), 8, "z_values translated 8 translated");
    }
}

#[test]
fn tc_803_07_pdp_2d_empty_data() {
    let x_matrix: Vec<Vec<f64>> = vec![vec![1.0, 2.0]];
    let y = vec![1.0f64];
    let names = vec!["x1".to_string(), "x2".to_string()];

    let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 5);

    assert!(
        result.x_values.is_empty(),
        "n<2 translated x_values translated"
    );
    assert!(
        result.y_values.is_empty(),
        "n<2 translated y_values translated"
    );
    assert!(
        result.z_values.is_empty(),
        "n<2 translated z_values translated"
    );
}

#[test]
fn tc_803_08_pdp_2d_r_squared() {
    let n = 200;
    let (x_matrix, y, names) = make_linear_data_multi(n);

    let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 5);

    assert!(
        result.r_squared > 0.95,
        "translated 2parameterPDP R² {} translated 0.95 translated",
        result.r_squared
    );
}

#[test]
fn tc_803_09_result_names() {
    let n = 50;
    let (x_matrix, y, names) = make_linear_data_1d(n);

    let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj_target", 0, 5);

    assert_eq!(
        result.param_name, "x1",
        "param_name translated 'x1' translated"
    );
    assert_eq!(
        result.objective_name, "obj_target",
        "objective_name translated 'obj_target' translated"
    );
}

#[test]
fn tc_803_p01_pdp_1d_performance() {
    #[cfg(debug_assertions)]
    let (n, p) = (1_000, 4);
    #[cfg(not(debug_assertions))]
    let (n, p) = (50_000, 10);

    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..p)
                .map(|j| i as f64 / n as f64 + j as f64 * 0.1)
                .collect()
        })
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|row| row[0] * 2.0 + row[1] * 0.5)
        .collect();
    let names: Vec<String> = (0..p).map(|j| format!("x{}", j)).collect();

    let result = compute_pdp_from_matrix(&x_matrix, &y, &names, "obj0", 0, 20);

    assert_eq!(result.grid.len(), 20, "translated 20 translated");
}

#[test]
fn tc_803_p02_pdp_2d_performance() {
    #[cfg(debug_assertions)]
    let (n, p) = (1_000, 4);
    #[cfg(not(debug_assertions))]
    let (n, p) = (50_000, 10);

    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..p)
                .map(|j| i as f64 / n as f64 + j as f64 * 0.1)
                .collect()
        })
        .collect();
    let y: Vec<f64> = x_matrix
        .iter()
        .map(|row| row[0] * 2.0 + row[1] * 0.5)
        .collect();
    let names: Vec<String> = (0..p).map(|j| format!("x{}", j)).collect();

    let result = compute_pdp_2d_from_matrix(&x_matrix, &y, &names, "obj0", 0, 1, 15);

    assert_eq!(
        result.z_values.len(),
        15,
        "z_values translated 15 translated"
    );
    assert_eq!(
        result.z_values[0].len(),
        15,
        "z_values translated 15 translated"
    );
}

#[test]
fn tc_1645_01_gp_fitc_raw_grid_shape() {
    use crate::gaussian_process::GpMethod;
    let n = 30;
    let x_2d: Vec<Vec<f64>> = (0..n)
        .map(|i| vec![i as f64 / n as f64, (i as f64 * 0.3).sin()])
        .collect();
    let y: Vec<f64> = x_2d.iter().map(|xi| xi[0] + xi[1]).collect();
    let n_grid = 10;

    let result = compute_pdp_2d_gp_raw(&x_2d, &y, 0, 1, n_grid, GpMethod::Fitc)
        .expect("compute_pdp_2d_gp_raw (FITC) should succeed");

    assert_eq!(
        result.x_values.len(),
        n_grid,
        "x_values should have n_grid points"
    );
    assert_eq!(
        result.y_values.len(),
        n_grid,
        "y_values should have n_grid points"
    );
    assert_eq!(
        result.z_values.len(),
        n_grid,
        "z_values outer dim should be n_grid"
    );
    assert_eq!(
        result.z_values[0].len(),
        n_grid,
        "z_values inner dim should be n_grid"
    );
}

#[test]
fn tc_1645_e01_insufficient_data_returns_none() {
    use crate::gaussian_process::GpMethod;
    let x_2d = vec![vec![0.0, 0.0], vec![0.5, 0.5]];
    let y = vec![0.0, 1.0];

    let result = compute_pdp_2d_gp_raw(&x_2d, &y, 0, 1, 10, GpMethod::Fitc);
    assert!(result.is_none(), "n < 3 should return None");
}

#[test]
fn tc_1652_tc_005_02_gp_fitc_n100_grid_shape() {
    use crate::gaussian_process::GpMethod;
    let n = 100;
    let x_2d: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            vec![t, (t * 3.0).sin() * 0.5 + 0.5]
        })
        .collect();
    let y: Vec<f64> = x_2d.iter().map(|r| r[0] + 0.3 * r[1]).collect();
    let n_grid = 10;

    let result = compute_pdp_2d_gp_raw(&x_2d, &y, 0, 1, n_grid, GpMethod::Fitc);
    assert!(result.is_some(), "Should succeed for N=100 with FITC");
    let r = result.unwrap();
    assert_eq!(r.x_values.len(), n_grid, "x_values.len() should be n_grid");
    assert_eq!(r.y_values.len(), n_grid, "y_values.len() should be n_grid");
    assert_eq!(r.z_values.len(), n_grid, "z_values.len() should be n_grid");
    assert_eq!(
        r.z_values[0].len(),
        n_grid,
        "z_values[0].len() should be n_grid"
    );
}

#[test]
fn tc_1652_tc_005_03_gp_vfe_small_n() {
    use crate::gaussian_process::GpMethod;
    let n = 30;
    let x_2d: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            vec![t, 1.0 - t]
        })
        .collect();
    let y: Vec<f64> = x_2d.iter().map(|r| r[0] * 2.0).collect();
    let n_grid = 5;

    let result = compute_pdp_2d_gp_raw(&x_2d, &y, 0, 1, n_grid, GpMethod::Vfe);
    assert!(result.is_some(), "VFE should succeed for N=30");
    let r = result.unwrap();
    for row in &r.z_values {
        for &v in row {
            assert!(v.is_finite(), "Grid value should be finite: {}", v);
        }
    }
}

#[test]
#[ignore]
fn tc_nfr_001_01_gp_fitc_n1000_under_10s() {
    use crate::gaussian_process::GpMethod;
    let n = 1000;
    let x_2d: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            vec![t, (t * 5.0).sin() * 0.5 + 0.5]
        })
        .collect();
    let y: Vec<f64> = x_2d.iter().map(|r| r[0] + 0.3 * r[1]).collect();

    let start = std::time::Instant::now();
    let result = compute_pdp_2d_gp_raw(&x_2d, &y, 0, 1, 50, GpMethod::Fitc);
    let elapsed = start.elapsed().as_millis();

    println!("GP-FITC N=1000: {}ms", elapsed);
    assert!(result.is_some(), "Should return Some for N=1000");
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed < 10_000,
        "NFR-001 target missed: {}ms > 10,000ms",
        elapsed
    );
}

#[test]
#[ignore]
fn tc_nfr_002_01_gp_fitc_n5000_under_5s() {
    use crate::gaussian_process::GpMethod;
    let n = 5000;
    let x_2d: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            vec![t, (t * 7.0).cos() * 0.5 + 0.5]
        })
        .collect();
    let y: Vec<f64> = x_2d.iter().map(|r| r[0] * 2.0 + r[1] * 0.5).collect();

    let start = std::time::Instant::now();
    let result = compute_pdp_2d_gp_raw(&x_2d, &y, 0, 1, 50, GpMethod::Fitc);
    let elapsed = start.elapsed().as_millis();

    println!("GP-FITC N=5000: {}ms", elapsed);
    assert!(result.is_some(), "Should return Some for N=5000");
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed < 5_000,
        "NFR-002 target missed: {}ms > 5,000ms",
        elapsed
    );
}

#[test]
fn gp_2d_pdp_marginalises_third_dimension() {
    // y = 2*x0 + 0.5*x1 + 5*x2, where x2 varies independently of (x0, x1).
    // A true 2D PDP over (x0, x1) must marginalise x2 and therefore depend
    // only weakly (through the GP fit) on it — the surface should be close to
    // the additive trend 2*x0 + 0.5*x1 + 5*mean(x2), NOT a function that tracks
    // individual x2 values. We check the surface increases along the x0 axis.
    use crate::gaussian_process::GpMethod;
    // Keep n <= max_inducing (100) so we take the Z = X (no inducing-point
    // selection) path. Inducing-point subset selection is sensitive to
    // platform floating-point differences, and was the cause of a flake
    // where training failed only on Windows.
    let n = 96;
    // Low-discrepancy (golden-ratio / sqrt-2) sequences keep x0 and x1 spread
    // over [0,1] and mutually decorrelated; perfectly collinear inputs (e.g.
    // x1 = 1 - x0) make the kernel matrix singular and FITC training fail.
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let x0 = (i as f64 * 0.618_033_988_749_895).fract();
            let x1 = (i as f64 * 0.414_213_562_373_095).fract();
            // x2 alternates to be (largely) decorrelated from x0/x1.
            let x2 = if i % 2 == 0 { 0.2 } else { 0.8 };
            vec![x0, x1, x2]
        })
        .collect();
    // Perfectly noiseless linear data tends to make the kernel matrix
    // singular, making noise-floor estimation succeed or fail depending on
    // the platform. Add a deterministic tiny perturbation (amplitude 0.05,
    // much smaller than the x0-direction trend of 2.0) to regularize it.
    let y: Vec<f64> = x_matrix
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let jitter = 0.05 * ((i as f64) * 12.989_8).sin();
            2.0 * r[0] + 0.5 * r[1] + 5.0 * r[2] + jitter
        })
        .collect();
    let n_grid = 6;

    let result = compute_pdp_2d_gp_raw(&x_matrix, &y, 0, 1, n_grid, GpMethod::Fitc)
        .expect("2D PDP over a 3-dim dataset should succeed");

    assert_eq!(result.x_values.len(), n_grid);
    assert_eq!(result.y_values.len(), n_grid);
    assert_eq!(result.z_values.len(), n_grid);
    for row in &result.z_values {
        assert_eq!(row.len(), n_grid);
        for &v in row {
            assert!(v.is_finite(), "grid value must be finite: {v}");
        }
    }
    assert!(
        result.uncertainties.is_some(),
        "GP path provides uncertainty"
    );

    // The marginalised surface must increase as x0 (first axis / outer index)
    // grows, for a fixed x1 column — reflecting the +2*x0 term.
    let col = 0;
    let first = result.z_values[0][col];
    let last = result.z_values[n_grid - 1][col];
    assert!(
        last > first,
        "surface should increase along x0: first={first}, last={last}"
    );
}

#[test]
fn tc_1653_01_gp_fitc_dispatch_returns_finite_results() {
    use crate::gaussian_process::GpMethod;
    let n = 60;
    let x_2d: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            vec![t, 1.0 - t * 0.7]
        })
        .collect();
    let y: Vec<f64> = x_2d.iter().map(|r| r[0] * 1.5 + r[1] * 0.5).collect();
    let n_grid = 5;

    let result = compute_pdp_2d_gp_raw(&x_2d, &y, 0, 1, n_grid, GpMethod::Fitc);
    assert!(result.is_some(), "GP-FITC dispatch should succeed for N=60");
    let r = result.unwrap();
    assert_eq!(r.x_values.len(), n_grid);
    assert_eq!(r.y_values.len(), n_grid);
    for row in &r.z_values {
        for &v in row {
            assert!(v.is_finite(), "All grid values should be finite: {}", v);
        }
    }
}
