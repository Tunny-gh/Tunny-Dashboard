use super::super::constants::{
    PFI_MAX_ROWS, PFI_N_REPEATS, PFI_RF_MAX_DEPTH, PFI_RF_MIN_DATA_LEAF, PFI_RF_TREES,
    PFI_SEED_BASE, PFI_SPLIT_SEED,
};
use super::common::{
    normalize, permute_column_inplace, restore_column, run_importances_pipeline, PreparedData,
};
use crate::lgbm::{lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};

/// 前処理済みデータから PFI 重要度を計算する（`metrics::PermutationMetric` からも呼ばれる）。
pub(in crate::sensitivity) fn compute_from_prepared(
    data: &PreparedData,
) -> Option<(Vec<f64>, f64)> {
    let p = data.x_shuffled[0].len();
    let (x_train, x_eval, y_train, y_eval) = data.split();
    let config = LgbmRfConfig {
        num_iterations: PFI_RF_TREES,
        max_depth: PFI_RF_MAX_DEPTH,
        min_data_in_leaf: PFI_RF_MIN_DATA_LEAF,
        seed: PFI_SEED_BASE as i32,
        ..Default::default()
    };
    // モデルは 1 回だけ学習し、全特徴量のパーミュテーション評価で共有する。
    // LightGBM の予測は同一 booster への並行呼び出しが安全でないため、
    // RF-ANOVA と同じく特徴量ループは逐次実行する。
    let booster = train_lgbm_rf(x_train, y_train, &config)?;

    let baseline_mse = lgbm_mse(&booster, x_eval, y_eval)
        .unwrap_or(0.0)
        .max(f64::EPSILON);

    // eval 行列を 1 回だけクローンし、特徴量ごとにインプレース置換→MSE→復元する。
    let mut x_work = x_eval.to_vec();
    let mut importances = vec![0.0; p];
    for feature_idx in 0..p {
        let orig_col: Vec<f64> = x_work.iter().map(|r| r[feature_idx]).collect();
        let mut delta_sum = 0.0f64;
        for repeat_idx in 0..PFI_N_REPEATS {
            let seed =
                PFI_SEED_BASE + (feature_idx as u64) * (PFI_N_REPEATS as u64) + (repeat_idx as u64);
            permute_column_inplace(&mut x_work, feature_idx, seed);
            let permuted_mse = lgbm_mse(&booster, &x_work, y_eval).unwrap_or(baseline_mse);
            delta_sum += (permuted_mse - baseline_mse).max(0.0);
            restore_column(&mut x_work, feature_idx, &orig_col);
        }
        importances[feature_idx] = delta_sum / PFI_N_REPEATS as f64;
    }

    normalize(&mut importances);
    let r_squared = mse_to_r_squared(baseline_mse, y_eval);
    Some((importances, r_squared))
}

pub fn compute_permutation_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    run_importances_pipeline(
        x_matrix,
        y,
        PFI_MAX_ROWS,
        PFI_SEED_BASE,
        PFI_SPLIT_SEED,
        compute_from_prepared,
    )
}
