use super::super::constants::{
    RF_ANOVA_MAX_ROWS, RF_ANOVA_RF_MAX_DEPTH, RF_ANOVA_RF_MIN_SAMPLES_LEAF, RF_ANOVA_RF_TREES,
    RF_ANOVA_SEED,
};
use super::common::{normalize, permute_column_inplace, prepare_training_data, PreparedData};
use crate::lgbm::{lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};

/// 前処理済みデータから RF-ANOVA 重要度を計算する（`metrics::RfAnovaMetric` からも呼ばれる）。
pub(in crate::sensitivity) fn compute_from_prepared(data: &PreparedData) -> Option<(Vec<f64>, f64)> {
    let p = data.x_shuffled[0].len();
    let (x_train, x_eval, y_train, y_eval) = data.split();
    let config = LgbmRfConfig {
        num_iterations: RF_ANOVA_RF_TREES,
        max_depth: RF_ANOVA_RF_MAX_DEPTH as i32,
        min_data_in_leaf: RF_ANOVA_RF_MIN_SAMPLES_LEAF as i32,
        seed: RF_ANOVA_SEED as i32,
        ..Default::default()
    };
    let booster = train_lgbm_rf(x_train, y_train, &config)?;

    let baseline_mse = lgbm_mse(&booster, x_eval, y_eval)
        .unwrap_or(0.0)
        .max(f64::EPSILON);
    let r_squared = mse_to_r_squared(baseline_mse, y_eval).max(0.0);

    // eval 行列を 1 回だけクローンし、特徴量ごとにインプレース置換→MSE計算→復元する。
    // permute_single_column(p 回クローン) と比べてアロケーションを O(p) → O(1) に削減。
    let mut x_eval_work = x_eval.to_vec();
    let mut importances = vec![0.0; p];
    for feature_idx in 0..p {
        let orig_col: Vec<f64> = x_eval_work.iter().map(|r| r[feature_idx]).collect();
        permute_column_inplace(
            &mut x_eval_work,
            feature_idx,
            RF_ANOVA_SEED + feature_idx as u64,
        );
        let permuted_mse = lgbm_mse(&booster, &x_eval_work, y_eval).unwrap_or(baseline_mse);
        importances[feature_idx] = (permuted_mse - baseline_mse).max(0.0);
        for (i, row) in x_eval_work.iter_mut().enumerate() {
            row[feature_idx] = orig_col[i];
        }
    }

    normalize(&mut importances);
    Some((importances, r_squared))
}

/// Returns (importances, r_squared).
pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
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
        RF_ANOVA_MAX_ROWS,
        RF_ANOVA_SEED,
        RF_ANOVA_SEED.wrapping_add(1),
    ) {
        Some(data) => compute_from_prepared(&data).unwrap_or((vec![0.0; p], 0.0)),
        None => (vec![0.0; p], 0.0),
    }
}
