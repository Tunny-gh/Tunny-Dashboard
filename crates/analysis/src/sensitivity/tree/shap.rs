use super::super::constants::{
    SHAP_MAX_ROWS, SHAP_RF_MAX_DEPTH, SHAP_RF_MIN_SAMPLES_LEAF, SHAP_RF_TREES, SHAP_SEED,
};
use super::common::{run_importances_pipeline, PreparedData};
use crate::lgbm::{lgbm_mse, lgbm_predict_contrib, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};

/// Compute SHAP importances from preprocessed data (also called from `metrics::ShapMetric`).
pub(in crate::sensitivity) fn compute_from_prepared(
    data: &PreparedData,
) -> Option<(Vec<f64>, f64)> {
    let p = data.x_shuffled[0].len();
    let (x_train, x_eval, y_train, y_eval) = data.split();
    let config = LgbmRfConfig {
        num_iterations: SHAP_RF_TREES,
        max_depth: SHAP_RF_MAX_DEPTH as i32,
        min_data_in_leaf: SHAP_RF_MIN_SAMPLES_LEAF as i32,
        seed: SHAP_SEED as i32,
        ..Default::default()
    };
    let booster = train_lgbm_rf(x_train, y_train, &config)?;

    // phi: shape [n_train][p+1], last column is bias term (excluded)
    let phi = lgbm_predict_contrib(&booster, x_train);
    let n_train = phi.len();
    if n_train == 0 {
        return Some((vec![0.0; p], 0.0));
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
    Some((importances, r_squared))
}

/// Uses `predict_contrib` (TreeSHAP) per sample; global importance is mean |phi_j| normalised to sum = 1.
pub fn compute_shap_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    run_importances_pipeline(
        x_matrix,
        y,
        SHAP_MAX_ROWS,
        SHAP_SEED,
        SHAP_SEED.wrapping_add(1),
        compute_from_prepared,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_xy(n: usize, dominant: usize, n_feats: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut rng = crate::math::rng::SeededRng::from_seed(77);
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
