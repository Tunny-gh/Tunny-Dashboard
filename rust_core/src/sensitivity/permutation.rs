use super::constants::{
    PFI_MAX_ROWS, PFI_N_REPEATS, PFI_RF_MAX_DEPTH, PFI_RF_MIN_DATA_LEAF, PFI_RF_TREES,
    PFI_SEED_BASE, PFI_SPLIT_SEED,
};
use super::tree_common::{normalize, permute_column_inplace, prepare_training_data, PreparedData};
use crate::core::lgbm::{lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};

/// 前処理済みデータから PFI 重要度を計算する（`metrics::PermutationMetric` からも呼ばれる）。
pub(super) fn compute_from_prepared(data: &PreparedData) -> Option<(Vec<f64>, f64)> {
    let p = data.x_shuffled[0].len();
    let (x_train, x_eval, y_train, y_eval) = data.split();
    let config = LgbmRfConfig {
        num_iterations: PFI_RF_TREES,
        max_depth: PFI_RF_MAX_DEPTH,
        min_data_in_leaf: PFI_RF_MIN_DATA_LEAF,
        seed: PFI_SEED_BASE as i32,
        ..Default::default()
    };
    let booster = train_lgbm_rf(x_train, y_train, &config)?;

    let baseline_mse = lgbm_mse(&booster, x_eval, y_eval)
        .unwrap_or(0.0)
        .max(f64::EPSILON);

    // eval 行列を 1 回だけクローンし、特徴量ごとに列をインプレース置換して MSE を測定する。
    // permute_single_column(p×N_REPEATS 回クローン) と比べてアロケーションを O(1) に削減。
    let mut x_eval_work = x_eval.to_vec();
    let mut importances = vec![0.0f64; p];
    for feature_idx in 0..p {
        let orig_col: Vec<f64> = x_eval_work.iter().map(|r| r[feature_idx]).collect();
        let mut delta_sum = 0.0f64;
        for repeat_idx in 0..PFI_N_REPEATS {
            let seed =
                PFI_SEED_BASE + (feature_idx as u64) * (PFI_N_REPEATS as u64) + (repeat_idx as u64);
            // 各リピートの前に元の列値を復元してから再置換する
            for (i, row) in x_eval_work.iter_mut().enumerate() {
                row[feature_idx] = orig_col[i];
            }
            permute_column_inplace(&mut x_eval_work, feature_idx, seed);
            let permuted_mse = lgbm_mse(&booster, &x_eval_work, y_eval).unwrap_or(baseline_mse);
            delta_sum += (permuted_mse - baseline_mse).max(0.0);
        }
        // 次の特徴量処理前に列を復元する
        for (i, row) in x_eval_work.iter_mut().enumerate() {
            row[feature_idx] = orig_col[i];
        }
        importances[feature_idx] = delta_sum / PFI_N_REPEATS as f64;
    }

    normalize(&mut importances);
    let r_squared = mse_to_r_squared(baseline_mse, y_eval);
    Some((importances, r_squared))
}

/// Permutation Feature Importance via LightGBM Random Forest (n_repeats=5).
/// Returns (importances_normalized, r_squared).
/// importances.sum() ≈ 1.0 when valid data exists.
pub fn compute_permutation_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n == 0 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }
    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }
    match prepare_training_data(x_matrix, y, PFI_MAX_ROWS, PFI_SEED_BASE, PFI_SPLIT_SEED) {
        Some(data) => compute_from_prepared(&data).unwrap_or((vec![0.0; p], 0.0)),
        None => (vec![0.0; p], 0.0),
    }
}
