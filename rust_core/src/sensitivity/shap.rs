use super::data::sample_rows;
use crate::core::lgbm::{
    lgbm_mse, lgbm_predict_contrib, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig,
};
use crate::core::random_forest::Lcg;

const RF_TREES: usize = 64;
const RF_MAX_DEPTH: usize = 10;
const RF_MIN_SAMPLES_LEAF: usize = 2;
const RF_SEED: u64 = 42;
const SHAP_MAX_ROWS: usize = 1_000;

/// Compute global SHAP feature importance via LightGBM native TreeSHAP.
///
/// Uses `predict_contrib` (C_API_PREDICT_CONTRIB) which returns exact Shapley
/// values per sample. Global importance is mean |phi_j| normalised to sum = 1.
/// Returns `(importances, r_squared)`.
pub fn compute_shap_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }

    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }

    // Filter non-finite rows
    let valid: Vec<usize> = (0..n)
        .filter(|&i| y[i].is_finite() && x_matrix[i].iter().all(|v| v.is_finite()))
        .collect();

    let n_valid = valid.len();
    if n_valid < 2 {
        return (vec![0.0; p], 0.0);
    }

    // Filter and/or downsample, avoiding a redundant full clone when all rows are valid
    let (x_data, y_data) = if n_valid < n {
        let x_clean: Vec<Vec<f64>> = valid.iter().map(|&i| x_matrix[i].clone()).collect();
        let y_clean: Vec<f64> = valid.iter().map(|&i| y[i]).collect();
        if n_valid > SHAP_MAX_ROWS {
            sample_rows(&x_clean, &y_clean, SHAP_MAX_ROWS, RF_SEED)
        } else {
            (x_clean, y_clean)
        }
    } else if n > SHAP_MAX_ROWS {
        sample_rows(x_matrix, y, SHAP_MAX_ROWS, RF_SEED)
    } else {
        (x_matrix.to_vec(), y.to_vec())
    };

    let n = y_data.len();

    // 80/20 holdout split
    const MIN_EVAL: usize = 2;
    const MIN_TRAIN: usize = 2;
    let use_holdout = n >= MIN_TRAIN + MIN_EVAL;
    let split_idx = if use_holdout {
        ((n * 4) / 5).max(MIN_TRAIN)
    } else {
        n
    };

    let mut shuffle_idx: Vec<usize> = (0..n).collect();
    let mut rng_split = Lcg::new(RF_SEED.wrapping_add(1));
    for i in (1..n).rev() {
        let j = rng_split.next_usize(i + 1);
        shuffle_idx.swap(i, j);
    }
    let x_sh: Vec<Vec<f64>> = shuffle_idx.iter().map(|&i| x_data[i].clone()).collect();
    let y_sh: Vec<f64> = shuffle_idx.iter().map(|&i| y_data[i]).collect();

    let (x_train, x_eval, y_train, y_eval) = if use_holdout {
        (
            &x_sh[..split_idx],
            &x_sh[split_idx..],
            &y_sh[..split_idx],
            &y_sh[split_idx..],
        )
    } else {
        (
            x_sh.as_slice(),
            x_sh.as_slice(),
            y_sh.as_slice(),
            y_sh.as_slice(),
        )
    };

    let config = LgbmRfConfig {
        num_iterations: RF_TREES,
        max_depth: RF_MAX_DEPTH as i32,
        min_data_in_leaf: RF_MIN_SAMPLES_LEAF as i32,
        seed: RF_SEED as i32,
        ..Default::default()
    };
    let booster = match train_lgbm_rf(x_train, y_train, &config) {
        Some(b) => b,
        None => return (vec![0.0; p], 0.0),
    };

    // phi: shape [n_train][p+1], last column is bias term (excluded)
    let phi = lgbm_predict_contrib(&booster, x_train);
    let n_train = phi.len();
    if n_train == 0 {
        return (vec![0.0; p], 0.0);
    }

    let mut phi_sum = vec![0.0f64; p];
    for sample_phi in &phi {
        for j in 0..p {
            phi_sum[j] += sample_phi[j].abs();
        }
    }

    let total: f64 = phi_sum.iter().sum();
    let importances = if total < f64::EPSILON {
        vec![0.0; p]
    } else {
        phi_sum.iter().map(|v| v / total).collect()
    };

    let mse = lgbm_mse(&booster, x_eval, y_eval).unwrap_or(f64::INFINITY);
    let r_squared = mse_to_r_squared(mse, y_eval);

    (importances, r_squared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_xy(n: usize, dominant: usize, n_feats: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut rng = Lcg::new(77);
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
    fn importances_sum_to_one() {
        let (x, y) = make_xy(60, 0, 3);
        let (imp, _) = compute_shap_importances(&x, &y);
        assert_eq!(imp.len(), 3);
        let sum: f64 = imp.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9 || sum == 0.0, "sum={sum}");
    }

    #[test]
    fn dominant_feature_ranks_first() {
        let (x, y) = make_xy(100, 1, 3);
        let (imp, _) = compute_shap_importances(&x, &y);
        let max_idx = imp
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(max_idx, 1, "importances={imp:?}");
    }

    #[test]
    fn bias_column_excluded() {
        let (x, y) = make_xy(40, 0, 2);
        let (imp, _) = compute_shap_importances(&x, &y);
        // p=2 features; must not include the bias column (p+1=3)
        assert_eq!(imp.len(), 2);
    }

    #[test]
    fn empty_input_returns_empty() {
        let (imp, r2) = compute_shap_importances(&[], &[]);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }

    #[test]
    fn single_sample_returns_empty() {
        let x = vec![vec![1.0, 2.0]];
        let y = vec![3.0];
        let (imp, r2) = compute_shap_importances(&x, &y);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }
}
