use super::constants::{
    MDI_MAX_ROWS, MDI_RF_MAX_DEPTH, MDI_RF_MIN_SAMPLES_LEAF, MDI_RF_TREES, MDI_SEED,
};
use super::tree_common::{prepare_training_data, PreparedData};
use crate::core::lgbm::{
    lgbm_feature_importance, lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig,
};

/// 前処理済みデータから MDI 重要度を計算する（`metrics::MdiMetric` からも呼ばれる）。
pub(super) fn compute_from_prepared(data: &PreparedData) -> Option<(Vec<f64>, f64)> {
    let p = data.x_shuffled[0].len();
    let (x_train, x_eval, y_train, y_eval) = data.split();
    let config = LgbmRfConfig {
        num_iterations: MDI_RF_TREES,
        max_depth: MDI_RF_MAX_DEPTH as i32,
        min_data_in_leaf: MDI_RF_MIN_SAMPLES_LEAF as i32,
        seed: MDI_SEED as i32,
        ..Default::default()
    };
    let booster = train_lgbm_rf(x_train, y_train, &config)?;
    let importances = lgbm_feature_importance(&booster, p);
    let mse = lgbm_mse(&booster, x_eval, y_eval).unwrap_or(f64::INFINITY);
    let r_squared = mse_to_r_squared(mse, y_eval);
    Some((importances, r_squared))
}

/// Compute MDI (Mean Decrease Impurity) importances via LightGBM gain-based feature importance.
/// Returns `(importances, r_squared)` where importances sum to 1.0 (or all-zero on failure).
pub fn compute_mdi_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }
    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }
    match prepare_training_data(
        x_matrix,
        y,
        MDI_MAX_ROWS,
        MDI_SEED,
        MDI_SEED.wrapping_add(1),
    ) {
        Some(data) => compute_from_prepared(&data).unwrap_or((vec![0.0; p], 0.0)),
        None => (vec![0.0; p], 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_xy(n: usize, dominant_feat: usize, n_feats: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut rng = crate::core::math::rng::SeededRng::from_seed(99);
        let x: Vec<Vec<f64>> = (0..n)
            .map(|_| {
                (0..n_feats)
                    .map(|_| rng.next_usize(1000) as f64 / 1000.0)
                    .collect()
            })
            .collect();
        let y: Vec<f64> = x.iter().map(|row| row[dominant_feat] * 10.0).collect();
        (x, y)
    }

    #[test]
    fn importances_sum_to_one() {
        let (x, y) = make_xy(60, 0, 3);
        let (importances, _) = compute_mdi_importances(&x, &y);
        assert_eq!(importances.len(), 3);
        let sum: f64 = importances.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9 || sum == 0.0, "sum={sum}");
    }

    #[test]
    fn dominant_feature_ranks_first() {
        let (x, y) = make_xy(80, 1, 3);
        let (importances, _) = compute_mdi_importances(&x, &y);
        assert_eq!(importances.len(), 3);
        let max_idx = importances
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(max_idx, 1, "importances={importances:?}");
    }

    #[test]
    fn empty_input_returns_empty() {
        let (imp, r2) = compute_mdi_importances(&[], &[]);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }

    #[test]
    fn single_sample_returns_empty() {
        let x = vec![vec![1.0, 2.0]];
        let y = vec![3.0];
        let (imp, r2) = compute_mdi_importances(&x, &y);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }
}
